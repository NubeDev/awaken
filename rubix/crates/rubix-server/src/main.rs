use rubix_query::{HisTier, QueryEngine};
use rubix_server::bus::ZenohBus;
use rubix_server::dispatch::Dispatcher;
use rubix_server::profile::{self, StoreKind};
use rubix_server::store::Store;
use rubix_server::scheduler::Scheduler;
use rubix_server::supervisor::{load_manifests, Supervisor};
use rubix_server::{app, AppState};

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    // Resolve the deployment profile first: it gates which backends boot. The
    // cargo feature decides what is compilable, RUBIX_PROFILE selects among the
    // compiled profiles (STACK-DEISGN.md "Single binary, two profiles").
    let profile = profile::select()?;
    tracing::info!(profile = %profile.kind, "deployment profile selected");

    // The relational store the profile expects. SQLite is the edge default;
    // the cloud profile defaults to Postgres, selected at runtime by a
    // `postgres://` RUBIX_DB url (STACK-DEISGN.md "Postgres (cloud), SQLite
    // (edge)"). The Postgres backend is compiled in only under the cloud
    // feature; selecting it without that feature fails closed in
    // `Store::connect`.
    let default_db = match profile.store {
        StoreKind::Sqlite => "rubix.db",
        StoreKind::Postgres => "postgres://localhost/rubix",
    };
    let db_path = env_or("RUBIX_DB", default_db);
    if profile.store == StoreKind::Postgres && !Store::is_postgres_target(&db_path) {
        tracing::warn!(
            db = %db_path,
            "cloud profile expects a postgres:// RUBIX_DB url; using the given target as a SQLite path"
        );
    }
    let addr = env_or("RUBIX_ADDR", "0.0.0.0:8080");
    let ai_min_priority: u8 = env_or("RUBIX_AI_MIN_PRIORITY", "13")
        .parse()
        .map_err(|e| anyhow::anyhow!("RUBIX_AI_MIN_PRIORITY must be 1..=16: {e}"))?;
    if !(1..=16).contains(&ai_min_priority) {
        anyhow::bail!("RUBIX_AI_MIN_PRIORITY must be 1..=16, got {ai_min_priority}");
    }
    let ai_escalation_floor: u8 = env_or("RUBIX_AI_ESCALATION_FLOOR", "1")
        .parse()
        .map_err(|e| anyhow::anyhow!("RUBIX_AI_ESCALATION_FLOOR must be 1..=16: {e}"))?;
    if !(1..=ai_min_priority).contains(&ai_escalation_floor) {
        anyhow::bail!(
            "RUBIX_AI_ESCALATION_FLOOR must be 1..={ai_min_priority}, got {ai_escalation_floor}"
        );
    }

    let store = Store::connect(&db_path)?;

    let mut supervisor: Option<Supervisor> = None;
    let bus = if env_or("RUBIX_ZENOH", "1") == "0" {
        tracing::info!("zenoh disabled (RUBIX_ZENOH=0); HTTP-only mode");
        None
    } else {
        let bus = ZenohBus::open(store.clone()).await?;
        bus.serve().await?;
        tracing::info!("zenoh data plane up: cur pub/sub, write + his queryables");

        // Spawn manifest-described drivers on the same mesh. The supervisor
        // watches their liveliness tokens, so it must share the bus session.
        // Only profiles that supervise on-box drivers (edge) launch it; the
        // cloud profile leaves driver supervision to edge stations.
        if !profile.supervise_drivers {
            tracing::info!(profile = %profile.kind, "driver supervision off for this profile");
        } else {
            let drivers_path = env_or("RUBIX_DRIVERS", "drivers.json");
            let manifests = load_manifests(std::path::Path::new(&drivers_path))?;
            if manifests.is_empty() {
                tracing::info!(path = %drivers_path, "no driver manifests; supervisor idle");
            } else {
                let names: Vec<_> = manifests.iter().map(|m| m.identity.name.clone()).collect();
                supervisor = Some(Supervisor::launch(bus.session_clone(), manifests)?);
                tracing::info!(drivers = ?names, "driver supervisor launched");
            }
        }
        Some(bus)
    };

    // Open the Parquet `his` cold tier when configured. When present, `his`
    // queries union the SQLite recent tier with the Parquet partitions and
    // `/his/flush` ages rows out of SQLite. Absent, `his` stays SQLite-only.
    let his_tier = match std::env::var("RUBIX_HIS_PARQUET") {
        Ok(root) if !root.is_empty() => {
            let tier = HisTier::open_local(std::path::Path::new(&root))?;
            tracing::info!(root = %root, "his parquet cold tier up: POST /api/v1/his/flush");
            Some(tier)
        }
        _ => {
            tracing::info!("his parquet tier disabled (RUBIX_HIS_PARQUET unset); SQLite-only his");
            None
        }
    };

    let query = if env_or("RUBIX_QUERY", "1") == "0" {
        tracing::info!("query engine disabled (RUBIX_QUERY=0)");
        None
    } else if Store::is_postgres_target(&db_path) {
        // The DataFusion surface is SQLite-backed; Postgres federation is a
        // separate workstream. Under a Postgres store the query engine stays
        // off rather than reading a stale SQLite file.
        tracing::info!("query engine off: datafusion has no postgres provider yet");
        None
    } else {
        let mut engine = QueryEngine::open(std::path::Path::new(&db_path)).await?;
        if let Some(tier) = &his_tier {
            engine = engine.with_his_tier(tier.clone());
        }
        tracing::info!("datafusion query surface up: POST /api/v1/query");
        Some(engine)
    };

    let mut state = AppState {
        profile,
        store,
        bus,
        query,
        his_tier,
        agent: None,
        ai_min_priority,
        ai_escalation_floor,
    };

    // Embed the awaken agent over the BMS tools when enabled. The genai
    // provider reads its API key from env at run time, so this only requires a
    // key when a chat turn actually calls the model.
    if env_or("RUBIX_AI", "0") == "1" {
        let provider = env_or("RUBIX_AI_PROVIDER", "openai");
        let model_id = env_or("RUBIX_AI_MODEL_ID", "gpt-4o-mini");
        let upstream = env_or("RUBIX_AI_MODEL", &model_id);
        let max_rounds: usize = env_or("RUBIX_AI_MAX_ROUNDS", "8")
            .parse()
            .map_err(|e| anyhow::anyhow!("RUBIX_AI_MAX_ROUNDS must be a positive integer: {e}"))?;
        let runtime = rubix_server::agent::build_runtime(
            &state, &provider, &model_id, &upstream, max_rounds,
        )?;
        state.agent = Some(std::sync::Arc::new(runtime));
        tracing::info!(
            provider = %provider,
            model = %model_id,
            "embedded agent up: POST /api/v1/agent/chat"
        );
    } else {
        tracing::info!("agent disabled (RUBIX_AI != 1)");
    }

    // Launch the board scheduler: fire stored boards on their interval or cur
    // subscription. Reads the current set of scheduled boards from the store at
    // boot; a board added later is picked up on the next restart.
    let scheduler = if env_or("RUBIX_SCHEDULER", "1") == "0" {
        tracing::info!("board scheduler disabled (RUBIX_SCHEDULER=0)");
        None
    } else {
        let boards = state.store.latest_boards()?;
        let scheduled: Vec<_> = boards.into_iter().filter(|b| b.is_scheduled()).collect();
        if scheduled.is_empty() {
            tracing::info!("no scheduled boards; scheduler idle");
            None
        } else {
            let scheduler = Scheduler::launch(
                state.store.clone(),
                state.bus.clone(),
                state.agent.clone(),
                scheduled,
            );
            tracing::info!(boards = scheduler.active(), "board scheduler running");
            Some(scheduler)
        }
    };

    // Launch inbound spark dispatch: subscribe to spark findings on the bus and
    // activate the agent per finding (a job, not a chat). Needs both the bus
    // (transport) and the agent runtime; without either, dispatch is off.
    let dispatcher = if env_or("RUBIX_AI_DISPATCH", "1") == "0" {
        tracing::info!("spark dispatch disabled (RUBIX_AI_DISPATCH=0)");
        None
    } else {
        match (state.bus.clone(), state.agent.clone()) {
            (Some(bus), Some(runtime)) => {
                Some(Dispatcher::launch(bus, runtime, state.store.clone()))
            }
            _ => {
                tracing::info!("spark dispatch idle: needs both zenoh and the agent");
                None
            }
        }
    };

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, db = %db_path, "rubix server listening");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Stop the spark dispatcher first so no new agent runs start during teardown.
    if let Some(dispatcher) = dispatcher {
        tracing::info!("stopping spark dispatcher");
        dispatcher.shutdown().await;
    }

    // Stop the board scheduler so its loops drain before drivers go down.
    if let Some(scheduler) = scheduler {
        tracing::info!("stopping board scheduler");
        scheduler.shutdown().await;
    }

    // Stop supervised drivers cleanly so liveliness tokens clear before exit.
    if let Some(supervisor) = supervisor {
        tracing::info!("stopping driver supervisor");
        supervisor.shutdown().await;
    }
    Ok(())
}

/// Resolve on Ctrl-C (SIGINT). Drives axum's graceful shutdown so the
/// supervisor can stop drivers on the way out.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

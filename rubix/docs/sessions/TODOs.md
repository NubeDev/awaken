# Rubix Build — Blockers & Follow-ups

Append-only log of things an unattended session could NOT do properly and refused to hack. The human
resolves an entry, then strikes it through (`~~...~~`) or deletes it; the loop resets the
corresponding ⛔ row to ⬜ on its next wake.

## Format

```
### <utc-date> — <WS-xx> — <one-line title>
- **What's blocked:** ...
- **Why (the ambiguity / missing dep / guardrail conflict):** ...
- **What the human must decide/provide:** ...
- **Committed so far:** <commit sha or "nothing — clean working tree">
```

---

~~### 2026-06-12 — (pre-existing, surfaced by WS-01) — `chat_reports_awaiting_approval_when_a_write_suspends` hangs~~
**RESOLVED by WS-02 (92f2d912):** the full-local backend blocks the agent loop after suspending,
waiting for an operator decision on a live channel this architecture never feeds. `run_and_persist`
now captures the suspend from the event stream and `cancel_by_run_id`s the loop to release it.
`cargo test --workspace` terminates; 125 tests green.

- **What's blocked:** A clean `cargo test --workspace` run does not terminate — the test
  `api_tests::agent::chat_reports_awaiting_approval_when_a_write_suspends`
  (`rubix-server/tests/api_tests/agent.rs`) runs past 200s and is killed by timeout.
- **Why:** Confirmed pre-existing, NOT caused by WS-01: reproduced on a clean HEAD with all WS-01
  changes stashed (source + tests), the test still hangs >200s. It exercises the HITL escalation-band
  suspend path (`write_point` at priority 5 → suspended run → `awaiting_approval`); the scripted-agent
  suspend path in awaken-runtime 0.6 appears to block. WS-01 touches none of this path (only the
  `agent_call` board node, `request_agent_blocking`, and `BoardGraph::run` settle loop), and all WS-01
  tests plus `rubix-server` lib/`agent_call` integration tests pass.
- **What the human must decide/provide:** whether the suspend/await-approval run path has a missing
  resume/terminate signal (likely lands with WS-02's persistent run store), or the test needs a bound.
- **Committed so far:** WS-01 work committed on `rubix-gaps`; this entry is a follow-up, not a WS-01
  blocker (WS-01 is Done for its own scope and green).

---

~~### 2026-06-12 — WS-05 — DataFusion has no Postgres provider (federation + `/query` under cloud)~~
**RESOLVED (838081b2):** the DataFusion surface now federates the canonical tables from Postgres via a
`datafusion-table-providers` connector behind the `cloud` feature (`QueryEngine::open_postgres` /
`Source::Postgres`); `main.rs` re-enables `/query` under a Postgres store instead of disabling it, and
the tenant-scoped surface (`scoped_query`, from the WS-07 follow-up) works over Postgres too.
`datafusion-table-providers 0.11` aligns with the in-tree DataFusion 53. Tests: `pg_query.rs`
(`RUBIX_TEST_PG`-gated) proves unscoped federation sees all tenants and scoped confines to one;
edge stays SQLite-only; clippy + tests green on edge and `--features cloud`.

The **users/teams/config tables are intentionally NOT added here.** An audit found each would have zero
readers today (identity is delegated to the OIDC issuer; `scope.rs` explicitly does not own the
team→site mapping and `covers_resource` ignores team; all config is env-driven at boot with no runtime
lookup). Adding bare unused tables violates the repo's "no placeholder implementations" rule, so each
remains a dedicated follow-up gated on a real consumer (user provisioning, a team→site admin surface, a
per-tenant settings surface). The query subsystem the WS-05/WS-07 follow-ups actually required is done.

- **What's blocked:** Under a Postgres store target the DataFusion `/query` SQL surface is disabled
  (`main.rs`: "datafusion has no postgres provider yet"), and the cloud relational tables
  (users/teams/config — STATUS.md "Postgres federation") are not implemented on the Postgres backend.
- **Why (the ambiguity / missing dep / guardrail conflict):** The WS-05 store contract (sites/equips/
  points/his/sparks/boards/widgets/runs) is fully ported to Postgres and passes the shared suite, but
  the query engine reads through a DataFusion `TableProvider` that only has a SQLite/Parquet
  implementation. A Postgres `TableProvider` (or a federation layer) is a distinct subsystem outside
  WS-05's "store backend" scope and was not specced here.
- **What the human must decide/provide:** whether the cloud `/query` surface federates to Postgres via
  a DataFusion `TableProvider`/connector, or runs SQL natively in Postgres; and where users/teams/config
  tables and their DDL/migrations live. Likely its own workstream.
- **Committed so far:** WS-05 store backend committed on `rubix-gaps` (200269f6); WS-05 is Done for its
  own scope (store contract green on SQLite and Postgres). This entry is a follow-up, not a WS-05 blocker.

---

~~### 2026-06-12 — WS-07 — tenant-scoped runs withhold the `query` SQL tool (fail-closed)~~
**RESOLVED (fd37cad0, ba37da2d):** a tenant-filtered DataFusion session (`QueryEngine::scoped_query`
/ `QueryScope`) now confines a scoped run's ad-hoc SQL to its own `{org}/{site}`, so
`build_tools_scoped` hands a scoped run the `query` tool again (filtered) instead of withholding it.
The fail-closed fallback remains for a scope that cannot map to a `QueryScope`.

- **What's blocked:** A tenant-scoped agent run (chat with a sited principal, or a dispatched spark)
  gets the BMS read/write/his/board/widget tools confined to its `{org}/{site}`, but the SQL `query`
  tool is **omitted** from the scoped tool set (`build_tools_scoped` returns no query tool when
  `scope.is_some()`). So a scoped run cannot run ad-hoc SQL at all, rather than running SQL confined
  to its tenant.
- **Why (the ambiguity / missing dep / guardrail conflict):** The `query` tool runs free-form SQL
  through the DataFusion engine over the canonical tables; there is no tenant-aware view/row-filter
  subsystem to safely scope that SQL to one `{org}/{site}` without a query rewriter or per-tenant
  views. Shipping an unscoped query tool inside a scoped run would be a cross-tenant read hole, so it
  is withheld (fail-closed) until a tenant-aware query surface exists. Unscoped runs (no principal
  site, no spark tenant) keep the full tool set including `query`.
- **What the human must decide/provide:** whether scoped runs get a tenant-filtered query surface
  (DataFusion views keyed by `{org}/{site}`, or a SQL rewriter that injects the tenant predicate),
  or query stays operator-only and out of scoped agent reach. Couples with WS-05's Postgres
  federation follow-up (the query engine subsystem).
- **Committed so far:** WS-07 committed on `rubix-gaps` (c96f6996, 2bccee07, 385f0e1d); WS-07 is Done
  for its own scope (point/board/widget tools enforce the tenant boundary; cross-tenant denial is
  tested on both paths). This entry is a follow-up, not a WS-07 blocker.

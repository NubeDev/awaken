//! The executor: one validation+caps path, two entry points over it.
//!
//! Raw-SQL execution (operator-authored widgets/spark nodes) and named-query
//! invocation (the AI tier) both flow through [`Executor::execute`] — the same
//! single-statement check, the same bound parameters, the same caps (docs "Both
//! tiers run through the same executor and the same caps"). The executor is
//! generic over [`SqlBackend`] so the whole path is unit-testable against a fake
//! backend with no live database.

use super::cap;
use crate::backend::{ResultSet, SqlBackend};
use crate::caps::Caps;
use crate::error::{DatasourceError, DatasourceResult};
use crate::manifest::NamedQuery;
use crate::statement::{ensure_single_statement, Params};

/// Runs reads against one datasource's backend under its caps and named queries.
/// Borrowed from the registry, which owns the backend and credentials.
pub struct Executor<'a, B: SqlBackend> {
    pub(crate) datasource: &'a str,
    pub(crate) backend: &'a B,
    pub(crate) caps: Caps,
    pub(crate) named: &'a [NamedQuery],
}

impl<'a, B: SqlBackend> Executor<'a, B> {
    /// Run operator-authored native SQL with bound parameters. Rejects
    /// multi-statement input, binds params positionally, applies the caps, and
    /// returns a possibly-truncated [`ResultSet`] (the lenient/dashboard path).
    /// Callers wanting the strict path wrap the result with [`Self::strict`].
    pub async fn execute(&self, sql: &str, params: &Params) -> DatasourceResult<ResultSet> {
        let stmt = ensure_single_statement(sql)?;
        // Fetch one past the row cap so a breach is detectable from the pull
        // itself, while still bounding what the backend reads into memory.
        let fetch_bound = self.caps.max_rows.map(|n| n + 1);
        let raw = self
            .backend
            .run(stmt, params, self.caps.max_duration, fetch_bound)
            .await?;
        Ok(cap::apply(raw, &self.caps))
    }

    /// Invoke an operator-registered named query by name with bound parameters.
    /// The SQL is never supplied by the caller — only the name and params —
    /// keeping the AI tier on operator-authored SQL (docs "AI"). Validates the
    /// parameter arity against the declared `param_count`.
    pub async fn invoke_named(
        &self,
        name: &str,
        params: &Params,
    ) -> DatasourceResult<ResultSet> {
        let query = self
            .named
            .iter()
            .find(|q| q.name == name)
            .ok_or_else(|| DatasourceError::UnknownQuery {
                datasource: self.datasource.to_string(),
                query: name.to_string(),
            })?;
        if params.len() != query.param_count {
            return Err(DatasourceError::ParamCount {
                query: name.to_string(),
                expected: query.param_count,
                got: params.len(),
            });
        }
        self.execute(&query.sql, params).await
    }

    /// Convert a result into the strict (spark) outcome: a cap breach becomes an
    /// error rather than a truncated result (docs "Truncation on the spark
    /// path"). Call after [`Self::execute`]/[`Self::invoke_named`].
    pub fn strict(&self, result: ResultSet) -> DatasourceResult<ResultSet> {
        cap::into_strict(self.datasource, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Column, RawResult};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;
    use std::time::Duration;

    /// What the executor handed the backend on the last `run` call.
    #[derive(Clone)]
    struct Seen {
        sql: String,
        params: Params,
        wall_clock: Option<Duration>,
        fetch_bound: Option<u64>,
    }

    /// Records what the executor handed the backend and returns canned rows.
    #[derive(Default)]
    struct FakeBackend {
        seen: Mutex<Option<Seen>>,
        n_rows: usize,
    }

    #[async_trait]
    impl SqlBackend for FakeBackend {
        async fn run(
            &self,
            sql: &str,
            params: &Params,
            wall_clock: Option<Duration>,
            fetch_bound: Option<u64>,
        ) -> DatasourceResult<RawResult> {
            *self.seen.lock().unwrap() = Some(Seen {
                sql: sql.to_string(),
                params: params.clone(),
                wall_clock,
                fetch_bound,
            });
            Ok(RawResult {
                columns: vec![Column {
                    name: "n".into(),
                    type_name: "json".into(),
                }],
                rows: (0..self.n_rows).map(|i| vec![json!(i)]).collect(),
            })
        }

        async fn introspect(&self) -> DatasourceResult<RawResult> {
            Ok(RawResult {
                columns: vec![],
                rows: vec![],
            })
        }
    }

    fn exec<'a>(b: &'a FakeBackend, caps: Caps, named: &'a [NamedQuery]) -> Executor<'a, FakeBackend> {
        Executor {
            datasource: "h",
            backend: b,
            caps,
            named,
        }
    }

    #[tokio::test]
    async fn execute_passes_validated_sql_params_and_bounds() {
        let b = FakeBackend {
            n_rows: 2,
            ..Default::default()
        };
        let caps = Caps::new(10, 1 << 20, Duration::from_secs(5));
        let rs = exec(&b, caps, &[])
            .execute("SELECT 1 ;", &vec![crate::statement::Param::Int(7)])
            .await
            .unwrap();
        assert_eq!(rs.rows.len(), 2);
        let seen = b.seen.lock().unwrap().clone().unwrap();
        assert_eq!(seen.sql, "SELECT 1 ;", "trimmed single statement forwarded");
        assert_eq!(seen.params, vec![crate::statement::Param::Int(7)]);
        assert_eq!(seen.wall_clock, Some(Duration::from_secs(5)), "wall-clock forwarded");
        assert_eq!(seen.fetch_bound, Some(11), "fetch bound is row cap + 1");
    }

    #[tokio::test]
    async fn execute_rejects_multi_statement_before_backend() {
        let b = FakeBackend::default();
        let err = exec(&b, Caps::unbounded(), &[])
            .execute("SELECT 1; DROP TABLE t", &vec![])
            .await
            .unwrap_err();
        assert!(matches!(err, DatasourceError::MultiStatement));
        assert!(b.seen.lock().unwrap().is_none(), "backend never called");
    }

    #[tokio::test]
    async fn invoke_named_resolves_and_runs_operator_sql() {
        let b = FakeBackend {
            n_rows: 1,
            ..Default::default()
        };
        let named = vec![NamedQuery {
            name: "daily".into(),
            sql: "SELECT $1::int".into(),
            param_count: 1,
        }];
        let rs = exec(&b, Caps::unbounded(), &named)
            .invoke_named("daily", &vec![crate::statement::Param::Int(3)])
            .await
            .unwrap();
        assert_eq!(rs.rows.len(), 1);
        assert_eq!(b.seen.lock().unwrap().clone().unwrap().sql, "SELECT $1::int");
    }

    #[tokio::test]
    async fn invoke_named_rejects_unknown_query() {
        let b = FakeBackend::default();
        let err = exec(&b, Caps::unbounded(), &[])
            .invoke_named("nope", &vec![])
            .await
            .unwrap_err();
        assert!(matches!(err, DatasourceError::UnknownQuery { .. }));
    }

    #[tokio::test]
    async fn invoke_named_rejects_wrong_param_count() {
        let b = FakeBackend::default();
        let named = vec![NamedQuery {
            name: "daily".into(),
            sql: "SELECT $1".into(),
            param_count: 1,
        }];
        let err = exec(&b, Caps::unbounded(), &named)
            .invoke_named("daily", &vec![])
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DatasourceError::ParamCount {
                expected: 1,
                got: 0,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn strict_path_turns_breach_into_error() {
        let b = FakeBackend {
            n_rows: 5,
            ..Default::default()
        };
        let e = exec(&b, Caps::rows(3), &[]);
        let rs = e.execute("SELECT 1", &vec![]).await.unwrap();
        assert!(rs.breached);
        assert!(matches!(
            e.strict(rs).unwrap_err(),
            DatasourceError::CapBreached { .. }
        ));
    }
}

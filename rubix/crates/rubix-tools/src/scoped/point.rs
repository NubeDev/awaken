//! [`PointAccess`] decorator that confines every keyexpr-addressed call to a
//! [`TenantScope`]. A tenant-scoped run reads, commands, and queries history only
//! within its `{org}/{site}`; a call that names a point outside the scope is
//! refused here, at the tool/board boundary, before the inner access (the store)
//! is ever touched. STACK-DEISGN.md "Tenancy: org/site hierarchy mirrors into
//! awaken `ScopeId`".

use std::sync::Arc;

use async_trait::async_trait;
use rubix_core::{HisSample, PointValue};
use rubix_flow::{AgentOutcome, AgentRequest, FlowAccessError, PointAccess, SparkDraft};

use crate::scope::TenantScope;

/// Wraps a [`PointAccess`], denying any read/write/history call whose keyexpr
/// falls outside `scope`. Spark emission and agent requests delegate unchanged —
/// those carry no point keyexpr to gate, and a board run through this access
/// already has its point calls confined.
pub struct ScopedPointAccess {
    inner: Arc<dyn PointAccess>,
    scope: TenantScope,
}

impl ScopedPointAccess {
    /// Confine `inner` to `scope`.
    pub fn new(inner: Arc<dyn PointAccess>, scope: TenantScope) -> Self {
        Self { inner, scope }
    }

    /// Authorize a keyexpr against the scope, or return the tenant-denial error.
    fn guard(&self, keyexpr: &str) -> Result<(), FlowAccessError> {
        if self.scope.covers(keyexpr) {
            Ok(())
        } else {
            Err(FlowAccessError::Denied(format!(
                "point `{keyexpr}` is outside the run's tenant scope `{}`",
                self.scope.scope_id()
            )))
        }
    }
}

#[async_trait]
impl PointAccess for ScopedPointAccess {
    async fn read_point(&self, keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        self.guard(keyexpr)?;
        self.inner.read_point(keyexpr).await
    }

    async fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        self.guard(keyexpr)?;
        self.inner.write_point(keyexpr, priority, value).await
    }

    async fn query_his(
        &self,
        keyexpr: &str,
        limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        self.guard(keyexpr)?;
        self.inner.query_his(keyexpr, limit).await
    }

    async fn emit_spark(&self, draft: SparkDraft) -> Result<(), FlowAccessError> {
        self.inner.emit_spark(draft).await
    }

    async fn request_agent(&self, request: AgentRequest) -> Result<(), FlowAccessError> {
        self.inner.request_agent(request).await
    }

    async fn request_agent_awaited(
        &self,
        request: AgentRequest,
    ) -> Result<AgentOutcome, FlowAccessError> {
        self.inner.request_agent_awaited(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records the keyexprs the inner access was actually asked for, so a test
    /// can prove a denied call never reached it.
    #[derive(Default)]
    struct RecordingAccess {
        seen: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl PointAccess for RecordingAccess {
        async fn read_point(&self, keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
            self.seen.lock().unwrap().push(keyexpr.to_string());
            Ok(Some(PointValue::Number(1.0)))
        }

        async fn write_point(
            &self,
            keyexpr: &str,
            _priority: u8,
            _value: PointValue,
        ) -> Result<Option<PointValue>, FlowAccessError> {
            self.seen.lock().unwrap().push(keyexpr.to_string());
            Ok(None)
        }

        async fn query_his(
            &self,
            keyexpr: &str,
            _limit: usize,
        ) -> Result<Vec<HisSample>, FlowAccessError> {
            self.seen.lock().unwrap().push(keyexpr.to_string());
            Ok(Vec::new())
        }
    }

    fn scoped() -> (Arc<RecordingAccess>, ScopedPointAccess) {
        let inner = Arc::new(RecordingAccess::default());
        let scoped = ScopedPointAccess::new(inner.clone(), TenantScope::new("nube", "hq"));
        (inner, scoped)
    }

    #[tokio::test]
    async fn in_scope_calls_reach_the_inner_access() {
        let (inner, access) = scoped();
        assert!(access.read_point("nube/hq/ahu-3/fan").await.is_ok());
        assert!(access
            .write_point("nube/hq/ahu-3/fan", 16, PointValue::Bool(true))
            .await
            .is_ok());
        assert!(access.query_his("nube/hq/ahu-3/fan", 10).await.is_ok());
        assert_eq!(inner.seen.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn out_of_scope_calls_are_refused_before_the_inner_access() {
        let (inner, access) = scoped();
        for key in ["acme/hq/ahu-3/fan", "nube/dc1/ahu-3/fan", "nube/hq2/ahu/fan"] {
            assert!(
                access.read_point(key).await.is_err(),
                "read {key} must be denied"
            );
            assert!(
                access
                    .write_point(key, 16, PointValue::Bool(true))
                    .await
                    .is_err(),
                "write {key} must be denied"
            );
            assert!(
                access.query_his(key, 10).await.is_err(),
                "his {key} must be denied"
            );
        }
        // No denied call ever touched the inner store.
        assert!(inner.seen.lock().unwrap().is_empty());
    }
}

//! A zenoh session confined to a driver's granted [`CapabilitySet`]. Every
//! publish and subscribe is authorized against the grant before it touches the
//! bus, so an out-of-scope operation fails locally with a named denial instead
//! of being silently dropped by the bus-side responder. This is the driver-side
//! half of the capability enforcement STACK-DEISGN.md describes ("each driver
//! gets a scoped zenoh session limited to its granted keyexpr prefixes").

use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::query::Reply;
use zenoh::sample::Sample;
use zenoh::Session;

use rubix_driver::{CapabilitySet, DriverError};

/// Wraps a zenoh [`Session`] and refuses any publish/subscribe outside the
/// driver's [`CapabilitySet`]. The wrapped session is otherwise unchanged — the
/// confinement is purely additive gating in front of `put`/`declare_subscriber`.
pub struct ScopedSession {
    driver: String,
    caps: CapabilitySet,
    session: Session,
}

impl ScopedSession {
    /// Confine `session` to `caps` under the identity `driver` (named in
    /// denials). The caps are assumed already validated by the caller
    /// (`CapabilitySet::validate`); confinement is on the access check, not the
    /// prefix shape.
    pub fn new(driver: impl Into<String>, caps: CapabilitySet, session: Session) -> Self {
        Self {
            driver: driver.into(),
            caps,
            session,
        }
    }

    /// Publish `payload` on `key`, or return [`DriverError::Denied`] if the
    /// grant does not permit publishing there — without ever touching the bus.
    pub async fn put(&self, key: &str, payload: Vec<u8>) -> Result<(), DriverError> {
        self.caps.authorize_publish(&self.driver, key)?;
        self.session
            .put(key, payload)
            .await
            .map_err(|e| DriverError::InvalidManifest(format!("publish {key}: {e}")))
    }

    /// Declare a subscriber on `key`, or return [`DriverError::Denied`] if the
    /// grant does not permit subscribing there — without touching the bus.
    ///
    /// The sim is publish-only, so its binary never calls this; it is the
    /// subscribe half of the same confinement a command-consuming driver uses,
    /// and is covered by the wrapper's tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn declare_subscriber(
        &self,
        key: &str,
    ) -> Result<Subscriber<FifoChannelHandler<Sample>>, DriverError> {
        self.caps.authorize_subscribe(&self.driver, key)?;
        self.session
            .declare_subscriber(key)
            .await
            .map_err(|e| DriverError::InvalidManifest(format!("subscribe {key}: {e}")))
    }

    /// Issue a query (`get`) on `key` with `payload`, returning the reply stream,
    /// or [`DriverError::Denied`] if the grant does not permit subscribing/
    /// querying there. A `write` command is a query whose reply is the responder's
    /// ack, so this is the publish/subscribe-direction gate for the reliable write
    /// path (the responder side `covers` is enforced server-side).
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn get(
        &self,
        key: &str,
        payload: Vec<u8>,
        timeout: std::time::Duration,
    ) -> Result<FifoChannelHandler<Reply>, DriverError> {
        self.caps.authorize_subscribe(&self.driver, key)?;
        self.session
            .get(key)
            .payload(payload)
            .timeout(timeout)
            .await
            .map_err(|e| DriverError::InvalidManifest(format!("query {key}: {e}")))
    }

    /// The driver's name, as used in denial errors.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn driver(&self) -> &str {
        &self.driver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubix_driver::{Access, Capability};

    fn caps() -> CapabilitySet {
        CapabilitySet {
            grants: vec![Capability {
                prefix: "nube/hq/ahu-3".into(),
                access: Access::Publish,
            }],
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn publish_outside_grant_is_refused_before_the_bus() {
        let session = zenoh::open(zenoh::Config::default()).await.expect("session");
        let scoped = ScopedSession::new("sim", caps(), session);

        let err = scoped
            .put("nube/hq/ahu-9/fan/cur", b"21.0".to_vec())
            .await
            .expect_err("out-of-grant publish must be denied");
        assert_eq!(
            err,
            DriverError::Denied {
                driver: "sim".into(),
                action: "publish",
                key: "nube/hq/ahu-9/fan/cur".into(),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn subscribe_outside_grant_is_refused_before_the_bus() {
        let session = zenoh::open(zenoh::Config::default()).await.expect("session");
        let scoped = ScopedSession::new("sim", caps(), session);

        let err = scoped
            .declare_subscriber("nube/hq/ahu-3/fan/cur")
            .await
            .expect_err("publish-only grant must deny subscribe");
        assert_eq!(
            err,
            DriverError::Denied {
                driver: "sim".into(),
                action: "subscribe",
                key: "nube/hq/ahu-3/fan/cur".into(),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn publish_within_grant_is_allowed() {
        let session = zenoh::open(zenoh::Config::default()).await.expect("session");
        let scoped = ScopedSession::new("sim", caps(), session);

        assert_eq!(scoped.driver(), "sim");
        scoped
            .put("nube/hq/ahu-3/fan/cur", b"21.0".to_vec())
            .await
            .expect("in-grant publish is allowed");
    }
}

//! Reliable `write` sender: drains a bounded [`ReliableQueue`] of [`PendingWrite`]
//! commands over the [`ScopedSession`], retrying each until the responder acks
//! application or the retry budget is exhausted — at which point it gives up with
//! [`DriverError::AckTimeout`] (no infinite retry, no silent loss). This is the
//! data-plane counterpart to the latest-wins `cur` path in `simulate`: `write`
//! is reliable, per STACK-DEISGN.md "Ack/backpressure" and
//! `docs/sessions/WS-10.md`.
//!
//! The sim binary itself is publish-only and never enqueues a write, but this is
//! the reliable-write half of the driver contract a command-consuming driver
//! (BACnet/Modbus) uses; it is exercised end-to-end by the crate's tests.

#![cfg_attr(not(test), allow(dead_code))]

use std::time::Duration;

use rubix_driver::{DriverError, PendingWrite, ReliableQueue, WriteAck, WriteAction};

use crate::scoped::ScopedSession;

/// A single write command as it travels on the `write` channel: a value plus the
/// priority-array slot it targets. Matches the server's `WriteCommand` shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WriteCommand {
    pub value: serde_json::Value,
    pub priority: u8,
}

/// Drain `queue` head-to-tail, sending and retrying each pending write until its
/// ack or give-up. Stops at the first command that gives up, returning its
/// [`DriverError::AckTimeout`] so the failure surfaces to the caller as a spark/
/// error rather than being swallowed. `retry_gap` is the wait between attempts.
///
/// On success every command is acked and removed; the queue is empty on `Ok`.
pub async fn drain_writes(
    session: &ScopedSession,
    queue: &mut ReliableQueue<PendingWrite<WriteCommand>>,
    ack_timeout: Duration,
    retry_gap: Duration,
) -> Result<(), DriverError> {
    while let Some(pending) = queue.front_mut() {
        match send_until_ack(session, pending, ack_timeout, retry_gap).await {
            Ok(()) => {
                queue.ack_front();
            }
            Err(e) => {
                // Give-up: drop the dead command so the queue can make progress,
                // then surface the failure. The command is not silently lost —
                // the error names the key and attempt count.
                queue.ack_front();
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Send one pending write, retrying until an applied ack or give-up. A negative
/// ack (responder answered but did not apply) is a definitive outcome surfaced
/// as its own error carrying the reason; a *missing* ack is a retry case.
async fn send_until_ack(
    session: &ScopedSession,
    pending: &mut PendingWrite<WriteCommand>,
    ack_timeout: Duration,
    retry_gap: Duration,
) -> Result<(), DriverError> {
    let key = pending.key().to_string();
    let payload = serde_json::to_vec(pending.command())
        .map_err(|e| DriverError::InvalidManifest(format!("encode write {key}: {e}")))?;
    loop {
        let action = pending.record_attempt();
        if let Some(ack) = attempt(session, &key, payload.clone(), ack_timeout).await {
            return resolve_ack(&key, ack);
        }
        // No ack this attempt. Give up if the budget is spent, else wait + retry.
        if action == WriteAction::GiveUp {
            return Err(pending.give_up_error());
        }
        tokio::time::sleep(retry_gap).await;
    }
}

/// One attempt: query the `write` keyexpr and decode the first reply as a
/// [`WriteAck`]. `None` means no usable reply (timeout/transport) — a retry case.
async fn attempt(
    session: &ScopedSession,
    key: &str,
    payload: Vec<u8>,
    timeout: Duration,
) -> Option<WriteAck> {
    let replies = session.get(key, payload, timeout).await.ok()?;
    let reply = replies.recv_async().await.ok()?;
    let sample = reply.result().ok()?;
    serde_json::from_slice::<WriteAck>(&sample.payload().to_bytes()).ok()
}

/// Map an ack to an outcome: applied → success; rejected → a named error so the
/// caller learns *why* (e.g. a higher-priority override) without retrying — a
/// rejection is definitive, not a timeout.
fn resolve_ack(key: &str, ack: WriteAck) -> Result<(), DriverError> {
    if ack.applied {
        Ok(())
    } else {
        Err(DriverError::InvalidManifest(format!(
            "write to {key} rejected: {}",
            ack.reason.unwrap_or_else(|| "no reason given".into())
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use futures::StreamExt;
    use rubix_driver::{Access, Capability, CapabilitySet};
    use zenoh::Session;

    use super::*;
    use crate::scoped::ScopedSession;

    fn caps(prefix: &str) -> CapabilitySet {
        CapabilitySet {
            grants: vec![Capability {
                prefix: prefix.into(),
                access: Access::All,
            }],
        }
    }

    fn command(priority: u8) -> WriteCommand {
        WriteCommand {
            value: serde_json::json!(21.0),
            priority,
        }
    }

    /// Spawn a queryable on `key` that replies `WriteAck::applied` only once it
    /// has seen `ack_after` requests; earlier requests get no reply (a timeout
    /// the sender retries). Returns the session (kept alive) and the seen counter.
    async fn responder_acks_after(
        session: Session,
        key: &str,
        ack_after: u32,
    ) -> Arc<AtomicU32> {
        let queryable = session
            .declare_queryable(key)
            .await
            .expect("declare responder");
        let seen = Arc::new(AtomicU32::new(0));
        let seen_task = seen.clone();
        tokio::spawn(async move {
            let mut stream = queryable.stream();
            while let Some(query) = stream.next().await {
                let n = seen_task.fetch_add(1, Ordering::SeqCst) + 1;
                if n >= ack_after {
                    let body = serde_json::to_vec(&WriteAck::applied()).unwrap();
                    let _ = query.reply(query.key_expr().clone(), body).await;
                }
                // else: stay silent so the sender's attempt times out and retries.
            }
            // hold the session for the queryable's lifetime
            drop(session);
        });
        seen
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn write_is_retried_then_acked() {
        let key = "nube/hq/ahu-3/temp/write";
        let resp = zenoh::open(zenoh::Config::default()).await.unwrap();
        let seen = responder_acks_after(resp, key, 3).await;

        let sender = zenoh::open(zenoh::Config::default()).await.unwrap();
        let scoped = ScopedSession::new("bacnet", caps("nube/hq/ahu-3"), sender);
        let mut queue = ReliableQueue::new(key, 8);
        queue
            .enqueue(PendingWrite::new(key, command(8), 5))
            .unwrap();

        drain_writes(
            &scoped,
            &mut queue,
            Duration::from_millis(500),
            Duration::from_millis(50),
        )
        .await
        .expect("retried write is eventually acked");
        assert!(queue.is_empty(), "acked write is removed");
        assert!(seen.load(Ordering::SeqCst) >= 3, "responder saw retries");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn write_gives_up_without_ack() {
        let key = "nube/hq/ahu-4/temp/write";
        let resp = zenoh::open(zenoh::Config::default()).await.unwrap();
        // ack_after far beyond the retry budget -> never acked within budget.
        let _seen = responder_acks_after(resp, key, 999).await;

        let sender = zenoh::open(zenoh::Config::default()).await.unwrap();
        let scoped = ScopedSession::new("bacnet", caps("nube/hq/ahu-4"), sender);
        let mut queue = ReliableQueue::new(key, 8);
        queue
            .enqueue(PendingWrite::new(key, command(8), 3))
            .unwrap();

        let err = drain_writes(
            &scoped,
            &mut queue,
            Duration::from_millis(300),
            Duration::from_millis(20),
        )
        .await
        .expect_err("unacked write gives up");
        assert_eq!(
            err,
            DriverError::AckTimeout {
                key: key.into(),
                attempts: 3,
            }
        );
        // The dead command is removed so the queue can make progress.
        assert!(queue.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn saturated_reliable_queue_surfaces_error() {
        let key = "nube/hq/ahu-5/temp/write";
        let mut queue: ReliableQueue<PendingWrite<WriteCommand>> = ReliableQueue::new(key, 2);
        queue.enqueue(PendingWrite::new(key, command(8), 3)).unwrap();
        queue.enqueue(PendingWrite::new(key, command(8), 3)).unwrap();
        let err = queue
            .enqueue(PendingWrite::new(key, command(8), 3))
            .expect_err("a saturated reliable queue refuses, never drops");
        assert_eq!(
            err,
            DriverError::BufferFull {
                key: key.into(),
                capacity: 2,
            }
        );
    }
}

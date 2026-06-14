//! Map SurrealDB live-query notifications into a [`DataChange`] stream.
//!
//! The engine pushes a `Notification<Value>` per record change on the scoped
//! session; this wrapper translates each into a typed [`DataChange`], decoding
//! the payload through the one record decode path in `rubix-core`
//! (`decode_record`). The scope is already applied by the session — this stream
//! adds no filter (contract #1, `rubix/STACK-DEISGN.md`).

use futures::StreamExt;
use rubix_core::decode_record;
use surrealdb::method::Stream as LiveStream;
use surrealdb::types::{Action, Value};

use crate::error::{BusError, Result};

use super::change::{DataChange, DataChangeKind};

/// A stream of [`DataChange`] events for one subscribed table.
///
/// Yields `Some(Ok(change))` per record change visible to the principal,
/// `Some(Err(_))` if a notification cannot be decoded or the query raised an
/// evaluation error, and `None` once the live query is killed (e.g. the session
/// is dropped).
pub struct DataChangeStream {
    inner: LiveStream<Vec<Value>>,
}

impl DataChangeStream {
    pub(crate) fn new(inner: LiveStream<Vec<Value>>) -> Self {
        Self { inner }
    }

    /// Await the next data-change event.
    ///
    /// Returns `None` when the live query ends. A killed query is reported as
    /// the end of the stream, not as a change.
    ///
    /// # Errors
    /// Returns [`BusError::Decode`] if a notification's record cannot be decoded
    /// and [`BusError::Evaluation`] if the engine reported a live-query
    /// evaluation error.
    pub async fn next(&mut self) -> Option<Result<DataChange>> {
        let notification = match self.inner.next().await? {
            Ok(notification) => notification,
            Err(error) => return Some(Err(BusError::Subscribe(error))),
        };

        let kind = match notification.action {
            Action::Create => DataChangeKind::Created,
            Action::Update => DataChangeKind::Updated,
            Action::Delete => DataChangeKind::Deleted,
            // The stream wrapper yields only record changes; a killed query ends
            // the stream and an evaluation error surfaces the engine's message.
            Action::Killed => return None,
            Action::Error => {
                return Some(Err(BusError::Evaluation(format!("{:?}", notification.data))));
            }
        };

        Some(decode_record(notification.data).map_or_else(
            |error| Err(BusError::Decode(error.to_string())),
            |record| Ok(DataChange::new(kind, record)),
        ))
    }
}

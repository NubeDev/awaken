//! Mint or carry the correlation id at the gate for a command.
//!
//! Contract #3 (`rubix/STACK-DEISGN.md`): the correlation id is minted at the
//! gate for principal-initiated actions, then carried onto the audit record (and
//! later undo/trace/bus events). This is the single chokepoint where a command's
//! id comes into being: a fresh command mints one, a command that already
//! carries an id (e.g. an undo replaying through the gate in WS-06) keeps it, so
//! the whole chain pivots on one thread.

use rubix_core::CorrelationId;

/// Resolve the correlation id a command runs under.
///
/// `carried` is the id propagated from an upstream chokepoint, if any. A
/// principal-initiated command arrives with `None` and is minted a fresh id
/// here; a command replaying an existing chain passes its id through unchanged.
#[must_use]
pub(crate) fn correlate(carried: Option<CorrelationId>) -> CorrelationId {
    carried.unwrap_or_else(CorrelationId::mint)
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use super::correlate;

    #[test]
    fn a_principal_command_mints_a_fresh_id() {
        let first = correlate(None);
        let second = correlate(None);
        assert_ne!(first, second, "each principal command mints its own id");
    }

    #[test]
    fn a_carried_id_is_preserved() {
        let carried = CorrelationId::carry("corr-42");
        assert_eq!(correlate(Some(carried.clone())), carried);
    }
}

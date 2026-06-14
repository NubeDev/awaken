//! Integration coverage for the correlation id minted at the chokepoints.

use std::collections::HashSet;

use rubix_core::CorrelationId;

#[test]
fn many_minted_ids_stay_unique() {
    let mut seen = HashSet::new();
    for _ in 0..1_000 {
        let id = CorrelationId::mint();
        assert!(seen.insert(id.as_str().to_owned()), "minted a duplicate correlation id");
    }
}

#[test]
fn carry_round_trips_a_propagated_id() {
    let original = CorrelationId::mint();
    let carried = CorrelationId::carry(original.as_str());
    assert_eq!(original, carried);
}

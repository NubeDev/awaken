//! Tests for `store/memory.rs`, `store/load.rs`, and `store/record.rs`.

use rubix_rules::{MemoryRuleStore, RuleError, RuleStore, StoredRule};

#[test]
fn load_returns_inserted_rule() {
    let store = MemoryRuleStore::new().with(StoredRule::new("id", "name", "clear()"));
    let r = store.load("name").unwrap();
    assert_eq!(r.id, "id");
    assert_eq!(r.script, "clear()");
}

#[test]
fn load_missing_is_resolve_error() {
    let err = MemoryRuleStore::new().load("ghost").unwrap_err();
    assert!(matches!(err, RuleError::Resolve(_)), "{err:?}");
}

#[test]
fn insert_replaces_by_name() {
    let mut store = MemoryRuleStore::new();
    store.insert(StoredRule::new("a", "dup", "clear()"));
    store.insert(StoredRule::new("b", "dup", "finding(\"info\", \"x\")"));
    assert_eq!(store.load("dup").unwrap().id, "b");
}

#[test]
fn stored_rule_round_trips_through_json() {
    let rule = StoredRule::new("id", "name", "clear()");
    let json = serde_json::to_string(&rule).unwrap();
    let back: StoredRule = serde_json::from_str(&json).unwrap();
    assert_eq!(rule, back);
}

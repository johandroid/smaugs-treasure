use super::*;

#[test]
fn test_mark_disputed() {
    let mut store = DisputeStore::new();

    assert!(mark_disputed(&mut store, 1, 1).is_ok());
    assert_eq!(store.get(&1), Some(&DisputeStatus::Active));
}

#[test]
fn test_already_disputed() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    assert!(mark_disputed(&mut store, 1, 1).is_err());
}

#[test]
fn test_repeat_dispute_after_resolve() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    mark_resolved(&mut store, 1, 1).unwrap();
    assert!(mark_disputed(&mut store, 1, 1).is_err());
}

#[test]
fn test_repeat_dispute_after_chargeback() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    mark_chargedback(&mut store, 1, 1).unwrap();
    assert!(mark_disputed(&mut store, 1, 1).is_err());
}

#[test]
fn test_mark_resolved() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    assert!(mark_resolved(&mut store, 1, 1).is_ok());
    assert_eq!(store.get(&1), Some(&DisputeStatus::Resolved));
}

#[test]
fn test_resolve_without_dispute() {
    let mut store = DisputeStore::new();

    assert!(mark_resolved(&mut store, 1, 1).is_err());
}

#[test]
fn test_mark_chargedback() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    assert!(mark_chargedback(&mut store, 1, 1).is_ok());
    assert_eq!(store.get(&1), Some(&DisputeStatus::Chargedback));
}

#[test]
fn test_active_count() {
    let mut store = DisputeStore::new();

    mark_disputed(&mut store, 1, 1).unwrap();
    mark_disputed(&mut store, 1, 2).unwrap();
    assert_eq!(active_count(&store), 2);

    mark_resolved(&mut store, 1, 1).unwrap();
    assert_eq!(active_count(&store), 1);
}

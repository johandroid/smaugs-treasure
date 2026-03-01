use super::*;

#[test]
fn test_begin_dispute() {
    let mut store = DisputeStore::new();

    assert!(begin_dispute(&mut store, 1));
    assert_eq!(store.get(&1), Some(&DisputeStatus::Active));
}

#[test]
fn test_already_disputed() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    assert!(!begin_dispute(&mut store, 1));
}

#[test]
fn test_repeat_dispute_after_resolve() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    resolve_dispute(&mut store, 1);
    assert!(!begin_dispute(&mut store, 1));
}

#[test]
fn test_repeat_dispute_after_chargeback() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    chargeback_dispute(&mut store, 1);
    assert!(!begin_dispute(&mut store, 1));
}

#[test]
fn test_resolve_dispute() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    assert!(resolve_dispute(&mut store, 1));
    assert_eq!(store.get(&1), Some(&DisputeStatus::Resolved));
}

#[test]
fn test_resolve_without_dispute() {
    let mut store = DisputeStore::new();

    assert!(!resolve_dispute(&mut store, 1));
}

#[test]
fn test_chargeback_dispute_transition() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    assert!(chargeback_dispute(&mut store, 1));
    assert_eq!(store.get(&1), Some(&DisputeStatus::Chargedback));
}

#[test]
fn test_is_active_dispute() {
    let mut store = DisputeStore::new();
    assert!(!is_active_dispute(&store, 1));

    begin_dispute(&mut store, 1);
    assert!(is_active_dispute(&store, 1));

    resolve_dispute(&mut store, 1);
    assert!(!is_active_dispute(&store, 1));
}

#[test]
fn test_active_count() {
    let mut store = DisputeStore::new();

    begin_dispute(&mut store, 1);
    begin_dispute(&mut store, 2);
    assert_eq!(active_count(&store), 2);

    resolve_dispute(&mut store, 1);
    assert_eq!(active_count(&store), 1);
}

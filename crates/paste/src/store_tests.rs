use super::*;

#[test]
fn stored_codes_read_back_under_fresh_valid_ids() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path(), u64::MAX).unwrap();

    let first = store.create("first-code").unwrap();
    let second = store.create("second-code").unwrap();

    assert_ne!(first, second);
    assert!(is_valid_id(&first) && is_valid_id(&second));
    assert_eq!(store.get(&first).unwrap().as_deref(), Some("first-code"));
    assert_eq!(store.get(&second).unwrap().as_deref(), Some("second-code"));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn ids_that_could_leave_the_directory_are_never_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("pastes"), u64::MAX).unwrap();
    std::fs::write(dir.path().join("secretfile"), "secret").unwrap();

    for id in ["../secretfile", "", "aaaaaaaaa/", "aaaaaaaaa.", "żżżżż"] {
        assert_eq!(store.get(id).unwrap(), None, "{id}");
    }
    assert_eq!(store.get("aaaaaaaaaa").unwrap(), None);
}

#[test]
fn quota_counts_existing_pastes_and_refuses_what_does_not_fit() {
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path(), u64::MAX)
        .unwrap()
        .create("12345678")
        .unwrap();
    let store = Store::open(dir.path(), 12).unwrap();

    let refused = store.create("12345").unwrap_err();
    let fits = store.create("1234");

    assert_eq!(refused.kind(), io::ErrorKind::StorageFull);
    assert!(fits.is_ok());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

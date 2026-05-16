use rondo_core::store::lock::{LockError, LockGuard};

#[test]
fn acquire_and_release() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test.lock");
    let g = LockGuard::acquire(path.clone()).unwrap();
    assert!(path.exists());
    drop(g);
    assert!(!path.exists());
}

#[test]
fn second_acquire_conflicts() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test.lock");
    let _g = LockGuard::acquire(path.clone()).unwrap();
    let err = LockGuard::acquire(path).unwrap_err();
    assert!(matches!(err, LockError::Conflict(_)));
}

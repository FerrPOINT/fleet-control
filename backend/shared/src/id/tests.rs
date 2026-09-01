use uuid::Uuid;

#[test]
fn new_ids_are_plain_uuids() {
    assert_ne!(crate::new_id(), Uuid::nil());
}

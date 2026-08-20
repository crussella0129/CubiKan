use std::num::NonZeroU64;

use cubikan_backend::{BackendError, ProjectionCheckpoint, ReadError};

const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const SQLITE_SOURCE: &str = include_str!("../src/sqlite.rs");
const VERIFIED_READ_SOURCE: &str = include_str!("../src/verified_read.rs");

#[test]
fn test_public_reads_are_uncallable_without_verified_snapshot() {
    assert!(LIB_SOURCE.contains(
        "pub use verified_read::{ProjectionCheckpoint, ReadError, VerifiedReadSnapshot}"
    ));
    assert!(SQLITE_SOURCE.contains("pub(crate) fn open_projection_reader"));
    assert!(!SQLITE_SOURCE.contains("pub fn open_projection_reader"));
    assert!(!LIB_SOURCE.contains("ProjectionReaderConnection"));
    assert!(VERIFIED_READ_SOURCE.contains("pub struct VerifiedReadSnapshot"));
    assert!(VERIFIED_READ_SOURCE.contains("pub(crate) fn consume"));
    assert!(VERIFIED_READ_SOURCE.contains("#[cfg(test)]\npub(crate) fn issue_test_snapshot"));
    assert!(!VERIFIED_READ_SOURCE.contains("pub fn issue_test_snapshot"));
    assert!(!VERIFIED_READ_SOURCE.contains("impl Clone for VerifiedReadSnapshot"));
    assert!(!VERIFIED_READ_SOURCE.contains("Serialize for VerifiedReadSnapshot"));
    assert!(VERIFIED_READ_SOURCE.contains("VerifiedQueryStatement::OneBoundedRead"));

    let retired_get = SQLITE_SOURCE
        .find("pub fn get(&self")
        .expect("retired get boundary");
    let retired_get_body = &SQLITE_SOURCE[retired_get..];
    let rejection = retired_get_body
        .find("reject_retired_schema()?")
        .expect("retired read rejection");
    let load = retired_get_body
        .find("load_validated_unit")
        .expect("retained unreachable implementation");
    assert!(rejection < load);

    for (method, unreachable_read) in [
        (
            "pub fn list(&self",
            "query::list(&self.connection, &command)",
        ),
        (
            "pub fn get_relationship_definition",
            "relationship_store::get_definition",
        ),
        (
            "pub fn list_relationships",
            "relationship_store::list_relationships",
        ),
        ("pub fn project(&self", "relationship_store::project"),
    ] {
        let start = SQLITE_SOURCE.find(method).expect("retired read method");
        let source = &SQLITE_SOURCE[start..];
        let rejection = source
            .find(if method == "pub fn list(&self" {
                "reject_retired_schema()?"
            } else {
                "self.require_relationship_schema()?"
            })
            .expect("retired read rejection");
        let read = source
            .find(unreachable_read)
            .expect("retained unreachable read");
        assert!(
            rejection < read,
            "{method} reaches storage before rejection"
        );
    }
}

#[test]
fn checkpoint_and_read_errors_are_typed_without_exposing_a_capability_token() {
    fn assert_checkpoint_traits<T: Clone + std::fmt::Debug + Eq + std::hash::Hash>() {}
    assert_checkpoint_traits::<ProjectionCheckpoint>();
    let _: fn(&ProjectionCheckpoint) -> u64 = ProjectionCheckpoint::block_number;
    let _: fn(&ProjectionCheckpoint) -> &[u8; 32] = ProjectionCheckpoint::block_hash;
    let _: fn(&ProjectionCheckpoint) -> Option<NonZeroU64> =
        ProjectionCheckpoint::last_global_sequence;
    let _: fn(&ProjectionCheckpoint) -> u32 = ProjectionCheckpoint::runtime_spec_version;
    let _: fn(&ProjectionCheckpoint) -> &[u8; 32] = ProjectionCheckpoint::runtime_code_hash;
    assert!(VERIFIED_READ_SOURCE.contains("pub(crate) const fn new("));
    assert!(!VERIFIED_READ_SOURCE.contains("pub const fn new("));
    assert_eq!(
        ReadError::RefreshRequired.to_string(),
        "the projection advanced before the read snapshot was pinned"
    );
    assert_eq!(
        ReadError::ProjectionUnavailable.to_string(),
        "the projection has no finalized checkpoint"
    );
    assert_eq!(
        ReadError::from(BackendError::ProjectionMismatch),
        ReadError::Backend(BackendError::ProjectionMismatch)
    );
}

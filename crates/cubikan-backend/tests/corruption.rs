mod common;

use common::{TestDatabase, fixed_id, linear_workflow, stored_rows};
use cubikan_backend::{BackendError, CreateIntentUnit, SqliteBackend};
use cubikan_core::{IntentSpecies, IntentUnitId};
use rusqlite::{Connection, params};

const ORIGINAL_ID: &str = "50000000-0000-0000-0000-000000000005";
const CHANGED_ID: &str = "50000000-0000-0000-0000-000000000006";

#[derive(Clone, Copy)]
enum Mutation {
    MalformedEnvelope,
    UnsupportedEnvelope,
    UnsupportedEnvelopeProjection,
    NegativeEnvelopeProjection,
    IdProjection,
    WorkflowProjection,
    SpeciesProjection,
    PhaseProjection,
    StatusProjection,
    RevisionProjection,
    MalformedRevisionProjection,
}

#[derive(Clone, Copy)]
enum ExpectedError {
    CorruptEnvelope,
    UnsupportedEnvelope,
    ProjectionMismatch,
}

struct Case {
    name: &'static str,
    mutation: Mutation,
    expected: ExpectedError,
}

fn apply_mutation(connection: &Connection, mutation: Mutation) -> IntentUnitId {
    let original = fixed_id(ORIGINAL_ID);
    match mutation {
        Mutation::MalformedEnvelope => {
            connection
                .execute(
                    "UPDATE intent_units SET envelope='{' WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("malformed envelope fixture should update");
            original
        }
        Mutation::UnsupportedEnvelope => {
            let envelope: String = connection
                .query_row(
                    "SELECT envelope FROM intent_units WHERE id=?1",
                    [ORIGINAL_ID],
                    |row| row.get(0),
                )
                .expect("stored envelope should be readable");
            let mut envelope: serde_json::Value =
                serde_json::from_str(&envelope).expect("fixture envelope should be JSON");
            envelope["representation_version"] = serde_json::json!(2);
            let envelope = serde_json::to_string(&envelope).expect("fixture should serialize");
            connection
                .execute(
                    "UPDATE intent_units SET envelope=?1 WHERE id=?2",
                    params![envelope, ORIGINAL_ID],
                )
                .expect("unsupported envelope fixture should update");
            original
        }
        Mutation::UnsupportedEnvelopeProjection => {
            connection
                .pragma_update(None, "ignore_check_constraints", 1_i64)
                .expect("fixture should bypass CHECK constraints");
            connection
                .execute(
                    "UPDATE intent_units SET envelope_version=2 WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("envelope-version projection should update");
            original
        }
        Mutation::NegativeEnvelopeProjection => {
            connection
                .pragma_update(None, "ignore_check_constraints", 1_i64)
                .expect("fixture should bypass CHECK constraints");
            connection
                .execute(
                    "UPDATE intent_units SET envelope_version=-1 WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("negative envelope-version projection should update");
            original
        }
        Mutation::IdProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET id=?1 WHERE id=?2",
                    params![CHANGED_ID, ORIGINAL_ID],
                )
                .expect("ID projection should update");
            fixed_id(CHANGED_ID)
        }
        Mutation::WorkflowProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET workflow_id='other-flow' WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("workflow projection should update");
            original
        }
        Mutation::SpeciesProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET species='other-species' WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("species projection should update");
            original
        }
        Mutation::PhaseProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET phase='other-phase' WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("phase projection should update");
            original
        }
        Mutation::StatusProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET status='completed' WHERE id=?1",
                    [ORIGINAL_ID],
                )
                .expect("status projection should update");
            original
        }
        Mutation::RevisionProjection => {
            connection
                .execute(
                    "UPDATE intent_units SET revision=?1 WHERE id=?2",
                    params![1_u64.to_be_bytes(), ORIGINAL_ID],
                )
                .expect("revision projection should update");
            original
        }
        Mutation::MalformedRevisionProjection => {
            connection
                .pragma_update(None, "ignore_check_constraints", 1_i64)
                .expect("fixture should bypass CHECK constraints");
            connection
                .execute(
                    "UPDATE intent_units SET revision=?1 WHERE id=?2",
                    params![vec![0_u8; 7], ORIGINAL_ID],
                )
                .expect("malformed revision projection should update");
            original
        }
    }
}

fn assert_expected_error(case: &Case, error: &BackendError) {
    let matches = match case.expected {
        ExpectedError::CorruptEnvelope => matches!(error, BackendError::CorruptEnvelope),
        ExpectedError::UnsupportedEnvelope => {
            matches!(error, BackendError::UnsupportedEnvelopeVersion { found: 2 })
        }
        ExpectedError::ProjectionMismatch => matches!(error, BackendError::ProjectionMismatch),
    };
    assert!(
        matches,
        "{} returned unexpected error: {error:?}",
        case.name
    );
}

#[test]
fn test_get_rejects_envelope_and_each_projection_mismatch_without_repair() {
    let cases = [
        Case {
            name: "malformed-envelope",
            mutation: Mutation::MalformedEnvelope,
            expected: ExpectedError::CorruptEnvelope,
        },
        Case {
            name: "unsupported-envelope",
            mutation: Mutation::UnsupportedEnvelope,
            expected: ExpectedError::UnsupportedEnvelope,
        },
        Case {
            name: "unsupported-envelope-projection",
            mutation: Mutation::UnsupportedEnvelopeProjection,
            expected: ExpectedError::UnsupportedEnvelope,
        },
        Case {
            name: "negative-envelope-projection",
            mutation: Mutation::NegativeEnvelopeProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "id-projection",
            mutation: Mutation::IdProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "workflow-projection",
            mutation: Mutation::WorkflowProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "species-projection",
            mutation: Mutation::SpeciesProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "phase-projection",
            mutation: Mutation::PhaseProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "status-projection",
            mutation: Mutation::StatusProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "revision-projection",
            mutation: Mutation::RevisionProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
        Case {
            name: "malformed-revision-projection",
            mutation: Mutation::MalformedRevisionProjection,
            expected: ExpectedError::ProjectionMismatch,
        },
    ];

    for case in cases {
        let database = TestDatabase::new(case.name);
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        backend
            .create(CreateIntentUnit::new(
                Some(fixed_id(ORIGINAL_ID)),
                IntentSpecies::new("feature").expect("fixture species should be valid"),
                linear_workflow("delivery", "queued", "done"),
            ))
            .expect("fixture Intent Unit should create");

        let lookup_id = {
            let connection = database.connect();
            apply_mutation(&connection, case.mutation)
        };
        let tampered = stored_rows(&database.connect());
        assert_eq!(tampered.len(), 1);

        let error = backend
            .get(lookup_id)
            .expect_err("tampered row must not be returned");
        assert_expected_error(&case, &error);
        assert_eq!(
            stored_rows(&database.connect()),
            tampered,
            "{} must not be repaired or deleted",
            case.name
        );
    }
}

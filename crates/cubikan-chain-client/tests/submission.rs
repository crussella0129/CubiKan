use cubikan_chain_client::{
    DevSigner, MutationOperation, SubmissionErrorKind, SubmissionFailureCode,
    SubmissionOutcomeKind, submit_finalized,
};
use sha2::{Digest, Sha256};

const SOURCE: &str = include_str!("../src/submission.rs");
const SIGNING_ORACLE: &[u8] =
    include_bytes!("../../../tests/fixtures/submission-journal-v1/signed-extrinsic-v1.json");

#[test]
fn test_public_submission_surface_is_closed_to_raw_authority() {
    let _closed_submit_symbol = submit_finalized;
    assert!(SOURCE.contains("trait SubmissionChain: Send + Sync"));
    assert!(!SOURCE.contains("pub trait SubmissionChain"));
    assert!(SOURCE.contains("pub struct SubmissionOutcome(SubmissionOutcomeDetail);"));
    assert!(SOURCE.contains("acknowledge_response_durable(mut self)"));
    assert!(SOURCE.contains("client: &VerifiedArchiveClient"));
    assert!(SOURCE.contains("projection_directory: &Path"));
    assert!(SOURCE.contains("signer: DevSigner"));
    assert!(SOURCE.contains("mutation: Mutation"));
    for forbidden in [
        "pub fn from_raw",
        "pub fn with_nonce",
        "pub fn with_signing_block",
        "pub fn with_rpc",
        "pub fn from_seed",
        "pub fn from_secret",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "production submission surface exposes forbidden authority: {forbidden}"
        );
    }

    let alice = decode_hash("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");
    assert_ne!(DevSigner::Charlie.account_id(), alice);
    assert_ne!(DevSigner::Dave.account_id(), alice);
    assert_ne!(
        DevSigner::Charlie.account_id(),
        DevSigner::Dave.account_id()
    );

    let operation_inventory = [
        MutationOperation::CreateUnit,
        MutationOperation::TransitionUnit,
        MutationOperation::CompleteUnit,
        MutationOperation::CreateRelationshipDefinition,
        MutationOperation::CreateRelationship,
        MutationOperation::DeleteRelationship,
        MutationOperation::RecordAssociation,
        MutationOperation::RevokeAssociation,
    ];
    assert_eq!(operation_inventory.len(), 8);

    let outcome_inventory = [
        SubmissionOutcomeKind::SubmissionRejected,
        SubmissionOutcomeKind::SubmissionLaneUnresolved,
        SubmissionOutcomeKind::ExpiredNotIncluded,
        SubmissionOutcomeKind::FinalizedDispatchRejected,
        SubmissionOutcomeKind::FinalizedInvariantFailed,
        SubmissionOutcomeKind::DeliveryIndeterminate,
        SubmissionOutcomeKind::FinalizedAccepted,
    ];
    assert_eq!(outcome_inventory.len(), 7);
    let _typed_failure_examples = [
        SubmissionFailureCode::NonceConflict,
        SubmissionFailureCode::SubmissionTimeout,
        SubmissionFailureCode::FinalizedInvariantFailed,
        SubmissionFailureCode::RevisionConflict,
    ];
    let _typed_operational_examples = [
        SubmissionErrorKind::UnsupportedPlatform,
        SubmissionErrorKind::InsecureProjectionPath,
        SubmissionErrorKind::SubmissionLaneCorrupt,
        SubmissionErrorKind::ArchiveRpcUnavailable,
        SubmissionErrorKind::RuntimeMismatch,
        SubmissionErrorKind::AcknowledgementUnavailable,
    ];
}

#[test]
fn test_submission_codec_oracle_is_independent_and_non_authorizing() {
    assert_eq!(
        lower_hex(Sha256::digest(SIGNING_ORACLE)),
        "ed971f3032334f8d99de5d0a41000191f8985aea28c3c5b8277d1e0d195385b1"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(SIGNING_ORACLE).expect("decode signing oracle");
    assert_eq!(oracle["format"], "cubikan-submission-signed-extrinsic-v1");
    assert_eq!(
        oracle["authority"]["purpose"],
        "independent_offline_signature_and_scale_codec_oracle"
    );
    assert_eq!(
        oracle["authority"]["signer_authorized_by_local_runtime"],
        false
    );
    assert_eq!(oracle["authority"]["signer_name"], "Alice");
    assert_eq!(oracle["authority"]["production_submitters"][0], "Charlie");
    assert_eq!(oracle["authority"]["production_submitters"][1], "Dave");
    assert_eq!(oracle["parameters"]["period"], 64);
    assert_eq!(oracle["parameters"]["phase"], 3);
    assert_eq!(oracle["parameters"]["inclusive_birth"], "131");
    assert_eq!(oracle["parameters"]["inclusive_death"], "194");
    assert_eq!(oracle["parameters"]["tip"], "0");
    assert_eq!(
        oracle["signer_payload"]["additional_block_hash_is_encoded_in_extrinsic"],
        false
    );
}

fn decode_hash(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    let mut bytes = [0_u8; 32];
    for (output, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *output = hex_nibble(pair[0]) << 4 | hex_nibble(pair[1]);
    }
    bytes
}

const fn hex_nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => panic!("hash must be canonical lowercase hex"),
    }
}

fn lower_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

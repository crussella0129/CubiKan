use std::io::{self, Write};

use cubikan_cli::{RunStatus, run};

const SUCCESS_REQUEST: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan/requests/success_transition.json");
const SUCCESS_STDOUT: &[u8] =
    include_bytes!("../../../tests/fixtures/protocol-v2/cubikan/stdout/success_transition.jsonl");
const V1_REQUEST: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/requests/error_unsupported_protocol_v1.json"
);
const V1_STDOUT: &[u8] = include_bytes!(
    "../../../tests/fixtures/protocol-v2/cubikan/stdout/error_unsupported_protocol_v1.jsonl"
);

#[derive(Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    flush_offsets: Vec<usize>,
}

impl Write for RecordingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_offsets.push(self.bytes.len());
        Ok(())
    }
}

#[test]
fn public_runner_emits_the_locked_compact_simulation_response() {
    let mut writer = RecordingWriter::default();

    let status = run(SUCCESS_REQUEST, &mut writer).expect("response should be delivered");

    assert_eq!(status, RunStatus::Success);
    assert_eq!(writer.bytes, SUCCESS_STDOUT);
    assert_eq!(writer.flush_offsets, [writer.bytes.len()]);
}

#[test]
fn public_runner_rejects_v1_before_interpreting_removed_authority() {
    let mut writer = RecordingWriter::default();

    let status = run(V1_REQUEST, &mut writer).expect("response should be delivered");

    assert_eq!(status, RunStatus::RequestRejected);
    assert_eq!(writer.bytes, V1_STDOUT);
    assert_eq!(writer.flush_offsets, [writer.bytes.len()]);
}

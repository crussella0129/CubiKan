use std::path::Path;

use crate::protocol::{ExecutedRequest, unsupported_bridge_response};

/// Rejects the retired protocol before inspecting or opening the requested path.
#[must_use]
pub fn execute_request(_path: impl AsRef<Path>, request: &[u8]) -> ExecutedRequest {
    unsupported_bridge_response(request)
}

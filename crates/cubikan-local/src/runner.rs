use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::{ResponseClass, execute_request, protocol::request_too_large_response};

/// Maximum number of raw request bytes accepted by the local runner.
pub const MAX_REQUEST_BYTES: usize = 1_048_576;

const EXIT_SUCCESS: u8 = 0;
const EXIT_OPERATIONAL: u8 = 1;
const EXIT_REQUEST: u8 = 2;
const EXIT_COMMAND: u8 = 3;
const EXIT_STORAGE: u8 = 4;
const USAGE: &[u8] = b"usage: cubikan-local --database PATH\n";

/// Operational stdin/stdout failure outside the modeled JSON protocol.
#[derive(Debug)]
pub enum RunError {
    /// Reading the bounded request failed before a modeled outcome existed.
    ReadRequest(io::Error),
    /// Writing the compact modeled response body failed.
    WriteResponse(io::Error),
    /// Writing the one required response newline failed.
    WriteNewline(io::Error),
    /// The one required final stdout flush failed.
    FlushResponse(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadRequest(error) => write!(formatter, "failed to read local request: {error}"),
            Self::WriteResponse(error) => write!(
                formatter,
                "failed to write local response; committed outcome is unknown: {error}"
            ),
            Self::WriteNewline(error) => write!(
                formatter,
                "failed to terminate local response; committed outcome is unknown: {error}"
            ),
            Self::FlushResponse(error) => write!(
                formatter,
                "failed to flush local response; committed outcome is unknown: {error}"
            ),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadRequest(error)
            | Self::WriteResponse(error)
            | Self::WriteNewline(error)
            | Self::FlushResponse(error) => Some(error),
        }
    }
}

/// Reads, executes, and fully delivers one modeled local request.
///
/// A successful return is produced only after the compact response body, one
/// newline, and one final flush have all succeeded.
pub fn run(
    database_path: impl AsRef<Path>,
    reader: impl Read,
    mut writer: impl Write,
) -> Result<ResponseClass, RunError> {
    let maximum_with_lookahead = MAX_REQUEST_BYTES
        .checked_add(1)
        .expect("the request bound plus one byte must fit usize");
    let read_limit = u64::try_from(maximum_with_lookahead)
        .expect("the request bound plus one byte must fit u64");
    let mut request = Vec::with_capacity(maximum_with_lookahead);
    reader
        .take(read_limit)
        .read_to_end(&mut request)
        .map_err(RunError::ReadRequest)?;

    let response = if request.len() > MAX_REQUEST_BYTES {
        request_too_large_response(MAX_REQUEST_BYTES)
    } else {
        execute_request(database_path, &request)
    };
    writer
        .write_all(response.body())
        .map_err(RunError::WriteResponse)?;
    writer.write_all(b"\n").map_err(RunError::WriteNewline)?;
    writer.flush().map_err(RunError::FlushResponse)?;
    Ok(response.class())
}

/// Runs the injectable process shell and returns its exact process exit code.
pub fn run_process<I, R, W, E>(args_os: I, reader: R, writer: W, mut stderr: E) -> u8
where
    I: IntoIterator<Item = OsString>,
    R: Read,
    W: Write,
    E: Write,
{
    let database_path = match parse_database_path(args_os) {
        Ok(path) => path,
        Err(()) => {
            let _ = stderr.write_all(USAGE);
            let _ = stderr.flush();
            return EXIT_REQUEST;
        }
    };

    match run(database_path, reader, writer) {
        Ok(ResponseClass::Success) => EXIT_SUCCESS,
        Ok(ResponseClass::RequestRejected) => EXIT_REQUEST,
        Ok(ResponseClass::CommandRejected) => EXIT_COMMAND,
        Ok(ResponseClass::StorageRejected) => EXIT_STORAGE,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            let _ = stderr.flush();
            EXIT_OPERATIONAL
        }
    }
}

fn parse_database_path<I>(args_os: I) -> Result<PathBuf, ()>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = args_os.into_iter();
    let _program = arguments.next().ok_or(())?;
    if arguments.next().as_deref() != Some(OsStr::new("--database")) {
        return Err(());
    }
    let path = arguments.next().ok_or(())?;
    if arguments.next().is_some() || path.is_empty() || Path::new(&path) == Path::new(":memory:") {
        return Err(());
    }
    Ok(PathBuf::from(path))
}

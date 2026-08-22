use std::{
    ffi::OsString,
    io::{self, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::GitReferenceError;

const MAX_STDOUT_BYTES: usize = 4_096;
const MAX_STDERR_BYTES: usize = 4_096;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub(crate) struct ProcessOutput {
    pub(crate) success: bool,
    pub(crate) stdout: Vec<u8>,
}

pub(crate) fn run_git(
    executable: &Path,
    arguments: &[OsString],
) -> Result<ProcessOutput, GitReferenceError> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .env_clear()
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);

    let mut child = command
        .spawn()
        .map_err(|_| GitReferenceError::CommandFailed)?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            kill_and_reap(&mut child);
            return Err(GitReferenceError::CommandFailed);
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            kill_and_reap(&mut child);
            return Err(GitReferenceError::CommandFailed);
        }
    };

    let output_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_handle = spawn_reader(stdout, MAX_STDOUT_BYTES, Arc::clone(&output_exceeded))
        .map_err(|_| {
            kill_and_reap(&mut child);
            GitReferenceError::CommandFailed
        })?;
    let stderr_handle = match spawn_reader(stderr, MAX_STDERR_BYTES, Arc::clone(&output_exceeded)) {
        Ok(handle) => handle,
        Err(_) => {
            kill_and_reap(&mut child);
            let _ = stdout_handle.join();
            return Err(GitReferenceError::CommandFailed);
        }
    };

    let started = Instant::now();
    let mut status = None;
    loop {
        if output_exceeded.load(Ordering::Acquire) {
            terminate_and_join(&mut child, stdout_handle, stderr_handle);
            return Err(GitReferenceError::OutputLimitExceeded);
        }
        if started.elapsed() >= COMMAND_TIMEOUT {
            terminate_and_join(&mut child, stdout_handle, stderr_handle);
            return Err(GitReferenceError::CommandTimedOut);
        }

        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exit_status)) => status = Some(exit_status),
                Ok(None) => {}
                Err(_) => {
                    terminate_and_join(&mut child, stdout_handle, stderr_handle);
                    return Err(GitReferenceError::CommandFailed);
                }
            }
        }

        if status.is_some() && stdout_handle.is_finished() && stderr_handle.is_finished() {
            break;
        }
        thread::sleep(WAIT_POLL_INTERVAL);
    }

    let stdout = join_reader(stdout_handle)?;
    let _stderr = join_reader(stderr_handle)?;
    if output_exceeded.load(Ordering::Acquire) {
        return Err(GitReferenceError::OutputLimitExceeded);
    }

    Ok(ProcessOutput {
        success: status.ok_or(GitReferenceError::CommandFailed)?.success(),
        stdout,
    })
}

fn spawn_reader<R>(
    reader: R,
    maximum: usize,
    output_exceeded: Arc<AtomicBool>,
) -> io::Result<thread::JoinHandle<io::Result<Vec<u8>>>>
where
    R: Read + Send + 'static,
{
    thread::Builder::new()
        .name("cubikan-git-output".to_owned())
        .spawn(move || read_bounded(reader, maximum, &output_exceeded))
}

fn read_bounded(
    mut reader: impl Read,
    maximum: usize,
    output_exceeded: &AtomicBool,
) -> io::Result<Vec<u8>> {
    let mut retained = Vec::with_capacity(maximum);
    let mut chunk = [0_u8; 1_024];
    loop {
        let count = reader.read(&mut chunk)?;
        if count == 0 {
            return Ok(retained);
        }

        let remaining = maximum.saturating_sub(retained.len());
        let keep = remaining.min(count);
        retained.extend_from_slice(&chunk[..keep]);
        if keep != count {
            output_exceeded.store(true, Ordering::Release);
            return Ok(retained);
        }
    }
}

fn join_reader(
    handle: thread::JoinHandle<io::Result<Vec<u8>>>,
) -> Result<Vec<u8>, GitReferenceError> {
    handle
        .join()
        .map_err(|_| GitReferenceError::CommandFailed)?
        .map_err(|_| GitReferenceError::CommandFailed)
}

fn kill_and_reap(child: &mut Child) {
    #[cfg(unix)]
    {
        let process_group = rustix::process::Pid::from_child(child);
        let _ = rustix::process::kill_process_group(process_group, rustix::process::Signal::KILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn terminate_and_join(
    child: &mut Child,
    stdout: thread::JoinHandle<io::Result<Vec<u8>>>,
    stderr: thread::JoinHandle<io::Result<Vec<u8>>>,
) {
    kill_and_reap(child);
    let _ = stdout.join();
    let _ = stderr.join();
}

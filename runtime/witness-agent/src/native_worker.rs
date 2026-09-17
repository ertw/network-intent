//! libubus uses process-global state and lookup has no caller-supplied deadline.
//! Isolate exactly one native call in an exec'd worker. The supervisor bounds
//! connect, lookup, invocation, IPC and exit with one monotonic deadline.
#![cfg_attr(
    not(all(target_os = "linux", feature = "native-ubus")),
    allow(dead_code)
)]
use super::{parse_response_json, read_only_method, ObservationError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    os::{fd::OwnedFd, unix::net::UnixStream},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

const HELPER: &str = "/usr/libexec/intent-ubus-observe";
const MAX_REQUEST: usize = 64 * 1024;
const MAX_RESPONSE: usize = 16 * 1024 * 1024;
const MAX_WORKERS: usize = 4;
const MAX_TIMEOUT: Duration = Duration::from_secs(30);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    object: String,
    method: String,
    request: Value,
    timeout_ms: u32,
    max_response_bytes: usize,
}

struct Slot<'a>(&'a AtomicUsize);
impl<'a> Slot<'a> {
    fn acquire(active: &'a AtomicUsize) -> Result<Self, ObservationError> {
        active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_WORKERS).then_some(count + 1)
            })
            .map_err(|_| {
                ObservationError::Unavailable("native observation worker capacity exhausted".into())
            })?;
        Ok(Self(active))
    }
}
impl Drop for Slot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

// All paths, including malformed/oversized replies and unwinding, terminate and
// reap the worker before releasing its slot. There are no abandoned I/O threads.
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn remaining(deadline: Instant) -> Result<Duration, ObservationError> {
    let duration = deadline.saturating_duration_since(Instant::now());
    if duration.is_zero() {
        Err(ObservationError::Timeout)
    } else {
        Ok(duration)
    }
}
fn io_error(error: io::Error) -> ObservationError {
    match error.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ObservationError::Timeout,
        _ => ObservationError::Unavailable(format!("native worker IPC: {error}")),
    }
}
fn status_error(status: i32) -> ObservationError {
    match status {
        7001 => ObservationError::ResponseTooLarge,
        7002 => ObservationError::Timeout,
        7003 => ObservationError::Denied("ubus permission denied".into()),
        7004 => ObservationError::Malformed("multipart, empty or invalid native ubus reply".into()),
        _ => ObservationError::Unavailable(format!("libubus status {status}")),
    }
}

// Refresh the socket timeout before *each* syscall: a trickle of bytes cannot
// extend the overall deadline as it would with read_exact/write_all alone.
fn send(
    stream: &mut UnixStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<(), ObservationError> {
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(io_error)?;
        match stream.write(bytes) {
            Ok(0) => return Err(io_error(io::ErrorKind::WriteZero.into())),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(())
}
fn receive(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), ObservationError> {
    while !bytes.is_empty() {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(io_error)?;
        match stream.read(bytes) {
            Ok(0) => {
                return Err(ObservationError::Malformed(
                    "truncated native worker frame".into(),
                ))
            }
            Ok(n) => bytes = &mut bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(())
}

struct BoundedRequest(Vec<u8>);
impl Write for BoundedRequest {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_REQUEST.saturating_sub(self.0.len()) {
            return Err(io::Error::other("native request too large"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn invoke(
    object: &str,
    method: &str,
    request: &Value,
    timeout: Duration,
    max_response_bytes: usize,
) -> Result<Value, ObservationError> {
    let deadline = Instant::now() + timeout.min(MAX_TIMEOUT);
    if !read_only_method(object, method) {
        return Err(ObservationError::UnsupportedMethod {
            object: object.into(),
            method: method.into(),
        });
    }
    remaining(deadline)?;
    if max_response_bytes == 0 || max_response_bytes > MAX_RESPONSE {
        return Err(ObservationError::Malformed(
            "native response limit must be 1..=16777216 bytes".into(),
        ));
    }
    let _slot = Slot::acquire(&ACTIVE)?;
    // Serialize by reference so even rejected caller input is never cloned.
    #[derive(Serialize)]
    struct BorrowedRequest<'a> {
        object: &'a str,
        method: &'a str,
        request: &'a Value,
        timeout_ms: u32,
        max_response_bytes: usize,
    }
    let mut encoded = BoundedRequest(Vec::new());
    serde_json::to_writer(
        &mut encoded,
        &BorrowedRequest {
            object,
            method,
            request,
            timeout_ms: timeout.min(MAX_TIMEOUT).as_millis().max(1) as u32,
            max_response_bytes,
        },
    )
    .map_err(|_| {
        ObservationError::Malformed(
            "native request exceeds 65536 bytes or cannot be serialized".into(),
        )
    })?;
    let mut command = Command::new(HELPER);
    // No PATH lookup, shell, inherited dynamic-loader configuration or recursive
    // invocation of the witness service. Session credentials travel only in IPC.
    command.env_clear();
    run(&mut command, &encoded.0, deadline, max_response_bytes)
}

fn run(
    command: &mut Command,
    request: &[u8],
    deadline: Instant,
    max_response_bytes: usize,
) -> Result<Value, ObservationError> {
    remaining(deadline)?;
    let (mut parent, child) = UnixStream::pair().map_err(io_error)?;
    let input: OwnedFd = child.try_clone().map_err(io_error)?.into();
    let output: OwnedFd = child.into();
    command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output))
        .stderr(Stdio::null());
    let mut worker = Worker(command.spawn().map_err(io_error)?);
    // Command retains its configured Stdio descriptors. Replace them immediately
    // so the parent doesn't itself keep the worker socket endpoint open.
    command.stdin(Stdio::null()).stdout(Stdio::null());
    send(&mut parent, &(request.len() as u32).to_be_bytes(), deadline)?;
    send(&mut parent, request, deadline)?;
    parent.shutdown(Shutdown::Write).map_err(io_error)?;
    let mut header = [0; 8];
    receive(&mut parent, &mut header, deadline)?;
    let status = i32::from_be_bytes(header[..4].try_into().unwrap());
    let length = u32::from_be_bytes(header[4..].try_into().unwrap()) as usize;
    if length > max_response_bytes {
        return Err(ObservationError::ResponseTooLarge);
    }
    if status != 0 {
        if length != 0 {
            return Err(ObservationError::Malformed(
                "native worker error included payload".into(),
            ));
        }
        return Err(status_error(status));
    }
    let mut bytes = vec![0; length];
    receive(&mut parent, &mut bytes, deadline)?;
    // Require exactly one frame and successful worker exit before accepting evidence.
    loop {
        parent
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(io_error)?;
        match parent.read(&mut [0]) {
            Ok(0) => break,
            Ok(_) => {
                return Err(ObservationError::Malformed(
                    "extra native worker frame data".into(),
                ))
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_error(e)),
        }
    }
    loop {
        remaining(deadline)?;
        match worker.0.try_wait().map_err(io_error)? {
            Some(status) if status.success() => break,
            Some(_) => {
                return Err(ObservationError::Unavailable(
                    "native worker exited unsuccessfully".into(),
                ))
            }
            None => thread::sleep(remaining(deadline)?.min(Duration::from_millis(1))),
        }
    }
    let result = parse_response_json(&bytes);
    remaining(deadline)?;
    result
}

/// Private executable entry point. Must be called once in a fresh process.
#[cfg(all(target_os = "linux", feature = "native-ubus"))]
pub fn helper_main() -> io::Result<()> {
    use std::{
        ffi::{CStr, CString},
        os::raw::{c_char, c_int},
        ptr,
        sync::atomic::AtomicBool,
    };
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::AcqRel) {
        return Err(io::Error::other("worker already used"));
    }
    unsafe extern "C" {
        fn intent_ubus_invoke_json(
            object: *const c_char,
            method: *const c_char,
            request: *const c_char,
            timeout_ms: c_int,
            max_response_bytes: usize,
            response: *mut *mut c_char,
        ) -> c_int;
        fn intent_ubus_free(value: *mut c_char);
    }
    let mut input = io::stdin().lock();
    let mut size = [0; 4];
    input.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if size > MAX_REQUEST {
        return Err(io::Error::other("request frame too large"));
    }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes)?;
    if input.read(&mut [0])? != 0 {
        return Err(io::Error::other("extra request data"));
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    if !read_only_method(&request.object, &request.method)
        || request.timeout_ms == 0
        || request.timeout_ms > MAX_TIMEOUT.as_millis() as u32
        || request.max_response_bytes == 0
        || request.max_response_bytes > MAX_RESPONSE
    {
        return Err(io::Error::other("invalid native worker request"));
    }
    let object = CString::new(request.object)?;
    let method = CString::new(request.method)?;
    let body = CString::new(request.request.to_string())?;
    let mut response = ptr::null_mut();
    let mut status = unsafe {
        intent_ubus_invoke_json(
            object.as_ptr(),
            method.as_ptr(),
            body.as_ptr(),
            request.timeout_ms as c_int,
            request.max_response_bytes,
            &mut response,
        )
    };
    let mut result = Vec::new();
    if !response.is_null() {
        let bytes = unsafe { CStr::from_ptr(response).to_bytes() };
        if bytes.len() > request.max_response_bytes {
            status = 7001;
        } else if status == 0 {
            result.extend_from_slice(bytes);
        }
        unsafe {
            intent_ubus_free(response);
        }
    } else if status == 0 {
        status = 7004;
    }
    let mut output = io::stdout().lock();
    output.write_all(&status.to_be_bytes())?;
    output.write_all(&(result.len() as u32).to_be_bytes())?;
    output.write_all(&result)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shell(script: &str, timeout: Duration, maximum: usize) -> Result<Value, ObservationError> {
        // Test fixtures only. The production command is the fixed native binary.
        let mut command = Command::new("/bin/sh");
        // The real helper consumes the request before responding. Draining stdin
        // also prevents a fast fixture exit from racing the parent's writes.
        command.args(["-c", &format!("cat >/dev/null; {script}")]);
        run(&mut command, b"{}", Instant::now() + timeout, maximum)
    }
    #[test]
    fn worker_times_out_and_reaps_hung_process() {
        let start = Instant::now();
        assert_eq!(
            shell("exec sleep 10", Duration::from_millis(80), 1024),
            Err(ObservationError::Timeout)
        );
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn worker_cleanup_reaps_the_actual_child() {
        // Resolve test utilities from PATH so Nix sandboxes need no /bin tools.
        let child = Command::new("sleep").arg("10").spawn().unwrap();
        let pid = child.id();
        drop(Worker(child));
        // A zombie still exists for kill(0); the PID must be absent after Drop.
        assert!(!Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
    }
    #[test]
    fn worker_complete_payload_does_not_bypass_exit_deadline() {
        assert_eq!(
            shell(
                "printf '\\000\\000\\000\\000\\000\\000\\000\\002{}'; exec sleep 10",
                Duration::from_millis(80),
                100
            ),
            Err(ObservationError::Timeout)
        );
    }
    #[test]
    fn trickled_bytes_do_not_restart_the_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        let sender = thread::spawn(move || {
            for _ in 0..20 {
                if writer.write_all(&[0]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let start = Instant::now();
        assert_eq!(
            receive(&mut reader, &mut [0; 20], start + Duration::from_millis(80)),
            Err(ObservationError::Timeout)
        );
        assert!(start.elapsed() < Duration::from_millis(300));
        drop(reader);
        sender.join().unwrap();
    }
    #[test]
    fn worker_rejects_oversize_before_allocating_or_waiting_for_payload() {
        assert_eq!(
            shell(
                "printf '\\000\\000\\000\\000\\377\\377\\377\\377'; exec sleep 10",
                Duration::from_secs(1),
                100
            ),
            Err(ObservationError::ResponseTooLarge)
        );
    }
    #[test]
    fn worker_rejects_truncation_and_failed_exit() {
        assert!(matches!(
            shell("exit 1", Duration::from_secs(1), 100),
            Err(ObservationError::Malformed(_)) | Err(ObservationError::Unavailable(_))
        ));
        assert!(matches!(
            shell(
                "printf '\\000\\000\\000\\000\\000\\000\\000\\002{}'; exit 1",
                Duration::from_secs(1),
                100
            ),
            Err(ObservationError::Unavailable(_))
        ));
    }
    #[test]
    fn worker_accepts_only_one_complete_success_frame() {
        assert_eq!(
            shell(
                "printf '\\000\\000\\000\\000\\000\\000\\000\\002{}'",
                Duration::from_secs(1),
                100
            )
            .unwrap(),
            serde_json::json!({})
        );
        assert!(matches!(
            shell(
                "printf '\\000\\000\\000\\000\\000\\000\\000\\002{}x'",
                Duration::from_secs(1),
                100
            ),
            Err(ObservationError::Malformed(_))
        ));
    }
    #[test]
    fn worker_preserves_native_error_codes() {
        for (code, expected) in [
            (7001, ObservationError::ResponseTooLarge),
            (7002, ObservationError::Timeout),
            (
                7003,
                ObservationError::Denied("ubus permission denied".into()),
            ),
            (
                7004,
                ObservationError::Malformed("multipart, empty or invalid native ubus reply".into()),
            ),
        ] {
            let frame: String = (code as i32)
                .to_be_bytes()
                .into_iter()
                .chain([0; 4])
                .map(|byte| format!("\\{byte:03o}"))
                .collect();
            assert_eq!(
                shell(&format!("printf '{frame}'"), Duration::from_secs(1), 100),
                Err(expected)
            );
        }
    }
    #[test]
    fn worker_capacity_is_fail_fast_and_released() {
        let active = AtomicUsize::new(0);
        let slots: Vec<_> = (0..MAX_WORKERS)
            .map(|_| Slot::acquire(&active).unwrap())
            .collect();
        assert!(Slot::acquire(&active).is_err());
        drop(slots);
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert!(Slot::acquire(&active).is_ok());
    }
    #[test]
    fn worker_rejects_request_before_spawn() {
        assert!(matches!(
            invoke(
                "uci",
                "get",
                &Value::String("x".repeat(MAX_REQUEST)),
                Duration::from_secs(1),
                100
            ),
            Err(ObservationError::Malformed(_))
        ));
        assert_eq!(
            invoke("uci", "get", &Value::Null, Duration::ZERO, 100),
            Err(ObservationError::Timeout)
        );
    }
}

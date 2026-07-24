use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/**
 * One-request-per-connection transport for Herdr's newline-delimited protocol.
 */
#[derive(Debug, Clone)]
pub(crate) struct UnixSocketTransport {
    socket_path: PathBuf,
}

impl UnixSocketTransport {
    pub(crate) fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /**
     * Sends one request through the configured socket and returns its response.
     */
    pub(crate) fn exchange(&self, request: &impl Serialize) -> Result<Value> {
        exchange_at(&self.socket_path, request)
    }
}

/**
 * Performs one newline-delimited request-response exchange at an explicit socket path.
 */
fn exchange_at(socket_path: &Path, request: &impl Serialize) -> Result<Value> {
    let mut stream = UnixStream::connect(socket_path).with_context(|| {
        format!(
            "failed to connect to Herdr socket {}",
            socket_path.display()
        )
    })?;

    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    serde_json::to_writer(&mut stream, request).context("failed to serialize Herdr request")?;
    stream
        .write_all(b"\n")
        .context("failed to write Herdr request")?;
    stream.flush().context("failed to flush Herdr request")?;

    let mut bytes = Vec::new();

    BufReader::new(stream)
        .take(MAX_RESPONSE_BYTES + 1)
        .read_until(b'\n', &mut bytes)
        .context("failed to read Herdr response")?;

    if bytes.is_empty() {
        bail!("Herdr socket closed before sending a response");
    }

    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        bail!("Herdr response exceeded {MAX_RESPONSE_BYTES} bytes");
    }

    if !bytes.ends_with(b"\n") {
        bail!("Herdr response was truncated before newline framing");
    }

    serde_json::from_slice(&bytes).context("malformed Herdr response JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    #[test]
    fn writes_and_reads_one_newline_delimited_message() {
        let dir = std::env::temp_dir().join(format!("pluck-socket-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let listener = UnixListener::bind(&dir).unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            let mut reader = BufReader::new(stream);
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, "{\"id\":\"x\"}\n");
            reader
                .get_mut()
                .write_all(b"{\"id\":\"x\",\"result\":{}}\n")
                .unwrap();
        });
        let value = exchange_at(&dir, &serde_json::json!({"id": "x"})).unwrap();
        assert_eq!(value["id"], "x");
        server.join().unwrap();
        std::fs::remove_file(dir).unwrap();
    }

    #[test]
    fn rejects_truncated_responses() {
        let dir = std::env::temp_dir().join(format!("pluck-truncated-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let listener = UnixListener::bind(&dir).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            stream.write_all(b"{}").unwrap();
        });
        assert!(exchange_at(&dir, &serde_json::json!({"id": "x"}))
            .unwrap_err()
            .to_string()
            .contains("truncated"));
        std::fs::remove_file(dir).unwrap();
    }
}

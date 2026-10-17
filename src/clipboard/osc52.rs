use super::error::ClipboardError;
use super::{Clipboard, CopySuccess};
use base64::Engine;
use std::io::{self, Write};

const TOOL_NAME: &str = "osc52";

/// Clipboard that asks the hosting terminal to copy text with an OSC 52 escape sequence.
///
/// Inside Herdr, the server captures pane OSC 52 writes and forwards them only to the
/// client the user is viewing from, which writes to that machine's clipboard. Copies
/// therefore land on the viewer's machine even when the picker runs on a remote server.
#[derive(Debug, Clone, Copy)]
pub struct Osc52Clipboard<T = StdoutTerminal> {
    terminal: T,
}

impl<T: Osc52Terminal> Osc52Clipboard<T> {
    pub fn new(terminal: T) -> Self {
        Self { terminal }
    }
}

impl<T: Osc52Terminal> Clipboard for Osc52Clipboard<T> {
    fn copy(&self, text: &str) -> Result<CopySuccess, ClipboardError> {
        self.terminal
            .write_sequence(osc52_sequence(text).as_bytes())
            .map_err(|err| ClipboardError::WriteFailed {
                tool: TOOL_NAME.to_string(),
                message: err.to_string(),
            })?;
        Ok(CopySuccess {
            tool: TOOL_NAME.to_string(),
        })
    }
}

/// Injectable terminal boundary that receives OSC 52 sequences.
pub trait Osc52Terminal {
    fn write_sequence(&self, sequence: &[u8]) -> io::Result<()>;
}

/// Writes OSC 52 sequences to the process stdout, which is the picker pane's PTY.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdoutTerminal;

impl Osc52Terminal for StdoutTerminal {
    fn write_sequence(&self, sequence: &[u8]) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(sequence)?;
        stdout.flush()
    }
}

/// Builds `ESC ] 52 ; c ; <base64> BEL`, targeting the standard clipboard.
///
/// BEL rather than ST terminates the sequence because some terminals only honor BEL.
fn osc52_sequence(text: &str) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(text);
    format!("\x1b]52;c;{payload}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeTerminal {
        written: RefCell<Vec<u8>>,
        failure: Option<io::ErrorKind>,
    }

    impl Osc52Terminal for &FakeTerminal {
        fn write_sequence(&self, sequence: &[u8]) -> io::Result<()> {
            if let Some(kind) = self.failure {
                return Err(io::Error::new(kind, "terminal closed"));
            }
            self.written.borrow_mut().extend_from_slice(sequence);
            Ok(())
        }
    }

    #[test]
    fn copy_writes_bel_terminated_standard_clipboard_sequence() {
        let terminal = FakeTerminal::default();

        let success = Osc52Clipboard::new(&terminal)
            .copy("https://example.com")
            .unwrap();

        assert_eq!(success.tool, "osc52");
        assert_eq!(
            terminal.written.borrow().as_slice(),
            b"\x1b]52;c;aHR0cHM6Ly9leGFtcGxlLmNvbQ==\x07"
        );
    }

    #[test]
    fn copy_encodes_utf8_text() {
        assert_eq!(osc52_sequence("héllo"), "\x1b]52;c;aMOpbGxv\x07");
    }

    #[test]
    fn copy_surfaces_terminal_write_failure() {
        let terminal = FakeTerminal {
            failure: Some(io::ErrorKind::BrokenPipe),
            ..FakeTerminal::default()
        };

        let error = Osc52Clipboard::new(&terminal).copy("token").unwrap_err();

        assert_eq!(
            error,
            ClipboardError::WriteFailed {
                tool: "osc52".to_string(),
                message: "terminal closed".to_string(),
            }
        );
    }
}

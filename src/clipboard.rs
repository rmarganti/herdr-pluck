use crate::model::CopyResult;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardTool {
    pub name: &'static str,
    pub args: &'static [&'static str],
}

pub fn fallback_tools() -> Vec<ClipboardTool> {
    vec![
        ClipboardTool {
            name: "pbcopy",
            args: &[],
        },
        ClipboardTool {
            name: "wl-copy",
            args: &[],
        },
        ClipboardTool {
            name: "xclip",
            args: &["-selection", "clipboard"],
        },
        ClipboardTool {
            name: "xsel",
            args: &["--clipboard", "--input"],
        },
    ]
}

pub fn first_available_tool() -> Option<ClipboardTool> {
    fallback_tools()
        .into_iter()
        .find(|tool| which::which(tool.name).is_ok())
}

pub fn copy_to_clipboard(text: &str) -> CopyResult {
    let Some(tool) = first_available_tool() else {
        return CopyResult::Failed {
            message: "no supported clipboard tool found: tried pbcopy, wl-copy, xclip, xsel"
                .to_string(),
        };
    };

    let mut child = match Command::new(tool.name)
        .args(tool.args)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return CopyResult::Failed {
                message: format!("failed to start {}: {err}", tool.name),
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if let Err(err) = stdin.write_all(text.as_bytes()) {
            return CopyResult::Failed {
                message: format!("failed to write clipboard text to {}: {err}", tool.name),
            };
        }
    }

    match child.wait() {
        Ok(status) if status.success() => CopyResult::Copied {
            tool: tool.name.to_string(),
        },
        Ok(status) => CopyResult::Failed {
            message: format!("{} exited with status {status}", tool.name),
        },
        Err(err) => CopyResult::Failed {
            message: format!("failed waiting for {}: {err}", tool.name),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_order_prefers_platform_specific_tools() {
        let names: Vec<_> = fallback_tools().into_iter().map(|tool| tool.name).collect();
        assert_eq!(names, vec!["pbcopy", "wl-copy", "xclip", "xsel"]);
    }
}

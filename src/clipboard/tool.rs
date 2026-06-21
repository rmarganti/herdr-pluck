/// Clipboard command plus arguments used for one copy attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipboardTool {
    pub(crate) name: &'static str,
    pub(crate) args: &'static [&'static str],
}

const PBCOPY: ClipboardTool = ClipboardTool {
    name: "pbcopy",
    args: &[],
};
const WL_COPY: ClipboardTool = ClipboardTool {
    name: "wl-copy",
    args: &[],
};
const XCLIP: ClipboardTool = ClipboardTool {
    name: "xclip",
    args: &["-selection", "clipboard"],
};
const XSEL: ClipboardTool = ClipboardTool {
    name: "xsel",
    args: &["--clipboard", "--input"],
};

/// Minimal platform/session facts used to order clipboard fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipboardEnvironment {
    pub(crate) os: ClipboardOs,
    pub(crate) wayland: bool,
    pub(crate) x11: bool,
}

impl ClipboardEnvironment {
    pub(crate) fn current() -> Self {
        Self {
            os: ClipboardOs::current(),
            wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
            x11: std::env::var_os("DISPLAY").is_some(),
        }
    }
}

/// Coarse operating-system family for clipboard fallback ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardOs {
    Macos,
    Other,
}

impl ClipboardOs {
    fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Other
        }
    }
}

/// Returns supported tools in platform/session-preferred order.
pub(crate) fn candidate_tools(env: ClipboardEnvironment) -> Vec<ClipboardTool> {
    let mut candidates = Vec::with_capacity(4);

    if env.os == ClipboardOs::Macos {
        push_unique(&mut candidates, PBCOPY);
    }
    if env.wayland {
        push_unique(&mut candidates, WL_COPY);
    }
    if env.x11 {
        push_unique(&mut candidates, XCLIP);
        push_unique(&mut candidates, XSEL);
    }

    for fallback in [PBCOPY, WL_COPY, XCLIP, XSEL] {
        push_unique(&mut candidates, fallback);
    }

    candidates
}

fn push_unique(candidates: &mut Vec<ClipboardTool>, tool: ClipboardTool) {
    if !candidates
        .iter()
        .any(|candidate| candidate.name == tool.name)
    {
        candidates.push(tool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(tools: Vec<ClipboardTool>) -> Vec<&'static str> {
        tools.into_iter().map(|tool| tool.name).collect()
    }

    #[test]
    fn macos_prefers_pbcopy() {
        let tools = candidate_tools(ClipboardEnvironment {
            os: ClipboardOs::Macos,
            wayland: true,
            x11: true,
        });

        assert_eq!(names(tools), vec!["pbcopy", "wl-copy", "xclip", "xsel"]);
    }

    #[test]
    fn wayland_prefers_wl_copy_before_generic_fallbacks() {
        let tools = candidate_tools(ClipboardEnvironment {
            os: ClipboardOs::Other,
            wayland: true,
            x11: false,
        });

        assert_eq!(names(tools), vec!["wl-copy", "pbcopy", "xclip", "xsel"]);
    }

    #[test]
    fn x11_prefers_xclip_then_xsel() {
        let tools = candidate_tools(ClipboardEnvironment {
            os: ClipboardOs::Other,
            wayland: false,
            x11: true,
        });

        assert_eq!(names(tools), vec!["xclip", "xsel", "pbcopy", "wl-copy"]);
    }
}

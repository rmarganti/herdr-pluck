use super::error::UrlOpenError;

/**
 * Platform URL launcher command.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UrlOpenTool {
    pub(crate) name: &'static str,
}

/**
 * Operating-system family used to choose the URL launcher.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UrlOpenEnvironment {
    Macos,
    Linux,
    Unsupported,
}

impl UrlOpenEnvironment {
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unsupported
        }
    }
}

impl UrlOpenTool {
    pub(crate) fn for_environment(environment: UrlOpenEnvironment) -> Result<Self, UrlOpenError> {
        match environment {
            UrlOpenEnvironment::Macos => Ok(Self { name: "open" }),
            UrlOpenEnvironment::Linux => Ok(Self { name: "xdg-open" }),
            UrlOpenEnvironment::Unsupported => Err(UrlOpenError::UnsupportedPlatform),
        }
    }
}

mod error;
mod runner;
mod system;
mod tool;

pub use error::UrlOpenError;

use runner::UrlOpenCommandRunner;
use system::SystemCommandRunner;
use tool::{UrlOpenEnvironment, UrlOpenTool};

/**
 * Successful system URL launch metadata.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenUrlSuccess {
    pub tool: String,
}

/**
 * Opens URLs through the system's default handler.
 */
pub trait UrlOpener {
    fn open(&self, url: &str) -> Result<OpenUrlSuccess, UrlOpenError>;
}

/**
 * Production URL opener using the platform's default launcher.
 */
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &str) -> Result<OpenUrlSuccess, UrlOpenError> {
        open_with_runner(url, &SystemCommandRunner, UrlOpenEnvironment::current())
    }
}

fn open_with_runner(
    url: &str,
    runner: &impl UrlOpenCommandRunner,
    env: UrlOpenEnvironment,
) -> Result<OpenUrlSuccess, UrlOpenError> {
    let tool = UrlOpenTool::for_environment(env)?;
    if !runner.command_exists(tool.name) {
        return Err(UrlOpenError::NoToolFound { tool: tool.name });
    }
    runner.run(tool, url)?;
    Ok(OpenUrlSuccess {
        tool: tool.name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeRunner {
        available: bool,
        runs: RefCell<Vec<(&'static str, String)>>,
        failure: Option<UrlOpenError>,
    }

    impl UrlOpenCommandRunner for FakeRunner {
        fn command_exists(&self, _command: &str) -> bool {
            self.available
        }

        fn run(&self, tool: UrlOpenTool, url: &str) -> Result<(), UrlOpenError> {
            self.runs.borrow_mut().push((tool.name, url.to_string()));
            self.failure.clone().map_or(Ok(()), Err)
        }
    }

    #[test]
    fn macos_uses_open_with_the_url_as_one_argument() {
        let runner = FakeRunner {
            available: true,
            ..FakeRunner::default()
        };
        let success = open_with_runner(
            "https://example.com/a b",
            &runner,
            UrlOpenEnvironment::Macos,
        )
        .unwrap();

        assert_eq!(success.tool, "open");
        assert_eq!(
            runner.runs.borrow()[0],
            ("open", "https://example.com/a b".into())
        );
    }

    #[test]
    fn linux_uses_xdg_open() {
        let runner = FakeRunner {
            available: true,
            ..FakeRunner::default()
        };
        open_with_runner("file:///tmp/test", &runner, UrlOpenEnvironment::Linux).unwrap();

        assert_eq!(runner.runs.borrow()[0].0, "xdg-open");
    }

    #[test]
    fn reports_missing_and_unsupported_launchers() {
        let error = open_with_runner(
            "https://example.com",
            &FakeRunner::default(),
            UrlOpenEnvironment::Linux,
        )
        .unwrap_err();
        assert_eq!(error, UrlOpenError::NoToolFound { tool: "xdg-open" });

        let error = open_with_runner(
            "https://example.com",
            &FakeRunner::default(),
            UrlOpenEnvironment::Unsupported,
        )
        .unwrap_err();
        assert_eq!(error, UrlOpenError::UnsupportedPlatform);
    }
}

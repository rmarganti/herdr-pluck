use crate::herdr::commands::{HerdrCommands, ProcessCommandRunner};
use crate::herdr::context::HerdrContext;
use crate::patterns::CustomPatternDefinition;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Herdr plugin id used for config directory discovery outside plugin action env.
pub const PLUGIN_ID: &str = "rmarganti.herdr-pluck";

const PATTERNS_FILE: &str = "patterns.toml";

/// User-editable global pattern configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PatternConfigFile {
    #[serde(default)]
    patterns: Vec<PatternConfigEntry>,
}

/// One custom regex pattern loaded from user configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PatternConfigEntry {
    name: String,
    regex: String,
    #[serde(default = "default_custom_priority")]
    priority: u16,
}

fn default_custom_priority() -> u16 {
    25
}

/// Loads global custom patterns, printing non-fatal config errors to stderr.
pub fn load_global_custom_patterns() -> Vec<CustomPatternDefinition> {
    match try_load_global_custom_patterns() {
        Ok(patterns) => patterns,
        Err(error) => {
            eprintln!("Herdr Pluck: failed to load custom patterns: {error:#}");
            Vec::new()
        }
    }
}

fn try_load_global_custom_patterns() -> Result<Vec<CustomPatternDefinition>> {
    let Some(config_dir) = global_config_dir()? else {
        return Ok(Vec::new());
    };
    load_patterns_file(&config_dir.join(PATTERNS_FILE))
}

fn global_config_dir() -> Result<Option<PathBuf>> {
    if let Some(path) = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        return Ok(Some(PathBuf::from(path)));
    }
    if cfg!(test) {
        return Ok(None);
    }

    let context = HerdrContext::from_env();
    let mut runner = ProcessCommandRunner;
    let mut commands = HerdrCommands::new(&context.herdr_bin, &mut runner);
    let path = commands.plugin_config_dir(PLUGIN_ID)?;

    if path.as_os_str().is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

fn load_patterns_file(path: &Path) -> Result<Vec<CustomPatternDefinition>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()))
        }
    };
    let config: PatternConfigFile =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;

    config
        .patterns
        .into_iter()
        .map(|entry| {
            let name = entry.name;
            CustomPatternDefinition::compile(name.clone(), entry.priority, &entry.regex)
                .with_context(|| format!("invalid regex for pattern {name}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_file_uses_default_priority_and_named_capture() {
        let dir = tempfile_dir();
        let path = dir.join(PATTERNS_FILE);
        std::fs::write(
            &path,
            r#"[[patterns]]
name = "ticket"
regex = "ABC-(?<match>[0-9]+)"
"#,
        )
        .unwrap();

        let patterns = load_patterns_file(&path).unwrap();

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].name(), "ticket");
        assert_eq!(patterns[0].priority(), 25);
    }

    fn tempfile_dir() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("herdr-pluck-config-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

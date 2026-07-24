use crate::model::PatternSpec;
use crate::patterns::CustomPatternDefinition;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Herdr plugin id used for config directory discovery outside plugin action env.
pub const PLUGIN_ID: &str = "rmarganti.herdr-pluck";

const CONFIG_FILE: &str = "config.toml";
const DEFAULT_PROJECT_CONFIG_FILE: &str = ".herdr-pluck.toml";

/// User-editable global Herdr Pluck configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct GlobalConfigFile {
    #[serde(default)]
    project: ProjectConfig,
    #[serde(default)]
    patterns: Vec<PatternConfigEntry>,
}

/// Project-local pattern discovery settings from global config.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ProjectConfig {
    #[serde(default = "default_project_patterns_enabled")]
    patterns: bool,
    #[serde(default = "default_project_pattern_files")]
    pattern_files: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            patterns: default_project_patterns_enabled(),
            pattern_files: default_project_pattern_files(),
        }
    }
}

/// One custom regex pattern loaded from user or project configuration.
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

fn default_project_patterns_enabled() -> bool {
    true
}

fn default_project_pattern_files() -> Vec<String> {
    vec![DEFAULT_PROJECT_CONFIG_FILE.to_string()]
}

/// Resolves picker pattern specs before launching the temporary picker pane.
pub fn resolve_pattern_specs(focused_pane_cwd: Option<&Path>) -> Vec<PatternSpec> {
    match try_resolve_pattern_specs(focused_pane_cwd) {
        Ok(patterns) => patterns,
        Err(error) => {
            eprintln!("Herdr Pluck: failed to load custom patterns: {error:#}");
            Vec::new()
        }
    }
}

/// Compiles snapshot-provided custom pattern specs, ignoring invalid entries.
pub fn compile_pattern_specs(specs: &[PatternSpec]) -> Vec<CustomPatternDefinition> {
    specs
        .iter()
        .filter_map(|spec| {
            CustomPatternDefinition::compile(spec.name.clone(), spec.priority, &spec.regex)
                .inspect_err(|error| {
                    eprintln!(
                        "Herdr Pluck: ignoring invalid pattern {}: {error}",
                        spec.name
                    );
                })
                .ok()
        })
        .collect()
}

fn try_resolve_pattern_specs(focused_pane_cwd: Option<&Path>) -> Result<Vec<PatternSpec>> {
    let global_config = load_global_config()?;
    let mut specs = Vec::new();

    if global_config.project.patterns {
        if let Some(cwd) = focused_pane_cwd {
            specs.extend(load_project_pattern_specs(
                cwd,
                &global_config.project.pattern_files,
            ));
        }
    }
    specs.extend(entries_to_specs(global_config.patterns));
    Ok(specs)
}

fn load_global_config() -> Result<GlobalConfigFile> {
    let Some(config_dir) = global_config_dir()? else {
        return Ok(GlobalConfigFile {
            project: ProjectConfig::default(),
            patterns: Vec::new(),
        });
    };
    load_config_file(&config_dir.join(CONFIG_FILE)).map(|config| config.unwrap_or_default())
}

fn global_config_dir() -> Result<Option<PathBuf>> {
    if let Some(path) = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        return Ok(Some(PathBuf::from(path)));
    }
    Ok(None)
}

fn load_project_pattern_specs(cwd: &Path, pattern_files: &[String]) -> Vec<PatternSpec> {
    let Some(git_root) = find_git_root(cwd) else {
        return Vec::new();
    };

    for dir in ancestors_until(cwd, &git_root) {
        for file_name in pattern_files {
            let path = dir.join(file_name);
            match load_config_file(&path) {
                Ok(Some(config)) => return entries_to_specs(config.patterns),
                Ok(None) => continue,
                Err(error) => {
                    eprintln!(
                        "Herdr Pluck: ignoring project pattern config {}: {error:#}",
                        path.display()
                    );
                    return Vec::new();
                }
            }
        }
    }

    Vec::new()
}

fn load_config_file(path: &Path) -> Result<Option<GlobalConfigFile>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()))
        }
    };
    toml::from_str(&content)
        .map(Some)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn entries_to_specs(entries: Vec<PatternConfigEntry>) -> Vec<PatternSpec> {
    entries
        .into_iter()
        .map(|entry| PatternSpec {
            name: entry.name,
            regex: entry.regex,
            priority: entry.priority,
        })
        .collect()
}

fn find_git_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

fn ancestors_until<'a>(cwd: &'a Path, root: &'a Path) -> impl Iterator<Item = &'a Path> {
    cwd.ancestors()
        .take_while(move |ancestor| ancestor.starts_with(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_uses_default_priority_and_project_settings() {
        let dir = tempfile_dir("config-default-priority");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            r#"[[patterns]]
name = "ticket"
regex = "ABC-(?<match>[0-9]+)"
"#,
        )
        .unwrap();

        let config = load_config_file(&path).unwrap().unwrap();
        let specs = entries_to_specs(config.patterns);

        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "ticket");
        assert_eq!(specs[0].priority, 25);
        assert!(config.project.patterns);
        assert_eq!(config.project.pattern_files, vec![".herdr-pluck.toml"]);
    }

    #[test]
    fn project_config_is_discovered_up_to_git_root() {
        let root = tempfile_dir("project-discovery");
        std::fs::create_dir(root.join(".git")).unwrap();
        let nested = root.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join(DEFAULT_PROJECT_CONFIG_FILE),
            r#"[[patterns]]
name = "project"
regex = "PROJECT-[0-9]+"
"#,
        )
        .unwrap();

        let specs = load_project_pattern_specs(&nested, &[DEFAULT_PROJECT_CONFIG_FILE.to_string()]);

        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "project");
    }

    #[test]
    fn invalid_project_config_is_ignored() {
        let root = tempfile_dir("invalid-project");
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::write(root.join(DEFAULT_PROJECT_CONFIG_FILE), "not toml =").unwrap();

        let specs = load_project_pattern_specs(&root, &[DEFAULT_PROJECT_CONFIG_FILE.to_string()]);

        assert!(specs.is_empty());
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("herdr-pluck-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

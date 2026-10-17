use crate::model::{ClipboardBackend, PatternSpec, PickerAction, PickerSettings};
use crate::patterns::CustomPatternDefinition;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Herdr plugin id used for config directory discovery outside plugin action env.
pub const PLUGIN_ID: &str = "rmarganti.herdr-pluck";

const CONFIG_FILE: &str = "config.toml";
const DEFAULT_PROJECT_CONFIG_FILE: &str = ".herdr-pluck.toml";

/// User-editable global Herdr Pluck configuration file.
///
/// Project-local files share this shape, but only their `patterns` are used.
///
/// Unknown fields are rejected so typos and keys misplaced under a table header (for
/// example `clipboard` after `[[patterns]]`) fail loudly instead of being ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalConfigFile {
    #[serde(default)]
    clipboard: ClipboardBackend,
    #[serde(default)]
    project: ProjectConfig,
    #[serde(default)]
    patterns: Vec<PatternConfigEntry>,
}

/// Project-local pattern discovery settings from global config.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

/// Resolves picker settings for an action before launching the temporary picker pane.
///
/// Config is only read for copying; a config that fails to load falls back to defaults.
pub fn resolve_picker_settings(
    action: PickerAction,
    focused_pane_cwd: Option<&Path>,
) -> PickerSettings {
    match action {
        PickerAction::Copy => load_global_config()
            .map(|config| copy_settings(config, focused_pane_cwd))
            .unwrap_or_else(|error| {
                eprintln!("Herdr Pluck: failed to load config, using defaults: {error:#}");
                PickerSettings::new(action)
            }),
        PickerAction::OpenUrl => PickerSettings::new(action),
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

fn copy_settings(
    global_config: GlobalConfigFile,
    focused_pane_cwd: Option<&Path>,
) -> PickerSettings {
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
    PickerSettings {
        action: PickerAction::Copy,
        custom_patterns: specs,
        clipboard: global_config.clipboard,
    }
}

fn load_global_config() -> Result<GlobalConfigFile> {
    let Some(config_dir) = global_config_dir()? else {
        return Ok(GlobalConfigFile::default());
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
    fn clipboard_defaults_to_osc52() {
        let config: GlobalConfigFile = toml::from_str("").unwrap();

        assert_eq!(
            copy_settings(config, None).clipboard,
            ClipboardBackend::Osc52
        );
    }

    #[test]
    fn clipboard_system_is_carried_into_copy_settings() {
        let config: GlobalConfigFile = toml::from_str(
            r#"clipboard = "system"

[[patterns]]
name = "ticket"
regex = "ABC-[0-9]+"
"#,
        )
        .unwrap();

        let settings = copy_settings(config, None);

        assert_eq!(settings.action, PickerAction::Copy);
        assert_eq!(settings.clipboard, ClipboardBackend::System);
        assert_eq!(settings.custom_patterns.len(), 1);
    }

    #[test]
    fn unknown_clipboard_backend_is_rejected() {
        let error = toml::from_str::<GlobalConfigFile>(r#"clipboard = "pbcopy""#).unwrap_err();

        assert!(error.to_string().contains("unknown variant `pbcopy`"));
    }

    #[test]
    fn clipboard_under_a_pattern_header_is_rejected() {
        let error = toml::from_str::<GlobalConfigFile>(
            r#"[[patterns]]
name = "ticket"
regex = "ABC-[0-9]+"
clipboard = "system"
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field `clipboard`"));
    }

    #[test]
    fn unknown_top_level_and_project_keys_are_rejected() {
        assert!(toml::from_str::<GlobalConfigFile>("clipbaord = \"system\"").is_err());
        assert!(toml::from_str::<GlobalConfigFile>("[project]\npaterns = false").is_err());
    }

    #[test]
    fn open_url_settings_use_defaults_without_reading_config() {
        assert_eq!(
            resolve_picker_settings(PickerAction::OpenUrl, None),
            PickerSettings::new(PickerAction::OpenUrl)
        );
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

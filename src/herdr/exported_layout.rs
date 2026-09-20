use crate::herdr::client::LaunchLayoutNode;
use crate::model::{PaneId, SplitDirection};
use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Portable tab layout returned by Herdr's `layout.export` API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExportedLayout {
    pub workspace_id: String,
    pub tab_id: String,
    pub zoomed: bool,
    pub focused_pane_id: PaneId,
    pub root: ExportedLayoutNode,
}

/// Portable BSP tree whose leaves retain source-pane launch metadata.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportedLayoutNode {
    Pane {
        pane_id: Option<PaneId>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        command: Option<Vec<String>>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<ExportedLayoutNode>,
        second: Box<ExportedLayoutNode>,
    },
}

impl ExportedLayoutNode {
    /// Replaces every exported command without ever replaying source argv.
    pub fn replace_commands<F>(&self, command_for: &mut F) -> Result<LaunchLayoutNode>
    where
        F: FnMut(&PaneId) -> Vec<String>,
    {
        match self {
            Self::Pane {
                pane_id,
                label,
                cwd,
                env,
                ..
            } => {
                let source_pane_id = pane_id
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("layout.export pane is missing pane_id"))?;
                Ok(LaunchLayoutNode::Pane {
                    command: command_for(&source_pane_id),
                    source_pane_id: Some(source_pane_id),
                    label: label.clone(),
                    cwd: cwd.clone(),
                    env: env.clone(),
                })
            }
            Self::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                if !ratio.is_finite() || !(0.0..=1.0).contains(ratio) {
                    bail!("layout.export split has invalid ratio {ratio}");
                }
                Ok(LaunchLayoutNode::Split {
                    direction: *direction,
                    ratio: *ratio,
                    first: Box::new(first.replace_commands(command_for)?),
                    second: Box::new(second.replace_commands(command_for)?),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_commands_and_preserves_nested_pane_metadata() {
        let exported = ExportedLayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.6,
            first: Box::new(ExportedLayoutNode::Pane {
                pane_id: Some(PaneId::new("source-1")),
                label: Some("editor".into()),
                cwd: Some("/repo".into()),
                command: Some(vec!["dangerous-source-command".into()]),
                env: HashMap::from([("ROLE".into(), "editor".into())]),
            }),
            second: Box::new(ExportedLayoutNode::Pane {
                pane_id: Some(PaneId::new("source-2")),
                label: None,
                cwd: Some("/tmp".into()),
                command: None,
                env: HashMap::new(),
            }),
        };

        let replay = exported
            .replace_commands(&mut |pane| vec!["pluck".into(), pane.0.clone()])
            .unwrap();
        let LaunchLayoutNode::Split { first, second, .. } = replay else {
            panic!("expected split");
        };
        assert!(matches!(
            first.as_ref(),
            LaunchLayoutNode::Pane { command, label, cwd, env, .. }
                if command == &vec!["pluck", "source-1"]
                    && label.as_deref() == Some("editor")
                    && cwd.as_deref() == Some("/repo")
                    && env.get("ROLE").map(String::as_str) == Some("editor")
        ));
        assert!(matches!(
            second.as_ref(),
            LaunchLayoutNode::Pane { command, .. }
                if command == &vec!["pluck", "source-2"]
        ));
    }

    #[test]
    fn rejects_leaves_without_source_identity() {
        let exported = ExportedLayoutNode::Pane {
            pane_id: None,
            label: None,
            cwd: None,
            command: Some(vec!["must-not-run".into()]),
            env: HashMap::new(),
        };
        assert!(exported.replace_commands(&mut |_| vec![]).is_err());
    }
}

use crate::herdr::client::{AppliedLayout, LaunchLayoutNode};
use crate::herdr::exported_layout::ExportedLayout;
use crate::herdr::layout::LayoutSnapshot;
use crate::model::{PaneId, SplitDirection};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(crate) struct Request<'a, P> {
    pub id: String,
    pub method: &'a str,
    pub params: P,
}

#[derive(Debug, Serialize)]
pub(crate) struct PaneTarget<'a> {
    pane_id: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct TabTarget<'a> {
    tab_id: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct PaneReadParams<'a> {
    pane_id: &'a str,
    source: &'static str,
    lines: u32,
    format: &'static str,
    strip_ansi: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct PaneZoomParams<'a> {
    pane_id: &'a str,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct LayoutApplyParams<'a> {
    workspace_id: &'a str,
    tab_label: &'a str,
    focus: bool,
    root: WireLayoutNode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireLayoutNode {
    Pane {
        command: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<WireLayoutNode>,
        second: Box<WireLayoutNode>,
    },
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    id: String,
    result: Option<T>,
    error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PaneLayoutResult {
    PaneLayout { layout: LayoutSnapshot },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PaneReadResult {
    PaneRead { read: ReadBody },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LayoutExportResult {
    LayoutExport { layout: ExportedLayout },
}

#[derive(Debug, Deserialize)]
struct ReadBody {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LayoutApplyResult {
    LayoutApply { layout: AppliedLayoutBody },
}

#[derive(Debug, Deserialize)]
struct AppliedLayoutBody {
    tab_id: String,
    root: AppliedNode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AppliedNode {
    Pane {
        pane_id: String,
    },
    Split {
        first: Box<AppliedNode>,
        second: Box<AppliedNode>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PaneInfoResult {
    PaneInfo {},
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PaneZoomResult {
    PaneZoom {},
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TabInfoResult {
    TabInfo {},
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OkResult {
    Ok {},
}

pub(crate) fn request<P>(id: String, method: &str, params: P) -> Request<'_, P> {
    Request { id, method, params }
}

pub(crate) fn pane_target(pane_id: &str) -> PaneTarget<'_> {
    PaneTarget { pane_id }
}

pub(crate) fn tab_target(tab_id: &str) -> TabTarget<'_> {
    TabTarget { tab_id }
}

pub(crate) fn pane_read_params(pane_id: &str, lines: u16) -> PaneReadParams<'_> {
    PaneReadParams {
        pane_id,
        source: "visible",
        lines: u32::from(lines),
        format: "text",
        strip_ansi: true,
    }
}

pub(crate) fn pane_zoom_params(pane_id: &str) -> PaneZoomParams<'_> {
    PaneZoomParams {
        pane_id,
        mode: "on",
    }
}

pub(crate) fn layout_apply_params<'a>(
    workspace_id: &'a str,
    tab_label: &'a str,
    root: &LaunchLayoutNode,
) -> LayoutApplyParams<'a> {
    LayoutApplyParams {
        workspace_id,
        tab_label,
        focus: true,
        root: WireLayoutNode::from(root),
    }
}

pub(crate) fn pane_layout(value: Value, id: &str) -> Result<LayoutSnapshot> {
    match decode::<PaneLayoutResult>(value, id)? {
        PaneLayoutResult::PaneLayout { layout } => Ok(layout),
    }
}

pub(crate) fn exported_layout(value: Value, id: &str) -> Result<ExportedLayout> {
    match decode::<LayoutExportResult>(value, id)? {
        LayoutExportResult::LayoutExport { layout } => Ok(layout),
    }
}

pub(crate) fn pane_read(value: Value, id: &str) -> Result<String> {
    match decode::<PaneReadResult>(value, id)? {
        PaneReadResult::PaneRead { read } => Ok(read.text),
    }
}

pub(crate) fn pane_focused(value: Value, id: &str) -> Result<()> {
    decode::<PaneInfoResult>(value, id).map(|_| ())
}

pub(crate) fn pane_zoomed(value: Value, id: &str) -> Result<()> {
    decode::<PaneZoomResult>(value, id).map(|_| ())
}

pub(crate) fn tab_focused(value: Value, id: &str) -> Result<()> {
    decode::<TabInfoResult>(value, id).map(|_| ())
}

pub(crate) fn tab_closed(value: Value, id: &str) -> Result<()> {
    decode::<OkResult>(value, id).map(|_| ())
}

pub(crate) fn applied_layout(
    value: Value,
    id: &str,
    submitted: &LaunchLayoutNode,
) -> Result<AppliedLayout> {
    let LayoutApplyResult::LayoutApply { layout } = decode(value, id)?;
    if layout.tab_id.is_empty() {
        bail!("layout.apply returned an empty tab id");
    }
    let mut pane_ids = HashMap::new();
    map_applied_panes(&layout.root, submitted, &mut pane_ids)?;
    let picker = find_picker_pane(&layout.root, submitted)
        .context("layout.apply response tree did not contain the picker pane")?;
    Ok(AppliedLayout {
        tab_id: layout.tab_id,
        picker_pane_id: PaneId::new(picker),
        pane_ids,
    })
}

fn decode<T: DeserializeOwned>(value: Value, id: &str) -> Result<T> {
    let envelope: Envelope<T> =
        serde_json::from_value(value).context("invalid Herdr response envelope")?;
    if envelope.id != id {
        bail!(
            "Herdr response id mismatch: expected {id}, got {}",
            envelope.id
        );
    }
    match (envelope.result, envelope.error) {
        (Some(result), None) => Ok(result),
        (None, Some(error)) => bail!("Herdr error {}: {}", error.code, error.message),
        (Some(_), Some(_)) => bail!("Herdr response contained both result and error"),
        (None, None) => bail!("Herdr response contained neither result nor error"),
    }
}

fn map_applied_panes(
    response: &AppliedNode,
    request: &LaunchLayoutNode,
    mapped: &mut HashMap<PaneId, PaneId>,
) -> Result<()> {
    match (response, request) {
        (AppliedNode::Pane { pane_id }, LaunchLayoutNode::Pane { source_pane_id, .. }) => {
            if pane_id.is_empty() {
                bail!("layout.apply returned an empty pane id");
            }
            if let Some(source) = source_pane_id {
                if mapped
                    .insert(source.clone(), PaneId::new(pane_id))
                    .is_some()
                {
                    bail!("layout.apply request contains duplicate source pane {source}");
                }
            }
            Ok(())
        }
        (
            AppliedNode::Split { first, second },
            LaunchLayoutNode::Split {
                first: requested_first,
                second: requested_second,
                ..
            },
        ) => {
            map_applied_panes(first, requested_first, mapped)?;
            map_applied_panes(second, requested_second, mapped)
        }
        _ => bail!("layout.apply response tree does not match submitted layout structure"),
    }
}

fn find_picker_pane(response: &AppliedNode, request: &LaunchLayoutNode) -> Option<String> {
    match (response, request) {
        (AppliedNode::Pane { pane_id }, LaunchLayoutNode::Pane { command, .. })
            if command.get(1).is_some_and(|arg| arg == "pick") =>
        {
            Some(pane_id.clone())
        }
        (
            AppliedNode::Split {
                first: response_first,
                second: response_second,
            },
            LaunchLayoutNode::Split {
                first: request_first,
                second: request_second,
                ..
            },
        ) => find_picker_pane(response_first, request_first)
            .or_else(|| find_picker_pane(response_second, request_second)),
        _ => None,
    }
}

impl From<&LaunchLayoutNode> for WireLayoutNode {
    fn from(value: &LaunchLayoutNode) -> Self {
        match value {
            LaunchLayoutNode::Pane {
                command,
                label,
                cwd,
                env,
                ..
            } => Self::Pane {
                command: command.clone(),
                label: label.clone(),
                cwd: cwd.clone(),
                env: env.clone(),
            },
            LaunchLayoutNode::Split {
                direction,
                ratio,
                first,
                second,
            } => Self::Split {
                direction: *direction,
                ratio: *ratio,
                first: Box::new(Self::from(first.as_ref())),
                second: Box::new(Self::from(second.as_ref())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_exported_nested_layout_and_metadata() {
        let layout = exported_layout(
            json!({"id":"x","result":{"type":"layout_export","layout":{
                "workspace_id":"w1","tab_id":"w1:t1","zoomed":true,
                "focused_pane_id":"w1:p2","root":{"type":"split","direction":"right","ratio":0.6,
                    "first":{"type":"pane","pane_id":"w1:p1","label":"editor","cwd":"/repo","command":["vim"],"env":{"ROLE":"editor"}},
                    "second":{"type":"pane","pane_id":"w1:p2"}
                }}}}),
            "x",
        )
        .unwrap();
        assert!(layout.zoomed);
        assert_eq!(layout.focused_pane_id, PaneId::new("w1:p2"));
        assert!(matches!(
            layout.root,
            crate::herdr::exported_layout::ExportedLayoutNode::Split { ratio, .. }
                if ratio == 0.6
        ));
    }

    #[test]
    fn preserves_declared_errors_and_checks_ids() {
        let error = pane_read(
            json!({"id":"x","error":{"code":"bad","message":"oops"}}),
            "x",
        )
        .unwrap_err();
        assert!(error.to_string().contains("bad") && error.to_string().contains("oops"));
        assert!(tab_closed(json!({"id":"wrong","result":{"type":"ok"}}), "x").is_err());
    }

    #[test]
    fn rejects_ambiguous_envelopes_and_wrong_result_types() {
        assert!(tab_closed(
            json!({"id":"x","result":{"type":"ok"},"error":{"code":"bad","message":"oops"}}),
            "x"
        )
        .unwrap_err()
        .to_string()
        .contains("both"));
        assert!(tab_closed(json!({"id":"x","result":{"type":"tab_info"}}), "x").is_err());
    }

    fn launch_pane(source: &str, role: &str) -> LaunchLayoutNode {
        LaunchLayoutNode::Pane {
            command: vec!["pluck".into(), role.into()],
            source_pane_id: Some(PaneId::new(source)),
            label: None,
            cwd: None,
            env: HashMap::new(),
        }
    }

    #[test]
    fn matches_all_pane_identities_by_submitted_tree_position() {
        let submitted = LaunchLayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.5,
            first: Box::new(launch_pane("source-1", "idle")),
            second: Box::new(launch_pane("source-2", "pick")),
        };
        let applied = applied_layout(
            json!({"id":"x","result":{"type":"layout_apply","layout":{"tab_id":"w:t2","root":{"type":"split","first":{"type":"pane","pane_id":"w:p1"},"second":{"type":"pane","pane_id":"w:p2"}}}}}),
            "x",
            &submitted,
        )
        .unwrap();
        assert_eq!(applied.tab_id, "w:t2");
        assert_eq!(applied.picker_pane_id, PaneId::new("w:p2"));
        assert_eq!(
            applied.pane_ids[&PaneId::new("source-1")],
            PaneId::new("w:p1")
        );
        assert_eq!(
            applied.pane_ids[&PaneId::new("source-2")],
            PaneId::new("w:p2")
        );
    }

    #[test]
    fn rejects_structurally_mismatched_apply_trees() {
        let submitted = LaunchLayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.5,
            first: Box::new(launch_pane("source-1", "idle")),
            second: Box::new(launch_pane("source-2", "pick")),
        };
        let error = applied_layout(
            json!({"id":"x","result":{"type":"layout_apply","layout":{"tab_id":"w:t2","root":{"type":"pane","pane_id":"w:p1"}}}}),
            "x",
            &submitted,
        )
        .unwrap_err();
        assert!(error.to_string().contains("does not match"));
    }
}

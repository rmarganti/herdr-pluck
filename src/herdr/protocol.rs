use crate::herdr::client::LaunchLayoutNode;
use crate::herdr::layout::LayoutSnapshot;
use crate::model::SplitDirection;
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

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
) -> Result<(String, String)> {
    let LayoutApplyResult::LayoutApply { layout } = decode(value, id)?;
    if layout.tab_id.is_empty() {
        bail!("layout.apply returned an empty tab id");
    }
    let picker = find_picker_pane(&layout.root, submitted)
        .context("layout.apply response tree did not contain the picker pane")?;
    Ok((layout.tab_id, picker))
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

fn find_picker_pane(response: &AppliedNode, request: &LaunchLayoutNode) -> Option<String> {
    match (response, request) {
        (AppliedNode::Pane { pane_id }, LaunchLayoutNode::Pane { command })
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
            LaunchLayoutNode::Pane { command } => Self::Pane {
                command: command.clone(),
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

    #[test]
    fn matches_picker_identity_by_submitted_tree_position() {
        let submitted = LaunchLayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.5,
            first: Box::new(LaunchLayoutNode::Pane {
                command: vec!["pluck".into(), "idle".into()],
            }),
            second: Box::new(LaunchLayoutNode::Pane {
                command: vec!["pluck".into(), "pick".into()],
            }),
        };
        let applied = applied_layout(
            json!({"id":"x","result":{"type":"layout_apply","layout":{"tab_id":"w:t2","root":{"type":"split","first":{"type":"pane","pane_id":"w:p1"},"second":{"type":"pane","pane_id":"w:p2"}}}}}),
            "x",
            &submitted,
        )
        .unwrap();
        assert_eq!(applied, ("w:t2".into(), "w:p2".into()));
    }
}

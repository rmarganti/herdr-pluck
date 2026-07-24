use crate::herdr::context::HerdrContext;
use crate::model::SplitDirection;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// A minimal argv-backed Herdr layout node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ApplyLayoutNode {
    Pane {
        command: Vec<String>,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<ApplyLayoutNode>,
        second: Box<ApplyLayoutNode>,
    },
}

/// Parameters consumed by Herdr's `layout.apply` method.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutApplyParams {
    pub workspace_id: String,
    pub tab_label: String,
    pub focus: bool,
    pub root: ApplyLayoutNode,
}

/// One newline-delimited `layout.apply` request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutApplyRequest {
    pub id: String,
    pub method: &'static str,
    pub params: LayoutApplyParams,
}

/// Identity of the tab created by an applied layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedLayout {
    pub tab_id: String,
    /// Pane created for the picker leaf in the submitted tree.
    pub picker_pane_id: String,
}

/// Narrow seam for atomically creating an argv-backed layout.
pub trait LayoutApplier {
    fn apply_layout(&mut self, request: &LayoutApplyRequest) -> Result<AppliedLayout>;

    /// Focuses the picker leaf before its launch barrier is released.
    fn focus_pane(&mut self, pane_id: &str) -> Result<()>;
}

/// Production newline-delimited JSON socket adapter.
pub struct UnixSocketLayoutApplier {
    socket_path: PathBuf,
}

impl UnixSocketLayoutApplier {
    pub fn from_context(context: &HerdrContext) -> Result<Self> {
        let socket_path = context.socket_path.clone().context(
            "HERDR_SOCKET_PATH is missing; Herdr Pluck requires Herdr 0.7.4+ socket context",
        )?;
        Ok(Self { socket_path })
    }
}

impl LayoutApplier for UnixSocketLayoutApplier {
    fn apply_layout(&mut self, request: &LayoutApplyRequest) -> Result<AppliedLayout> {
        let mut stream = UnixStream::connect(&self.socket_path).with_context(|| {
            format!(
                "failed to connect to Herdr socket {}",
                self.socket_path.display()
            )
        })?;
        let timeout = Some(Duration::from_secs(10));
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;
        serde_json::to_writer(&mut stream, request)
            .context("failed to serialize layout.apply request")?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .context("failed to read layout.apply response")?;
        if line.is_empty() {
            bail!("Herdr socket closed before layout.apply response");
        }
        parse_response(&line, request)
    }

    fn focus_pane(&mut self, pane_id: &str) -> Result<()> {
        let id = format!("pluck-focus-{}", std::process::id());
        let request =
            serde_json::json!({"id": id, "method": "pane.focus", "params": {"pane_id": pane_id}});
        let mut stream = UnixStream::connect(&self.socket_path).with_context(|| {
            format!(
                "failed to connect to Herdr socket {}",
                self.socket_path.display()
            )
        })?;
        let timeout = Some(Duration::from_secs(10));
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;
        serde_json::to_writer(&mut stream, &request)?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line)?;
        if line.is_empty() {
            bail!("Herdr socket closed before pane.focus response");
        }
        let response: serde_json::Value =
            serde_json::from_str(&line).context("malformed Herdr pane.focus response JSON")?;
        if response.get("id").and_then(|value| value.as_str()) != Some(id.as_str()) {
            bail!("pane.focus response id mismatch");
        }
        if let Some(error) = response.get("error") {
            bail!("Herdr pane.focus error: {error}");
        }
        if response.get("result").is_none() {
            bail!("pane.focus response contained neither result nor error");
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Response {
    id: String,
    result: Option<ResultBody>,
    error: Option<ErrorBody>,
}
#[derive(Deserialize)]
struct ResultBody {
    #[serde(rename = "type")]
    kind: String,
    layout: ResponseLayout,
}
#[derive(Deserialize)]
struct ResponseLayout {
    tab_id: String,
    root: ResponseNode,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ResponseNode {
    Pane {
        pane_id: String,
    },
    Split {
        first: Box<ResponseNode>,
        second: Box<ResponseNode>,
    },
}
#[derive(Deserialize)]
struct ErrorBody {
    code: serde_json::Value,
    message: String,
}

fn parse_response(line: &str, request: &LayoutApplyRequest) -> Result<AppliedLayout> {
    let response: Response =
        serde_json::from_str(line).context("malformed Herdr layout.apply response JSON")?;
    if response.id != request.id {
        bail!(
            "layout.apply response id mismatch: expected {}, got {}",
            request.id,
            response.id
        );
    }
    if let Some(error) = response.error {
        bail!("Herdr layout.apply error {}: {}", error.code, error.message);
    }
    let result = response
        .result
        .context("layout.apply response contained neither result nor error")?;
    if result.kind != "layout_apply" {
        bail!("unexpected layout.apply result type `{}`", result.kind);
    }
    if result.layout.tab_id.is_empty() {
        bail!("layout.apply returned an empty tab id");
    }
    let picker_pane_id = find_picker_pane(&result.layout.root, &request.params.root)
        .context("layout.apply response tree did not contain the picker pane")?;
    Ok(AppliedLayout {
        tab_id: result.layout.tab_id,
        picker_pane_id,
    })
}

fn find_picker_pane(response: &ResponseNode, request: &ApplyLayoutNode) -> Option<String> {
    match (response, request) {
        (ResponseNode::Pane { pane_id }, ApplyLayoutNode::Pane { command })
            if command.get(1).is_some_and(|arg| arg == "pick") =>
        {
            Some(pane_id.clone())
        }
        (
            ResponseNode::Split {
                first: rf,
                second: rs,
            },
            ApplyLayoutNode::Split {
                first: qf,
                second: qs,
                ..
            },
        ) => find_picker_pane(rf, qf).or_else(|| find_picker_pane(rs, qs)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> LayoutApplyRequest {
        LayoutApplyRequest {
            id: "x".into(),
            method: "layout.apply",
            params: LayoutApplyParams {
                workspace_id: "w".into(),
                tab_label: "Pluck".into(),
                focus: true,
                root: ApplyLayoutNode::Pane {
                    command: vec!["pluck".into(), "pick".into()],
                },
            },
        }
    }

    #[test]
    fn validates_responses_and_returns_picker_pane() {
        let applied = parse_response(r#"{"id":"x","result":{"type":"layout_apply","layout":{"tab_id":"t2","root":{"type":"pane","pane_id":"p2"}}}}"#, &request()).unwrap();
        assert_eq!(
            applied,
            AppliedLayout {
                tab_id: "t2".into(),
                picker_pane_id: "p2".into()
            }
        );
        let error = parse_response(
            r#"{"id":"x","error":{"code":"bad","message":"oops"}}"#,
            &request(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("bad") && error.contains("oops"));
        assert!(parse_response(r#"{"id":"y","result":{"type":"layout_apply","layout":{"tab_id":"t2","root":{"type":"pane","pane_id":"p2"}}}}"#, &request()).is_err());
    }
}

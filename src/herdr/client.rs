use crate::herdr::context::HerdrContext;
use crate::herdr::layout::LayoutSnapshot;
use crate::herdr::protocol;
use crate::herdr::socket::UnixSocketTransport;
use crate::model::PaneId;
use anyhow::{Context, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/**
 * Application-facing argv layout for a temporary Herdr tab.
 */
#[derive(Debug, Clone, PartialEq)]
pub enum LaunchLayoutNode {
    Pane {
        command: Vec<String>,
    },
    Split {
        direction: crate::model::SplitDirection,
        ratio: f32,
        first: Box<LaunchLayoutNode>,
        second: Box<LaunchLayoutNode>,
    },
}

/**
 * Identities created for the temporary picker layout.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedLayout {
    pub tab_id: String,
    pub picker_pane_id: PaneId,
}

/**
 * Domain seam for all Herdr operations used by picker orchestration.
 */
pub trait HerdrClient {
    fn pane_layout(&mut self, pane: &PaneId) -> Result<LayoutSnapshot>;
    fn pane_read_visible(&mut self, pane: &PaneId, lines: u16) -> Result<String>;
    fn apply_layout(
        &mut self,
        workspace_id: &str,
        tab_label: &str,
        root: &LaunchLayoutNode,
    ) -> Result<AppliedLayout>;
    fn focus_pane(&mut self, pane: &PaneId) -> Result<()>;
    fn zoom_pane(&mut self, pane: &PaneId) -> Result<()>;
    fn focus_tab(&mut self, tab_id: &str) -> Result<()>;
    fn close_tab(&mut self, tab_id: &str) -> Result<()>;
}

/**
 * Production Herdr client over the inherited Unix socket.
 */
#[derive(Debug, Clone)]
pub struct SocketHerdrClient {
    transport: UnixSocketTransport,
}

impl SocketHerdrClient {
    pub fn from_context(context: &HerdrContext) -> Result<Self> {
        let path = context
            .socket_path
            .clone()
            .context("HERDR_SOCKET_PATH is missing; Herdr Pluck requires Herdr 0.7.4 or newer")?;
        Ok(Self {
            transport: UnixSocketTransport::new(path),
        })
    }

    fn call(&self, method: &str, params: impl Serialize) -> Result<(String, serde_json::Value)> {
        let id = format!(
            "pluck-{}-{}",
            std::process::id(),
            REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let value = self
            .transport
            .exchange(&protocol::request(id.clone(), method, params))?;
        Ok((id, value))
    }
}

impl HerdrClient for SocketHerdrClient {
    fn pane_layout(&mut self, pane: &PaneId) -> Result<LayoutSnapshot> {
        let (id, value) = self.call("pane.layout", protocol::pane_target(&pane.0))?;
        protocol::pane_layout(value, &id)
    }

    fn pane_read_visible(&mut self, pane: &PaneId, lines: u16) -> Result<String> {
        let (id, value) = self.call("pane.read", protocol::pane_read_params(&pane.0, lines))?;
        protocol::pane_read(value, &id)
    }

    fn apply_layout(
        &mut self,
        workspace_id: &str,
        tab_label: &str,
        root: &LaunchLayoutNode,
    ) -> Result<AppliedLayout> {
        let (id, value) = self.call(
            "layout.apply",
            protocol::layout_apply_params(workspace_id, tab_label, root),
        )?;
        let (tab_id, picker_pane_id) = protocol::applied_layout(value, &id, root)?;
        Ok(AppliedLayout {
            tab_id,
            picker_pane_id: PaneId::new(picker_pane_id),
        })
    }

    fn focus_pane(&mut self, pane: &PaneId) -> Result<()> {
        let (id, value) = self.call("pane.focus", protocol::pane_target(&pane.0))?;
        protocol::pane_focused(value, &id)
    }

    fn zoom_pane(&mut self, pane: &PaneId) -> Result<()> {
        let (id, value) = self.call("pane.zoom", protocol::pane_zoom_params(&pane.0))?;
        protocol::pane_zoomed(value, &id)
    }

    fn focus_tab(&mut self, tab_id: &str) -> Result<()> {
        let (id, value) = self.call("tab.focus", protocol::tab_target(tab_id))?;
        protocol::tab_focused(value, &id)
    }

    fn close_tab(&mut self, tab_id: &str) -> Result<()> {
        let (id, value) = self.call("tab.close", protocol::tab_target(tab_id))?;
        protocol::tab_closed(value, &id)
    }
}

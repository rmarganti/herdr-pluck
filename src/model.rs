use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl PaneId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneDimensions {
    pub width: u16,
    pub height: u16,
}

/// Cell-space rectangle from Herdr layout or pane-local rendering coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns this rect after removing an equal border on all sides.
    pub fn inset(self, amount: u16) -> Self {
        let doubled = amount.saturating_mul(2);
        Self {
            x: self.x.saturating_add(amount),
            y: self.y.saturating_add(amount),
            width: self.width.saturating_sub(doubled),
            height: self.height.saturating_sub(doubled),
        }
    }

    /// Returns this rect with columns reserved from the right edge.
    pub fn reserve_right_gutter(self, amount: u16) -> Self {
        Self {
            width: self.width.saturating_sub(amount.min(self.width)),
            ..self
        }
    }

    /// Converts this rect from an absolute coordinate space to one relative to `origin`.
    pub fn relative_to(self, origin: Rect) -> Self {
        Self {
            x: self.x.saturating_sub(origin.x),
            y: self.y.saturating_sub(origin.y),
            ..self
        }
    }
}

/// Frozen pre-overlay source-pane geometry derived from Herdr-global layout coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceGeometrySnapshot {
    pub target_pane_id: PaneId,
    pub terminal_area: Rect,
    pub source_outer_rect: Rect,
    pub source_content_rect: Rect,
    pub pane_count: usize,
    pub zoomed: bool,
    pub target_focused: bool,
}

impl SourceGeometrySnapshot {
    /// Source content rect relative to Herdr's terminal area, excluding sidebar/tab-bar offsets.
    pub fn source_content_rect_in_terminal(&self) -> Rect {
        self.source_content_rect.relative_to(self.terminal_area)
    }

    /// Source outer rect relative to Herdr's terminal area, excluding sidebar/tab-bar offsets.
    pub fn source_outer_rect_in_terminal(&self) -> Rect {
        self.source_outer_rect.relative_to(self.terminal_area)
    }
}

/// How pane text was captured for a picker snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneTextCaptureMode {
    ExactVisibleUnwrapped,
    RecentUnwrappedBottomApproximation,
    VisibleWrapped,
}

/// One source pane's Herdr-global geometry captured before creating a temporary layout tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePaneGeometry {
    pub pane_id: PaneId,
    pub outer_rect: Rect,
    pub content_rect: Rect,
    pub content_width: u16,
    pub content_height: u16,
}

/// Immutable source tab state needed to launch and render a layout-tab picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePaneSnapshot {
    pub target_pane_id: PaneId,
    pub source_tab_id: String,
    pub workspace_id: String,
    pub source_panes: Vec<SourcePaneGeometry>,
    pub target_content_width: u16,
    pub target_content_height: u16,
    pub logical_lines: Vec<String>,
    pub visible_viewport: Option<VisibleViewport>,
    pub capture_mode: PaneTextCaptureMode,
}

/// Exact visible pane rows plus the logical lines reconstructed from soft wraps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleViewport {
    pub rows: Vec<String>,
    pub logical_lines: Vec<String>,
    pub segments: Vec<LogicalLineVisualSegment>,
}

/// Maps a logical byte range onto a row/column range in the exact visible viewport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalLineVisualSegment {
    pub logical_line: usize,
    pub logical_start: usize,
    pub logical_end: usize,
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize,
}

/// Temporary layout-tab session ids required for explicit cleanup and focus restoration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempTabSession {
    pub temp_tab_id: String,
    pub return_tab_id: String,
    pub return_pane_id: PaneId,
}

/// Full picker launch payload passed from the action process to picker mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickerSnapshot {
    pub source: SourcePaneSnapshot,
    pub session: TempTabSession,
}

/// Direction of a Herdr binary pane split as exposed by layout snapshots and replay commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    Right,
    Down,
}

impl SplitDirection {
    pub fn as_cli_arg(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

/// Binary Herdr layout tree with source pane ids preserved at leaves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutNode {
    Pane {
        source_pane_id: PaneId,
        rect: Rect,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
        rect: Rect,
    },
}

/// Replayable layout plan plus the source pane that must receive the picker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutRecreationPlan {
    pub root: LayoutNode,
    pub target_source_pane_id: PaneId,
}

/// Unwrapped logical pane text lines and dimensions at the time of picker activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneText {
    pub lines: Vec<String>,
    pub dimensions: PaneDimensions,
}

/// Copied/highlighted occurrence found on one unwrapped logical pane line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSpan {
    /// Zero-based logical line index.
    pub line: usize,
    /// UTF-8 byte offset where the copied/highlighted substring starts.
    pub start: usize,
    /// UTF-8 byte offset immediately after the copied/highlighted substring.
    pub end: usize,
    /// Copied text; for regexes with named capture `match`, this is that capture.
    pub text: String,
    /// Built-in pattern name that produced this occurrence.
    pub pattern: String,
    /// Match precedence where lower numbers are higher priority.
    pub priority: u16,
}

impl MatchSpan {
    /// Returns the matched byte length used by matcher tie-breaking.
    pub fn len_bytes(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns whether copied/highlighted byte ranges overlap on the same logical line.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.line == other.line && self.start < other.end && other.start < self.end
    }
}

/// A unique matched text pattern and all its occurrences in the pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintAssignment {
    pub hint: String,
    pub text: String,
    pub occurrences: Vec<MatchSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderStyle {
    Unmatched,
    Match,
    Hint,
}

/// A contiguous span of text to render in the picker, with a single style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderSpan {
    pub text: String,
    pub style: RenderStyle,
}

/// A single line of text to render in the picker,
/// with style spans for matched/highlighted regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderLine {
    pub spans: Vec<RenderSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickerOutcome {
    Copied { text: String },
    Cancelled,
    NoMatches,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyResult {
    Copied { tool: String },
    Failed { message: String },
}

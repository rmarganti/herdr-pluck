use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl PaneId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneDimensions {
    pub width: u16,
    pub height: u16,
}

/// Cell-space rectangle from Herdr layout or overlay rendering coordinates.
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
    Dim,
    Match,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderSpan {
    pub text: String,
    pub style: RenderStyle,
}

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

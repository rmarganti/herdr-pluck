use crate::config::compile_pattern_specs;
use crate::hints::{assign_hints, HintAssignments};
use crate::model::{
    PickerOutcome, PickerSnapshot, Rect, RenderLine, RenderSpan, RenderStyle, SourcePaneSnapshot,
};
use crate::patterns::find_matches;
use crate::renderer::{render_inline_hints, render_visible_inline_hints, terminal};
use anyhow::{Context, Result};

/// Rendered picker state and hint assignments derived from a captured pane snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerView {
    pub lines: Vec<RenderLine>,
    pub assignments: HintAssignments,
    pub match_count: usize,
}

impl PickerView {
    /// Number of unique copied texts that can be selected by hint input.
    pub fn hint_count(&self) -> usize {
        self.assignments.len()
    }
}

/// Rendered, readonly picker state derived from a captured pane snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadonlyPickerView {
    pub lines: Vec<RenderLine>,
    pub match_count: usize,
    pub hint_count: usize,
}

/// Maps the source pane's content rect into overlay-local coordinates.
///
/// Assumes the overlay pane covers the source tab area minus symmetric chrome
/// (borders/title), then clamps so the content never spills past the overlay.
pub fn place_content_in_overlay(
    source: &SourcePaneSnapshot,
    overlay_cols: u16,
    overlay_rows: u16,
) -> Rect {
    let inset_x = source.tab_area.width.saturating_sub(overlay_cols) / 2;
    let inset_y = source.tab_area.height.saturating_sub(overlay_rows) / 2;
    let content = source.target_content_rect.relative_to(source.tab_area);

    let width = source.target_content_width.min(overlay_cols);
    let height = source.target_content_height.min(overlay_rows);
    let x = content
        .x
        .saturating_sub(inset_x)
        .min(overlay_cols.saturating_sub(width));
    let y = content
        .y
        .saturating_sub(inset_y)
        .min(overlay_rows.saturating_sub(height));

    Rect::new(x, y, width, height)
}

/// Builds the production picker view from captured pane text.
pub fn build_picker_view(snapshot: &PickerSnapshot) -> PickerView {
    let logical_lines = snapshot
        .source
        .visible_viewport
        .as_ref()
        .map(|viewport| viewport.logical_lines.as_slice())
        .unwrap_or(&snapshot.source.logical_lines);
    let custom_patterns = compile_pattern_specs(&snapshot.custom_patterns);
    let matches = find_matches(logical_lines, &custom_patterns);
    let assignments = assign_hints(matches.clone());

    let lines = if assignments.is_empty() {
        no_matches_view(
            snapshot.source.target_content_width,
            snapshot.source.target_content_height,
        )
    } else if let Some(viewport) = &snapshot.source.visible_viewport {
        render_visible_inline_hints(
            viewport,
            &assignments,
            snapshot.source.target_content_width,
            snapshot.source.target_content_height,
        )
    } else {
        render_inline_hints(
            &snapshot.source.logical_lines,
            &assignments,
            snapshot.source.target_content_width,
            snapshot.source.target_content_height,
        )
    };

    PickerView {
        lines,
        assignments,
        match_count: matches.len(),
    }
}

/// Builds the production readonly picker view from captured pane text.
pub fn build_readonly_picker_view(snapshot: &PickerSnapshot) -> ReadonlyPickerView {
    let view = build_picker_view(snapshot);
    let hint_count = view.hint_count();
    ReadonlyPickerView {
        lines: view.lines,
        match_count: view.match_count,
        hint_count,
    }
}

/// Runs the readonly picker renderer and waits for an explicit close key.
pub fn run_readonly_picker(snapshot: &PickerSnapshot) -> Result<PickerOutcome> {
    use crossterm::event::{read, Event, KeyCode};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use std::io::{self, Write};

    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }

    let view = build_readonly_picker_view(snapshot);
    let mut stdout = io::stdout();
    terminal::emit_render_lines(&mut stdout, &view.lines)?;
    stdout.flush()?;

    enable_raw_mode().context("failed to enable raw mode for readonly picker")?;
    let _guard = RawModeGuard;
    loop {
        match read()? {
            Event::Key(key) if key.code != KeyCode::Enter => break,
            Event::Key(_) => continue,
            _ => continue,
        }
    }

    if view.hint_count == 0 {
        Ok(PickerOutcome::NoMatches)
    } else {
        Ok(PickerOutcome::Cancelled)
    }
}

fn no_matches_view(width: u16, height: u16) -> Vec<RenderLine> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let width = width as usize;
    let height = height as usize;
    let mut lines = Vec::with_capacity(height);
    let message = "Herdr Pluck: no copyable matches found";
    let hint = "Press any non-Enter key to close";

    for row in 0..height {
        let text = match row {
            0 => fit_to_width(message, width),
            2 if height > 2 => fit_to_width(hint, width),
            _ => " ".repeat(width),
        };
        lines.push(RenderLine {
            spans: vec![RenderSpan {
                text,
                style: RenderStyle::Unmatched,
            }],
        });
    }

    lines
}

fn fit_to_width(text: &str, width: usize) -> String {
    let mut output = text.chars().take(width).collect::<String>();
    let current_width = output.chars().count();
    if current_width < width {
        output.push_str(&" ".repeat(width - current_width));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PaneId, PaneTextCaptureMode, SourcePaneSnapshot};

    fn snapshot(lines: Vec<&str>, width: u16, height: u16) -> PickerSnapshot {
        PickerSnapshot {
            source: SourcePaneSnapshot {
                target_pane_id: PaneId::new("p1"),
                source_tab_id: "t1".to_string(),
                workspace_id: "w1".to_string(),
                tab_area: Rect::new(0, 0, width, height),
                target_content_rect: Rect::new(0, 0, width, height),
                target_content_width: width,
                target_content_height: height,
                logical_lines: lines.into_iter().map(str::to_string).collect(),
                visible_viewport: None,
                capture_mode: PaneTextCaptureMode::RecentUnwrappedBottomApproximation,
            },
            custom_patterns: Vec::new(),
        }
    }

    fn geometry_snapshot(tab_area: Rect, content: Rect) -> SourcePaneSnapshot {
        SourcePaneSnapshot {
            target_pane_id: PaneId::new("p1"),
            source_tab_id: "t1".to_string(),
            workspace_id: "w1".to_string(),
            tab_area,
            target_content_rect: content,
            target_content_width: content.width,
            target_content_height: content.height,
            logical_lines: Vec::new(),
            visible_viewport: None,
            capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
        }
    }

    #[test]
    fn placement_offsets_content_by_overlay_chrome_inset() {
        // Tab area 100x40 at (26,1); overlay tty is 98x38 → symmetric inset of 1.
        let source = geometry_snapshot(Rect::new(26, 1, 100, 40), Rect::new(31, 3, 60, 20));

        let placement = place_content_in_overlay(&source, 98, 38);

        assert_eq!(placement, Rect::new(4, 1, 60, 20));
    }

    #[test]
    fn placement_clamps_content_to_overlay_bounds() {
        // Content as wide as the tab area but overlay is smaller: clip and pin to origin.
        let source = geometry_snapshot(Rect::new(0, 0, 100, 40), Rect::new(0, 0, 100, 40));

        let placement = place_content_in_overlay(&source, 98, 38);

        assert_eq!(placement, Rect::new(0, 0, 98, 38));
    }

    #[test]
    fn placement_shifts_bottom_right_content_inside_overlay() {
        let source = geometry_snapshot(Rect::new(0, 0, 100, 40), Rect::new(60, 30, 40, 10));

        let placement = place_content_in_overlay(&source, 98, 38);

        assert_eq!(placement, Rect::new(58, 28, 40, 10));
    }

    #[test]
    fn readonly_view_renders_inline_hints_for_matches() {
        let view =
            build_readonly_picker_view(&snapshot(vec!["open https://example.com/path"], 40, 1));

        assert_eq!(view.match_count, 1);
        assert_eq!(view.hint_count, 1);
        assert_eq!(view.lines.len(), 1);
        assert!(view.lines[0]
            .spans
            .iter()
            .any(|span| span.style == RenderStyle::Hint && span.text == "a"));
    }

    #[test]
    fn readonly_view_reports_no_matches_with_full_size_message() {
        let view = build_readonly_picker_view(&snapshot(vec!["plain text only"], 20, 3));

        assert_eq!(view.match_count, 0);
        assert_eq!(view.hint_count, 0);
        assert_eq!(view.lines.len(), 3);
        assert_eq!(view.lines[0].spans[0].text.len(), 20);
        assert!(view.lines[0].spans[0].text.starts_with("Herdr Pluck"));
        assert!(view.lines[2].spans[0].text.starts_with("Press"));
    }
}

use crate::config::compile_pattern_specs;
use crate::hints::{assign_hints, HintAssignments};
use crate::model::{PickerOutcome, PickerSnapshot, RenderLine, RenderSpan, RenderStyle};
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
    use crate::model::{PaneId, PaneTextCaptureMode, SourcePaneSnapshot, TempTabSession};

    fn snapshot(lines: Vec<&str>, width: u16, height: u16) -> PickerSnapshot {
        PickerSnapshot {
            source: SourcePaneSnapshot {
                target_pane_id: PaneId::new("p1"),
                source_tab_id: "t1".to_string(),
                workspace_id: "w1".to_string(),
                source_panes: Vec::new(),
                target_content_width: width,
                target_content_height: height,
                logical_lines: lines.into_iter().map(str::to_string).collect(),
                visible_viewport: None,
                capture_mode: PaneTextCaptureMode::RecentUnwrappedBottomApproximation,
            },
            session: TempTabSession {
                temp_tab_id: "t2".to_string(),
                return_tab_id: "t1".to_string(),
                return_pane_id: PaneId::new("p1"),
            },
            custom_patterns: Vec::new(),
        }
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

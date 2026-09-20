use crate::config::compile_pattern_specs;
use crate::hints::{assign_hints, assign_tab_hints, HintAssignments, TabHintAssignments};
use crate::model::{
    PaneMatchSpan, PickerAction, PickerOutcome, PickerPaneSnapshot, PickerSnapshot, RenderLine,
    RenderSpan, RenderStyle, TabPickerSnapshot,
};
use crate::patterns::{find_matches, find_openable_urls};
use crate::picker::input::CursorGuard;
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

/// Render output for one pane in a tab-wide picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabPanePickerView {
    pub source_pane_id: crate::model::PaneId,
    pub lines: Vec<RenderLine>,
    pub match_count: usize,
}

/// Pane-partitioned rendering with one tab-wide input namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabPickerView {
    pub panes: Vec<TabPanePickerView>,
    pub assignments: TabHintAssignments,
    pub match_count: usize,
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
    let matches = match snapshot.action {
        PickerAction::Copy => {
            let custom_patterns = compile_pattern_specs(&snapshot.custom_patterns);
            find_matches(logical_lines, &custom_patterns)
        }
        PickerAction::OpenUrl => find_openable_urls(logical_lines),
    };
    let assignments = assign_hints(matches.clone());

    let lines = if assignments.is_empty() {
        no_matches_view(
            snapshot.action,
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

/// Builds pane-local render output while assigning and resolving hints tab-wide.
pub fn build_tab_picker_view(snapshot: &TabPickerSnapshot) -> TabPickerView {
    let custom_patterns = compile_pattern_specs(&snapshot.custom_patterns);
    let pane_matches = snapshot
        .panes
        .iter()
        .map(|pane| {
            let logical_lines = pane_logical_lines(pane);
            let matches = match snapshot.action {
                PickerAction::Copy => find_matches(logical_lines, &custom_patterns),
                PickerAction::OpenUrl => find_openable_urls(logical_lines),
            };
            (pane, matches)
        })
        .collect::<Vec<_>>();
    let match_count = pane_matches.iter().map(|(_, matches)| matches.len()).sum();
    let assignments = assign_tab_hints(
        pane_matches
            .iter()
            .flat_map(|(pane, matches)| {
                matches.iter().cloned().map(|span| PaneMatchSpan {
                    source_pane_id: pane.source_pane_id.clone(),
                    span,
                })
            })
            .collect(),
    );

    let panes = pane_matches
        .into_iter()
        .map(|(pane, matches)| {
            let local_assignments = assignments.for_pane(&pane.source_pane_id);
            let dimensions = pane.content_dimensions;
            let lines = if assignments.is_empty() {
                no_matches_view(snapshot.action, dimensions.width, dimensions.height)
            } else if let Some(viewport) = &pane.visible_viewport {
                render_visible_inline_hints(
                    viewport,
                    &local_assignments,
                    dimensions.width,
                    dimensions.height,
                )
            } else {
                render_inline_hints(
                    &pane.logical_lines,
                    &local_assignments,
                    dimensions.width,
                    dimensions.height,
                )
            };
            TabPanePickerView {
                source_pane_id: pane.source_pane_id.clone(),
                lines,
                match_count: matches.len(),
            }
        })
        .collect();

    TabPickerView {
        panes,
        assignments,
        match_count,
    }
}

fn pane_logical_lines(pane: &PickerPaneSnapshot) -> &[String] {
    pane.visible_viewport
        .as_ref()
        .map(|viewport| viewport.logical_lines.as_slice())
        .unwrap_or(&pane.logical_lines)
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
    let _cursor = CursorGuard::hide()?;
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

fn no_matches_view(action: PickerAction, width: u16, height: u16) -> Vec<RenderLine> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let width = width as usize;
    let height = height as usize;
    let mut lines = Vec::with_capacity(height);
    let message = match action {
        PickerAction::Copy => "Herdr Pluck: no copyable matches found",
        PickerAction::OpenUrl => "Herdr Pluck: no openable URLs found",
    };
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
    use crate::model::{
        PaneDimensions, PaneId, PaneTextCaptureMode, PickerReturnContext, SourcePaneSnapshot,
    };

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
            session: PickerReturnContext {
                return_tab_id: "t1".to_string(),
                return_pane_id: PaneId::new("p1"),
                zoom_picker: false,
            },
            action: PickerAction::Copy,
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
    fn open_url_view_assigns_hints_only_to_openable_urls() {
        let mut snapshot = snapshot(vec!["https://example.com ftp://host/repo /tmp/file"], 50, 1);
        snapshot.action = PickerAction::OpenUrl;

        let view = build_picker_view(&snapshot);

        assert_eq!(view.hint_count(), 1);
        assert_eq!(
            view.assignments.copied_text_for_hint("a"),
            Some("https://example.com")
        );
    }

    #[test]
    fn open_url_no_match_view_uses_action_specific_message() {
        let mut snapshot = snapshot(vec!["ftp://host/repo"], 40, 3);
        snapshot.action = PickerAction::OpenUrl;

        let view = build_picker_view(&snapshot);

        assert!(view.lines[0].spans[0].text.contains("no openable URLs"));
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

    fn tab_snapshot(panes: Vec<(&str, Vec<&str>)>) -> TabPickerSnapshot {
        TabPickerSnapshot {
            panes: panes
                .into_iter()
                .map(|(id, lines)| PickerPaneSnapshot {
                    source_pane_id: PaneId::new(id),
                    content_dimensions: PaneDimensions {
                        width: 30,
                        height: 1,
                    },
                    logical_lines: lines.into_iter().map(str::to_string).collect(),
                    visible_viewport: None,
                    capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
                })
                .collect(),
            session: PickerReturnContext {
                return_tab_id: "t1".to_string(),
                return_pane_id: PaneId::new("p1"),
                zoom_picker: false,
            },
            action: PickerAction::Copy,
            custom_patterns: Vec::new(),
        }
    }

    #[test]
    fn tab_view_assigns_unique_hints_and_resolves_other_panes() {
        let view = build_tab_picker_view(&tab_snapshot(vec![
            ("p1", vec!["first /tmp/one"]),
            ("p2", vec!["second /tmp/two"]),
        ]));

        assert_eq!(view.assignments.len(), 2);
        assert_eq!(view.assignments.copied_text_for_hint("a"), Some("/tmp/one"));
        assert_eq!(view.assignments.copied_text_for_hint("s"), Some("/tmp/two"));
        assert_eq!(view.assignments.for_pane(&PaneId::new("p1")).len(), 1);
        assert_eq!(view.assignments.for_pane(&PaneId::new("p2")).len(), 1);
    }

    #[test]
    fn duplicate_text_shares_a_hint_across_panes() {
        let view = build_tab_picker_view(&tab_snapshot(vec![
            ("p1", vec!["/tmp/shared"]),
            ("p2", vec!["again /tmp/shared"]),
        ]));

        assert_eq!(view.match_count, 2);
        assert_eq!(view.assignments.len(), 1);
        assert_eq!(view.assignments.assignments()[0].occurrences.len(), 2);
        assert_eq!(view.assignments.for_pane(&PaneId::new("p1")).len(), 1);
        assert_eq!(view.assignments.for_pane(&PaneId::new("p2")).len(), 1);
    }

    #[test]
    fn pane_without_matches_keeps_its_own_content() {
        let view = build_tab_picker_view(&tab_snapshot(vec![
            ("p1", vec!["/tmp/match"]),
            ("p2", vec!["wide 界 text"]),
            ("p3", Vec::new()),
        ]));

        assert_eq!(view.panes[1].match_count, 0);
        assert!(view.panes[1].lines[0].spans[0]
            .text
            .contains("wide 界 text"));
        assert_eq!(view.panes[2].lines.len(), 1);
        assert_eq!(view.panes[2].lines[0].spans[0].text.chars().count(), 30);
    }
}

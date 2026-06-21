pub mod terminal;

use crate::hints::HintAssignments;
use crate::model::{
    LogicalLineVisualSegment, MatchSpan, Rect, RenderLine, RenderSpan, RenderStyle, VisibleViewport,
};
use std::collections::HashSet;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Renders logical pane lines as a fixed-size styled viewport with destructive inline hints.
pub fn render_inline_hints(
    logical_lines: &[String],
    assignments: &HintAssignments,
    width: u16,
    height: u16,
) -> Vec<RenderLine> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let width = width as usize;
    let height = height as usize;
    let occurrences = collect_occurrences(logical_lines, assignments);
    let styled_lines = build_styled_logical_lines(logical_lines, &occurrences);
    let mut wrapped = wrap_styled_lines(&styled_lines, width);

    if wrapped.is_empty() {
        wrapped.push(blank_line(width));
    }

    if wrapped.len() > height {
        wrapped.split_off(wrapped.len() - height)
    } else {
        let mut padded = Vec::with_capacity(height);
        padded.extend((0..height - wrapped.len()).map(|_| blank_line(width)));
        padded.extend(wrapped);
        padded
    }
}

/// Renders exact visible rows with inline hints placed at their source row and column.
pub fn render_visible_inline_hints(
    viewport: &VisibleViewport,
    assignments: &HintAssignments,
    width: u16,
    height: u16,
) -> Vec<RenderLine> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let width = width as usize;
    let height = height as usize;
    let mut cells = viewport
        .rows
        .iter()
        .take(height)
        .map(|row| row_cells(row, width))
        .collect::<Vec<_>>();
    while cells.len() < height {
        cells.push(row_cells("", width));
    }

    for assignment in assignments.assignments() {
        for occurrence in &assignment.occurrences {
            apply_visible_occurrence(
                &mut cells,
                &viewport.rows,
                &viewport.segments,
                occurrence,
                &assignment.hint,
            );
        }
    }

    cells.into_iter().map(cells_to_render_line).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    text: String,
    style: RenderStyle,
}

fn row_cells(row: &str, width: usize) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(width);
    for ch in row.chars() {
        let char_width = ch.width().unwrap_or(0);
        if char_width == 0 {
            continue;
        }
        if cells.len() + char_width > width {
            break;
        }

        cells.push(Cell {
            text: ch.to_string(),
            style: RenderStyle::Unmatched,
        });
        for _ in 1..char_width {
            cells.push(Cell {
                text: String::new(),
                style: RenderStyle::Unmatched,
            });
        }
    }

    while cells.len() < width {
        cells.push(Cell {
            text: " ".to_string(),
            style: RenderStyle::Unmatched,
        });
    }
    cells
}

fn apply_visible_occurrence(
    cells: &mut [Vec<Cell>],
    rows: &[String],
    segments: &[LogicalLineVisualSegment],
    occurrence: &MatchSpan,
    hint: &str,
) {
    let mut positions = Vec::new();
    for segment in segments
        .iter()
        .filter(|segment| segment.logical_line == occurrence.line)
    {
        let start = occurrence.start.max(segment.logical_start);
        let end = occurrence.end.min(segment.logical_end);
        if start >= end {
            continue;
        }

        let Some(segment_text) = rows.get(segment.row) else {
            continue;
        };
        let start_in_segment = start.saturating_sub(segment.logical_start);
        let end_in_segment = end.saturating_sub(segment.logical_start);
        let Some(start_cols) = display_width_until_byte(segment_text, start_in_segment) else {
            continue;
        };
        let Some(end_cols) = display_width_until_byte(segment_text, end_in_segment) else {
            continue;
        };
        let col_start = segment.col_start + start_cols;
        let col_end = segment.col_start + end_cols;
        for col in col_start..col_end.min(segment.col_end) {
            positions.push((segment.row, col));
        }
    }

    if positions.is_empty() {
        return;
    }

    for &(row, col) in &positions {
        if let Some(cell) = cells.get_mut(row).and_then(|row| row.get_mut(col)) {
            cell.style = RenderStyle::Match;
        }
    }

    for ((row, col), hint_ch) in positions.into_iter().zip(hint.chars()) {
        if let Some(cell) = cells.get_mut(row).and_then(|row| row.get_mut(col)) {
            cell.text = hint_ch.to_string();
            cell.style = RenderStyle::Hint;
        }
    }
}

fn cells_to_render_line(cells: Vec<Cell>) -> RenderLine {
    let mut spans = Vec::new();
    for cell in cells {
        push_span(&mut spans, &cell.text, cell.style);
    }
    RenderLine { spans }
}

fn display_width_until_byte(text: &str, byte_offset: usize) -> Option<usize> {
    if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
        return None;
    }
    Some(UnicodeWidthStr::width(&text[..byte_offset]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderOccurrence {
    line: usize,
    start: usize,
    end: usize,
    hint: String,
}

fn collect_occurrences(
    logical_lines: &[String],
    assignments: &HintAssignments,
) -> Vec<RenderOccurrence> {
    let mut occurrences = Vec::new();

    for assignment in assignments.assignments() {
        for occurrence in &assignment.occurrences {
            if is_valid_occurrence(logical_lines, occurrence) {
                occurrences.push(RenderOccurrence {
                    line: occurrence.line,
                    start: occurrence.start,
                    end: occurrence.end,
                    hint: assignment.hint.clone(),
                });
            }
        }
    }

    occurrences.sort_by(|left, right| {
        (left.line, left.start, left.end).cmp(&(right.line, right.start, right.end))
    });

    let mut seen = HashSet::new();
    occurrences
        .into_iter()
        .filter(|occurrence| seen.insert((occurrence.line, occurrence.start, occurrence.end)))
        .collect()
}

fn is_valid_occurrence(logical_lines: &[String], occurrence: &MatchSpan) -> bool {
    let Some(line) = logical_lines.get(occurrence.line) else {
        return false;
    };

    occurrence.start <= occurrence.end
        && occurrence.end <= line.len()
        && line.is_char_boundary(occurrence.start)
        && line.is_char_boundary(occurrence.end)
}

/// Builds styled render lines from logical lines and their matched occurrences, applying destructive hints.
fn build_styled_logical_lines(
    logical_lines: &[String],
    occurrences: &[RenderOccurrence],
) -> Vec<RenderLine> {
    logical_lines
        .iter()
        .enumerate()
        .map(|(line_index, line)| {
            let mut spans = Vec::new();
            let mut cursor = 0;

            for occurrence in occurrences
                .iter()
                .filter(|occurrence| occurrence.line == line_index)
            {
                if occurrence.start < cursor {
                    continue;
                }

                push_span(
                    &mut spans,
                    &line[cursor..occurrence.start],
                    RenderStyle::Unmatched,
                );
                push_destructive_hint_spans(
                    &mut spans,
                    &line[occurrence.start..occurrence.end],
                    occurrence,
                );
                cursor = occurrence.end;
            }

            push_span(&mut spans, &line[cursor..], RenderStyle::Unmatched);
            RenderLine { spans }
        })
        .collect()
}

/// Pushes spans for a matched occurrence, replacing the match prefix with the hint if the hint is shorter.
fn push_destructive_hint_spans(
    spans: &mut Vec<RenderSpan>,
    matched_text: &str,
    occurrence: &RenderOccurrence,
) {
    let hint_width = occurrence.hint.chars().count();
    let Some(remainder_start) = byte_index_after_chars(matched_text, hint_width) else {
        push_span(spans, matched_text, RenderStyle::Match);
        return;
    };

    push_span(spans, &occurrence.hint, RenderStyle::Hint);
    push_span(spans, &matched_text[remainder_start..], RenderStyle::Match);
}

/// Returns the byte index immediately after the specified number of chars, or None if the text has fewer chars.
fn byte_index_after_chars(text: &str, count: usize) -> Option<usize> {
    if count == 0 {
        return Some(0);
    }

    text.char_indices()
        .nth(count)
        .map(|(index, _)| index)
        .or_else(|| (text.chars().count() == count).then_some(text.len()))
}

// Wraps styled lines to a given width, preserving styles and padding short lines with spaces.
fn wrap_styled_lines(lines: &[RenderLine], width: usize) -> Vec<RenderLine> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut wrapped = Vec::new();

    for line in lines {
        let mut current = RenderLine { spans: Vec::new() };
        let mut current_width = 0;
        let mut saw_content = false;

        for span in &line.spans {
            for ch in span.text.chars() {
                saw_content = true;
                let char_width = ch.width().unwrap_or(0);
                if current_width > 0 && current_width + char_width > width {
                    pad_line(&mut current, width, current_width);
                    wrapped.push(merge_render_line(current));
                    current = RenderLine { spans: Vec::new() };
                    current_width = 0;
                }

                push_char(&mut current.spans, ch, span.style);
                current_width += char_width;

                if current_width == width {
                    wrapped.push(merge_render_line(current));
                    current = RenderLine { spans: Vec::new() };
                    current_width = 0;
                }
            }
        }

        if !current.spans.is_empty() || !saw_content {
            pad_line(&mut current, width, current_width);
            wrapped.push(merge_render_line(current));
        }
    }

    wrapped
}

/// Pads the line with spaces to reach the target width, if it's currently shorter.
fn pad_line(line: &mut RenderLine, width: usize, current_width: usize) {
    if current_width < width {
        push_span(
            &mut line.spans,
            &" ".repeat(width - current_width),
            RenderStyle::Unmatched,
        );
    }
}

/// Creates a blank line of the specified width with unmatched style.
fn blank_line(width: usize) -> RenderLine {
    RenderLine {
        spans: vec![RenderSpan {
            text: " ".repeat(width),
            style: RenderStyle::Unmatched,
        }],
    }
}

/// Pushes a single character as a span, merging with the previous span if styles match.
fn push_char(spans: &mut Vec<RenderSpan>, ch: char, style: RenderStyle) {
    let mut buffer = [0; 4];
    push_span(spans, ch.encode_utf8(&mut buffer), style);
}

/// Pushes a text span into the line, merging with the previous span if styles match.
fn push_span(spans: &mut Vec<RenderSpan>, text: &str, style: RenderStyle) {
    if text.is_empty() {
        return;
    }

    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.text.push_str(text);
            return;
        }
    }

    spans.push(RenderSpan {
        text: text.to_string(),
        style,
    });
}

/// Merges adjacent spans with the same style into a single span.
fn merge_render_line(line: RenderLine) -> RenderLine {
    let mut spans = Vec::new();
    for span in line.spans {
        push_span(&mut spans, &span.text, span.style);
    }
    RenderLine { spans }
}

/// Places source-sized text into an overlay-local viewport, padding or clipping as needed.
pub fn compose_plain_into_overlay(
    source_lines: &[String],
    source_content_rect: Rect,
    overlay_content_rect: Rect,
    overlay_size: Rect,
) -> Vec<String> {
    let overlay_width = overlay_size.width as usize;
    let overlay_height = overlay_size.height as usize;
    let offset_x = source_content_rect.x as i32 - overlay_content_rect.x as i32;
    let offset_y = source_content_rect.y as i32 - overlay_content_rect.y as i32;

    (0..overlay_height)
        .map(|overlay_y| {
            let mut row = vec![' '; overlay_width];
            let source_y = overlay_y as i32 - offset_y;
            if source_y >= 0 && source_y < source_lines.len() as i32 {
                for (source_x, ch) in source_lines[source_y as usize].chars().enumerate() {
                    let overlay_x = source_x as i32 + offset_x;
                    if overlay_x >= 0 && overlay_x < overlay_width as i32 {
                        row[overlay_x as usize] = ch;
                    }
                }
            }
            row.into_iter().collect()
        })
        .collect()
}

/// Places rendered source lines into an overlay-local viewport, flattening styles temporarily.
pub fn compose_render_lines_into_overlay(
    source_lines: &[RenderLine],
    source_content_rect: Rect,
    overlay_content_rect: Rect,
    overlay_size: Rect,
) -> Vec<RenderLine> {
    let plain_source: Vec<String> = source_lines.iter().map(flatten_render_line).collect();
    compose_plain_into_overlay(
        &plain_source,
        source_content_rect,
        overlay_content_rect,
        overlay_size,
    )
    .into_iter()
    .map(|text| RenderLine {
        spans: vec![RenderSpan {
            text,
            style: RenderStyle::Unmatched,
        }],
    })
    .collect()
}

/// Flattens a render line into plain text by concatenating its spans, ignoring styles.
fn flatten_render_line(line: &RenderLine) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::assign_hints;
    use crate::model::HintAssignment;

    fn span(text: impl Into<String>, line: usize, start: usize) -> MatchSpan {
        let text = text.into();
        MatchSpan {
            line,
            start,
            end: start + text.len(),
            text,
            pattern: "test".to_string(),
            priority: 10,
        }
    }

    fn assignment(hint: &str, text: &str, occurrences: Vec<MatchSpan>) -> HintAssignment {
        HintAssignment {
            hint: hint.to_string(),
            text: text.to_string(),
            occurrences,
        }
    }

    fn assert_rows(lines: &[RenderLine], expected: &[&[(RenderStyle, &str)]]) {
        let actual: Vec<Vec<(RenderStyle, &str)>> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| (span.style, span.text.as_str()))
                    .collect()
            })
            .collect();
        let expected: Vec<Vec<(RenderStyle, &str)>> =
            expected.iter().map(|row| row.to_vec()).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn zero_dimensions_return_no_rows() {
        let assignments = HintAssignments::new(Vec::new());

        assert!(render_inline_hints(&["abc".to_string()], &assignments, 0, 3).is_empty());
        assert!(render_inline_hints(&["abc".to_string()], &assignments, 3, 0).is_empty());
    }

    #[test]
    fn visible_render_preserves_top_row_position() {
        let viewport = crate::viewport::map_visible_viewport(
            vec![
                "https://example.com".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            30,
            3,
        );
        let assignments = assign_hints(vec![span("https://example.com", 0, 0)]);

        let lines = render_visible_inline_hints(&viewport, &assignments, 30, 3);

        assert_rows(
            &lines,
            &[
                &[
                    (RenderStyle::Hint, "a"),
                    (RenderStyle::Match, "ttps://example.com"),
                    (RenderStyle::Unmatched, "           "),
                ],
                &[(RenderStyle::Unmatched, "                              ")],
                &[(RenderStyle::Unmatched, "                              ")],
            ],
        );
    }

    #[test]
    fn visible_render_maps_match_after_wide_utf8_prefix() {
        let row = "界 https://example.com";
        let viewport = crate::viewport::map_visible_viewport(vec![row.to_string()], 30, 1);
        let assignments = assign_hints(vec![MatchSpan {
            line: 0,
            start: "界 ".len(),
            end: row.len(),
            text: "https://example.com".to_string(),
            pattern: "url".to_string(),
            priority: 10,
        }]);

        let lines = render_visible_inline_hints(&viewport, &assignments, 30, 1);

        assert_rows(
            &lines,
            &[&[
                (RenderStyle::Unmatched, "界 "),
                (RenderStyle::Hint, "a"),
                (RenderStyle::Match, "ttps://example.com"),
                (RenderStyle::Unmatched, "        "),
            ]],
        );
    }

    #[test]
    fn visible_render_highlights_wrapped_match_across_rows() {
        let viewport = crate::viewport::map_visible_viewport(
            vec!["https://exa".to_string(), "mple.com".to_string()],
            11,
            2,
        );
        let assignments = assign_hints(vec![span("https://example.com", 0, 0)]);

        let lines = render_visible_inline_hints(&viewport, &assignments, 11, 2);

        assert_rows(
            &lines,
            &[
                &[(RenderStyle::Hint, "a"), (RenderStyle::Match, "ttps://exa")],
                &[
                    (RenderStyle::Match, "mple.com"),
                    (RenderStyle::Unmatched, "   "),
                ],
            ],
        );
    }

    #[test]
    fn empty_input_returns_blank_viewport() {
        let assignments = HintAssignments::new(Vec::new());
        let rows = render_inline_hints(&[], &assignments, 4, 2);

        assert_rows(
            &rows,
            &[
                &[(RenderStyle::Unmatched, "    ")],
                &[(RenderStyle::Unmatched, "    ")],
            ],
        );
    }

    #[test]
    fn destructive_hint_replaces_match_prefix() {
        let lines = vec!["foo bar baz".to_string()];
        let assignments =
            HintAssignments::new(vec![assignment("a", "bar", vec![span("bar", 0, 4)])]);

        let rows = render_inline_hints(&lines, &assignments, 11, 1);

        assert_rows(
            &rows,
            &[&[
                (RenderStyle::Unmatched, "foo "),
                (RenderStyle::Hint, "a"),
                (RenderStyle::Match, "ar"),
                (RenderStyle::Unmatched, " baz"),
            ]],
        );
    }

    #[test]
    fn duplicate_text_occurrences_render_same_hint() {
        let lines = vec!["foo then foo".to_string()];
        let assignments = assign_hints(vec![span("foo", 0, 0), span("foo", 0, 9)]);

        let rows = render_inline_hints(&lines, &assignments, 12, 1);

        assert_rows(
            &rows,
            &[&[
                (RenderStyle::Hint, "a"),
                (RenderStyle::Match, "oo"),
                (RenderStyle::Unmatched, " then "),
                (RenderStyle::Hint, "a"),
                (RenderStyle::Match, "oo"),
            ]],
        );
    }

    #[test]
    fn duplicate_exact_occurrence_is_rendered_once() {
        let lines = vec!["foo".to_string()];
        let assignments = HintAssignments::new(vec![assignment(
            "a",
            "foo",
            vec![span("foo", 0, 0), span("foo", 0, 0)],
        )]);

        let rows = render_inline_hints(&lines, &assignments, 3, 1);

        assert_rows(
            &rows,
            &[&[(RenderStyle::Hint, "a"), (RenderStyle::Match, "oo")]],
        );
    }

    #[test]
    fn wraps_styled_content_and_preserves_styles() {
        let lines = vec!["xx abcdef yy".to_string()];
        let assignments =
            HintAssignments::new(vec![assignment("a", "abcdef", vec![span("abcdef", 0, 3)])]);

        let rows = render_inline_hints(&lines, &assignments, 5, 3);

        assert_rows(
            &rows,
            &[
                &[
                    (RenderStyle::Unmatched, "xx "),
                    (RenderStyle::Hint, "a"),
                    (RenderStyle::Match, "b"),
                ],
                &[(RenderStyle::Match, "cdef"), (RenderStyle::Unmatched, " ")],
                &[(RenderStyle::Unmatched, "yy   ")],
            ],
        );
    }

    #[test]
    fn crops_to_bottom_viewport_after_wrapping() {
        let lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let assignments = HintAssignments::new(Vec::new());

        let rows = render_inline_hints(&lines, &assignments, 5, 2);

        assert_rows(
            &rows,
            &[
                &[(RenderStyle::Unmatched, "two  ")],
                &[(RenderStyle::Unmatched, "three")],
            ],
        );
    }

    #[test]
    fn top_pads_when_content_is_shorter_than_viewport() {
        let lines = vec!["hi".to_string()];
        let assignments = HintAssignments::new(Vec::new());

        let rows = render_inline_hints(&lines, &assignments, 4, 3);

        assert_rows(
            &rows,
            &[
                &[(RenderStyle::Unmatched, "    ")],
                &[(RenderStyle::Unmatched, "    ")],
                &[(RenderStyle::Unmatched, "hi  ")],
            ],
        );
    }

    #[test]
    fn invalid_occurrences_are_ignored() {
        let lines = vec!["abc".to_string()];
        let assignments = HintAssignments::new(vec![assignment(
            "a",
            "bad",
            vec![
                MatchSpan {
                    end: 5,
                    ..span("abc", 0, 0)
                },
                MatchSpan {
                    line: 9,
                    ..span("abc", 0, 0)
                },
            ],
        )]);

        let rows = render_inline_hints(&lines, &assignments, 3, 1);

        assert_rows(&rows, &[&[(RenderStyle::Unmatched, "abc")]]);
    }

    #[test]
    fn non_char_boundary_occurrence_is_ignored() {
        let lines = vec!["éx".to_string()];
        let assignments = HintAssignments::new(vec![assignment(
            "a",
            "bad",
            vec![MatchSpan {
                line: 0,
                start: 1,
                end: 2,
                text: "bad".to_string(),
                pattern: "test".to_string(),
                priority: 10,
            }],
        )]);

        let rows = render_inline_hints(&lines, &assignments, 3, 1);

        assert_rows(&rows, &[&[(RenderStyle::Unmatched, "éx ")]]);
    }

    #[test]
    fn short_match_renders_as_match_without_hint() {
        let lines = vec!["x a y".to_string()];
        let assignments = HintAssignments::new(vec![assignment("as", "a", vec![span("a", 0, 2)])]);

        let rows = render_inline_hints(&lines, &assignments, 5, 1);

        assert_rows(
            &rows,
            &[&[
                (RenderStyle::Unmatched, "x "),
                (RenderStyle::Match, "a"),
                (RenderStyle::Unmatched, " y"),
            ]],
        );
    }

    #[test]
    fn composition_does_not_add_sidebar_padding_when_rects_are_normalized() {
        let output = compose_plain_into_overlay(
            &["abc".to_string()],
            Rect::new(0, 0, 3, 1),
            Rect::new(0, 0, 10, 2),
            Rect::new(0, 0, 10, 2),
        );

        assert_eq!(output[0], "abc       ");
    }

    #[test]
    fn composition_pads_source_to_its_position_in_larger_overlay() {
        let output = compose_plain_into_overlay(
            &["abc".to_string()],
            Rect::new(3, 2, 3, 1),
            Rect::new(0, 0, 10, 5),
            Rect::new(0, 0, 10, 5),
        );

        assert_eq!(output[0], "          ");
        assert_eq!(output[1], "          ");
        assert_eq!(output[2], "   abc    ");
    }

    #[test]
    fn composition_crops_source_left_and_right_to_overlay_viewport() {
        let output = compose_plain_into_overlay(
            &["abcdef".to_string()],
            Rect::new(0, 0, 6, 1),
            Rect::new(2, 0, 4, 1),
            Rect::new(0, 0, 4, 1),
        );

        assert_eq!(output, vec!["cdef".to_string()]);
    }

    #[test]
    fn composition_crops_source_above_overlay_viewport() {
        let output = compose_plain_into_overlay(
            &["top".to_string(), "bottom".to_string()],
            Rect::new(0, 0, 6, 2),
            Rect::new(0, 1, 6, 1),
            Rect::new(0, 0, 6, 1),
        );

        assert_eq!(output, vec!["bottom".to_string()]);
    }
}

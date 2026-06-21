use crate::hints::display_width;
use crate::model::{LogicalLineVisualSegment, VisibleViewport};

/// Builds logical lines from exact visible terminal rows while preserving row/column mapping.
pub fn map_visible_viewport(rows: Vec<String>, width: u16, height: u16) -> VisibleViewport {
    let width = width as usize;
    let height = height as usize;
    let mut rows = rows;
    rows.truncate(height);
    while rows.len() < height {
        rows.push(String::new());
    }

    let mut logical_lines = Vec::new();
    let mut segments = Vec::new();
    let mut current_line = String::new();
    let mut current_segments = Vec::new();

    for (row_index, row) in rows.iter().enumerate() {
        let start = current_line.len();
        current_line.push_str(row);
        let end = current_line.len();
        current_segments.push(LogicalLineVisualSegment {
            logical_line: logical_lines.len(),
            logical_start: start,
            logical_end: end,
            row: row_index,
            col_start: 0,
            col_end: display_width(row),
        });

        if width == 0 || display_width(row) < width {
            logical_lines.push(std::mem::take(&mut current_line));
            segments.append(&mut current_segments);
        }
    }

    if !current_segments.is_empty() {
        logical_lines.push(current_line);
        segments.append(&mut current_segments);
    }

    VisibleViewport {
        rows,
        logical_lines,
        segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_top_row_position_for_short_visible_line() {
        let viewport = map_visible_viewport(vec!["https://example.com".to_string()], 80, 3);

        assert_eq!(viewport.rows.len(), 3);
        assert_eq!(viewport.logical_lines[0], "https://example.com");
        assert_eq!(viewport.segments[0].row, 0);
    }

    #[test]
    fn segment_columns_use_display_width() {
        let viewport = map_visible_viewport(vec!["界x".to_string()], 10, 1);

        assert_eq!(viewport.segments[0].logical_end, "界x".len());
        assert_eq!(viewport.segments[0].col_end, 3);
    }

    #[test]
    fn joins_full_width_rows_for_wrapped_logical_line() {
        let viewport = map_visible_viewport(
            vec!["https://exa".to_string(), "mple.com".to_string()],
            11,
            2,
        );

        assert_eq!(viewport.logical_lines, vec!["https://example.com"]);
        assert_eq!(viewport.segments.len(), 2);
        assert_eq!(viewport.segments[0].logical_start, 0);
        assert_eq!(viewport.segments[0].logical_end, 11);
        assert_eq!(viewport.segments[1].logical_start, 11);
        assert_eq!(viewport.segments[1].logical_end, 19);
        assert_eq!(viewport.segments[1].row, 1);
    }
}

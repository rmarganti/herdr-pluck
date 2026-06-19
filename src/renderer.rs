use crate::model::{HintAssignment, Rect, RenderLine, RenderSpan, RenderStyle};

pub fn render_placeholder(assignments: &[HintAssignment]) -> Vec<RenderLine> {
    assignments
        .iter()
        .map(|assignment| RenderLine {
            spans: vec![
                RenderSpan {
                    text: assignment.hint.clone(),
                    style: RenderStyle::Hint,
                },
                RenderSpan {
                    text: assignment.text.clone(),
                    style: RenderStyle::Match,
                },
            ],
        })
        .collect()
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
            style: RenderStyle::Dim,
        }],
    })
    .collect()
}

fn flatten_render_line(line: &RenderLine) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MatchSpan;

    #[test]
    fn placeholder_renderer_emits_hint_and_match_spans() {
        let lines = render_placeholder(&[HintAssignment {
            hint: "a".to_string(),
            text: "https://example.com".to_string(),
            occurrences: vec![MatchSpan {
                line: 0,
                start: 0,
                end: 19,
                text: "https://example.com".to_string(),
                pattern: "url".to_string(),
                priority: 10,
            }],
        }]);

        assert_eq!(lines[0].spans[0].style, RenderStyle::Hint);
        assert_eq!(lines[0].spans[1].style, RenderStyle::Match);
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

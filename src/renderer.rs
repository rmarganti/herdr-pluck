use crate::model::{HintAssignment, RenderLine, RenderSpan, RenderStyle};

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
}

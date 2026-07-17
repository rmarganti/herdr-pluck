use crate::model::{Rect, RenderLine, RenderSpan, RenderStyle};
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

/// Emits abstract picker render lines to a terminal writer using v1 styling.
pub fn emit_render_lines(writer: &mut impl Write, lines: &[RenderLine]) -> Result<()> {
    queue!(writer, Clear(ClearType::All), MoveTo(0, 0))?;

    for (line_index, line) in lines.iter().enumerate() {
        for span in &line.spans {
            queue_style(writer, span.style)?;
            queue!(writer, Print(&span.text))?;
        }
        if line_index + 1 < lines.len() {
            queue!(writer, Print("\r\n"))?;
        }
    }

    queue!(writer, ResetColor, SetAttribute(Attribute::Reset))?;
    Ok(())
}

/// Emits render lines positioned at `placement` inside the overlay pane,
/// clipping to the placement size so nothing wraps past the overlay edge.
pub fn emit_render_lines_at(
    writer: &mut impl Write,
    lines: &[RenderLine],
    placement: Rect,
) -> Result<()> {
    queue!(writer, Clear(ClearType::All))?;

    for (row, line) in lines.iter().take(placement.height as usize).enumerate() {
        queue!(writer, MoveTo(placement.x, placement.y + row as u16))?;
        for span in clip_spans(&line.spans, placement.width as usize) {
            queue_style(writer, span.style)?;
            queue!(writer, Print(&span.text))?;
        }
    }

    queue!(writer, ResetColor, SetAttribute(Attribute::Reset))?;
    Ok(())
}

/// Clips styled spans to a maximum display width, splitting mid-span if needed.
fn clip_spans(spans: &[RenderSpan], max_width: usize) -> Vec<RenderSpan> {
    let mut clipped = Vec::new();
    let mut used = 0;
    for span in spans {
        let span_width = span.text.width();
        if used + span_width <= max_width {
            clipped.push(span.clone());
            used += span_width;
            continue;
        }

        let mut text = String::new();
        for ch in span.text.chars() {
            let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + char_width > max_width {
                break;
            }
            text.push(ch);
            used += char_width;
        }
        if !text.is_empty() {
            clipped.push(RenderSpan {
                text,
                style: span.style,
            });
        }
        break;
    }
    clipped
}

fn queue_style(writer: &mut impl Write, style: RenderStyle) -> Result<()> {
    match style {
        RenderStyle::Unmatched => queue!(
            writer,
            SetForegroundColor(Color::DarkGrey),
            SetAttribute(Attribute::Dim)
        )?,
        RenderStyle::Match => queue!(
            writer,
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Yellow)
        )?,
        RenderStyle::Hint => queue!(
            writer,
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::Cyan),
            SetAttribute(Attribute::Bold)
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RenderSpan, RenderStyle};

    #[test]
    fn terminal_emission_clears_screen_and_writes_all_spans() {
        let lines = vec![RenderLine {
            spans: vec![
                RenderSpan {
                    text: "open ".to_string(),
                    style: RenderStyle::Unmatched,
                },
                RenderSpan {
                    text: "a".to_string(),
                    style: RenderStyle::Hint,
                },
                RenderSpan {
                    text: "ttps://example.com".to_string(),
                    style: RenderStyle::Match,
                },
            ],
        }];
        let mut output = Vec::new();

        emit_render_lines(&mut output, &lines).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.starts_with("\u{1b}[2J\u{1b}[1;1H"));
        assert!(output.contains("open "));
        assert!(output.contains("a"));
        assert!(output.contains("ttps://example.com"));
        assert!(output.contains("\u{1b}[38;5;0m"));
        assert!(output.contains("\u{1b}[48;5;14m"));
        assert!(output.contains("\u{1b}[38;5;11m"));
    }

    #[test]
    fn positioned_emission_moves_to_placement_origin_per_row() {
        let lines = vec![
            RenderLine {
                spans: vec![RenderSpan {
                    text: "one".to_string(),
                    style: RenderStyle::Unmatched,
                }],
            },
            RenderLine {
                spans: vec![RenderSpan {
                    text: "two".to_string(),
                    style: RenderStyle::Match,
                }],
            },
        ];
        let mut output = Vec::new();

        emit_render_lines_at(&mut output, &lines, Rect::new(4, 2, 10, 5)).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("\u{1b}[3;5H"));
        assert!(output.contains("\u{1b}[4;5H"));
        assert!(output.contains("one"));
        assert!(output.contains("two"));
    }

    #[test]
    fn positioned_emission_clips_rows_and_columns_to_placement() {
        let lines = vec![
            RenderLine {
                spans: vec![
                    RenderSpan {
                        text: "abc".to_string(),
                        style: RenderStyle::Unmatched,
                    },
                    RenderSpan {
                        text: "defgh".to_string(),
                        style: RenderStyle::Match,
                    },
                ],
            },
            RenderLine {
                spans: vec![RenderSpan {
                    text: "hidden".to_string(),
                    style: RenderStyle::Unmatched,
                }],
            },
        ];
        let mut output = Vec::new();

        emit_render_lines_at(&mut output, &lines, Rect::new(0, 0, 5, 1)).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("abc"));
        assert!(output.contains("de"));
        assert!(!output.contains("def"));
        assert!(!output.contains("hidden"));
    }

    #[test]
    fn terminal_emission_separates_lines_with_crlf() {
        let lines = vec![
            RenderLine {
                spans: vec![RenderSpan {
                    text: "one".to_string(),
                    style: RenderStyle::Unmatched,
                }],
            },
            RenderLine {
                spans: vec![RenderSpan {
                    text: "two".to_string(),
                    style: RenderStyle::Match,
                }],
            },
        ];
        let mut output = Vec::new();

        emit_render_lines(&mut output, &lines).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(strip_ansi(&output).contains("one\r\ntwo"));
    }

    fn strip_ansi(text: &str) -> String {
        let mut output = String::new();
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' && chars.peek() == Some(&'[') {
                chars.next();
                for code_ch in chars.by_ref() {
                    if code_ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                output.push(ch);
            }
        }
        output
    }
}

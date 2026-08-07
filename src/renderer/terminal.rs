use crate::model::{RenderLine, RenderStyle};
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

/// Emits abstract picker render lines to a terminal writer using v1 styling.
pub fn emit_render_lines(writer: &mut impl Write, lines: &[RenderLine]) -> Result<()> {
    queue!(writer, Clear(ClearType::All))?;

    for (line_index, line) in lines.iter().enumerate() {
        queue!(writer, MoveTo(0, line_index as u16))?;
        for span in &line.spans {
            queue_style(writer, span.style)?;
            queue!(writer, Print(&span.text))?;
        }
    }

    // Cancel any wrap-pending state left by a full-width final row.
    queue!(
        writer,
        MoveTo(0, 0),
        ResetColor,
        SetAttribute(Attribute::Reset)
    )?;
    Ok(())
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
    fn terminal_emission_positions_lines_without_newlines() {
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

        assert!(output.contains("\u{1b}[1;1H"));
        assert!(output.contains("\u{1b}[2;1H"));
        assert!(!output.contains("\r\n"));
        assert!(output.ends_with("\u{1b}[1;1H\u{1b}[0m\u{1b}[0m"));
    }
}

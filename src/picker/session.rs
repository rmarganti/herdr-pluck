use crate::clipboard::{Clipboard, SystemClipboard};
use crate::model::{PickerOutcome, PickerSnapshot, RenderLine, RenderSpan, RenderStyle};
use crate::picker::copy::copy_selected_text;
use crate::picker::input::{
    CrosstermInputSource, InputDecision, InputSource, InputState, PickerInputEvent, RawModeGuard,
};
use crate::picker::render::build_picker_view;
use crate::renderer::terminal;
use anyhow::{anyhow, Result};
use std::io::{self, Write};

/// Runs the production picker input/copy flow for a captured source snapshot.
pub fn run_picker(snapshot: &PickerSnapshot) -> Result<PickerOutcome> {
    let mut stdout = io::stdout();
    let mut input = CrosstermInputSource;
    let clipboard = SystemClipboard;
    let _raw_mode = RawModeGuard::enable()?;
    run_picker_with(snapshot, &mut input, &clipboard, &mut stdout)
}

pub(crate) fn run_picker_with<I, C, W>(
    snapshot: &PickerSnapshot,
    input: &mut I,
    clipboard: &C,
    output: &mut W,
) -> Result<PickerOutcome>
where
    I: InputSource,
    C: Clipboard,
    W: Write,
{
    let view = build_picker_view(snapshot);
    terminal::emit_render_lines(output, &view.lines)?;
    output.flush()?;

    let Some(width) = view.assignments.width() else {
        return run_no_match_input(input);
    };

    let valid_hints = view.assignments.valid_hints().collect::<Vec<_>>();
    let mut input_state = InputState::new(width);

    loop {
        match input_state.push(input.read_event()?, &valid_hints) {
            InputDecision::Continue | InputDecision::InvalidHint => continue,
            InputDecision::Cancel => return Ok(PickerOutcome::Cancelled),
            InputDecision::CopyHint(hint) => {
                let text = view
                    .assignments
                    .copied_text_for_hint(&hint)
                    .ok_or_else(|| anyhow!("accepted unknown picker hint {hint}"))?;
                if let Err(error) = copy_selected_text(clipboard, text) {
                    emit_copy_failure(output, text, &error)?;
                    return Err(error);
                }
                return Ok(PickerOutcome::Copied {
                    text: text.to_string(),
                });
            }
        }
    }
}

fn run_no_match_input(input: &mut impl InputSource) -> Result<PickerOutcome> {
    loop {
        match input.read_event()? {
            PickerInputEvent::Enter | PickerInputEvent::Other => continue,
            PickerInputEvent::Escape | PickerInputEvent::CtrlC => {
                return Ok(PickerOutcome::Cancelled)
            }
            PickerInputEvent::Char(_) => return Ok(PickerOutcome::NoMatches),
        }
    }
}

fn emit_copy_failure(output: &mut impl Write, text: &str, error: &anyhow::Error) -> Result<()> {
    let message = format!("Herdr Pluck: failed to copy {text:?}: {error}");
    let lines = vec![RenderLine {
        spans: vec![RenderSpan {
            text: message,
            style: RenderStyle::Unmatched,
        }],
    }];
    terminal::emit_render_lines(output, &lines)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::{ClipboardError, CopySuccess};
    use crate::model::{PaneId, PaneTextCaptureMode, SourcePaneSnapshot, TempTabSession};
    use std::cell::RefCell;

    struct FakeInput {
        events: Vec<PickerInputEvent>,
    }

    impl FakeInput {
        fn new(events: Vec<PickerInputEvent>) -> Self {
            Self {
                events: events.into_iter().rev().collect(),
            }
        }
    }

    impl InputSource for FakeInput {
        fn read_event(&mut self) -> Result<PickerInputEvent> {
            self.events
                .pop()
                .ok_or_else(|| anyhow!("fake input exhausted"))
        }
    }

    #[derive(Default)]
    struct FakeClipboard {
        copied: RefCell<Vec<String>>,
        error: Option<ClipboardError>,
    }

    impl Clipboard for FakeClipboard {
        fn copy(&self, text: &str) -> std::result::Result<CopySuccess, ClipboardError> {
            self.copied.borrow_mut().push(text.to_string());
            if let Some(error) = &self.error {
                Err(error.clone())
            } else {
                Ok(CopySuccess {
                    tool: "fake".to_string(),
                })
            }
        }
    }

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
        }
    }

    #[test]
    fn exact_hint_copies_matched_text_not_hint() {
        let mut input = FakeInput::new(vec![PickerInputEvent::Char('a')]);
        let clipboard = FakeClipboard::default();
        let mut output = Vec::new();

        let outcome = run_picker_with(
            &snapshot(vec!["open https://example.com/path"], 40, 1),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap();

        assert_eq!(
            outcome,
            PickerOutcome::Copied {
                text: "https://example.com/path".to_string()
            }
        );
        assert_eq!(
            clipboard.copied.borrow().as_slice(),
            &["https://example.com/path".to_string()]
        );
    }

    #[test]
    fn duplicate_hint_copies_shared_text_once() {
        let mut input = FakeInput::new(vec![PickerInputEvent::Char('a')]);
        let clipboard = FakeClipboard::default();
        let mut output = Vec::new();

        let outcome = run_picker_with(
            &snapshot(
                vec!["https://example.com first", "again https://example.com"],
                40,
                2,
            ),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap();

        assert_eq!(
            outcome,
            PickerOutcome::Copied {
                text: "https://example.com".to_string()
            }
        );
        assert_eq!(clipboard.copied.borrow().len(), 1);
    }

    #[test]
    fn invalid_hint_clears_buffer_and_keeps_picker_active() {
        let mut input = FakeInput::new(vec![
            PickerInputEvent::Char('x'),
            PickerInputEvent::Char('a'),
        ]);
        let clipboard = FakeClipboard::default();
        let mut output = Vec::new();

        let outcome = run_picker_with(
            &snapshot(vec!["open https://example.com/path"], 40, 1),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap();

        assert!(matches!(outcome, PickerOutcome::Copied { .. }));
        assert_eq!(clipboard.copied.borrow().len(), 1);
    }

    #[test]
    fn escape_and_ctrl_c_cancel_without_copying() {
        for event in [PickerInputEvent::Escape, PickerInputEvent::CtrlC] {
            let mut input = FakeInput::new(vec![event]);
            let clipboard = FakeClipboard::default();
            let mut output = Vec::new();

            let outcome = run_picker_with(
                &snapshot(vec!["open https://example.com/path"], 40, 1),
                &mut input,
                &clipboard,
                &mut output,
            )
            .unwrap();

            assert_eq!(outcome, PickerOutcome::Cancelled);
            assert!(clipboard.copied.borrow().is_empty());
        }
    }

    #[test]
    fn enter_is_ignored_before_valid_hint() {
        let mut input = FakeInput::new(vec![PickerInputEvent::Enter, PickerInputEvent::Char('a')]);
        let clipboard = FakeClipboard::default();
        let mut output = Vec::new();

        let outcome = run_picker_with(
            &snapshot(vec!["open https://example.com/path"], 40, 1),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap();

        assert!(matches!(outcome, PickerOutcome::Copied { .. }));
    }

    #[test]
    fn clipboard_failure_is_reported_and_not_treated_as_success() {
        let mut input = FakeInput::new(vec![PickerInputEvent::Char('a')]);
        let clipboard = FakeClipboard {
            error: Some(ClipboardError::NoToolFound {
                tried: "fake-copy".to_string(),
            }),
            ..FakeClipboard::default()
        };
        let mut output = Vec::new();

        let error = run_picker_with(
            &snapshot(vec!["open https://example.com/path"], 40, 1),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("failed to copy selected text"));
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("failed to copy"));
    }

    #[test]
    fn no_matches_waits_for_close_key_and_returns_no_matches() {
        let mut input = FakeInput::new(vec![PickerInputEvent::Enter, PickerInputEvent::Char('q')]);
        let clipboard = FakeClipboard::default();
        let mut output = Vec::new();

        let outcome = run_picker_with(
            &snapshot(vec!["plain text only"], 30, 3),
            &mut input,
            &clipboard,
            &mut output,
        )
        .unwrap();

        assert_eq!(outcome, PickerOutcome::NoMatches);
        assert!(clipboard.copied.borrow().is_empty());
    }
}

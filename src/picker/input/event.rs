use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Keyboard input understood by the picker state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerInputEvent {
    Char(char),
    Enter,
    Escape,
    CtrlC,
    Other,
}

/// Converts terminal events into picker-domain input events.
pub(crate) fn input_event_from_crossterm(event: Event) -> PickerInputEvent {
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        }) if modifiers.contains(KeyModifiers::CONTROL) => PickerInputEvent::CtrlC,
        Event::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        }) if !modifiers.contains(KeyModifiers::CONTROL)
            && !modifiers.contains(KeyModifiers::ALT) =>
        {
            PickerInputEvent::Char(ch)
        }
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            ..
        }) => PickerInputEvent::Enter,
        Event::Key(KeyEvent {
            code: KeyCode::Esc, ..
        }) => PickerInputEvent::Escape,
        _ => PickerInputEvent::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_crossterm_keys_to_picker_events() {
        assert_eq!(
            input_event_from_crossterm(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::empty()
            ))),
            PickerInputEvent::Char('a')
        );
        assert_eq!(
            input_event_from_crossterm(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::empty()
            ))),
            PickerInputEvent::Enter
        );
        assert_eq!(
            input_event_from_crossterm(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::empty()
            ))),
            PickerInputEvent::Escape
        );
        assert_eq!(
            input_event_from_crossterm(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ))),
            PickerInputEvent::CtrlC
        );
    }
}

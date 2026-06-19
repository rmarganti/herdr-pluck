use crate::model::PickerOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Char(char),
    Enter,
    Escape,
    CtrlC,
}

#[derive(Debug, Clone)]
pub struct InputState {
    width: usize,
    buffer: String,
}

impl InputState {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            buffer: String::new(),
        }
    }

    pub fn push(&mut self, event: InputEvent, valid_hints: &[String]) -> Option<PickerOutcome> {
        match event {
            InputEvent::Escape | InputEvent::CtrlC => Some(PickerOutcome::Cancelled),
            InputEvent::Enter => None,
            InputEvent::Char(ch) => {
                self.buffer.push(ch);
                if self.buffer.len() == self.width {
                    let entered = std::mem::take(&mut self.buffer);
                    if valid_hints.iter().any(|hint| hint == &entered) {
                        Some(PickerOutcome::Copied { text: entered })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_cancels() {
        let mut state = InputState::new(1);
        assert_eq!(
            state.push(InputEvent::Escape, &[]),
            Some(PickerOutcome::Cancelled)
        );
    }
}

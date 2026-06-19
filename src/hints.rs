use crate::model::{HintAssignment, MatchSpan};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

pub const HINT_ALPHABET: &str = "asdfghjklqwertyuiopzxcvbnm";
pub const MAX_HINT_WIDTH: usize = 2;
pub const MAX_HINT_CAPACITY: usize = HINT_ALPHABET.len() * HINT_ALPHABET.len();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintAssignments {
    assignments: Vec<HintAssignment>,
}

impl HintAssignments {
    pub fn new(assignments: Vec<HintAssignment>) -> Self {
        Self { assignments }
    }

    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty()
    }

    /// Returns the number of unique copied texts with assigned hints, not total occurrences.
    pub fn len(&self) -> usize {
        self.assignments.len()
    }

    /// Returns the fixed hint width in typed characters, not Unicode display columns.
    pub fn width(&self) -> Option<usize> {
        self.assignments
            .first()
            .map(|assignment| assignment.hint.chars().count())
    }

    pub fn assignments(&self) -> &[HintAssignment] {
        &self.assignments
    }

    pub fn into_assignments(self) -> Vec<HintAssignment> {
        self.assignments
    }

    pub fn copied_text_for_hint(&self, hint: &str) -> Option<&str> {
        self.assignments
            .iter()
            .find(|assignment| assignment.hint == hint)
            .map(|assignment| assignment.text.as_str())
    }

    pub fn valid_hints(&self) -> impl Iterator<Item = &str> {
        self.assignments
            .iter()
            .map(|assignment| assignment.hint.as_str())
    }
}

#[derive(Debug)]
struct AssignmentBuilder {
    text: String,
    occurrences: Vec<MatchSpan>,
}

pub fn hint_alphabet() -> &'static str {
    HINT_ALPHABET
}

/// Determines the appropriate fixed hint width for a number of unique copied texts.
pub fn hint_width(unique_match_count: usize) -> Option<usize> {
    match unique_match_count {
        0 => None,
        1..=26 => Some(1),
        _ => Some(MAX_HINT_WIDTH),
    }
}

/// Generates hints based on the specified width.
pub fn generate_hints(width: usize) -> Vec<String> {
    match width {
        1 => HINT_ALPHABET.chars().map(|ch| ch.to_string()).collect(),
        2 => HINT_ALPHABET
            .chars()
            .flat_map(|first| {
                HINT_ALPHABET
                    .chars()
                    .map(move |second| format!("{first}{second}"))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns the Unicode display width of text in terminal columns.
pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Assigns fixed-width hints to copied texts in the caller-provided first-visible order.
///
/// The caller is responsible for sorting matches top-to-bottom and left-to-right. Duplicate copied
/// text shares the first assigned hint while retaining every visible occurrence. New unique copied
/// texts beyond the v1 capacity are silently omitted, but later duplicates of already-assigned
/// copied texts are still retained.
pub fn assign_hints(matches: Vec<MatchSpan>) -> HintAssignments {
    if matches.is_empty() {
        return HintAssignments::new(Vec::new());
    }

    let mut by_text: HashMap<String, usize> = HashMap::new();
    let mut ordered: Vec<AssignmentBuilder> = Vec::new();

    for span in matches {
        if let Some(index) = by_text.get(&span.text).copied() {
            ordered[index].occurrences.push(span);
            continue;
        }

        if ordered.len() < MAX_HINT_CAPACITY {
            let index = ordered.len();
            by_text.insert(span.text.clone(), index);
            ordered.push(AssignmentBuilder {
                text: span.text.clone(),
                occurrences: vec![span],
            });
        }
    }

    let Some(width) = hint_width(ordered.len()) else {
        return HintAssignments::new(Vec::new());
    };

    let hints = generate_hints(width);
    let assignments = ordered
        .into_iter()
        .zip(hints)
        .map(|(builder, hint)| HintAssignment {
            hint,
            text: builder.text,
            occurrences: builder.occurrences,
        })
        .collect();

    HintAssignments::new(assignments)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper to create a MatchSpan with the specified text and position,
    /// using default pattern and priority.
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

    #[test]
    fn unsupported_hint_width_generates_no_hints() {
        assert!(generate_hints(0).is_empty());
        assert!(generate_hints(3).is_empty());
    }

    #[test]
    fn zero_matches_produce_empty_assignments() {
        let assignments = assign_hints(Vec::new());

        assert!(assignments.is_empty());
        assert_eq!(assignments.len(), 0);
        assert_eq!(assignments.width(), None);
    }

    #[test]
    fn assignment_preserves_caller_provided_order() {
        let assignments = assign_hints(vec![
            span("zeta", 3, 0),
            span("alpha", 0, 0),
            span("middle", 2, 0),
        ]);
        let texts: Vec<&str> = assignments
            .assignments()
            .iter()
            .map(|assignment| assignment.text.as_str())
            .collect();

        assert_eq!(texts, vec!["zeta", "alpha", "middle"]);
        assert_eq!(assignments.assignments()[0].hint, "a");
        assert_eq!(assignments.assignments()[1].hint, "s");
        assert_eq!(assignments.assignments()[2].hint, "d");
    }

    #[test]
    fn duplicate_text_shares_hint_and_retains_occurrences() {
        let assignments = assign_hints(vec![
            span("foo", 0, 0),
            span("bar", 0, 4),
            span("foo", 1, 2),
        ]);

        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments.assignments()[0].text, "foo");
        assert_eq!(assignments.assignments()[0].hint, "a");
        assert_eq!(assignments.assignments()[0].occurrences.len(), 2);
        assert_eq!(assignments.assignments()[1].text, "bar");
        assert_eq!(assignments.assignments()[1].hint, "s");
    }

    #[test]
    fn caps_unique_texts_at_two_character_capacity() {
        let matches: Vec<MatchSpan> = (0..700)
            .map(|index| span(format!("token-{index}"), index, 0))
            .collect();

        let assignments = assign_hints(matches);

        assert_eq!(assignments.len(), MAX_HINT_CAPACITY);
        assert_eq!(assignments.width(), Some(2));
        assert_eq!(assignments.assignments().first().unwrap().hint, "aa");
        assert_eq!(assignments.assignments().last().unwrap().hint, "mm");
        assert!(assignments
            .assignments()
            .iter()
            .all(|assignment| assignment.text != "token-676"));
    }

    #[test]
    fn duplicate_after_cap_is_retained_for_assigned_text() {
        let mut matches: Vec<MatchSpan> = (0..MAX_HINT_CAPACITY)
            .map(|index| span(format!("token-{index}"), index, 0))
            .collect();
        matches.push(span("beyond-cap", MAX_HINT_CAPACITY, 0));
        matches.push(span("token-0", MAX_HINT_CAPACITY + 1, 0));

        let assignments = assign_hints(matches);

        assert_eq!(assignments.len(), MAX_HINT_CAPACITY);
        assert_eq!(assignments.assignments()[0].text, "token-0");
        assert_eq!(assignments.assignments()[0].occurrences.len(), 2);
        assert!(assignments
            .assignments()
            .iter()
            .all(|assignment| assignment.text != "beyond-cap"));
    }

    #[test]
    fn lookup_returns_copied_text_for_exact_hint() {
        let assignments = assign_hints(vec![span("first text", 0, 0), span("second text", 0, 11)]);

        assert_eq!(assignments.copied_text_for_hint("a"), Some("first text"));
        assert_eq!(assignments.copied_text_for_hint("s"), Some("second text"));
        assert_eq!(assignments.copied_text_for_hint("x"), None);
    }

    #[test]
    fn lookup_works_for_two_character_hints() {
        let matches: Vec<MatchSpan> = (0..27)
            .map(|index| span(format!("token-{index}"), index, 0))
            .collect();

        let assignments = assign_hints(matches);

        assert_eq!(assignments.width(), Some(2));
        assert_eq!(assignments.copied_text_for_hint("aa"), Some("token-0"));
        assert_eq!(assignments.copied_text_for_hint("sa"), Some("token-26"));
        assert_eq!(assignments.copied_text_for_hint("a"), None);
    }

    #[test]
    fn valid_hints_iterates_assigned_hints() {
        let assignments = assign_hints(vec![span("foo", 0, 0), span("bar", 0, 4)]);
        let hints: Vec<&str> = assignments.valid_hints().collect();

        assert_eq!(hints, vec!["a", "s"]);
    }

    #[test]
    fn display_width_uses_unicode_terminal_columns() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("é"), 1);
        assert_eq!(display_width("🔥"), 2);
    }
}

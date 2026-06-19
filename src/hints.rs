use crate::model::{HintAssignment, MatchSpan};
use std::collections::BTreeMap;

pub const MAX_HINT_WIDTH: usize = 2;

pub fn hint_alphabet() -> Vec<char> {
    "asdfghjklqwertyuiopzxcvbnm".chars().collect()
}

/// Determines the appropriate hint width based on the number of unique match texts.
pub fn hint_width(unique_match_count: usize) -> usize {
    let alphabet_len = hint_alphabet().len();
    if unique_match_count <= alphabet_len {
        1
    } else {
        MAX_HINT_WIDTH
    }
}

/// Generates hints based on the specified width. For width 1, it generates single-character hints
/// from the alphabet. For width 2, it generates all possible two-character combinations from the
/// alphabet. Widths greater than 2 are not supported and will return an empty vector.
pub fn generate_hints(width: usize) -> Vec<String> {
    let alphabet = hint_alphabet();
    match width {
        0 => Vec::new(),
        1 => alphabet.iter().map(char::to_string).collect(),
        2 => alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect(),
        _ => Vec::new(),
    }
}

/// Assigns hints to the given matches. Hints are generated based on the number of unique match texts.
pub fn assign_hints(matches: Vec<MatchSpan>) -> Vec<HintAssignment> {
    let mut by_text: BTreeMap<String, Vec<MatchSpan>> = BTreeMap::new();
    for span in matches {
        by_text.entry(span.text.clone()).or_default().push(span);
    }

    let width = hint_width(by_text.len());
    let hints = generate_hints(width);

    by_text
        .into_iter()
        .zip(hints)
        .map(|((text, occurrences), hint)| HintAssignment {
            hint,
            text,
            occurrences,
        })
        .collect()
}

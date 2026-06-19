use crate::model::MatchSpan;
use regex::Regex;
use std::sync::OnceLock;

static BUILT_IN_PATTERNS: OnceLock<Vec<PatternDefinition>> = OnceLock::new();

#[derive(Debug)]
struct PatternDefinition {
    name: &'static str,
    /// Match precedence where lower numbers beat higher numbers.
    /// ie. `1` is the number one priority, while `5` is a lower priority
    priority: u16,
    regex: Regex,
}

impl PatternDefinition {
    fn compile(name: &'static str, priority: u16, pattern: &str) -> Self {
        Self {
            name,
            priority,
            regex: Regex::new(pattern).expect("built-in Herdr Pluck pattern must compile"),
        }
    }
}

/// Candidate match occurrence with its pattern definition order for tie-breaking.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchCandidate {
    span: MatchSpan,
    pattern_order: usize,
}

/// Finds accepted built-in pattern matches in first-visible order.
///
/// Matching runs on unwrapped logical lines. Returned spans are all accepted visible occurrences;
/// duplicate copied texts are intentionally preserved for the hint engine to deduplicate later.
pub fn find_matches(lines: &[String]) -> Vec<MatchSpan> {
    find_matches_with_patterns(lines, built_in_patterns())
}

fn built_in_patterns() -> &'static [PatternDefinition] {
    BUILT_IN_PATTERNS.get_or_init(|| {
        vec![
            PatternDefinition::compile(
                "url",
                10,
                r#"((https?://|git@|git://|ssh://|ftp://|file:///)[^\s()"']+)"#,
            ),
            PatternDefinition::compile("path", 20, r#"(([.\w\-~\$@]+)?(/[.\w\-@]+)+/?)"#),
            PatternDefinition::compile(
                "uuid",
                30,
                r#"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"#,
            ),
            PatternDefinition::compile("git-sha", 40, r#"\b[0-9a-fA-F]{7,40}\b"#),
            PatternDefinition::compile("ipv4", 50, r#"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}"#),
            PatternDefinition::compile("number", 60, r#"[0-9]{4,}"#),
        ]
    })
}

fn find_matches_with_patterns(lines: &[String], patterns: &[PatternDefinition]) -> Vec<MatchSpan> {
    let candidates = collect_candidates(lines, patterns);
    let mut accepted = resolve_overlaps(candidates);
    accepted.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.start.cmp(&right.start))
            .then_with(|| left.end.cmp(&right.end))
    });
    accepted
}

/// Collects all candidate matches for all patterns with their pattern definition order for tie-breaking.
fn collect_candidates(lines: &[String], patterns: &[PatternDefinition]) -> Vec<MatchCandidate> {
    let mut candidates = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        for (pattern_order, pattern) in patterns.iter().enumerate() {
            for captures in pattern.regex.captures_iter(line) {
                let Some(regex_match) = captures.name("match").or_else(|| captures.get(0)) else {
                    continue;
                };

                candidates.push(MatchCandidate {
                    span: MatchSpan {
                        line: line_idx,
                        start: regex_match.start(),
                        end: regex_match.end(),
                        text: regex_match.as_str().to_string(),
                        pattern: pattern.name.to_string(),
                        priority: pattern.priority,
                    },
                    pattern_order,
                });
            }
        }
    }

    candidates
}

/// Resolves overlapping candidates by acceptance priority where lower numbers are higher priority.
fn resolve_overlaps(mut candidates: Vec<MatchCandidate>) -> Vec<MatchSpan> {
    candidates.sort_by(|left, right| {
        left.span
            .priority
            .cmp(&right.span.priority)
            .then_with(|| right.span.len_bytes().cmp(&left.span.len_bytes()))
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.span.start.cmp(&right.span.start))
            .then_with(|| left.span.end.cmp(&right.span.end))
            .then_with(|| left.pattern_order.cmp(&right.pattern_order))
    });

    let mut accepted: Vec<MatchSpan> = Vec::new();
    for candidate in candidates {
        if accepted
            .iter()
            .all(|accepted_span| !candidate.span.overlaps(accepted_span))
        {
            accepted.push(candidate.span);
        }
    }

    accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines<const N: usize>(values: [&str; N]) -> Vec<String> {
        values.into_iter().map(String::from).collect()
    }

    fn test_pattern(name: &'static str, priority: u16, pattern: &str) -> PatternDefinition {
        PatternDefinition::compile(name, priority, pattern)
    }

    #[test]
    fn finds_url_schemes() {
        let matches = find_matches(&lines([
            "https://example.com http://example.com git@github.com:org/repo.git",
            "git://host/repo ssh://host/path ftp://host/path file:///tmp/file",
        ]));
        let urls: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "url")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            urls,
            vec![
                "https://example.com",
                "http://example.com",
                "git@github.com:org/repo.git",
                "git://host/repo",
                "ssh://host/path",
                "ftp://host/path",
                "file:///tmp/file",
            ]
        );
    }

    #[test]
    fn url_stops_at_delimiters() {
        let matches = find_matches(&lines([
            "(https://a.test/path) 'https://b.test' \"https://c.test\"",
        ]));
        let urls: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "url")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            urls,
            vec!["https://a.test/path", "https://b.test", "https://c.test"]
        );
    }

    #[test]
    fn url_preserves_non_delimiter_trailing_punctuation() {
        let matches = find_matches(&lines(["see https://example.com/path., next"]));
        let url = matches.iter().find(|span| span.pattern == "url").unwrap();

        assert_eq!(url.text, "https://example.com/path.,");
    }

    #[test]
    fn finds_style_paths() {
        let matches = find_matches(&lines(["/tmp/foo/bar src/main.rs foo/bar dir/sub/"]));
        let paths: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "path")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            paths,
            vec!["/tmp/foo/bar", "src/main.rs", "foo/bar", "dir/sub/"]
        );
    }

    #[test]
    fn finds_uppercase_and_lowercase_uuids() {
        let matches = find_matches(&lines([
            "ids 123e4567-e89b-12d3-a456-426614174000 123E4567-E89B-12D3-A456-426614174000",
        ]));
        let uuids: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "uuid")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            uuids,
            vec![
                "123e4567-e89b-12d3-a456-426614174000",
                "123E4567-E89B-12D3-A456-426614174000",
            ]
        );
    }

    #[test]
    fn finds_git_sha_lengths_with_uppercase_support() {
        let matches = find_matches(&lines([
            "sha abcDEF1 abcdef1234567890abcdef1234567890abcdef12 abcdef1234567890abcdef1234567890abcdef123",
        ]));
        let shas: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "git-sha")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            shas,
            vec!["abcDEF1", "abcdef1234567890abcdef1234567890abcdef12"]
        );
    }

    #[test]
    fn finds_loose_ipv4() {
        let matches = find_matches(&lines(["hosts 192.168.1.10 999.999.999.999"]));
        let ips: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "ipv4")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(ips, vec!["192.168.1.10", "999.999.999.999"]);
    }

    #[test]
    fn finds_long_numbers_inside_text() {
        let matches = find_matches(&lines(["zzz1234zzz issue-5678 foo_9012"]));
        let numbers: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "number")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(numbers, vec!["1234", "5678", "9012"]);
    }

    #[test]
    fn named_match_capture_defines_text_and_range() {
        let patterns = vec![test_pattern("status", 10, r#"status=\[(?<match>[^\]]+)\]"#)];
        let matches = find_matches_with_patterns(&lines(["status=[copy-me]"]), &patterns);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "copy-me");
        assert_eq!(matches[0].start, 8);
        assert_eq!(matches[0].end, 15);
    }

    #[test]
    fn full_match_is_used_without_named_capture() {
        let patterns = vec![test_pattern("word", 10, r#"copy-me"#)];
        let matches = find_matches_with_patterns(&lines(["xx copy-me yy"]), &patterns);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "copy-me");
        assert_eq!(matches[0].start, 3);
        assert_eq!(matches[0].end, 10);
    }

    #[test]
    fn overlap_resolution_uses_named_capture_range() {
        let patterns = vec![
            test_pattern("wrapped", 10, r#"prefix-(?<match>abc)-suffix"#),
            test_pattern("prefix", 20, r#"prefix"#),
        ];
        let matches = find_matches_with_patterns(&lines(["prefix-abc-suffix"]), &patterns);
        let texts: Vec<&str> = matches.iter().map(|span| span.text.as_str()).collect();

        assert_eq!(texts, vec!["prefix", "abc"]);
    }

    #[test]
    fn url_wins_over_path_inside_url() {
        let matches = find_matches(&lines(["open https://example.com/src/main.rs"]));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "url");
        assert_eq!(matches[0].text, "https://example.com/src/main.rs");
    }

    #[test]
    fn uuid_wins_over_sha_and_number_submatches() {
        let matches = find_matches(&lines(["id 123e4567-e89b-12d3-a456-426614174000"]));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "uuid");
    }

    #[test]
    fn git_sha_wins_over_number_submatch() {
        let matches = find_matches(&lines(["sha abc1234"]));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "git-sha");
        assert_eq!(matches[0].text, "abc1234");
    }

    #[test]
    fn higher_priority_beats_longer_lower_priority_overlap() {
        let patterns = vec![
            test_pattern("short-high", 10, r#"abc"#),
            test_pattern("long-low", 20, r#"abcdef"#),
        ];
        let matches = find_matches_with_patterns(&lines(["abcdef"]), &patterns);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "short-high");
    }

    #[test]
    fn longer_match_wins_within_same_priority() {
        let patterns = vec![
            test_pattern("short", 10, r#"abc"#),
            test_pattern("long", 10, r#"abcdef"#),
        ];
        let matches = find_matches_with_patterns(&lines(["abcdef"]), &patterns);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "long");
    }

    #[test]
    fn earlier_position_wins_within_same_priority_and_length() {
        let patterns = vec![test_pattern("word", 10, r#"abc"#)];
        let matches = find_matches_with_patterns(&lines(["abc abc"]), &patterns);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 4);
    }

    #[test]
    fn pattern_definition_order_breaks_exact_ties() {
        let patterns = vec![
            test_pattern("first", 10, r#"abc"#),
            test_pattern("second", 10, r#"abc"#),
        ];
        let matches = find_matches_with_patterns(&lines(["abc"]), &patterns);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "first");
    }

    #[test]
    fn output_order_is_visible_order_not_acceptance_priority() {
        let matches = find_matches(&lines(["1234", "https://example.com"]));
        let texts: Vec<&str> = matches.iter().map(|span| span.text.as_str()).collect();

        assert_eq!(texts, vec!["1234", "https://example.com"]);
    }

    #[test]
    fn duplicate_copied_text_occurrences_are_preserved() {
        let matches = find_matches(&lines(["/tmp/foo /tmp/foo"]));
        let paths: Vec<&MatchSpan> = matches
            .iter()
            .filter(|span| span.pattern == "path")
            .collect();

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].text, paths[1].text);
        assert_ne!(paths[0].start, paths[1].start);
    }

    #[test]
    fn overlap_is_line_local() {
        let patterns = vec![test_pattern("same", 10, r#"abc"#)];
        let matches = find_matches_with_patterns(&lines(["abc", "abc"]), &patterns);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
    }
}

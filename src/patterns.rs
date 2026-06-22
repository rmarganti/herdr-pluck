use crate::model::MatchSpan;
use regex::Regex;
use std::sync::OnceLock;

static BUILT_IN_PATTERNS: OnceLock<Vec<PatternDefinition>> = OnceLock::new();

const KUBERNETES_RESOURCE_KINDS: &str = "deployment.app|binding|componentstatuse|configmap|endpoint|event|limitrange|namespace|node|persistentvolumeclaim|persistentvolume|pod|podtemplate|replicationcontroller|resourcequota|secret|serviceaccount|service|mutatingwebhookconfiguration.admissionregistration.k8s.io|validatingwebhookconfiguration.admissionregistration.k8s.io|customresourcedefinition.apiextension.k8s.io|apiservice.apiregistration.k8s.io|controllerrevision.apps|daemonset.apps|deployment.apps|replicaset.apps|statefulset.apps|tokenreview.authentication.k8s.io|localsubjectaccessreview.authorization.k8s.io|selfsubjectaccessreviews.authorization.k8s.io|selfsubjectrulesreview.authorization.k8s.io|subjectaccessreview.authorization.k8s.io|horizontalpodautoscaler.autoscaling|cronjob.batch|job.batch|certificatesigningrequest.certificates.k8s.io|events.events.k8s.io|daemonset.extensions|deployment.extensions|ingress.extensions|networkpolicies.extensions|podsecuritypolicies.extensions|replicaset.extensions|networkpolicie.networking.k8s.io|poddisruptionbudget.policy|clusterrolebinding.rbac.authorization.k8s.io|clusterrole.rbac.authorization.k8s.io|rolebinding.rbac.authorization.k8s.io|role.rbac.authorization.k8s.io|storageclasse.storage.k8s.io";

#[derive(Debug)]
struct PatternDefinition {
    name: String,
    /// Match precedence where lower numbers beat higher numbers.
    /// ie. `1` is the number one priority, while `5` is a lower priority
    priority: u16,
    regex: Regex,
}

impl PatternDefinition {
    fn compile(name: impl Into<String>, priority: u16, pattern: &str) -> Self {
        Self::try_compile(name, priority, pattern)
            .expect("built-in Herdr Pluck pattern must compile")
    }

    fn try_compile(
        name: impl Into<String>,
        priority: u16,
        pattern: &str,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            name: name.into(),
            priority,
            regex: Regex::new(pattern)?,
        })
    }
}

/// User-defined regex pattern loaded from Herdr Pluck config.
#[derive(Debug)]
pub struct CustomPatternDefinition(PatternDefinition);

impl CustomPatternDefinition {
    /// Compiles a user pattern, preserving regex diagnostics for config errors.
    pub fn compile(
        name: impl Into<String>,
        priority: u16,
        pattern: &str,
    ) -> Result<Self, regex::Error> {
        PatternDefinition::try_compile(name, priority, pattern).map(Self)
    }

    /// User-visible pattern name.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Match precedence where lower numbers are higher priority.
    pub fn priority(&self) -> u16 {
        self.0.priority
    }
}

/// Candidate match occurrence with its pattern definition order for tie-breaking.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchCandidate {
    span: MatchSpan,
    pattern_order: usize,
}

/// Finds matches using user-defined custom patterns followed by built-ins.
pub fn find_matches(
    lines: &[String],
    custom_patterns: &[CustomPatternDefinition],
) -> Vec<MatchSpan> {
    let mut patterns = Vec::with_capacity(custom_patterns.len() + built_in_patterns().len());
    patterns.extend(custom_patterns.iter().map(|pattern| &pattern.0));
    patterns.extend(built_in_patterns().iter());
    find_matches_with_patterns(lines, &patterns)
}

fn built_in_patterns() -> &'static [PatternDefinition] {
    BUILT_IN_PATTERNS.get_or_init(|| {
        vec![
            PatternDefinition::compile(
                "url",
                10,
                r#"((https?://|git@|git://|ssh://|ftp://|file:///)[^\s()"']+)"#,
            ),
            PatternDefinition::compile("git-status", 15, r#"(modified|deleted|deleted by us|new file): +(?<match>.+)"#),
            PatternDefinition::compile(
                "git-status-branch",
                15,
                r#"Your branch is up to date with '(?<match>.*)'\."#,
            ),
            PatternDefinition::compile("diff", 15, r#"(---|\+\+\+) [ab]/(?<match>.*)"#),
            PatternDefinition::compile(
                "kubernetes",
                18,
                &format!(r#"\b({KUBERNETES_RESOURCE_KINDS})([/_#$%&+=@-][[:alnum:]_#$%&+=/@-]*)?\b"#),
            ),
            PatternDefinition::compile("path", 20, r#"(([.\w\-~\$@]+)?(/[.\w\-@]+)+/?)"#),
            PatternDefinition::compile(
                "uuid",
                30,
                r#"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"#,
            ),
            PatternDefinition::compile(
                "kubernetes-pod",
                35,
                r#"\b[a-z][a-z0-9-]*[a-z0-9]-[bcdfghjklmnpqrstvwxz2456789]{5,10}-[bcdfghjklmnpqrstvwxz2456789]{5}\b"#,
            ),
            PatternDefinition::compile("git-sha", 40, r#"\b[0-9a-fA-F]{7,40}\b"#),
            PatternDefinition::compile("hex", 45, r#"\b0x[0-9a-fA-F]+\b"#),
            PatternDefinition::compile("ipv4", 50, r#"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}"#),
            PatternDefinition::compile("number", 60, r#"[0-9]{4,}"#),
        ]
    })
}

fn find_matches_with_patterns(lines: &[String], patterns: &[&PatternDefinition]) -> Vec<MatchSpan> {
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
fn collect_candidates(lines: &[String], patterns: &[&PatternDefinition]) -> Vec<MatchCandidate> {
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
                        pattern: pattern.name.clone(),
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

    fn find_matches_with_defaults_only(lines: &[String]) -> Vec<MatchSpan> {
        let patterns = built_in_patterns().iter().collect::<Vec<_>>();
        find_matches_with_patterns(lines, &patterns)
    }

    fn lines<const N: usize>(values: [&str; N]) -> Vec<String> {
        values.into_iter().map(String::from).collect()
    }

    fn test_pattern(name: &'static str, priority: u16, pattern: &str) -> PatternDefinition {
        PatternDefinition::compile(name, priority, pattern)
    }

    fn test_patterns(patterns: &[PatternDefinition]) -> Vec<&PatternDefinition> {
        patterns.iter().collect()
    }

    #[test]
    fn finds_url_schemes() {
        let matches = find_matches_with_defaults_only(&lines([
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
        let matches = find_matches_with_defaults_only(&lines([
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
        let matches =
            find_matches_with_defaults_only(&lines(["see https://example.com/path., next"]));
        let url = matches.iter().find(|span| span.pattern == "url").unwrap();

        assert_eq!(url.text, "https://example.com/path.,");
    }

    #[test]
    fn finds_style_paths() {
        let matches =
            find_matches_with_defaults_only(&lines(["/tmp/foo/bar src/main.rs foo/bar dir/sub/"]));
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
        let matches = find_matches_with_defaults_only(&lines([
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
        let matches = find_matches_with_defaults_only(&lines([
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
        let matches =
            find_matches_with_defaults_only(&lines(["hosts 192.168.1.10 999.999.999.999"]));
        let ips: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "ipv4")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(ips, vec!["192.168.1.10", "999.999.999.999"]);
    }

    #[test]
    fn finds_long_numbers_inside_text() {
        let matches = find_matches_with_defaults_only(&lines(["zzz1234zzz issue-5678 foo_9012"]));
        let numbers: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "number")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(numbers, vec!["1234", "5678", "9012"]);
    }

    #[test]
    fn finds_hex_literals_before_numeric_submatches() {
        let matches =
            find_matches_with_defaults_only(&lines(["values 0xdeadBEEF 0x1234 not0xabc"]));
        let hexes: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "hex")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(hexes, vec!["0xdeadBEEF", "0x1234"]);
        assert!(!matches
            .iter()
            .any(|span| span.pattern == "number" && span.text == "1234"));
    }

    #[test]
    fn finds_kubernetes_resource_references() {
        let matches = find_matches_with_defaults_only(&lines([
            "kubectl get pod/nginx-123 service/api deployment.apps/frontend",
        ]));
        let resources: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "kubernetes")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(
            resources,
            vec!["pod/nginx-123", "service/api", "deployment.apps/frontend"]
        );
    }

    #[test]
    fn finds_deployment_managed_kubernetes_pod_names() {
        let matches = find_matches_with_defaults_only(&lines([
            "pods nginx-deployment-66b6c48dd5-7xb2r api-abcde-12345 bad-pod-abcde-aeiou",
        ]));
        let pods: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "kubernetes-pod")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(pods, vec!["nginx-deployment-66b6c48dd5-7xb2r"]);
    }

    #[test]
    fn finds_git_status_paths_with_named_capture() {
        let matches = find_matches_with_defaults_only(&lines([
            "modified:   src/main.rs",
            "deleted by us: docs/old.md",
            "new file:   README.md",
        ]));
        let statuses: Vec<&MatchSpan> = matches
            .iter()
            .filter(|span| span.pattern == "git-status")
            .collect();

        assert_eq!(
            statuses
                .iter()
                .map(|span| span.text.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.rs", "docs/old.md", "README.md"]
        );
        assert_eq!(statuses[0].start, "modified:   ".len());
    }

    #[test]
    fn finds_git_status_branch_name_with_named_capture() {
        let matches = find_matches_with_defaults_only(&lines([
            "Your branch is up to date with 'origin/main'.",
        ]));
        let branch = matches
            .iter()
            .find(|span| span.pattern == "git-status-branch")
            .unwrap();

        assert_eq!(branch.text, "origin/main");
    }

    #[test]
    fn finds_diff_paths_with_named_capture() {
        let matches =
            find_matches_with_defaults_only(&lines(["--- a/src/lib.rs", "+++ b/src/lib.rs"]));
        let diff_paths: Vec<&str> = matches
            .iter()
            .filter(|span| span.pattern == "diff")
            .map(|span| span.text.as_str())
            .collect();

        assert_eq!(diff_paths, vec!["src/lib.rs", "src/lib.rs"]);
    }

    #[test]
    fn expanded_patterns_win_over_lower_priority_path_and_number_overlaps() {
        let matches =
            find_matches_with_defaults_only(&lines(["+++ b/src/lib.rs", "pod/nginx 0x1234"]));
        let patterns: Vec<&str> = matches.iter().map(|span| span.pattern.as_str()).collect();

        assert_eq!(patterns, vec!["diff", "kubernetes", "hex"]);
    }

    #[test]
    fn expanded_patterns_avoid_common_false_positive_boundaries() {
        let matches =
            find_matches_with_defaults_only(&lines(["not0xabc serviceable api-abcde-aeiou"]));
        let expanded: Vec<&MatchSpan> = matches
            .iter()
            .filter(|span| {
                matches!(
                    span.pattern.as_str(),
                    "hex" | "kubernetes" | "kubernetes-pod"
                )
            })
            .collect();

        assert!(expanded.is_empty());
    }

    #[test]
    fn named_match_capture_defines_text_and_range() {
        let patterns = vec![test_pattern("status", 10, r#"status=\[(?<match>[^\]]+)\]"#)];
        let matches =
            find_matches_with_patterns(&lines(["status=[copy-me]"]), &test_patterns(&patterns));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "copy-me");
        assert_eq!(matches[0].start, 8);
        assert_eq!(matches[0].end, 15);
    }

    #[test]
    fn full_match_is_used_without_named_capture() {
        let patterns = vec![test_pattern("word", 10, r#"copy-me"#)];
        let matches =
            find_matches_with_patterns(&lines(["xx copy-me yy"]), &test_patterns(&patterns));

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
        let matches =
            find_matches_with_patterns(&lines(["prefix-abc-suffix"]), &test_patterns(&patterns));
        let texts: Vec<&str> = matches.iter().map(|span| span.text.as_str()).collect();

        assert_eq!(texts, vec!["prefix", "abc"]);
    }

    #[test]
    fn url_wins_over_path_inside_url() {
        let matches =
            find_matches_with_defaults_only(&lines(["open https://example.com/src/main.rs"]));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "url");
        assert_eq!(matches[0].text, "https://example.com/src/main.rs");
    }

    #[test]
    fn uuid_wins_over_sha_and_number_submatches() {
        let matches =
            find_matches_with_defaults_only(&lines(["id 123e4567-e89b-12d3-a456-426614174000"]));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "uuid");
    }

    #[test]
    fn git_sha_wins_over_number_submatch() {
        let matches = find_matches_with_defaults_only(&lines(["sha abc1234"]));

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
        let matches = find_matches_with_patterns(&lines(["abcdef"]), &test_patterns(&patterns));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "short-high");
    }

    #[test]
    fn longer_match_wins_within_same_priority() {
        let patterns = vec![
            test_pattern("short", 10, r#"abc"#),
            test_pattern("long", 10, r#"abcdef"#),
        ];
        let matches = find_matches_with_patterns(&lines(["abcdef"]), &test_patterns(&patterns));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "long");
    }

    #[test]
    fn earlier_position_wins_within_same_priority_and_length() {
        let patterns = vec![test_pattern("word", 10, r#"abc"#)];
        let matches = find_matches_with_patterns(&lines(["abc abc"]), &test_patterns(&patterns));

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
        let matches = find_matches_with_patterns(&lines(["abc"]), &test_patterns(&patterns));

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "first");
    }

    #[test]
    fn output_order_is_visible_order_not_acceptance_priority() {
        let matches = find_matches_with_defaults_only(&lines(["1234", "https://example.com"]));
        let texts: Vec<&str> = matches.iter().map(|span| span.text.as_str()).collect();

        assert_eq!(texts, vec!["1234", "https://example.com"]);
    }

    #[test]
    fn duplicate_copied_text_occurrences_are_preserved() {
        let matches = find_matches_with_defaults_only(&lines(["/tmp/foo /tmp/foo"]));
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
        let matches = find_matches_with_patterns(&lines(["abc", "abc"]), &test_patterns(&patterns));

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
    }

    #[test]
    fn custom_patterns_are_searched_before_builtins() {
        let custom = vec![CustomPatternDefinition::compile("ticket", 25, r#"ABC-[0-9]+"#).unwrap()];
        let matches = find_matches(&lines(["fix ABC-1234"]), &custom);

        assert!(matches
            .iter()
            .any(|span| span.pattern == "ticket" && span.text == "ABC-1234"));
    }

    #[test]
    fn custom_pattern_priority_participates_in_overlap_resolution() {
        let custom = vec![CustomPatternDefinition::compile(
            "command",
            12,
            r#"git branch -D [A-Za-z0-9._/-]+"#,
        )
        .unwrap()];
        let matches = find_matches(&lines(["run git branch -D feature/foo"]), &custom);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "command");
        assert_eq!(matches[0].text, "git branch -D feature/foo");
    }
}

use crate::model::MatchSpan;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PatternDefinition {
    pub name: &'static str,
    pub priority: u16,
    pub regex: Regex,
}

impl PatternDefinition {
    pub fn new(name: &'static str, priority: u16, pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            name,
            priority,
            regex: Regex::new(pattern)?,
        })
    }
}

pub fn built_in_patterns() -> Result<Vec<PatternDefinition>, regex::Error> {
    Ok(vec![
        PatternDefinition::new("url", 10, r#"https?://[^\s<>'\"]+"#)?,
        PatternDefinition::new(
            "path",
            20,
            r#"(?:\./|\.\./|/|~/?)[A-Za-z0-9._~+@%:=-]+(?:/[A-Za-z0-9._~+@%:=-]+)+"#,
        )?,
        PatternDefinition::new(
            "uuid",
            30,
            r#"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"#,
        )?,
        PatternDefinition::new("git-sha", 40, r#"\b[0-9a-fA-F]{7,40}\b"#)?,
        PatternDefinition::new("ipv4", 50, r#"\b(?:\d{1,3}\.){3}\d{1,3}\b"#)?,
        PatternDefinition::new("number", 60, r#"\b\d{4,}\b"#)?,
    ])
}

pub fn find_raw_matches(lines: &[String], patterns: &[PatternDefinition]) -> Vec<MatchSpan> {
    let mut spans = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        for pattern in patterns {
            for captures in pattern.regex.captures_iter(line) {
                let m = captures.name("match").or_else(|| captures.get(0));
                if let Some(m) = m {
                    spans.push(MatchSpan {
                        line: line_idx,
                        start: m.start(),
                        end: m.end(),
                        text: m.as_str().to_string(),
                        pattern: pattern.name.to_string(),
                        priority: pattern.priority,
                    });
                }
            }
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_patterns_compile_and_find_smoke_matches() {
        let patterns = built_in_patterns().unwrap();
        let lines = vec!["see https://example.com and /tmp/foo/bar".to_string()];
        let matches = find_raw_matches(&lines, &patterns);

        assert!(matches.iter().any(|m| m.pattern == "url"));
        assert!(matches.iter().any(|m| m.pattern == "path"));
    }
}

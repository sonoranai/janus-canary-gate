use std::collections::HashMap;

use crate::config::{EventConfig, MatchCondition, MatchRule};
use crate::events::{fingerprint, CanonicalEvent};
use crate::ingestion::RawLogLine;

/// Pre-compiled regex cache for efficient repeated matching.
pub struct CompiledRules {
    cache: HashMap<String, regex::Regex>,
}

impl CompiledRules {
    /// Build a compiled regex cache from event config rules.
    /// Regex patterns are validated during config load, so unwrap is safe here.
    pub fn new(rules: &[EventConfig]) -> Self {
        let mut cache = HashMap::new();
        for rule in rules {
            for condition in rule
                .match_rule
                .any
                .iter()
                .chain(rule.match_rule.all.iter())
                .chain(rule.match_rule.none.iter())
            {
                if let Some(ref pattern) = condition.regex {
                    if !cache.contains_key(pattern) {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            cache.insert(pattern.clone(), re);
                        }
                    }
                }
            }
        }
        CompiledRules { cache }
    }
}

/// Classify a raw log line against a list of event rules.
///
/// Uses first-match-wins semantics: the first matching rule produces the event.
/// Unmatched lines return None.
pub fn classify_line(
    line: &RawLogLine,
    rules: &[EventConfig],
    compiled: &CompiledRules,
) -> Option<CanonicalEvent> {
    for rule in rules {
        if matches_rule(&line.content, &rule.match_rule, compiled) {
            return Some(CanonicalEvent {
                timestamp: line
                    .timestamp
                    .clone()
                    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
                level: rule.level.clone(),
                event_type: rule.event_type.clone(),
                fingerprint: fingerprint(&rule.event_type, &rule.level),
                raw_line: Some(line.content.clone()),
            });
        }
    }
    None
}

/// Classify all lines in a stream, returning only matched events.
pub fn classify_stream(lines: &[RawLogLine], rules: &[EventConfig]) -> Vec<CanonicalEvent> {
    let compiled = CompiledRules::new(rules);
    lines
        .iter()
        .filter_map(|l| classify_line(l, rules, &compiled))
        .collect()
}

/// Check if a line matches a rule's match conditions.
fn matches_rule(line: &str, rule: &MatchRule, compiled: &CompiledRules) -> bool {
    let any_match = if rule.any.is_empty() {
        true
    } else {
        rule.any
            .iter()
            .any(|c| condition_matches(line, c, compiled))
    };

    let all_match = if rule.all.is_empty() {
        true
    } else {
        rule.all
            .iter()
            .all(|c| condition_matches(line, c, compiled))
    };

    let none_match = if rule.none.is_empty() {
        true
    } else {
        !rule
            .none
            .iter()
            .any(|c| condition_matches(line, c, compiled))
    };

    // If only 'any' is specified, it must match
    // If only 'all' is specified, all must match
    // If only 'none' is specified, none must match
    // Combinations: all specified groups must be satisfied
    any_match && all_match && none_match
}

/// Check a single match condition against a line.
fn condition_matches(line: &str, condition: &MatchCondition, compiled: &CompiledRules) -> bool {
    if let Some(ref substr) = condition.contains {
        if line.contains(substr.as_str()) {
            return true;
        }
    }

    if let Some(ref pattern) = condition.regex {
        if let Some(re) = compiled.cache.get(pattern) {
            if re.is_match(line) {
                return true;
            }
        }
    }

    false
}

use crate::config::{EventConfig, MatchCondition, MatchRule};
use crate::events::{fingerprint, CanonicalEvent};
use crate::ingestion::RawLogLine;

/// Classify a raw log line against a list of event rules.
///
/// Uses first-match-wins semantics: the first matching rule produces the event.
/// Unmatched lines return None.
pub fn classify_line(line: &RawLogLine, rules: &[EventConfig]) -> Option<CanonicalEvent> {
    for rule in rules {
        if matches_rule(&line.content, &rule.match_rule) {
            return Some(CanonicalEvent {
                timestamp: line
                    .timestamp
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
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
    lines
        .iter()
        .filter_map(|l| classify_line(l, rules))
        .collect()
}

/// Check if a line matches a rule's match conditions.
fn matches_rule(line: &str, rule: &MatchRule) -> bool {
    let any_match = if rule.any.is_empty() {
        true
    } else {
        rule.any.iter().any(|c| condition_matches(line, c))
    };

    let all_match = if rule.all.is_empty() {
        true
    } else {
        rule.all.iter().all(|c| condition_matches(line, c))
    };

    let none_match = if rule.none.is_empty() {
        true
    } else {
        !rule.none.iter().any(|c| condition_matches(line, c))
    };

    // If only 'any' is specified, it must match
    // If only 'all' is specified, all must match
    // If only 'none' is specified, none must match
    // Combinations: all specified groups must be satisfied
    any_match && all_match && none_match
}

/// Check a single match condition against a line.
fn condition_matches(line: &str, condition: &MatchCondition) -> bool {
    if let Some(ref substr) = condition.contains {
        if line.contains(substr.as_str()) {
            return true;
        }
    }

    if let Some(ref pattern) = condition.regex {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(line) {
                return true;
            }
        }
    }

    false
}

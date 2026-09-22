//! Parses chart-focused target drafts (Q-059) into symbol/timeframe pairs.
//!
//! A single token that matches a Q-056 supported timeframe is treated as timeframe-only;
//! otherwise it is a symbol. `ChartContext::request_target` remains the authority for
//! validation and retargeting.

use crate::chart_target::is_valid_symbol;

/// Supported timeframes from `ChartTargetPicker` (Q-056).
pub const SUPPORTED_TIMEFRAMES: [&str; 10] =
    ["1s", "5s", "1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTargetInput {
    pub symbol: String,
    pub timeframe: String,
    /// Explains a single-token parse (timeframe-only vs symbol-only).
    pub hint: String,
}

pub fn is_supported_timeframe(timeframe: &str) -> bool {
    let norm = timeframe.trim().to_lowercase();
    SUPPORTED_TIMEFRAMES.iter().any(|tf| *tf == norm)
}

pub fn canonical_timeframe(timeframe: &str) -> Option<String> {
    let norm = timeframe.trim().to_lowercase();
    SUPPORTED_TIMEFRAMES
        .iter()
        .find(|tf| **tf == norm)
        .map(|tf| (*tf).to_string())
}

fn looks_like_timeframe(token: &str) -> bool {
    let norm = token.trim().to_lowercase();
    if norm.len() < 2 {
        return false;
    }
    let unit = norm.chars().last().unwrap();
    if !matches!(unit, 's' | 'm' | 'h' | 'd' | 'w') {
        return false;
    }
    norm.chars()
        .take(norm.len() - 1)
        .all(|c| c.is_ascii_digit())
}

/// Turns operator draft text into the pair that would be submitted.
pub fn parse_target_draft(
    draft: &str,
    effective_symbol: &str,
    effective_timeframe: &str,
) -> Result<ParsedTargetInput, String> {
    let trimmed = draft.trim();
    if trimmed.is_empty() {
        return Err(format!("Enter a symbol, timeframe, or both in '{draft}'"));
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    let eff_sym = effective_symbol.trim();
    let eff_tf = effective_timeframe.trim();

    match tokens.len() {
        1 => {
            let token = tokens[0];
            if is_supported_timeframe(token) {
                let timeframe = canonical_timeframe(token).expect("supported timeframe");
                let symbol = if eff_sym.is_empty() {
                    return Err(format!(
                        "Symbol required with timeframe-only input '{draft}'"
                    ));
                } else {
                    eff_sym.to_uppercase()
                };
                let hint = format!("Timeframe only; keeping symbol {symbol}");
                Ok(ParsedTargetInput {
                    symbol,
                    timeframe,
                    hint,
                })
            } else {
                let symbol = token.trim().to_uppercase();
                if symbol.is_empty() {
                    return Err(format!("Symbol cannot be empty in '{draft}'"));
                }
                if !is_valid_symbol(&symbol) {
                    return Err(format!("Invalid symbol in '{draft}'"));
                }
                let timeframe = if eff_tf.is_empty() {
                    return Err(format!(
                        "Timeframe required with symbol-only input '{draft}'"
                    ));
                } else if is_supported_timeframe(eff_tf) {
                    canonical_timeframe(eff_tf).expect("effective timeframe")
                } else {
                    eff_tf.to_string()
                };
                let hint = format!("Symbol only; keeping timeframe {timeframe}");
                Ok(ParsedTargetInput {
                    symbol,
                    timeframe,
                    hint,
                })
            }
        }
        2 => {
            let symbol = tokens[0].trim().to_uppercase();
            let timeframe_token = tokens[1];
            if symbol.is_empty() {
                return Err(format!("Symbol cannot be empty in '{draft}'"));
            }
            if !is_valid_symbol(&symbol) {
                return Err(format!("Invalid symbol in '{draft}'"));
            }
            if !is_supported_timeframe(timeframe_token) {
                if is_valid_symbol(timeframe_token) && !looks_like_timeframe(timeframe_token) {
                    return Err(format!("Too many tokens in '{draft}'"));
                }
                return Err(format!(
                    "Unsupported timeframe '{timeframe_token}' in '{draft}'"
                ));
            }
            let timeframe = canonical_timeframe(timeframe_token).expect("supported timeframe");
            Ok(ParsedTargetInput {
                symbol,
                timeframe,
                hint: String::new(),
            })
        }
        _ => Err(format!("Too many tokens in '{draft}'")),
    }
}

/// JSON result for QML: `{"ok":true,"symbol":"...","timeframe":"...","hint":"..."}`
/// or `{"ok":false,"error":"..."}`.
pub fn parse_target_draft_json(
    draft: &str,
    effective_symbol: &str,
    effective_timeframe: &str,
) -> String {
    match parse_target_draft(draft, effective_symbol, effective_timeframe) {
        Ok(parsed) => format!(
            r#"{{"ok":true,"symbol":"{}","timeframe":"{}","hint":"{}"}}"#,
            escape_json(&parsed.symbol),
            escape_json(&parsed.timeframe),
            escape_json(&parsed.hint),
        ),
        Err(err) => format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(&err)),
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_only_preserves_timeframe() {
        let parsed = parse_target_draft("PETR4", "VALE3", "5m").expect("parsed");
        assert_eq!(parsed.symbol, "PETR4");
        assert_eq!(parsed.timeframe, "5m");
        assert!(parsed.hint.contains("Symbol only"));
        assert!(parsed.hint.contains("5m"));
    }

    #[test]
    fn timeframe_only_preserves_symbol() {
        let parsed = parse_target_draft("5m", "PETR4", "1m").expect("parsed");
        assert_eq!(parsed.symbol, "PETR4");
        assert_eq!(parsed.timeframe, "5m");
        assert!(parsed.hint.contains("Timeframe only"));
        assert!(parsed.hint.contains("PETR4"));
    }

    #[test]
    fn combined_input_sets_both_fields() {
        let parsed = parse_target_draft("PETR4 5m", "VALE3", "1m").expect("parsed");
        assert_eq!(parsed.symbol, "PETR4");
        assert_eq!(parsed.timeframe, "5m");
        assert!(parsed.hint.is_empty());
    }

    #[test]
    fn lowercase_and_whitespace_are_normalized() {
        let parsed = parse_target_draft("  petr4   5m  ", "VALE3", "1m").expect("parsed");
        assert_eq!(parsed.symbol, "PETR4");
        assert_eq!(parsed.timeframe, "5m");
    }

    #[test]
    fn blank_input_names_the_input() {
        let err = parse_target_draft("   ", "PETR4", "1m").unwrap_err();
        assert!(err.contains("Enter a symbol"));
    }

    #[test]
    fn two_symbols_is_rejected() {
        let err = parse_target_draft("PETR4 VALE3", "PETR4", "1m").unwrap_err();
        assert!(err.contains("Too many tokens"));
        assert!(err.contains("PETR4 VALE3"));
    }

    #[test]
    fn unsupported_timeframe_names_the_input() {
        let err = parse_target_draft("PETR4 99m", "PETR4", "1m").unwrap_err();
        assert!(err.contains("Unsupported timeframe '99m'"));
        assert!(err.contains("PETR4 99m"));
    }

    #[test]
    fn ambiguous_token_prefers_supported_timeframe() {
        let parsed = parse_target_draft("1m", "PETR4", "5m").expect("parsed");
        assert_eq!(parsed.timeframe, "1m");
        assert_eq!(parsed.symbol, "PETR4");
        assert!(parsed.hint.contains("Timeframe only"));
    }

    #[test]
    fn invalid_symbol_names_the_input() {
        let err = parse_target_draft("BAD!", "PETR4", "1m").unwrap_err();
        assert!(err.contains("Invalid symbol"));
        assert!(err.contains("BAD!"));
    }
}

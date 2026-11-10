//! Turn price text like "$1,299.99", "1 299,90 ₽" or "1.299,00 €" into a number.

use std::str::FromStr;

use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPrice {
    pub amount: Decimal,
    pub currency: Option<String>,
}

/// Parse the first price found in `text`.
pub fn parse_price(text: &str) -> Option<ParsedPrice> {
    let amount = parse_amount(text)?;
    Some(ParsedPrice {
        amount,
        currency: detect_currency(text),
    })
}

/// Parse just the number. Handles thousands separators (`,` `.` space,
/// non-breaking space, apostrophe) and both `.` and `,` as decimal mark.
pub fn parse_amount(text: &str) -> Option<Decimal> {
    let token = number_token(text)?;
    let normalized = normalize(&token)?;
    let value = Decimal::from_str(&normalized).ok()?;
    (value > Decimal::ZERO).then(|| value.round_dp(2))
}

/// Parse a machine-readable value (JSON-LD, `content="..."` attributes).
/// These always use `.` as the decimal mark, so "1299.000" means 1299,
/// not 1 299 000. Falls back to the human-text parser.
pub fn parse_machine_amount(text: &str) -> Option<Decimal> {
    match Decimal::from_str(text.trim()) {
        Ok(v) if v > Decimal::ZERO => Some(v.round_dp(2)),
        Ok(_) => None,
        Err(_) => parse_amount(text),
    }
}

fn is_group_space(c: char) -> bool {
    matches!(c, ' ' | '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\'')
}

/// Grab the first run that looks like a number: digits with separators between them.
fn number_token(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.iter().position(|c| c.is_ascii_digit())?;

    let mut token = String::new();
    let mut i = start;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            token.push(c);
        } else if c == '.' || c == ',' || is_group_space(c) {
            // A separator only counts if a digit follows it.
            match chars.get(i + 1) {
                Some(next) if next.is_ascii_digit() => {
                    if !is_group_space(c) {
                        token.push(c);
                    }
                    // Spaces are only ever thousands separators: drop them.
                }
                _ => break,
            }
        } else {
            break;
        }
        i += 1;
    }
    Some(token)
}

/// Convert a token like "1.299,99" into "1299.99".
fn normalize(token: &str) -> Option<String> {
    let last_dot = token.rfind('.');
    let last_comma = token.rfind(',');

    let decimal_sep = match (last_dot, last_comma) {
        (None, None) => None,
        // Both present: whichever comes last is the decimal mark.
        (Some(d), Some(c)) => Some(if d > c { '.' } else { ',' }),
        (Some(_), None) => single_kind_decimal(token, '.'),
        (None, Some(_)) => single_kind_decimal(token, ','),
    };

    let mut out = String::with_capacity(token.len());
    let decimal_pos = decimal_sep.and_then(|sep| token.rfind(sep));
    for (i, c) in token.char_indices() {
        match c {
            '0'..='9' => out.push(c),
            _ if Some(i) == decimal_pos => out.push('.'),
            _ => {} // thousands separator
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Only one kind of separator. It is a decimal mark when it appears once and
/// is followed by 1–2 digits ("12.5", "12,99"). "1,299" or "1.299.000" are
/// thousands groups.
fn single_kind_decimal(token: &str, sep: char) -> Option<char> {
    if token.matches(sep).count() != 1 {
        return None;
    }
    let after = token.rsplit(sep).next().unwrap_or("");
    (after.len() != 3).then_some(sep)
}

/// Best-effort currency detection from symbols, ISO codes and common words.
pub fn detect_currency(text: &str) -> Option<String> {
    const SYMBOLS: &[(&str, &str)] = &[
        ("US$", "USD"),
        ("$", "USD"),
        ("€", "EUR"),
        ("£", "GBP"),
        ("₽", "RUB"),
        ("руб", "RUB"),
        ("֏", "AMD"),
        ("драм", "AMD"),
        ("₴", "UAH"),
        ("грн", "UAH"),
        ("₸", "KZT"),
        ("¥", "JPY"),
        ("₹", "INR"),
        ("₺", "TRY"),
        ("zł", "PLN"),
    ];
    const CODES: &[&str] = &[
        "USD", "EUR", "GBP", "RUB", "AMD", "UAH", "KZT", "JPY", "CNY", "INR", "TRY", "PLN", "CHF",
        "CAD", "AUD", "GEL", "BYN",
    ];

    let upper = text.to_uppercase();
    if let Some(code) = CODES.iter().find(|code| upper.contains(*code)) {
        return Some((*code).to_string());
    }
    let lower = text.to_lowercase();
    SYMBOLS
        .iter()
        .find(|(sym, _)| lower.contains(sym))
        .map(|(_, code)| (*code).to_string())
}

use std::str::FromStr;

use pricewatchproject::scraper::parse::{parse_amount, parse_machine_amount, parse_price};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

#[test]
fn plain_numbers() {
    assert_eq!(parse_amount("1299"), Some(d("1299")));
    assert_eq!(parse_amount("12.5"), Some(d("12.5")));
    assert_eq!(parse_amount("12,99"), Some(d("12.99")));
}

#[test]
fn us_format() {
    let p = parse_price("$1,299.99").unwrap();
    assert_eq!(p.amount, d("1299.99"));
    assert_eq!(p.currency.as_deref(), Some("USD"));
    assert_eq!(parse_amount("1,234,567.89"), Some(d("1234567.89")));
}

#[test]
fn european_format() {
    let p = parse_price("1.299,90 €").unwrap();
    assert_eq!(p.amount, d("1299.90"));
    assert_eq!(p.currency.as_deref(), Some("EUR"));
}

#[test]
fn russian_format_with_spaces() {
    let p = parse_price("Цена: 89\u{00A0}990 ₽").unwrap();
    assert_eq!(p.amount, d("89990"));
    assert_eq!(p.currency.as_deref(), Some("RUB"));

    let p = parse_price("1 299,50 руб.").unwrap();
    assert_eq!(p.amount, d("1299.50"));
    assert_eq!(p.currency.as_deref(), Some("RUB"));
}

#[test]
fn armenian_dram() {
    let p = parse_price("45 000 ֏").unwrap();
    assert_eq!(p.amount, d("45000"));
    assert_eq!(p.currency.as_deref(), Some("AMD"));
}

#[test]
fn iso_code() {
    let p = parse_price("USD 49.00").unwrap();
    assert_eq!(p.amount, d("49.00"));
    assert_eq!(p.currency.as_deref(), Some("USD"));
}

#[test]
fn three_digits_after_single_separator_is_thousands() {
    assert_eq!(parse_amount("1,299"), Some(d("1299")));
    assert_eq!(parse_amount("1.299"), Some(d("1299")));
    assert_eq!(parse_amount("1.299.000"), Some(d("1299000")));
}

#[test]
fn rounds_to_cents() {
    assert_eq!(parse_amount("9.999"), Some(d("9999")));
    assert_eq!(parse_machine_amount("9.999"), Some(d("10.00")));
}

#[test]
fn machine_values() {
    assert_eq!(parse_machine_amount("1299.000"), Some(d("1299")));
    assert_eq!(parse_machine_amount(" 49.5 "), Some(d("49.5")));
    assert_eq!(parse_machine_amount("0"), None);
}

#[test]
fn stops_at_first_number() {
    assert_eq!(parse_amount("1 299 ₽ (was 1 599 ₽)"), Some(d("1299")));
    assert_eq!(parse_amount("Sale 499. Order now"), Some(d("499")));
}

#[test]
fn rejects_garbage() {
    assert_eq!(parse_price("out of stock"), None);
    assert_eq!(parse_price(""), None);
    assert_eq!(parse_amount("0.00"), None);
}

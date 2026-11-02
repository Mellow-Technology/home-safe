//! Display formatting for table cells and the detail panel.

pub fn number(v: Option<i64>) -> String {
    v.map(group_thousands).unwrap_or_default()
}

pub fn money(v: Option<i64>) -> String {
    v.map(|n| format!("${}", group_thousands(n))).unwrap_or_default()
}

pub fn decimal(v: Option<f64>) -> String {
    match v {
        Some(n) if n.fract() == 0.0 => format!("{n:.0}"),
        Some(n) => format!("{n:.1}"),
        None => String::new(),
    }
}

/// `"SINGLE_FAMILY"` -> `"Single family"`.
pub fn humanize(v: Option<&str>) -> String {
    let Some(v) = v else { return String::new() };
    let lower = v.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// RFC 3339 timestamp -> `YYYY-MM-DD`.
pub fn date(ts: &str) -> String {
    ts.get(..10).unwrap_or(ts).to_string()
}

fn group_thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(money(Some(1_234_567)), "$1,234,567");
        assert_eq!(money(Some(999)), "$999");
        assert_eq!(number(Some(-1000)), "-1,000");
        assert_eq!(decimal(Some(2.5)), "2.5");
        assert_eq!(decimal(Some(3.0)), "3");
        assert_eq!(humanize(Some("SINGLE_FAMILY")), "Single family");
        assert_eq!(date("2026-11-02T10:00:00+00:00"), "2026-11-02");
    }
}

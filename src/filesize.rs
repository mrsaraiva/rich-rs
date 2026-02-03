//! Filesize formatting utilities (subset of Python Rich's `filesize.py`).

/// Pick the largest unit <= `value` from `suffixes` with the given base.
///
/// Returns `(unit, suffix)` where `unit` is the divisor to apply to `value`.
pub fn pick_unit_and_suffix<'a>(value: u64, suffixes: &'a [&'a str], base: u64) -> (u64, &'a str) {
    if suffixes.is_empty() || base < 2 {
        return (1, "");
    }

    let mut unit: u64 = 1;
    let mut index: usize = 0;
    while index + 1 < suffixes.len() && value >= unit.saturating_mul(base) {
        unit = unit.saturating_mul(base);
        index += 1;
    }

    (unit, suffixes[index])
}

fn format_commas_u64(mut n: u64) -> String {
    let mut parts: Vec<String> = Vec::new();
    if n == 0 {
        return "0".to_string();
    }
    while n > 0 {
        let chunk = (n % 1000) as u16;
        n /= 1000;
        if n > 0 {
            parts.push(format!("{chunk:03}"));
        } else {
            parts.push(format!("{chunk}"));
        }
    }
    parts.reverse();
    parts.join(",")
}

fn format_float_with_commas(value: f64, precision: usize) -> String {
    let s = format!("{value:.precision$}", precision = precision);
    let Some((int_part, frac_part)) = s.split_once('.') else {
        // No decimal point; just group digits.
        return format_commas_u64(s.parse::<u64>().unwrap_or(0));
    };
    let grouped = format_commas_u64(int_part.parse::<u64>().unwrap_or(0));
    format!("{grouped}.{frac_part}")
}

/// Convert a filesize to a decimal string (powers of 1000), matching Rich's behavior.
pub fn decimal(size: u64) -> String {
    // Rich special-cases 1 byte and < 1000 bytes with "bytes".
    if size == 1 {
        return "1 byte".to_string();
    }
    if size < 1000 {
        return format!("{} bytes", format_commas_u64(size));
    }

    const SUFFIXES: [&str; 9] = ["bytes", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let (unit, suffix) = pick_unit_and_suffix(size, &SUFFIXES, 1000);
    let value = size as f64 / unit as f64;
    let formatted = format_float_with_commas(value, 1);
    format!("{formatted} {suffix}")
}

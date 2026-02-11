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

fn format_grouped_u64(mut n: u64, separator: &str) -> String {
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
    parts.join(separator)
}

fn format_float_with_separator(value: f64, precision: usize, separator: &str) -> String {
    let s = format!("{value:.precision$}", precision = precision);
    let Some((int_part, frac_part)) = s.split_once('.') else {
        return format_grouped_u64(s.parse::<u64>().unwrap_or(0), separator);
    };
    let grouped = format_grouped_u64(int_part.parse::<u64>().unwrap_or(0), separator);
    format!("{grouped}.{frac_part}")
}

/// Convert a filesize to a decimal string (powers of 1000), matching Rich's behavior.
pub fn decimal(size: u64) -> String {
    decimal_with_params(size, 1, ",")
}

/// Convert a filesize to a decimal string with configurable precision and thousands separator.
///
/// # Arguments
///
/// * `size` - The file size in bytes.
/// * `precision` - Number of decimal places (default in `decimal()`: 1).
/// * `separator` - Thousands separator string (default in `decimal()`: ",").
pub fn decimal_with_params(size: u64, precision: usize, separator: &str) -> String {
    if size == 1 {
        return "1 byte".to_string();
    }
    if size < 1000 {
        return format!("{} bytes", format_grouped_u64(size, separator));
    }

    const SUFFIXES: [&str; 9] = ["bytes", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let (unit, suffix) = pick_unit_and_suffix(size, &SUFFIXES, 1000);
    let value = size as f64 / unit as f64;
    let formatted = format_float_with_separator(value, precision, separator);
    format!("{formatted} {suffix}")
}

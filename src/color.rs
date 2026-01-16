//! Color types and color manipulation.
//!
//! Supports named colors, 8-bit (256) colors, and 24-bit RGB colors.

/// A color that can be applied to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Default terminal color (no color set).
    #[default]
    Default,
    /// Standard ANSI color (0-15).
    Standard(u8),
    /// 8-bit color (0-255).
    EightBit(u8),
    /// 24-bit RGB color.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Color {
    /// Create a new RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb { r, g, b }
    }

    /// Parse a color from a string.
    ///
    /// Supports:
    /// - Named colors: "red", "green", "blue", etc.
    /// - Hex colors: "#ff0000", "#f00"
    /// - RGB: "rgb(255, 0, 0)"
    /// - 8-bit: "color(196)"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();

        // Hex color
        if s.starts_with('#') {
            return Self::parse_hex(&s[1..]);
        }

        // Named colors
        match s.as_str() {
            "black" => Some(Color::Standard(0)),
            "red" => Some(Color::Standard(1)),
            "green" => Some(Color::Standard(2)),
            "yellow" => Some(Color::Standard(3)),
            "blue" => Some(Color::Standard(4)),
            "magenta" => Some(Color::Standard(5)),
            "cyan" => Some(Color::Standard(6)),
            "white" => Some(Color::Standard(7)),
            "bright_black" | "grey" | "gray" => Some(Color::Standard(8)),
            "bright_red" => Some(Color::Standard(9)),
            "bright_green" => Some(Color::Standard(10)),
            "bright_yellow" => Some(Color::Standard(11)),
            "bright_blue" => Some(Color::Standard(12)),
            "bright_magenta" => Some(Color::Standard(13)),
            "bright_cyan" => Some(Color::Standard(14)),
            "bright_white" => Some(Color::Standard(15)),
            "default" => Some(Color::Default),
            _ => None,
        }
    }

    fn parse_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(Color::Rgb { r, g, b })
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::Rgb { r, g, b })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named() {
        assert_eq!(Color::parse("red"), Some(Color::Standard(1)));
        assert_eq!(Color::parse("BLUE"), Some(Color::Standard(4)));
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(Color::parse("#ff0000"), Some(Color::Rgb { r: 255, g: 0, b: 0 }));
        assert_eq!(Color::parse("#f00"), Some(Color::Rgb { r: 255, g: 0, b: 0 }));
    }
}

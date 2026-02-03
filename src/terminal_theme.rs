//! Terminal themes for SVG/HTML export.
//!
//! This module provides color themes used when exporting console content to SVG or HTML.
//! The themes define background, foreground, and ANSI color palettes that determine how
//! styled text appears in the exported output.

use once_cell::sync::Lazy;

use crate::color::ColorTriplet;

/// A color theme used when exporting console content.
///
/// This struct holds the color information needed to render console output
/// in formats like SVG or HTML, where terminal colors need to be converted
/// to actual RGB values.
///
/// # Example
///
/// ```
/// use rich_rs::terminal_theme::TerminalTheme;
///
/// let theme = TerminalTheme::new(
///     (0, 0, 0),       // background
///     (255, 255, 255), // foreground
///     &[
///         (0, 0, 0),       // black
///         (128, 0, 0),     // red
///         (0, 128, 0),     // green
///         (128, 128, 0),   // yellow
///         (0, 0, 128),     // blue
///         (128, 0, 128),   // magenta
///         (0, 128, 128),   // cyan
///         (192, 192, 192), // white
///     ],
///     None, // bright colors (uses normal if None)
/// );
/// ```
#[derive(Debug, Clone)]
pub struct TerminalTheme {
    /// The background color of the terminal.
    pub background_color: ColorTriplet,
    /// The foreground (text) color of the terminal.
    pub foreground_color: ColorTriplet,
    /// ANSI colors (16 colors: 8 normal + 8 bright).
    pub ansi_colors: Vec<ColorTriplet>,
}

impl TerminalTheme {
    /// Create a new terminal theme.
    ///
    /// # Arguments
    ///
    /// * `background` - The background color as (r, g, b).
    /// * `foreground` - The foreground (text) color as (r, g, b).
    /// * `normal` - A slice of 8 normal intensity colors.
    /// * `bright` - Optional slice of 8 bright colors; if `None`, normal colors are repeated.
    pub fn new(
        background: (u8, u8, u8),
        foreground: (u8, u8, u8),
        normal: &[(u8, u8, u8)],
        bright: Option<&[(u8, u8, u8)]>,
    ) -> Self {
        let mut ansi_colors = Vec::with_capacity(16);

        // Add normal colors (0-7)
        for &(r, g, b) in normal {
            ansi_colors.push(ColorTriplet::new(r, g, b));
        }

        // Add bright colors (8-15), or repeat normal if not provided
        for &(r, g, b) in bright.unwrap_or(normal) {
            ansi_colors.push(ColorTriplet::new(r, g, b));
        }

        Self {
            background_color: ColorTriplet::new(background.0, background.1, background.2),
            foreground_color: ColorTriplet::new(foreground.0, foreground.1, foreground.2),
            ansi_colors,
        }
    }

    /// Get an ANSI color by index (0-15).
    ///
    /// Returns the foreground color if the index is out of bounds.
    pub fn get_ansi_color(&self, index: usize) -> ColorTriplet {
        self.ansi_colors
            .get(index)
            .copied()
            .unwrap_or(self.foreground_color)
    }
}

/// Default terminal theme (light background).
pub static DEFAULT_TERMINAL_THEME: Lazy<TerminalTheme> = Lazy::new(|| {
    TerminalTheme::new(
        (255, 255, 255), // white background
        (0, 0, 0),       // black foreground
        &[
            (0, 0, 0),       // black
            (128, 0, 0),     // red
            (0, 128, 0),     // green
            (128, 128, 0),   // yellow
            (0, 0, 128),     // blue
            (128, 0, 128),   // magenta
            (0, 128, 128),   // cyan
            (192, 192, 192), // white
        ],
        Some(&[
            (128, 128, 128), // bright black
            (255, 0, 0),     // bright red
            (0, 255, 0),     // bright green
            (255, 255, 0),   // bright yellow
            (0, 0, 255),     // bright blue
            (255, 0, 255),   // bright magenta
            (0, 255, 255),   // bright cyan
            (255, 255, 255), // bright white
        ]),
    )
});

/// SVG export theme (dark background, optimized for SVG rendering).
pub static SVG_EXPORT_THEME: Lazy<TerminalTheme> = Lazy::new(|| {
    TerminalTheme::new(
        (41, 41, 41),    // dark background
        (197, 200, 198), // light foreground
        &[
            (75, 78, 85),    // black
            (204, 85, 90),   // red
            (152, 168, 75),  // green
            (208, 179, 68),  // yellow
            (96, 138, 177),  // blue
            (152, 114, 159), // magenta
            (104, 160, 179), // cyan
            (197, 200, 198), // white
        ],
        Some(&[
            (154, 155, 153), // bright black
            (255, 38, 39),   // bright red
            (0, 130, 61),    // bright green
            (208, 132, 66),  // bright yellow
            (25, 132, 233),  // bright blue
            (255, 44, 122),  // bright magenta
            (57, 130, 128),  // bright cyan
            (253, 253, 197), // bright white
        ]),
    )
});

/// Monokai theme (popular dark theme).
pub static MONOKAI: Lazy<TerminalTheme> = Lazy::new(|| {
    TerminalTheme::new(
        (12, 12, 12),    // dark background
        (217, 217, 217), // light foreground
        &[
            (26, 26, 26),    // black
            (244, 0, 95),    // red (Monokai pink)
            (152, 224, 36),  // green
            (253, 151, 31),  // yellow/orange
            (157, 101, 255), // blue (actually purple in Monokai)
            (244, 0, 95),    // magenta (same as red)
            (88, 209, 235),  // cyan
            (196, 197, 181), // white
        ],
        Some(&[
            (98, 94, 76),    // bright black
            (244, 0, 95),    // bright red
            (152, 224, 36),  // bright green
            (224, 213, 97),  // bright yellow
            (157, 101, 255), // bright blue
            (244, 0, 95),    // bright magenta
            (88, 209, 235),  // bright cyan
            (246, 246, 239), // bright white
        ]),
    )
});

/// Dimmed Monokai theme (softer colors).
pub static DIMMED_MONOKAI: Lazy<TerminalTheme> = Lazy::new(|| {
    TerminalTheme::new(
        (25, 25, 25),    // dark background
        (185, 188, 186), // light foreground
        &[
            (58, 61, 67),    // black
            (190, 63, 72),   // red
            (135, 154, 59),  // green
            (197, 166, 53),  // yellow
            (79, 118, 161),  // blue
            (133, 92, 141),  // magenta
            (87, 143, 164),  // cyan
            (185, 188, 186), // white
        ],
        Some(&[
            (136, 137, 135), // bright black
            (251, 0, 31),    // bright red
            (15, 114, 47),   // bright green
            (196, 112, 51),  // bright yellow
            (24, 109, 227),  // bright blue
            (251, 0, 103),   // bright magenta
            (46, 112, 109),  // bright cyan
            (253, 255, 185), // bright white
        ]),
    )
});

/// Night Owlish theme (light background, colorful).
pub static NIGHT_OWLISH: Lazy<TerminalTheme> = Lazy::new(|| {
    TerminalTheme::new(
        (255, 255, 255), // white background
        (64, 63, 83),    // dark foreground
        &[
            (1, 22, 39),     // black
            (211, 66, 62),   // red
            (42, 162, 152),  // green
            (218, 170, 1),   // yellow
            (72, 118, 214),  // blue
            (64, 63, 83),    // magenta
            (8, 145, 106),   // cyan
            (122, 129, 129), // white
        ],
        Some(&[
            (122, 129, 129), // bright black
            (247, 110, 110), // bright red
            (73, 208, 197),  // bright green
            (218, 194, 107), // bright yellow
            (92, 167, 228),  // bright blue
            (105, 112, 152), // bright magenta
            (0, 201, 144),   // bright cyan
            (152, 159, 177), // bright white
        ]),
    )
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_theme_new() {
        let theme = TerminalTheme::new(
            (0, 0, 0),
            (255, 255, 255),
            &[
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
            ],
            None,
        );

        assert_eq!(theme.background_color, ColorTriplet::new(0, 0, 0));
        assert_eq!(theme.foreground_color, ColorTriplet::new(255, 255, 255));
        // Without bright colors, normal colors should be repeated
        assert_eq!(theme.ansi_colors.len(), 16);
        assert_eq!(theme.ansi_colors[0], theme.ansi_colors[8]);
    }

    #[test]
    fn test_terminal_theme_with_bright() {
        let theme = TerminalTheme::new(
            (0, 0, 0),
            (255, 255, 255),
            &[
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
            ],
            Some(&[
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ]),
        );

        assert_eq!(theme.ansi_colors.len(), 16);
        // Normal black
        assert_eq!(theme.ansi_colors[0], ColorTriplet::new(0, 0, 0));
        // Bright black (gray)
        assert_eq!(theme.ansi_colors[8], ColorTriplet::new(128, 128, 128));
    }

    #[test]
    fn test_get_ansi_color() {
        let theme = &*SVG_EXPORT_THEME;

        // Valid index
        let color = theme.get_ansi_color(1);
        assert_eq!(color, ColorTriplet::new(204, 85, 90));

        // Out of bounds returns foreground
        let color = theme.get_ansi_color(100);
        assert_eq!(color, theme.foreground_color);
    }

    #[test]
    fn test_svg_export_theme() {
        let theme = &*SVG_EXPORT_THEME;
        assert_eq!(theme.background_color, ColorTriplet::new(41, 41, 41));
        assert_eq!(theme.foreground_color, ColorTriplet::new(197, 200, 198));
        assert_eq!(theme.ansi_colors.len(), 16);
    }

    #[test]
    fn test_monokai_theme() {
        let theme = &*MONOKAI;
        assert_eq!(theme.background_color, ColorTriplet::new(12, 12, 12));
        assert_eq!(theme.foreground_color, ColorTriplet::new(217, 217, 217));
    }
}

//! Color types and color manipulation.
//!
//! This module provides comprehensive color support including:
//! - Named colors (~250 ANSI color names)
//! - 8-bit (256) colors
//! - 24-bit RGB (truecolor)
//! - Color parsing from various formats
//! - Color downgrading to lower color systems
//! - ANSI escape code generation

use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::error::ParseError;

// ============================================================================
// ColorTriplet
// ============================================================================

/// RGB color triplet with red, green, and blue components (0-255).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorTriplet {
    /// Red component (0-255).
    pub red: u8,
    /// Green component (0-255).
    pub green: u8,
    /// Blue component (0-255).
    pub blue: u8,
}

impl ColorTriplet {
    /// Create a new color triplet.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Get the color as a CSS-style hex string (e.g., "#ff0000").
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    /// Get the color as an RGB string (e.g., "rgb(255,0,0)").
    pub fn rgb(&self) -> String {
        format!("rgb({},{},{})", self.red, self.green, self.blue)
    }

    /// Get normalized color components as floats between 0.0 and 1.0.
    pub fn normalized(&self) -> (f64, f64, f64) {
        (
            self.red as f64 / 255.0,
            self.green as f64 / 255.0,
            self.blue as f64 / 255.0,
        )
    }
}

impl From<(u8, u8, u8)> for ColorTriplet {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::new(r, g, b)
    }
}

// ============================================================================
// ColorSystem
// ============================================================================

/// The color system supported by a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSystem {
    /// 16 standard ANSI colors (colors 0-15).
    Standard = 1,
    /// 256 colors (8-bit palette).
    EightBit = 2,
    /// 24-bit RGB (truecolor).
    TrueColor = 3,
    /// Windows legacy console colors.
    Windows = 4,
}

impl ColorSystem {
    /// Get the number of colors in this system.
    pub fn color_count(&self) -> usize {
        match self {
            ColorSystem::Standard => 16,
            ColorSystem::EightBit => 256,
            ColorSystem::TrueColor => 16_777_216,
            ColorSystem::Windows => 16,
        }
    }

    /// Get the capability rank of this color system for downgrade comparison.
    ///
    /// Windows is treated as equivalent to Standard (both are 16-color systems).
    /// This ensures TrueColor and EightBit colors are properly downgraded to Windows.
    fn rank(&self) -> u8 {
        match self {
            ColorSystem::Standard => 1,
            ColorSystem::Windows => 1, // treat as 16-color system
            ColorSystem::EightBit => 2,
            ColorSystem::TrueColor => 3,
        }
    }
}

// ============================================================================
// ColorType
// ============================================================================

/// The type of color stored in a Color struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorType {
    /// Default terminal color (no specific color set).
    #[default]
    Default,
    /// Standard ANSI color (index 0-15).
    Standard,
    /// 8-bit color (index 0-255).
    EightBit,
    /// 24-bit RGB (truecolor).
    TrueColor,
    /// Windows legacy console color.
    Windows,
}

impl ColorType {
    /// Get the corresponding color system for this color type.
    pub fn system(&self) -> ColorSystem {
        match self {
            ColorType::Default => ColorSystem::Standard,
            ColorType::Standard => ColorSystem::Standard,
            ColorType::EightBit => ColorSystem::EightBit,
            ColorType::TrueColor => ColorSystem::TrueColor,
            ColorType::Windows => ColorSystem::Windows,
        }
    }
}

// ============================================================================
// Palette
// ============================================================================

/// A palette of colors for color matching.
#[derive(Debug, Clone)]
pub struct Palette {
    colors: Vec<ColorTriplet>,
}

impl Palette {
    /// Create a new palette from a slice of RGB tuples.
    pub fn new(colors: &[(u8, u8, u8)]) -> Self {
        Self {
            colors: colors.iter().map(|&c| ColorTriplet::from(c)).collect(),
        }
    }

    /// Get a color triplet by index.
    pub fn get(&self, index: usize) -> Option<ColorTriplet> {
        self.colors.get(index).copied()
    }

    /// Get the number of colors in the palette.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Check if the palette is empty.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Find the closest color index in the palette to the given triplet.
    ///
    /// Uses weighted Euclidean distance in RGB space, with weights that
    /// account for human perception of color differences.
    pub fn match_color(&self, triplet: ColorTriplet) -> usize {
        let red1 = triplet.red as i32;
        let green1 = triplet.green as i32;
        let blue1 = triplet.blue as i32;

        let mut min_distance = f64::MAX;
        let mut min_index = 0;

        for (index, color) in self.colors.iter().enumerate() {
            let red2 = color.red as i32;
            let green2 = color.green as i32;
            let blue2 = color.blue as i32;

            // Weighted Euclidean distance (matches Python Rich's algorithm)
            let red_mean = (red1 + red2) / 2;
            let red_diff = red1 - red2;
            let green_diff = green1 - green2;
            let blue_diff = blue1 - blue2;

            let distance = (((512 + red_mean) * red_diff * red_diff) >> 8) as f64
                + (4 * green_diff * green_diff) as f64
                + (((767 - red_mean) * blue_diff * blue_diff) >> 8) as f64;
            let distance = distance.sqrt();

            if distance < min_distance {
                min_distance = distance;
                min_index = index;
            }
        }

        min_index
    }
}

impl std::ops::Index<usize> for Palette {
    type Output = ColorTriplet;

    fn index(&self, index: usize) -> &Self::Output {
        &self.colors[index]
    }
}

// ============================================================================
// Color Palettes (Static Data)
// ============================================================================

/// Windows 10 console color palette.
pub static WINDOWS_PALETTE: Lazy<Palette> = Lazy::new(|| {
    Palette::new(&[
        (12, 12, 12),
        (197, 15, 31),
        (19, 161, 14),
        (193, 156, 0),
        (0, 55, 218),
        (136, 23, 152),
        (58, 150, 221),
        (204, 204, 204),
        (118, 118, 118),
        (231, 72, 86),
        (22, 198, 12),
        (249, 241, 165),
        (59, 120, 255),
        (180, 0, 158),
        (97, 214, 214),
        (242, 242, 242),
    ])
});

/// Standard 16 ANSI color palette.
pub static STANDARD_PALETTE: Lazy<Palette> = Lazy::new(|| {
    Palette::new(&[
        (0, 0, 0),
        (170, 0, 0),
        (0, 170, 0),
        (170, 85, 0),
        (0, 0, 170),
        (170, 0, 170),
        (0, 170, 170),
        (170, 170, 170),
        (85, 85, 85),
        (255, 85, 85),
        (85, 255, 85),
        (255, 255, 85),
        (85, 85, 255),
        (255, 85, 255),
        (85, 255, 255),
        (255, 255, 255),
    ])
});

/// 256-color (8-bit) palette.
pub static EIGHT_BIT_PALETTE: Lazy<Palette> = Lazy::new(|| {
    Palette::new(&[
        // Standard colors (0-15)
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
        // 216 colors (6x6x6 color cube, indices 16-231)
        (0, 0, 0),
        (0, 0, 95),
        (0, 0, 135),
        (0, 0, 175),
        (0, 0, 215),
        (0, 0, 255),
        (0, 95, 0),
        (0, 95, 95),
        (0, 95, 135),
        (0, 95, 175),
        (0, 95, 215),
        (0, 95, 255),
        (0, 135, 0),
        (0, 135, 95),
        (0, 135, 135),
        (0, 135, 175),
        (0, 135, 215),
        (0, 135, 255),
        (0, 175, 0),
        (0, 175, 95),
        (0, 175, 135),
        (0, 175, 175),
        (0, 175, 215),
        (0, 175, 255),
        (0, 215, 0),
        (0, 215, 95),
        (0, 215, 135),
        (0, 215, 175),
        (0, 215, 215),
        (0, 215, 255),
        (0, 255, 0),
        (0, 255, 95),
        (0, 255, 135),
        (0, 255, 175),
        (0, 255, 215),
        (0, 255, 255),
        (95, 0, 0),
        (95, 0, 95),
        (95, 0, 135),
        (95, 0, 175),
        (95, 0, 215),
        (95, 0, 255),
        (95, 95, 0),
        (95, 95, 95),
        (95, 95, 135),
        (95, 95, 175),
        (95, 95, 215),
        (95, 95, 255),
        (95, 135, 0),
        (95, 135, 95),
        (95, 135, 135),
        (95, 135, 175),
        (95, 135, 215),
        (95, 135, 255),
        (95, 175, 0),
        (95, 175, 95),
        (95, 175, 135),
        (95, 175, 175),
        (95, 175, 215),
        (95, 175, 255),
        (95, 215, 0),
        (95, 215, 95),
        (95, 215, 135),
        (95, 215, 175),
        (95, 215, 215),
        (95, 215, 255),
        (95, 255, 0),
        (95, 255, 95),
        (95, 255, 135),
        (95, 255, 175),
        (95, 255, 215),
        (95, 255, 255),
        (135, 0, 0),
        (135, 0, 95),
        (135, 0, 135),
        (135, 0, 175),
        (135, 0, 215),
        (135, 0, 255),
        (135, 95, 0),
        (135, 95, 95),
        (135, 95, 135),
        (135, 95, 175),
        (135, 95, 215),
        (135, 95, 255),
        (135, 135, 0),
        (135, 135, 95),
        (135, 135, 135),
        (135, 135, 175),
        (135, 135, 215),
        (135, 135, 255),
        (135, 175, 0),
        (135, 175, 95),
        (135, 175, 135),
        (135, 175, 175),
        (135, 175, 215),
        (135, 175, 255),
        (135, 215, 0),
        (135, 215, 95),
        (135, 215, 135),
        (135, 215, 175),
        (135, 215, 215),
        (135, 215, 255),
        (135, 255, 0),
        (135, 255, 95),
        (135, 255, 135),
        (135, 255, 175),
        (135, 255, 215),
        (135, 255, 255),
        (175, 0, 0),
        (175, 0, 95),
        (175, 0, 135),
        (175, 0, 175),
        (175, 0, 215),
        (175, 0, 255),
        (175, 95, 0),
        (175, 95, 95),
        (175, 95, 135),
        (175, 95, 175),
        (175, 95, 215),
        (175, 95, 255),
        (175, 135, 0),
        (175, 135, 95),
        (175, 135, 135),
        (175, 135, 175),
        (175, 135, 215),
        (175, 135, 255),
        (175, 175, 0),
        (175, 175, 95),
        (175, 175, 135),
        (175, 175, 175),
        (175, 175, 215),
        (175, 175, 255),
        (175, 215, 0),
        (175, 215, 95),
        (175, 215, 135),
        (175, 215, 175),
        (175, 215, 215),
        (175, 215, 255),
        (175, 255, 0),
        (175, 255, 95),
        (175, 255, 135),
        (175, 255, 175),
        (175, 255, 215),
        (175, 255, 255),
        (215, 0, 0),
        (215, 0, 95),
        (215, 0, 135),
        (215, 0, 175),
        (215, 0, 215),
        (215, 0, 255),
        (215, 95, 0),
        (215, 95, 95),
        (215, 95, 135),
        (215, 95, 175),
        (215, 95, 215),
        (215, 95, 255),
        (215, 135, 0),
        (215, 135, 95),
        (215, 135, 135),
        (215, 135, 175),
        (215, 135, 215),
        (215, 135, 255),
        (215, 175, 0),
        (215, 175, 95),
        (215, 175, 135),
        (215, 175, 175),
        (215, 175, 215),
        (215, 175, 255),
        (215, 215, 0),
        (215, 215, 95),
        (215, 215, 135),
        (215, 215, 175),
        (215, 215, 215),
        (215, 215, 255),
        (215, 255, 0),
        (215, 255, 95),
        (215, 255, 135),
        (215, 255, 175),
        (215, 255, 215),
        (215, 255, 255),
        (255, 0, 0),
        (255, 0, 95),
        (255, 0, 135),
        (255, 0, 175),
        (255, 0, 215),
        (255, 0, 255),
        (255, 95, 0),
        (255, 95, 95),
        (255, 95, 135),
        (255, 95, 175),
        (255, 95, 215),
        (255, 95, 255),
        (255, 135, 0),
        (255, 135, 95),
        (255, 135, 135),
        (255, 135, 175),
        (255, 135, 215),
        (255, 135, 255),
        (255, 175, 0),
        (255, 175, 95),
        (255, 175, 135),
        (255, 175, 175),
        (255, 175, 215),
        (255, 175, 255),
        (255, 215, 0),
        (255, 215, 95),
        (255, 215, 135),
        (255, 215, 175),
        (255, 215, 215),
        (255, 215, 255),
        (255, 255, 0),
        (255, 255, 95),
        (255, 255, 135),
        (255, 255, 175),
        (255, 255, 215),
        (255, 255, 255),
        // Grayscale (24 shades, indices 232-255)
        (8, 8, 8),
        (18, 18, 18),
        (28, 28, 28),
        (38, 38, 38),
        (48, 48, 48),
        (58, 58, 58),
        (68, 68, 68),
        (78, 78, 78),
        (88, 88, 88),
        (98, 98, 98),
        (108, 108, 108),
        (118, 118, 118),
        (128, 128, 128),
        (138, 138, 138),
        (148, 148, 148),
        (158, 158, 158),
        (168, 168, 168),
        (178, 178, 178),
        (188, 188, 188),
        (198, 198, 198),
        (208, 208, 208),
        (218, 218, 218),
        (228, 228, 228),
        (238, 238, 238),
    ])
});

// ============================================================================
// ANSI Color Names
// ============================================================================

/// Map of ANSI color names to their color numbers.
/// Includes all ~250 named colors from Python Rich.
pub static ANSI_COLOR_NAMES: Lazy<HashMap<&'static str, u8>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Standard 16 colors
    m.insert("black", 0);
    m.insert("red", 1);
    m.insert("green", 2);
    m.insert("yellow", 3);
    m.insert("blue", 4);
    m.insert("magenta", 5);
    m.insert("cyan", 6);
    m.insert("white", 7);
    m.insert("bright_black", 8);
    m.insert("bright_red", 9);
    m.insert("bright_green", 10);
    m.insert("bright_yellow", 11);
    m.insert("bright_blue", 12);
    m.insert("bright_magenta", 13);
    m.insert("bright_cyan", 14);
    m.insert("bright_white", 15);
    // Extended colors (16-255)
    m.insert("grey0", 16);
    m.insert("gray0", 16);
    m.insert("navy_blue", 17);
    m.insert("dark_blue", 18);
    m.insert("blue3", 20);
    m.insert("blue1", 21);
    m.insert("dark_green", 22);
    m.insert("deep_sky_blue4", 25);
    m.insert("dodger_blue3", 26);
    m.insert("dodger_blue2", 27);
    m.insert("green4", 28);
    m.insert("spring_green4", 29);
    m.insert("turquoise4", 30);
    m.insert("deep_sky_blue3", 32);
    m.insert("dodger_blue1", 33);
    m.insert("green3", 40);
    m.insert("spring_green3", 41);
    m.insert("dark_cyan", 36);
    m.insert("light_sea_green", 37);
    m.insert("deep_sky_blue2", 38);
    m.insert("deep_sky_blue1", 39);
    m.insert("spring_green2", 47);
    m.insert("cyan3", 43);
    m.insert("dark_turquoise", 44);
    m.insert("turquoise2", 45);
    m.insert("green1", 46);
    m.insert("spring_green1", 48);
    m.insert("medium_spring_green", 49);
    m.insert("cyan2", 50);
    m.insert("cyan1", 51);
    m.insert("dark_red", 88);
    m.insert("deep_pink4", 125);
    m.insert("purple4", 55);
    m.insert("purple3", 56);
    m.insert("blue_violet", 57);
    m.insert("orange4", 94);
    m.insert("grey37", 59);
    m.insert("gray37", 59);
    m.insert("medium_purple4", 60);
    m.insert("slate_blue3", 62);
    m.insert("royal_blue1", 63);
    m.insert("chartreuse4", 64);
    m.insert("dark_sea_green4", 71);
    m.insert("pale_turquoise4", 66);
    m.insert("steel_blue", 67);
    m.insert("steel_blue3", 68);
    m.insert("cornflower_blue", 69);
    m.insert("chartreuse3", 76);
    m.insert("cadet_blue", 73);
    m.insert("sky_blue3", 74);
    m.insert("steel_blue1", 81);
    m.insert("pale_green3", 114);
    m.insert("sea_green3", 78);
    m.insert("aquamarine3", 79);
    m.insert("medium_turquoise", 80);
    m.insert("chartreuse2", 112);
    m.insert("sea_green2", 83);
    m.insert("sea_green1", 85);
    m.insert("aquamarine1", 122);
    m.insert("dark_slate_gray2", 87);
    m.insert("dark_magenta", 91);
    m.insert("dark_violet", 128);
    m.insert("purple", 129);
    m.insert("light_pink4", 95);
    m.insert("plum4", 96);
    m.insert("medium_purple3", 98);
    m.insert("slate_blue1", 99);
    m.insert("yellow4", 106);
    m.insert("wheat4", 101);
    m.insert("grey53", 102);
    m.insert("gray53", 102);
    m.insert("light_slate_grey", 103);
    m.insert("light_slate_gray", 103);
    m.insert("medium_purple", 104);
    m.insert("light_slate_blue", 105);
    m.insert("dark_olive_green3", 149);
    m.insert("dark_sea_green", 108);
    m.insert("light_sky_blue3", 110);
    m.insert("sky_blue2", 111);
    m.insert("dark_sea_green3", 150);
    m.insert("dark_slate_gray3", 116);
    m.insert("sky_blue1", 117);
    m.insert("chartreuse1", 118);
    m.insert("light_green", 120);
    m.insert("pale_green1", 156);
    m.insert("dark_slate_gray1", 123);
    m.insert("red3", 160);
    m.insert("medium_violet_red", 126);
    m.insert("magenta3", 164);
    m.insert("dark_orange3", 166);
    m.insert("indian_red", 167);
    m.insert("hot_pink3", 168);
    m.insert("medium_orchid3", 133);
    m.insert("medium_orchid", 134);
    m.insert("medium_purple2", 140);
    m.insert("dark_goldenrod", 136);
    m.insert("light_salmon3", 173);
    m.insert("rosy_brown", 138);
    m.insert("grey63", 139);
    m.insert("gray63", 139);
    m.insert("medium_purple1", 141);
    m.insert("gold3", 178);
    m.insert("dark_khaki", 143);
    m.insert("navajo_white3", 144);
    m.insert("grey69", 145);
    m.insert("gray69", 145);
    m.insert("light_steel_blue3", 146);
    m.insert("light_steel_blue", 147);
    m.insert("yellow3", 184);
    m.insert("dark_sea_green2", 157);
    m.insert("light_cyan3", 152);
    m.insert("light_sky_blue1", 153);
    m.insert("green_yellow", 154);
    m.insert("dark_olive_green2", 155);
    m.insert("dark_sea_green1", 193);
    m.insert("pale_turquoise1", 159);
    m.insert("deep_pink3", 162);
    m.insert("magenta2", 200);
    m.insert("hot_pink2", 169);
    m.insert("orchid", 170);
    m.insert("medium_orchid1", 207);
    m.insert("orange3", 172);
    m.insert("light_pink3", 174);
    m.insert("pink3", 175);
    m.insert("plum3", 176);
    m.insert("violet", 177);
    m.insert("light_goldenrod3", 179);
    m.insert("tan", 180);
    m.insert("misty_rose3", 181);
    m.insert("thistle3", 182);
    m.insert("plum2", 183);
    m.insert("khaki3", 185);
    m.insert("light_goldenrod2", 222);
    m.insert("light_yellow3", 187);
    m.insert("grey84", 188);
    m.insert("gray84", 188);
    m.insert("light_steel_blue1", 189);
    m.insert("yellow2", 190);
    m.insert("dark_olive_green1", 192);
    m.insert("honeydew2", 194);
    m.insert("light_cyan1", 195);
    m.insert("red1", 196);
    m.insert("deep_pink2", 197);
    m.insert("deep_pink1", 199);
    m.insert("magenta1", 201);
    m.insert("orange_red1", 202);
    m.insert("indian_red1", 204);
    m.insert("hot_pink", 206);
    m.insert("dark_orange", 208);
    m.insert("salmon1", 209);
    m.insert("light_coral", 210);
    m.insert("pale_violet_red1", 211);
    m.insert("orchid2", 212);
    m.insert("orchid1", 213);
    m.insert("orange1", 214);
    m.insert("sandy_brown", 215);
    m.insert("light_salmon1", 216);
    m.insert("light_pink1", 217);
    m.insert("pink1", 218);
    m.insert("plum1", 219);
    m.insert("gold1", 220);
    m.insert("navajo_white1", 223);
    m.insert("misty_rose1", 224);
    m.insert("thistle1", 225);
    m.insert("yellow1", 226);
    m.insert("light_goldenrod1", 227);
    m.insert("khaki1", 228);
    m.insert("wheat1", 229);
    m.insert("cornsilk1", 230);
    m.insert("grey100", 231);
    m.insert("gray100", 231);
    m.insert("grey3", 232);
    m.insert("gray3", 232);
    m.insert("grey7", 233);
    m.insert("gray7", 233);
    m.insert("grey11", 234);
    m.insert("gray11", 234);
    m.insert("grey15", 235);
    m.insert("gray15", 235);
    m.insert("grey19", 236);
    m.insert("gray19", 236);
    m.insert("grey23", 237);
    m.insert("gray23", 237);
    m.insert("grey27", 238);
    m.insert("gray27", 238);
    m.insert("grey30", 239);
    m.insert("gray30", 239);
    m.insert("grey35", 240);
    m.insert("gray35", 240);
    m.insert("grey39", 241);
    m.insert("gray39", 241);
    m.insert("grey42", 242);
    m.insert("gray42", 242);
    m.insert("grey46", 243);
    m.insert("gray46", 243);
    m.insert("grey50", 244);
    m.insert("gray50", 244);
    m.insert("grey54", 245);
    m.insert("gray54", 245);
    m.insert("grey58", 246);
    m.insert("gray58", 246);
    m.insert("grey62", 247);
    m.insert("gray62", 247);
    m.insert("grey66", 248);
    m.insert("gray66", 248);
    m.insert("grey70", 249);
    m.insert("gray70", 249);
    m.insert("grey74", 250);
    m.insert("gray74", 250);
    m.insert("grey78", 251);
    m.insert("gray78", 251);
    m.insert("grey82", 252);
    m.insert("gray82", 252);
    m.insert("grey85", 253);
    m.insert("gray85", 253);
    m.insert("grey89", 254);
    m.insert("gray89", 254);
    m.insert("grey93", 255);
    m.insert("gray93", 255);
    // Aliases
    m.insert("grey", 8);
    m.insert("gray", 8);
    m
});

// ============================================================================
// Color
// ============================================================================

/// A terminal color definition.
///
/// Colors can be:
/// - Default (terminal's default color)
/// - Standard (16 ANSI colors, 0-15)
/// - EightBit (256 colors, 0-255)
/// - TrueColor (24-bit RGB)
///
/// # Examples
///
/// ```
/// use rich_rs::Color;
///
/// // Parse from string
/// let red = Color::parse("red").unwrap();
/// let hex = Color::parse("#ff0000").unwrap();
/// let rgb = Color::parse("rgb(255,0,0)").unwrap();
/// let indexed = Color::parse("color(196)").unwrap();
///
/// // Create directly
/// let custom = Color::from_rgb(255, 128, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Color {
    /// The original name/representation of the color.
    pub name: String,
    /// The type of color.
    pub color_type: ColorType,
    /// The color number (for Standard and EightBit types).
    pub number: Option<u8>,
    /// The RGB triplet (for TrueColor type).
    pub triplet: Option<ColorTriplet>,
}

impl Default for Color {
    fn default() -> Self {
        Self::default_color()
    }
}

impl Color {
    /// Create a new Color with the given parameters.
    pub fn new(
        name: impl Into<String>,
        color_type: ColorType,
        number: Option<u8>,
        triplet: Option<ColorTriplet>,
    ) -> Self {
        Self {
            name: name.into(),
            color_type,
            number,
            triplet,
        }
    }

    /// Create the default terminal color.
    pub fn default_color() -> Self {
        Self::new("default", ColorType::Default, None, None)
    }

    /// Create a color from an 8-bit ANSI number (0-255).
    pub fn from_ansi(number: u8) -> Self {
        let color_type = if number < 16 {
            ColorType::Standard
        } else {
            ColorType::EightBit
        };
        Self::new(format!("color({})", number), color_type, Some(number), None)
    }

    /// Create a color from an RGB triplet.
    pub fn from_triplet(triplet: ColorTriplet) -> Self {
        Self::new(triplet.hex(), ColorType::TrueColor, None, Some(triplet))
    }

    /// Create a color from RGB values.
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::from_triplet(ColorTriplet::new(red, green, blue))
    }

    /// Get the color system for this color.
    pub fn system(&self) -> ColorSystem {
        self.color_type.system()
    }

    /// Check if this color is system-defined (Standard or Windows).
    pub fn is_system_defined(&self) -> bool {
        matches!(
            self.color_type.system(),
            ColorSystem::Standard | ColorSystem::Windows
        )
    }

    /// Check if this is the default color.
    pub fn is_default(&self) -> bool {
        self.color_type == ColorType::Default
    }

    /// Get the RGB triplet for this color.
    ///
    /// For Standard and EightBit colors, looks up the color in the palette.
    /// For Default colors, returns a typical terminal foreground/background.
    pub fn get_truecolor(&self, foreground: bool) -> ColorTriplet {
        match self.color_type {
            ColorType::TrueColor => self.triplet.unwrap_or_default(),
            ColorType::EightBit => {
                let number = self.number.unwrap_or(0) as usize;
                EIGHT_BIT_PALETTE.get(number).unwrap_or_default()
            }
            ColorType::Standard => {
                let number = self.number.unwrap_or(0) as usize;
                STANDARD_PALETTE.get(number).unwrap_or_default()
            }
            ColorType::Windows => {
                let number = self.number.unwrap_or(0) as usize;
                WINDOWS_PALETTE.get(number).unwrap_or_default()
            }
            ColorType::Default => {
                // Default terminal colors (typical values)
                if foreground {
                    ColorTriplet::new(255, 255, 255) // White
                } else {
                    ColorTriplet::new(0, 0, 0) // Black
                }
            }
        }
    }

    /// Parse a color from a string.
    ///
    /// Supported formats:
    /// - Named colors: "red", "bright_blue", "grey50"
    /// - Hex: "#ff0000" or "#f00"
    /// - RGB: "rgb(255,0,0)"
    /// - Indexed: "color(196)"
    /// - Default: "default"
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Color;
    ///
    /// assert!(Color::parse("red").is_ok());
    /// assert!(Color::parse("#ff0000").is_ok());
    /// assert!(Color::parse("rgb(255,0,0)").is_ok());
    /// assert!(Color::parse("color(196)").is_ok());
    /// assert!(Color::parse("invalid").is_err());
    /// ```
    pub fn parse(color: &str) -> Result<Self, ParseError> {
        let original_color = color;
        let color = color.to_lowercase();
        let color = color.trim();

        // Default color
        if color == "default" {
            return Ok(Self::default_color());
        }

        // Named color
        if let Some(&number) = ANSI_COLOR_NAMES.get(color) {
            let color_type = if number < 16 {
                ColorType::Standard
            } else {
                ColorType::EightBit
            };
            return Ok(Self::new(color.to_string(), color_type, Some(number), None));
        }

        // Hex color: #rrggbb or #rgb
        if let Some(hex) = color.strip_prefix('#') {
            return Self::parse_hex(hex, original_color);
        }

        // RGB: rgb(r,g,b)
        if let Some(inner) = color.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
            return Self::parse_rgb_string(inner, original_color);
        }

        // Indexed: color(n)
        if let Some(inner) = color
            .strip_prefix("color(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return Self::parse_color_number(inner, original_color);
        }

        Err(ParseError::invalid_color(format!(
            "{:?} is not a valid color",
            original_color
        )))
    }

    fn parse_hex(hex: &str, original: &str) -> Result<Self, ParseError> {
        let make_err =
            || ParseError::invalid_color(format!("{:?} is not a valid hex color", original));

        match hex.len() {
            3 => {
                // Short form: #rgb -> #rrggbb
                let chars: Vec<char> = hex.chars().collect();
                let r = u8::from_str_radix(&chars[0].to_string(), 16).map_err(|_| make_err())? * 17;
                let g = u8::from_str_radix(&chars[1].to_string(), 16).map_err(|_| make_err())? * 17;
                let b = u8::from_str_radix(&chars[2].to_string(), 16).map_err(|_| make_err())? * 17;
                let triplet = ColorTriplet::new(r, g, b);
                Ok(Self::new(
                    triplet.hex(),
                    ColorType::TrueColor,
                    None,
                    Some(triplet),
                ))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| make_err())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| make_err())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| make_err())?;
                let triplet = ColorTriplet::new(r, g, b);
                Ok(Self::new(
                    triplet.hex(),
                    ColorType::TrueColor,
                    None,
                    Some(triplet),
                ))
            }
            _ => Err(make_err()),
        }
    }

    fn parse_rgb_string(inner: &str, original: &str) -> Result<Self, ParseError> {
        let components: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if components.len() != 3 {
            return Err(ParseError::invalid_color(format!(
                "expected three components in {:?}",
                original
            )));
        }

        let parse_component = |s: &str| -> Result<u8, ParseError> {
            s.parse::<u16>()
                .map_err(|_| {
                    ParseError::invalid_color(format!("invalid RGB component in {:?}", original))
                })
                .and_then(|v| {
                    if v > 255 {
                        Err(ParseError::invalid_color(format!(
                            "color components must be <= 255 in {:?}",
                            original
                        )))
                    } else {
                        Ok(v as u8)
                    }
                })
        };

        let r = parse_component(components[0])?;
        let g = parse_component(components[1])?;
        let b = parse_component(components[2])?;

        let triplet = ColorTriplet::new(r, g, b);
        Ok(Self::new(
            format!("rgb({},{},{})", r, g, b),
            ColorType::TrueColor,
            None,
            Some(triplet),
        ))
    }

    fn parse_color_number(inner: &str, original: &str) -> Result<Self, ParseError> {
        let number: u16 = inner.trim().parse().map_err(|_| {
            ParseError::invalid_color(format!("invalid color number in {:?}", original))
        })?;

        if number > 255 {
            return Err(ParseError::invalid_color(format!(
                "color number must be <= 255 in {:?}",
                original
            )));
        }

        let number = number as u8;
        let color_type = if number < 16 {
            ColorType::Standard
        } else {
            ColorType::EightBit
        };

        Ok(Self::new(
            format!("color({})", number),
            color_type,
            Some(number),
            None,
        ))
    }

    /// Get the ANSI escape codes for this color.
    ///
    /// Returns a vector of strings that should be joined with ";" to form
    /// the SGR (Select Graphic Rendition) parameter.
    ///
    /// # Arguments
    ///
    /// * `foreground` - If true, generate foreground color codes; otherwise background.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Color;
    ///
    /// let red = Color::parse("red").unwrap();
    /// assert_eq!(red.get_ansi_codes(true), vec!["31"]);
    /// assert_eq!(red.get_ansi_codes(false), vec!["41"]);
    ///
    /// let bright_red = Color::parse("bright_red").unwrap();
    /// assert_eq!(bright_red.get_ansi_codes(true), vec!["91"]);
    ///
    /// let color256 = Color::parse("color(196)").unwrap();
    /// assert_eq!(color256.get_ansi_codes(true), vec!["38", "5", "196"]);
    ///
    /// let rgb = Color::parse("#ff0000").unwrap();
    /// assert_eq!(rgb.get_ansi_codes(true), vec!["38", "2", "255", "0", "0"]);
    /// ```
    pub fn get_ansi_codes(&self, foreground: bool) -> Vec<String> {
        match self.color_type {
            ColorType::Default => {
                vec![if foreground { "39" } else { "49" }.to_string()]
            }
            ColorType::Standard | ColorType::Windows => {
                let number = self.number.unwrap_or(0);
                let (fore_base, back_base) = if number < 8 { (30, 40) } else { (90, 100) };
                let code = if foreground {
                    fore_base + (number % 8) as u16
                } else {
                    back_base + (number % 8) as u16
                };
                vec![code.to_string()]
            }
            ColorType::EightBit => {
                let number = self.number.unwrap_or(0);
                vec![
                    if foreground { "38" } else { "48" }.to_string(),
                    "5".to_string(),
                    number.to_string(),
                ]
            }
            ColorType::TrueColor => {
                let triplet = self.triplet.unwrap_or_default();
                vec![
                    if foreground { "38" } else { "48" }.to_string(),
                    "2".to_string(),
                    triplet.red.to_string(),
                    triplet.green.to_string(),
                    triplet.blue.to_string(),
                ]
            }
        }
    }

    /// Downgrade this color to a lower color system.
    ///
    /// # Arguments
    ///
    /// * `system` - The target color system.
    ///
    /// # Returns
    ///
    /// A new Color that can be represented in the target system.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::{Color, ColorSystem, ColorType};
    ///
    /// let rgb = Color::from_rgb(255, 100, 50);
    /// let downgraded = rgb.downgrade(ColorSystem::EightBit);
    /// assert_eq!(downgraded.color_type, ColorType::EightBit);
    /// ```
    pub fn downgrade(&self, system: ColorSystem) -> Self {
        // If default or already at target system, return self
        if self.color_type == ColorType::Default {
            return self.clone();
        }

        let current_system = self.color_type.system();

        // If current system is same or lower capability, no downgrade needed
        // Use rank() to compare capabilities (Windows and Standard both rank as 16-color systems)
        if current_system.rank() <= system.rank() {
            return self.clone();
        }

        match system {
            ColorSystem::TrueColor => self.clone(),

            ColorSystem::EightBit => {
                // Downgrade from TrueColor to EightBit
                if current_system == ColorSystem::TrueColor {
                    let triplet = self.triplet.unwrap_or_default();
                    let (_, l, s) = rgb_to_hls(triplet);

                    // If saturation is low, use grayscale
                    if s < 0.15 {
                        let gray = (l * 25.0).round() as u8;
                        let color_number = if gray == 0 {
                            16
                        } else if gray == 25 {
                            231
                        } else {
                            231 + gray
                        };
                        return Self::new(
                            self.name.clone(),
                            ColorType::EightBit,
                            Some(color_number),
                            None,
                        );
                    }

                    // Convert to 6x6x6 color cube
                    let r = triplet.red as f64;
                    let g = triplet.green as f64;
                    let b = triplet.blue as f64;

                    let six_red = if r < 95.0 {
                        r / 95.0
                    } else {
                        1.0 + (r - 95.0) / 40.0
                    };
                    let six_green = if g < 95.0 {
                        g / 95.0
                    } else {
                        1.0 + (g - 95.0) / 40.0
                    };
                    let six_blue = if b < 95.0 {
                        b / 95.0
                    } else {
                        1.0 + (b - 95.0) / 40.0
                    };

                    let color_number = 16
                        + 36 * six_red.round() as u8
                        + 6 * six_green.round() as u8
                        + six_blue.round() as u8;

                    Self::new(
                        self.name.clone(),
                        ColorType::EightBit,
                        Some(color_number),
                        None,
                    )
                } else {
                    self.clone()
                }
            }

            ColorSystem::Standard => {
                // Get the triplet (either directly or from palette)
                let triplet = match self.color_type {
                    ColorType::TrueColor => self.triplet.unwrap_or_default(),
                    ColorType::EightBit => {
                        let number = self.number.unwrap_or(0) as usize;
                        EIGHT_BIT_PALETTE.get(number).unwrap_or_default()
                    }
                    _ => return self.clone(),
                };

                let color_number = STANDARD_PALETTE.match_color(triplet) as u8;
                Self::new(
                    self.name.clone(),
                    ColorType::Standard,
                    Some(color_number),
                    None,
                )
            }

            ColorSystem::Windows => {
                // Get the triplet
                let triplet = match self.color_type {
                    ColorType::TrueColor => self.triplet.unwrap_or_default(),
                    ColorType::EightBit => {
                        let number = self.number.unwrap_or(0) as usize;
                        if number < 16 {
                            return Self::new(
                                self.name.clone(),
                                ColorType::Windows,
                                Some(number as u8),
                                None,
                            );
                        }
                        EIGHT_BIT_PALETTE.get(number).unwrap_or_default()
                    }
                    _ => return self.clone(),
                };

                let color_number = WINDOWS_PALETTE.match_color(triplet) as u8;
                Self::new(
                    self.name.clone(),
                    ColorType::Windows,
                    Some(color_number),
                    None,
                )
            }
        }
    }
}

/// Convert RGB to HLS (Hue, Lightness, Saturation).
///
/// Returns (hue, lightness, saturation) where all values are 0.0-1.0.
fn rgb_to_hls(triplet: ColorTriplet) -> (f64, f64, f64) {
    let (r, g, b) = triplet.normalized();

    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    let l = (max_c + min_c) / 2.0;

    if (max_c - min_c).abs() < f64::EPSILON {
        return (0.0, l, 0.0);
    }

    let s = if l <= 0.5 {
        (max_c - min_c) / (max_c + min_c)
    } else {
        (max_c - min_c) / (2.0 - max_c - min_c)
    };

    let rc = (max_c - r) / (max_c - min_c);
    let gc = (max_c - g) / (max_c - min_c);
    let bc = (max_c - b) / (max_c - min_c);

    let h = if (r - max_c).abs() < f64::EPSILON {
        bc - gc
    } else if (g - max_c).abs() < f64::EPSILON {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };

    let h = (h / 6.0).rem_euclid(1.0);

    (h, l, s)
}

/// Parse a six-character hex string into a ColorTriplet.
pub fn parse_rgb_hex(hex_color: &str) -> Result<ColorTriplet, ParseError> {
    if hex_color.len() != 6 {
        return Err(ParseError::invalid_color("hex color must be 6 characters"));
    }
    let r = u8::from_str_radix(&hex_color[0..2], 16)
        .map_err(|_| ParseError::invalid_color("invalid hex color"))?;
    let g = u8::from_str_radix(&hex_color[2..4], 16)
        .map_err(|_| ParseError::invalid_color("invalid hex color"))?;
    let b = u8::from_str_radix(&hex_color[4..6], 16)
        .map_err(|_| ParseError::invalid_color("invalid hex color"))?;
    Ok(ColorTriplet::new(r, g, b))
}

/// Blend two colors together.
///
/// # Arguments
///
/// * `color1` - The first color.
/// * `color2` - The second color.
/// * `cross_fade` - The blend factor (0.0 = color1, 1.0 = color2).
pub fn blend_rgb(color1: ColorTriplet, color2: ColorTriplet, cross_fade: f64) -> ColorTriplet {
    let r1 = color1.red as f64;
    let g1 = color1.green as f64;
    let b1 = color1.blue as f64;
    let r2 = color2.red as f64;
    let g2 = color2.green as f64;
    let b2 = color2.blue as f64;

    ColorTriplet::new(
        (r1 + (r2 - r1) * cross_fade) as u8,
        (g1 + (g2 - g1) * cross_fade) as u8,
        (b1 + (b2 - b1) * cross_fade) as u8,
    )
}

// ============================================================================
// Backward Compatibility
// ============================================================================

// For backward compatibility with existing code that uses the simple Color enum,
// we provide a simpler interface. The style.rs module currently uses `Option<Color>`
// with the old enum. We need to maintain compatibility.

/// Simple color enum for backward compatibility.
///
/// This is kept for compatibility with existing style.rs code.
/// For new code, use the full `Color` struct via `Color::parse()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimpleColor {
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

impl SimpleColor {
    /// Create a new RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        SimpleColor::Rgb { r, g, b }
    }

    /// Parse a color from a string (simple version).
    ///
    /// This is a simpler parser for backward compatibility.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();

        // Hex color
        if let Some(hex) = s.strip_prefix('#') {
            return Self::parse_hex(hex);
        }

        // RGB: rgb(r,g,b)
        if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() == 3 {
                let r = parts[0].parse().ok()?;
                let g = parts[1].parse().ok()?;
                let b = parts[2].parse().ok()?;
                return Some(SimpleColor::Rgb { r, g, b });
            }
            return None;
        }

        // color(n)
        if let Some(inner) = s.strip_prefix("color(").and_then(|s| s.strip_suffix(')')) {
            let n: u8 = inner.trim().parse().ok()?;
            return Some(if n < 16 {
                SimpleColor::Standard(n)
            } else {
                SimpleColor::EightBit(n)
            });
        }

        // Named colors
        if let Some(&number) = ANSI_COLOR_NAMES.get(s.as_str()) {
            return Some(if number < 16 {
                SimpleColor::Standard(number)
            } else {
                SimpleColor::EightBit(number)
            });
        }

        if s == "default" {
            return Some(SimpleColor::Default);
        }

        None
    }

    fn parse_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(SimpleColor::Rgb { r, g, b })
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(SimpleColor::Rgb { r, g, b })
            }
            _ => None,
        }
    }

    /// Convert to the full Color struct.
    pub fn to_color(&self) -> Color {
        match self {
            SimpleColor::Default => Color::default_color(),
            SimpleColor::Standard(n) => Color::from_ansi(*n),
            SimpleColor::EightBit(n) => Color::from_ansi(*n),
            SimpleColor::Rgb { r, g, b } => Color::from_rgb(*r, *g, *b),
        }
    }

    /// Get ANSI codes for this color.
    pub fn get_ansi_codes(&self, foreground: bool) -> Vec<String> {
        self.to_color().get_ansi_codes(foreground)
    }

    /// Downgrade this color to a lower color system.
    ///
    /// # Arguments
    ///
    /// * `system` - The target color system.
    ///
    /// # Returns
    ///
    /// A new SimpleColor that can be represented in the target system.
    pub fn downgrade(&self, system: ColorSystem) -> Self {
        self.to_color().downgrade(system).into()
    }

    /// Get the hex color string for this color.
    ///
    /// For RGB colors, returns the hex string directly.
    /// For indexed colors, looks up the RGB value in the palette.
    pub fn get_hex(&self) -> String {
        match self {
            SimpleColor::Rgb { r, g, b } => format!("#{:02x}{:02x}{:02x}", r, g, b),
            SimpleColor::Default => "#ffffff".to_string(), // Default foreground
            SimpleColor::Standard(n) => {
                let triplet = STANDARD_PALETTE.get(*n as usize).unwrap_or_default();
                triplet.hex()
            }
            SimpleColor::EightBit(n) => {
                let triplet = EIGHT_BIT_PALETTE.get(*n as usize).unwrap_or_default();
                triplet.hex()
            }
        }
    }
}

impl From<SimpleColor> for Color {
    fn from(simple: SimpleColor) -> Self {
        simple.to_color()
    }
}

impl From<Color> for SimpleColor {
    fn from(color: Color) -> Self {
        match color.color_type {
            ColorType::Default => SimpleColor::Default,
            ColorType::Standard | ColorType::Windows => {
                SimpleColor::Standard(color.number.unwrap_or(0))
            }
            ColorType::EightBit => SimpleColor::EightBit(color.number.unwrap_or(0)),
            ColorType::TrueColor => {
                let t = color.triplet.unwrap_or_default();
                SimpleColor::Rgb {
                    r: t.red,
                    g: t.green,
                    b: t.blue,
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- ColorTriplet Tests ---

    #[test]
    fn test_color_triplet_new() {
        let t = ColorTriplet::new(255, 128, 64);
        assert_eq!(t.red, 255);
        assert_eq!(t.green, 128);
        assert_eq!(t.blue, 64);
    }

    #[test]
    fn test_color_triplet_hex() {
        assert_eq!(ColorTriplet::new(255, 0, 0).hex(), "#ff0000");
        assert_eq!(ColorTriplet::new(0, 255, 0).hex(), "#00ff00");
        assert_eq!(ColorTriplet::new(0, 0, 255).hex(), "#0000ff");
        assert_eq!(ColorTriplet::new(18, 52, 86).hex(), "#123456");
    }

    #[test]
    fn test_color_triplet_rgb() {
        assert_eq!(ColorTriplet::new(255, 0, 0).rgb(), "rgb(255,0,0)");
        assert_eq!(ColorTriplet::new(128, 64, 32).rgb(), "rgb(128,64,32)");
    }

    #[test]
    fn test_color_triplet_normalized() {
        let (r, g, b) = ColorTriplet::new(255, 127, 0).normalized();
        assert!((r - 1.0).abs() < 0.001);
        assert!((g - 0.498).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.001);
    }

    // --- Palette Tests ---

    #[test]
    fn test_palette_match() {
        let palette = Palette::new(&[(0, 0, 0), (255, 255, 255), (255, 0, 0)]);

        // Exact match
        assert_eq!(palette.match_color(ColorTriplet::new(0, 0, 0)), 0);
        assert_eq!(palette.match_color(ColorTriplet::new(255, 255, 255)), 1);
        assert_eq!(palette.match_color(ColorTriplet::new(255, 0, 0)), 2);

        // Close match - dark gray should match black
        assert_eq!(palette.match_color(ColorTriplet::new(10, 10, 10)), 0);

        // Light gray should match white
        assert_eq!(palette.match_color(ColorTriplet::new(240, 240, 240)), 1);
    }

    // --- Color Parsing Tests ---

    #[test]
    fn test_parse_named() {
        let red = Color::parse("red").unwrap();
        assert_eq!(red.color_type, ColorType::Standard);
        assert_eq!(red.number, Some(1));

        let bright_blue = Color::parse("bright_blue").unwrap();
        assert_eq!(bright_blue.color_type, ColorType::Standard);
        assert_eq!(bright_blue.number, Some(12));

        // Case insensitive
        let blue = Color::parse("BLUE").unwrap();
        assert_eq!(blue.number, Some(4));

        // Extended color
        let orchid = Color::parse("orchid").unwrap();
        assert_eq!(orchid.color_type, ColorType::EightBit);
        assert_eq!(orchid.number, Some(170));
    }

    #[test]
    fn test_parse_hex() {
        let red = Color::parse("#ff0000").unwrap();
        assert_eq!(red.color_type, ColorType::TrueColor);
        assert_eq!(red.triplet, Some(ColorTriplet::new(255, 0, 0)));

        // Short form
        let white = Color::parse("#fff").unwrap();
        assert_eq!(white.triplet, Some(ColorTriplet::new(255, 255, 255)));

        // Mixed case
        let color = Color::parse("#AbCdEf").unwrap();
        assert_eq!(color.triplet, Some(ColorTriplet::new(171, 205, 239)));
    }

    #[test]
    fn test_parse_rgb() {
        let red = Color::parse("rgb(255,0,0)").unwrap();
        assert_eq!(red.color_type, ColorType::TrueColor);
        assert_eq!(red.triplet, Some(ColorTriplet::new(255, 0, 0)));

        // With spaces
        let green = Color::parse("rgb(0, 255, 0)").unwrap();
        assert_eq!(green.triplet, Some(ColorTriplet::new(0, 255, 0)));
    }

    #[test]
    fn test_parse_color_number() {
        let standard = Color::parse("color(1)").unwrap();
        assert_eq!(standard.color_type, ColorType::Standard);
        assert_eq!(standard.number, Some(1));

        let extended = Color::parse("color(196)").unwrap();
        assert_eq!(extended.color_type, ColorType::EightBit);
        assert_eq!(extended.number, Some(196));
    }

    #[test]
    fn test_parse_default() {
        let default = Color::parse("default").unwrap();
        assert_eq!(default.color_type, ColorType::Default);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Color::parse("not_a_color").is_err());
        assert!(Color::parse("#gggggg").is_err());
        assert!(Color::parse("rgb(256,0,0)").is_err());
        assert!(Color::parse("color(256)").is_err());
    }

    // --- ANSI Code Tests ---

    #[test]
    fn test_ansi_codes_standard() {
        let black = Color::parse("black").unwrap();
        assert_eq!(black.get_ansi_codes(true), vec!["30"]);
        assert_eq!(black.get_ansi_codes(false), vec!["40"]);

        let red = Color::parse("red").unwrap();
        assert_eq!(red.get_ansi_codes(true), vec!["31"]);
        assert_eq!(red.get_ansi_codes(false), vec!["41"]);

        let bright_red = Color::parse("bright_red").unwrap();
        assert_eq!(bright_red.get_ansi_codes(true), vec!["91"]);
        assert_eq!(bright_red.get_ansi_codes(false), vec!["101"]);
    }

    #[test]
    fn test_ansi_codes_eight_bit() {
        let color = Color::parse("color(196)").unwrap();
        assert_eq!(color.get_ansi_codes(true), vec!["38", "5", "196"]);
        assert_eq!(color.get_ansi_codes(false), vec!["48", "5", "196"]);
    }

    #[test]
    fn test_ansi_codes_truecolor() {
        let color = Color::parse("#ff8000").unwrap();
        assert_eq!(
            color.get_ansi_codes(true),
            vec!["38", "2", "255", "128", "0"]
        );
        assert_eq!(
            color.get_ansi_codes(false),
            vec!["48", "2", "255", "128", "0"]
        );
    }

    #[test]
    fn test_ansi_codes_default() {
        let default = Color::default_color();
        assert_eq!(default.get_ansi_codes(true), vec!["39"]);
        assert_eq!(default.get_ansi_codes(false), vec!["49"]);
    }

    // --- Downgrade Tests ---

    #[test]
    fn test_downgrade_truecolor_to_eight_bit() {
        let rgb = Color::from_rgb(255, 0, 0);
        let downgraded = rgb.downgrade(ColorSystem::EightBit);
        assert_eq!(downgraded.color_type, ColorType::EightBit);
        // Should map to a red in the 256 palette
        assert!(downgraded.number.is_some());
    }

    #[test]
    fn test_downgrade_to_standard() {
        let rgb = Color::from_rgb(255, 0, 0);
        let downgraded = rgb.downgrade(ColorSystem::Standard);
        assert_eq!(downgraded.color_type, ColorType::Standard);
        // Red should map to standard red (1 or 9)
        let num = downgraded.number.unwrap();
        assert!(num == 1 || num == 9);
    }

    #[test]
    fn test_downgrade_grayscale() {
        // A gray color should use the grayscale ramp
        let gray = Color::from_rgb(128, 128, 128);
        let downgraded = gray.downgrade(ColorSystem::EightBit);
        assert_eq!(downgraded.color_type, ColorType::EightBit);
        // Should be in the grayscale range (232-255) or close
        let num = downgraded.number.unwrap();
        assert!(num >= 232 || num == 16 || num == 231);
    }

    #[test]
    fn test_downgrade_no_change() {
        // Default doesn't change
        let default = Color::default_color();
        let downgraded = default.downgrade(ColorSystem::Standard);
        assert_eq!(downgraded.color_type, ColorType::Default);

        // Standard to standard doesn't change
        let standard = Color::parse("red").unwrap();
        let downgraded = standard.downgrade(ColorSystem::Standard);
        assert_eq!(downgraded.number, Some(1));
    }

    #[test]
    fn test_downgrade_to_windows() {
        // TrueColor -> Windows should work (this was the bug: Windows rank=4 > TrueColor rank=3
        // meant TrueColor was never downgraded to Windows)
        let rgb = Color::from_rgb(255, 0, 0);
        let downgraded = rgb.downgrade(ColorSystem::Windows);
        assert_eq!(downgraded.color_type, ColorType::Windows);
        // Red should map to a red in the Windows palette
        assert!(downgraded.number.is_some());

        // EightBit -> Windows should also work
        let eight_bit = Color::from_ansi(196); // Red in 256 palette
        let downgraded = eight_bit.downgrade(ColorSystem::Windows);
        assert_eq!(downgraded.color_type, ColorType::Windows);
        assert!(downgraded.number.is_some());
    }

    // --- SimpleColor Backward Compatibility Tests ---

    #[test]
    fn test_simple_color_parse_named() {
        assert_eq!(SimpleColor::parse("red"), Some(SimpleColor::Standard(1)));
        assert_eq!(SimpleColor::parse("BLUE"), Some(SimpleColor::Standard(4)));
    }

    #[test]
    fn test_simple_color_parse_hex() {
        assert_eq!(
            SimpleColor::parse("#ff0000"),
            Some(SimpleColor::Rgb { r: 255, g: 0, b: 0 })
        );
        assert_eq!(
            SimpleColor::parse("#f00"),
            Some(SimpleColor::Rgb { r: 255, g: 0, b: 0 })
        );
    }

    #[test]
    fn test_simple_color_to_color() {
        let simple = SimpleColor::Standard(1);
        let color = simple.to_color();
        assert_eq!(color.color_type, ColorType::Standard);
        assert_eq!(color.number, Some(1));
    }

    // --- RGB to HLS Tests ---

    #[test]
    fn test_rgb_to_hls() {
        // Pure red
        let (h, l, s) = rgb_to_hls(ColorTriplet::new(255, 0, 0));
        assert!((h - 0.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);
        assert!((s - 1.0).abs() < 0.01);

        // Pure white (no saturation)
        let (_, _, s) = rgb_to_hls(ColorTriplet::new(255, 255, 255));
        assert!(s < 0.01);

        // Pure black (no saturation)
        let (_, _, s) = rgb_to_hls(ColorTriplet::new(0, 0, 0));
        assert!(s < 0.01);
    }

    // --- Blend Tests ---

    #[test]
    fn test_blend_rgb() {
        let black = ColorTriplet::new(0, 0, 0);
        let white = ColorTriplet::new(255, 255, 255);

        let mid = blend_rgb(black, white, 0.5);
        assert_eq!(mid.red, 127);
        assert_eq!(mid.green, 127);
        assert_eq!(mid.blue, 127);

        let quarter = blend_rgb(black, white, 0.25);
        assert_eq!(quarter.red, 63);
    }

    // --- Integration Tests ---

    #[test]
    fn test_palette_sizes() {
        assert_eq!(STANDARD_PALETTE.len(), 16);
        assert_eq!(EIGHT_BIT_PALETTE.len(), 256);
        assert_eq!(WINDOWS_PALETTE.len(), 16);
    }

    #[test]
    fn test_ansi_color_names_coverage() {
        // Test that common color names are present
        assert!(ANSI_COLOR_NAMES.contains_key("red"));
        assert!(ANSI_COLOR_NAMES.contains_key("green"));
        assert!(ANSI_COLOR_NAMES.contains_key("blue"));
        assert!(ANSI_COLOR_NAMES.contains_key("bright_red"));
        assert!(ANSI_COLOR_NAMES.contains_key("grey"));
        assert!(ANSI_COLOR_NAMES.contains_key("gray")); // Alias
        assert!(ANSI_COLOR_NAMES.contains_key("orchid"));
        assert!(ANSI_COLOR_NAMES.contains_key("grey93"));
    }
}

//! Frame recorder for animated terminal captures.
//!
//! Captures a sequence of rendered frames with timestamps and exports them as
//! animated SVGs (CSS keyframes) or asciicast v2 files (for GIF conversion via `agg`).
//!
//! # Example
//!
//! ```no_run
//! use rich_rs::{FrameRecorder, Text, MONOKAI};
//!
//! let mut recorder = FrameRecorder::new(60, 10);
//! recorder.capture(0.0, &Text::plain("Frame 1"));
//! recorder.capture(0.5, &Text::plain("Frame 2"));
//! recorder.save_animated_svg("output.svg", "Demo", Some(&MONOKAI), 0.61).unwrap();
//! ```

use std::collections::HashMap;
use std::io::{self, Stdout, Write};

use crate::cells::cell_len;
use crate::console::{
    adler32, escape_text, format_number, get_svg_style_for_segment, is_default_color, make_tag,
    resolve_color_for_svg,
};
use crate::segment::Segment;
use crate::style::Style;
use crate::terminal_theme::{SVG_EXPORT_THEME, TerminalTheme};
use crate::{ColorSystem, Console, ConsoleOptions, Renderable};

/// A single captured frame with its timestamp and rendered content.
#[derive(Clone)]
pub struct Frame {
    /// Seconds from the start of the recording.
    pub timestamp: f64,
    /// Rendered lines (each line is a vec of styled segments).
    pub lines: Vec<Vec<Segment>>,
}

/// Records a sequence of rendered frames for animated export.
///
/// This is the animated counterpart to `Console::set_record()` + `save_svg()`.
/// Instead of recording a single static output, it captures discrete frames
/// with timestamps that can be exported as animated SVG or asciicast files.
pub struct FrameRecorder {
    width: usize,
    height: usize,
    console: Console<Stdout>,
    options: ConsoleOptions,
    frames: Vec<Frame>,
}

impl FrameRecorder {
    /// Create a new recorder with fixed terminal dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        let options = ConsoleOptions {
            size: (width, height),
            max_width: width,
            min_width: width,
            max_height: height,
            is_terminal: true,
            color_system: Some(ColorSystem::TrueColor),
            ..Default::default()
        };
        let console = Console::with_options(options.clone());
        Self {
            width,
            height,
            console,
            options,
            frames: Vec::new(),
        }
    }

    /// Reference to the internal console (for measuring, etc.).
    pub fn console(&self) -> &Console<Stdout> {
        &self.console
    }

    /// Reference to the console options used for rendering.
    pub fn options(&self) -> &ConsoleOptions {
        &self.options
    }

    /// Capture a frame by rendering a `Renderable` at the given timestamp.
    ///
    /// Frames should be captured in chronological order.
    pub fn capture(&mut self, timestamp: f64, renderable: &dyn Renderable) {
        let lines = self
            .console
            .render_lines(renderable, Some(&self.options), None, true, false);
        let lines = Segment::set_shape(&lines, self.width, Some(self.height), None, false);
        self.frames.push(Frame { timestamp, lines });
    }

    /// Capture pre-rendered lines as a frame.
    ///
    /// Useful when you need to do custom layout (e.g., centering) before capture.
    /// Lines are padded/cropped to the recorder's dimensions.
    pub fn capture_lines(&mut self, timestamp: f64, lines: Vec<Vec<Segment>>) {
        let lines = Segment::set_shape(&lines, self.width, Some(self.height), None, false);
        self.frames.push(Frame { timestamp, lines });
    }

    /// Number of captured frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Export the recording as an animated SVG string.
    ///
    /// Uses CSS `@keyframes` with `visibility` toggling and `step-end` timing
    /// to cycle through frames. The animation loops infinitely.
    ///
    /// # Arguments
    ///
    /// * `title` - Title shown in the terminal chrome title bar.
    /// * `theme` - Color theme; defaults to `SVG_EXPORT_THEME`.
    /// * `font_aspect_ratio` - Width/height ratio for the monospace font (0.61 is typical).
    /// * `unique_id` - Optional prefix for CSS classes to avoid collisions.
    pub fn export_animated_svg(
        &self,
        title: &str,
        theme: Option<&TerminalTheme>,
        font_aspect_ratio: f64,
        unique_id: Option<&str>,
    ) -> String {
        assert!(!self.frames.is_empty(), "no frames captured");

        let theme = theme.unwrap_or(&*SVG_EXPORT_THEME);
        let width = self.width;
        let height = self.height;

        // Typography metrics (same as Console::export_svg)
        let char_height = 20.0;
        let char_width = char_height * font_aspect_ratio;
        let line_height = char_height * 1.22;

        let margin_top = 1.0;
        let margin_right = 1.0;
        let margin_bottom = 1.0;
        let margin_left = 1.0;

        let padding_top = 40.0;
        let padding_right = 8.0;
        let padding_bottom = 8.0;
        let padding_left = 8.0;

        let padding_width = padding_left + padding_right;
        let padding_height = padding_top + padding_bottom;
        let margin_width = margin_left + margin_right;
        let margin_height = margin_top + margin_bottom;

        // Deduplicate consecutive identical frames (extend duration of earlier frame).
        let deduped = self.dedup_frames();

        // Calculate total animation duration.
        let total_duration = if deduped.len() == 1 {
            // Single frame: just show it (no animation needed).
            1.0
        } else {
            let last = &deduped[deduped.len() - 1];
            // Add a reasonable hold time for the last frame.
            let prev_duration = if deduped.len() >= 2 {
                last.timestamp - deduped[deduped.len() - 2].timestamp
            } else {
                0.5
            };
            last.timestamp + prev_duration
        };

        // Generate unique ID.
        let unique_id = unique_id.map(|s| s.to_string()).unwrap_or_else(|| {
            let content = format!("{}:{}x{}:{}", title, width, height, deduped.len());
            format!("anim-{}", adler32(&content))
        });

        // Collect CSS classes across ALL frames (shared class pool).
        let mut classes: HashMap<String, usize> = HashMap::new();
        let mut style_no = 1usize;

        // Process each frame into SVG content groups.
        let mut frame_groups: Vec<String> = Vec::with_capacity(deduped.len());

        for frame in &deduped {
            let mut text_backgrounds = Vec::new();
            let mut text_group = Vec::new();

            for (y, line) in frame.lines.iter().enumerate() {
                let mut x = 0usize;

                for segment in line {
                    let style = segment.style.unwrap_or_default();
                    let rules = get_svg_style_for_segment(&style, theme);

                    if !classes.contains_key(&rules) {
                        classes.insert(rules.clone(), style_no);
                        style_no += 1;
                    }
                    let class_name = format!("r{}", classes[&rules]);

                    // Background rect
                    let has_background = if style.reverse.unwrap_or(false) {
                        true
                    } else {
                        style.bgcolor.is_some() && !is_default_color(style.bgcolor)
                    };

                    let background = if style.reverse.unwrap_or(false) {
                        style
                            .color
                            .map(|c| resolve_color_for_svg(c, theme, true))
                            .unwrap_or(theme.foreground_color)
                    } else {
                        style
                            .bgcolor
                            .map(|c| resolve_color_for_svg(c, theme, false))
                            .unwrap_or(theme.background_color)
                    };

                    let text_length = cell_len(&segment.text);

                    if has_background {
                        text_backgrounds.push(make_tag(
                            "rect",
                            None,
                            &[
                                ("fill", &background.hex()),
                                ("x", &format_number(x as f64 * char_width)),
                                ("y", &format_number(y as f64 * line_height + 1.5)),
                                ("width", &format_number(char_width * text_length as f64)),
                                ("height", &format_number(line_height + 0.25)),
                                ("shape-rendering", "crispEdges"),
                            ],
                        ));
                    }

                    // Text element (skip all-space segments)
                    if !segment.text.chars().all(|c| c == ' ') {
                        text_group.push(make_tag(
                            "text",
                            Some(&escape_text(&segment.text)),
                            &[
                                ("class", &format!("{}-{}", unique_id, class_name)),
                                ("x", &format_number(x as f64 * char_width)),
                                ("y", &format_number(y as f64 * line_height + char_height)),
                                (
                                    "textLength",
                                    &format_number(char_width * text_length as f64),
                                ),
                                ("clip-path", &format!("url(#{}-line-{})", unique_id, y)),
                            ],
                        ));
                    }

                    x += text_length;
                }
            }

            let backgrounds = text_backgrounds.join("\n        ");
            let matrix = text_group.join("\n        ");
            frame_groups.push(format!(
                "{}\n        <g class=\"{}-matrix\">\n        {}\n        </g>",
                backgrounds, unique_id, matrix
            ));
        }

        // Generate CSS keyframes for each frame.
        let mut animation_css = String::new();

        if deduped.len() == 1 {
            // Single frame: always visible, no animation needed.
            animation_css.push_str(&format!(
                "    .{}-f0 {{ visibility: visible; }}\n",
                unique_id
            ));
        } else {
            // All frames hidden by default.
            animation_css.push_str(&format!(
                "    .{}-frame {{ visibility: hidden; }}\n",
                unique_id
            ));

            for (i, frame) in deduped.iter().enumerate() {
                let start_pct = (frame.timestamp / total_duration) * 100.0;

                let mut keyframes = String::new();
                if i == 0 {
                    // First frame: visible from 0% to end_pct.
                    let end_pct = if deduped.len() > 1 {
                        (deduped[1].timestamp / total_duration) * 100.0
                    } else {
                        100.0
                    };
                    keyframes.push_str(&format!(
                        "0% {{ visibility: visible; }} {}% {{ visibility: hidden; }}",
                        format_number(end_pct)
                    ));
                } else if i == deduped.len() - 1 {
                    // Last frame: hidden until start_pct, visible to end.
                    keyframes.push_str(&format!(
                        "0% {{ visibility: hidden; }} {}% {{ visibility: visible; }}",
                        format_number(start_pct)
                    ));
                } else {
                    // Middle frame: hidden, then visible, then hidden again.
                    let end_pct = (deduped[i + 1].timestamp / total_duration) * 100.0;
                    keyframes.push_str(&format!(
                        "0% {{ visibility: hidden; }} {}% {{ visibility: visible; }} {}% {{ visibility: hidden; }}",
                        format_number(start_pct),
                        format_number(end_pct)
                    ));
                }

                animation_css.push_str(&format!(
                    "    @keyframes {uid}-f{i} {{ {kf} }}\n    .{uid}-f{i} {{ animation: {uid}-f{i} {dur}s step-end infinite; }}\n",
                    uid = unique_id,
                    i = i,
                    kf = keyframes,
                    dur = format_number(total_duration),
                ));
            }
        }

        // Generate line clip paths.
        let line_offsets: Vec<f64> = (0..height)
            .map(|line_no| line_no as f64 * line_height + 1.5)
            .collect();

        let lines_svg: String = line_offsets
            .iter()
            .enumerate()
            .map(|(line_no, offset)| {
                format!(
                    r#"<clipPath id="{}-line-{}">
    {}
    </clipPath>"#,
                    unique_id,
                    line_no,
                    make_tag(
                        "rect",
                        None,
                        &[
                            ("x", "0"),
                            ("y", &format_number(*offset)),
                            ("width", &format_number(char_width * width as f64)),
                            ("height", &format_number(line_height + 0.25)),
                        ],
                    )
                )
            })
            .collect::<Vec<_>>()
            .join("\n    ");

        // Generate text styling CSS (shared across frames).
        let styles: String = classes
            .iter()
            .map(|(css, rule_no)| format!("    .{}-r{} {{ {} }}", unique_id, rule_no, css))
            .collect::<Vec<_>>()
            .join("\n");

        // Terminal chrome dimensions.
        let terminal_width = (width as f64 * char_width + padding_width).ceil();
        let terminal_height = (height as f64 + 1.0) * line_height + padding_height;

        // Build chrome.
        let mut chrome = make_tag(
            "rect",
            None,
            &[
                ("fill", &theme.background_color.hex()),
                ("stroke", "rgba(255,255,255,0.35)"),
                ("stroke-width", "1"),
                ("x", &format_number(margin_left)),
                ("y", &format_number(margin_top)),
                ("width", &format_number(terminal_width)),
                ("height", &format_number(terminal_height)),
                ("rx", "8"),
            ],
        );

        if !title.is_empty() {
            chrome.push_str(&make_tag(
                "text",
                Some(&escape_text(title)),
                &[
                    ("class", &format!("{}-title", unique_id)),
                    ("fill", &theme.foreground_color.hex()),
                    ("text-anchor", "middle"),
                    ("x", &format_number(terminal_width / 2.0)),
                    ("y", &format_number(margin_top + char_height + 6.0)),
                ],
            ));
        }

        chrome.push_str(
            r##"
    <g transform="translate(26,22)">
    <circle cx="0" cy="0" r="7" fill="#ff5f57"/>
    <circle cx="22" cy="0" r="7" fill="#febc2e"/>
    <circle cx="44" cy="0" r="7" fill="#28c840"/>
    </g>"##,
        );

        // Assemble frame groups with animation classes.
        let mut frame_content = String::new();
        for (i, group_svg) in frame_groups.iter().enumerate() {
            let class = if deduped.len() == 1 {
                format!("{}-f0", unique_id)
            } else {
                format!("{}-frame {}-f{}", unique_id, unique_id, i)
            };
            frame_content.push_str(&format!(
                "\n      <g class=\"{}\">\n        {}\n      </g>",
                class, group_svg
            ));
        }

        // Terminal content dimensions for clip path.
        let terminal_content_width = format_number(char_width * width as f64 - 1.0);
        let terminal_content_height = format_number((height as f64 + 1.0) * line_height - 1.0);
        let terminal_x = format_number(margin_left + padding_left);
        let terminal_y = format_number(margin_top + padding_top);

        // Final SVG assembly.
        format!(
            r#"<svg class="rich-terminal" viewBox="0 0 {svg_width} {svg_height}" xmlns="http://www.w3.org/2000/svg">
    <!-- Generated with Rich-rs https://github.com/mrsaraiva/rich-rs -->
    <style>

    @font-face {{
        font-family: "Fira Code";
        src: local("FiraCode-Regular"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff2/FiraCode-Regular.woff2") format("woff2"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff/FiraCode-Regular.woff") format("woff");
        font-style: normal;
        font-weight: 400;
    }}
    @font-face {{
        font-family: "Fira Code";
        src: local("FiraCode-Bold"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff2/FiraCode-Bold.woff2") format("woff2"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff/FiraCode-Bold.woff") format("woff");
        font-style: bold;
        font-weight: 700;
    }}

    .{uid}-matrix {{
        font-family: Fira Code, monospace;
        font-size: {char_height}px;
        line-height: {line_height}px;
        font-variant-east-asian: full-width;
    }}

    .{uid}-title {{
        font-size: 18px;
        font-weight: bold;
        font-family: arial;
    }}

{styles}
{animation_css}
    </style>

    <defs>
    <clipPath id="{uid}-clip-terminal">
      <rect x="0" y="0" width="{tw}" height="{th}" />
    </clipPath>
    {lines_svg}
    </defs>

    {chrome}
    <g transform="translate({tx}, {ty})" clip-path="url(#{uid}-clip-terminal)">{frame_content}
    </g>
</svg>
"#,
            svg_width = format_number(terminal_width + margin_width),
            svg_height = format_number(terminal_height + margin_height),
            uid = unique_id,
            char_height = format_number(char_height),
            line_height = format_number(line_height),
            styles = styles,
            animation_css = animation_css,
            lines_svg = lines_svg,
            chrome = chrome,
            tw = terminal_content_width,
            th = terminal_content_height,
            tx = terminal_x,
            ty = terminal_y,
            frame_content = frame_content,
        )
    }

    /// Save the animated SVG to a file.
    pub fn save_animated_svg(
        &self,
        path: &str,
        title: &str,
        theme: Option<&TerminalTheme>,
        font_aspect_ratio: f64,
    ) -> io::Result<()> {
        let svg = self.export_animated_svg(title, theme, font_aspect_ratio, None);
        let mut file = std::fs::File::create(path)?;
        file.write_all(svg.as_bytes())
    }

    /// Export as asciicast v2 format (for GIF conversion via `agg`).
    ///
    /// The asciicast format is a JSONL file where the first line is a header
    /// and subsequent lines are `[timestamp, "o", data]` events containing
    /// ANSI-encoded terminal output.
    pub fn export_asciicast(&self, theme: Option<&TerminalTheme>) -> String {
        let theme = theme.unwrap_or(&*SVG_EXPORT_THEME);
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            r#"{{"version": 2, "width": {}, "height": {}, "theme": {{"fg": "{}", "bg": "{}"}}}}"#,
            self.width,
            self.height,
            theme.foreground_color.hex(),
            theme.background_color.hex(),
        ));
        output.push('\n');

        // Frame events
        for frame in &self.frames {
            let ansi = self.lines_to_ansi(&frame.lines, theme);
            // Clear screen + home cursor, then write frame
            let data = format!("\x1b[2J\x1b[H{}", ansi);
            // JSON-escape the data string
            let escaped = json_escape(&data);
            output.push_str(&format!("[{}, \"o\", \"{}\"]\n", frame.timestamp, escaped));
        }

        output
    }

    /// Save asciicast v2 to a file.
    pub fn save_asciicast(&self, path: &str, theme: Option<&TerminalTheme>) -> io::Result<()> {
        let cast = self.export_asciicast(theme);
        let mut file = std::fs::File::create(path)?;
        file.write_all(cast.as_bytes())
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Deduplicate consecutive frames with identical content.
    fn dedup_frames(&self) -> Vec<&Frame> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<&Frame> = vec![&self.frames[0]];
        for frame in &self.frames[1..] {
            let prev = result.last().unwrap();
            if !lines_equal(&prev.lines, &frame.lines) {
                result.push(frame);
            }
        }
        result
    }

    /// Convert styled lines to ANSI escape sequences.
    fn lines_to_ansi(&self, lines: &[Vec<Segment>], theme: &TerminalTheme) -> String {
        let mut out = String::new();

        for (i, line) in lines.iter().enumerate() {
            for segment in line {
                let style = segment.style.unwrap_or_default();
                let ansi_prefix = style_to_ansi(&style, theme);
                if !ansi_prefix.is_empty() {
                    out.push_str(&ansi_prefix);
                    out.push_str(&segment.text);
                    out.push_str("\x1b[0m");
                } else {
                    out.push_str(&segment.text);
                }
            }
            if i < lines.len() - 1 {
                out.push('\n');
            }
        }

        out
    }
}

/// Check if two sets of rendered lines are identical.
fn lines_equal(a: &[Vec<Segment>], b: &[Vec<Segment>]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (la, lb) in a.iter().zip(b.iter()) {
        if la.len() != lb.len() {
            return false;
        }
        for (sa, sb) in la.iter().zip(lb.iter()) {
            if sa.text != sb.text || sa.style != sb.style {
                return false;
            }
        }
    }
    true
}

/// Convert a Style to ANSI SGR escape sequence.
fn style_to_ansi(style: &Style, _theme: &TerminalTheme) -> String {
    let mut params: Vec<String> = Vec::new();

    if style.bold.unwrap_or(false) {
        params.push("1".to_string());
    }
    if style.dim.unwrap_or(false) {
        params.push("2".to_string());
    }
    if style.italic.unwrap_or(false) {
        params.push("3".to_string());
    }
    if style.underline.unwrap_or(false) {
        params.push("4".to_string());
    }
    if style.strike.unwrap_or(false) {
        params.push("9".to_string());
    }
    if style.reverse.unwrap_or(false) {
        params.push("7".to_string());
    }

    if let Some(color) = style.color {
        match color {
            crate::SimpleColor::Default => {}
            crate::SimpleColor::Standard(n) => {
                if n < 8 {
                    params.push(format!("{}", 30 + n));
                } else {
                    params.push(format!("{}", 90 + n - 8));
                }
            }
            crate::SimpleColor::EightBit(n) => {
                params.push(format!("38;5;{}", n));
            }
            crate::SimpleColor::Rgb { r, g, b } => {
                params.push(format!("38;2;{};{};{}", r, g, b));
            }
        }
    }

    if let Some(color) = style.bgcolor {
        match color {
            crate::SimpleColor::Default => {}
            crate::SimpleColor::Standard(n) => {
                if n < 8 {
                    params.push(format!("{}", 40 + n));
                } else {
                    params.push(format!("{}", 100 + n - 8));
                }
            }
            crate::SimpleColor::EightBit(n) => {
                params.push(format!("48;5;{}", n));
            }
            crate::SimpleColor::Rgb { r, g, b } => {
                params.push(format!("48;2;{};{};{}", r, g, b));
            }
        }
    }

    if params.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", params.join(";"))
    }
}

/// JSON-escape a string (for asciicast output).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn test_frame_recorder_basic() {
        let mut recorder = FrameRecorder::new(20, 3);
        recorder.capture(0.0, &Text::plain("Hello"));
        recorder.capture(0.5, &Text::plain("World"));
        assert_eq!(recorder.frame_count(), 2);
    }

    #[test]
    fn test_export_animated_svg_produces_valid_svg() {
        let mut recorder = FrameRecorder::new(20, 3);
        recorder.capture(0.0, &Text::plain("Frame 1"));
        recorder.capture(0.5, &Text::plain("Frame 2"));

        let svg = recorder.export_animated_svg("Test", None, 0.61, None);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("@keyframes"));
        assert!(svg.contains("Frame&#160;1"));
        assert!(svg.contains("Frame&#160;2"));
    }

    #[test]
    fn test_single_frame_no_animation() {
        let mut recorder = FrameRecorder::new(20, 3);
        recorder.capture(0.0, &Text::plain("Static"));

        let svg = recorder.export_animated_svg("Test", None, 0.61, None);
        assert!(svg.contains("<svg"));
        // Single frame should not have keyframe rules
        assert!(!svg.contains("@keyframes"));
        assert!(svg.contains("visibility: visible"));
    }

    #[test]
    fn test_dedup_identical_frames() {
        let mut recorder = FrameRecorder::new(20, 3);
        recorder.capture(0.0, &Text::plain("Same"));
        recorder.capture(0.5, &Text::plain("Same"));
        recorder.capture(1.0, &Text::plain("Different"));

        let deduped = recorder.dedup_frames();
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].timestamp, 0.0);
        assert_eq!(deduped[1].timestamp, 1.0);
    }

    #[test]
    fn test_export_asciicast_format() {
        let mut recorder = FrameRecorder::new(20, 3);
        recorder.capture(0.0, &Text::plain("Hello"));
        recorder.capture(0.5, &Text::plain("World"));

        let cast = recorder.export_asciicast(None);
        let lines: Vec<&str> = cast.lines().collect();
        assert!(lines.len() >= 3); // header + 2 events
        assert!(lines[0].contains("\"version\": 2"));
        assert!(lines[1].starts_with("[0,"));
        assert!(lines[2].starts_with("[0.5,"));
    }

    #[test]
    fn test_json_escape() {
        assert_eq!(json_escape("hello"), "hello");
        assert_eq!(json_escape("he\"llo"), "he\\\"llo");
        assert_eq!(json_escape("line\nnew"), "line\\nnew");
        assert_eq!(json_escape("\x1b[0m"), "\\u001b[0m");
    }
}

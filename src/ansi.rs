//! ANSI decoding utilities.
//!
//! Port of Python Rich's `rich/ansi.py` (subset).
//!
//! The primary entry points are:
//! - `AnsiDecoder` (stateful decoder that persists style across lines)
//! - `Text::from_ansi` (see `src/text.rs`)

use std::sync::Arc;

use crate::color::SimpleColor;
use crate::style::{Style, StyleMeta};
use crate::text::Text;

/// Translate ANSI escape codes in to styled `Text`.
///
/// This decoder is deliberately lenient: it ignores unknown / malformed escape codes.
/// Style state is preserved across lines (matches Python Rich).
#[derive(Debug, Clone)]
pub struct AnsiDecoder {
    style: Style,
    /// Current hyperlink URL from OSC 8, if any.
    link: Option<Arc<str>>,
    /// Current hyperlink id from OSC 8 params (`id=...`), if any.
    link_id: Option<Arc<str>>,
}

impl Default for AnsiDecoder {
    fn default() -> Self {
        Self {
            style: Style::new(),
            link: None,
            link_id: None,
        }
    }
}

impl AnsiDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode ANSI codes in a multi-line string.
    ///
    /// This splits on line boundaries and returns one `Text` per line, with style state
    /// persisting across lines (same behavior as Python Rich's `AnsiDecoder.decode`).
    pub fn decode(&mut self, terminal_text: &str) -> Vec<Text> {
        // Python Rich uses `str.splitlines()`, which splits on:
        // - \n
        // - \r\n
        // - \r
        // Rust's `str::lines()` does *not* split on bare \r, so we implement
        // a small compatible splitter here.
        splitlines_like_python(terminal_text)
            .into_iter()
            .map(|line| self.decode_line(line))
            .collect()
    }

    /// Decode a line containing ANSI escape codes.
    pub fn decode_line(&mut self, line: &str) -> Text {
        // Match Rich: only keep content after the last carriage return.
        let line = line.rsplit('\r').next().unwrap_or(line);

        let mut out = Text::new();

        let bytes = line.as_bytes();
        let mut index: usize = 0;
        let mut plain_start: usize = 0;

        while index < bytes.len() {
            if bytes[index] != 0x1b {
                index += 1;
                continue;
            }

            // Flush preceding plain text.
            if plain_start < index {
                let plain = &line[plain_start..index];
                if !plain.is_empty() {
                    self.append_to_text(&mut out, plain);
                }
            }

            // Parse escape sequence (best-effort).
            if index + 1 >= bytes.len() {
                break;
            }

            match bytes[index + 1] {
                b'[' => {
                    // CSI ... <final>
                    if let Some((final_byte, params_end, next_index)) = parse_csi(bytes, index + 2)
                    {
                        if final_byte == b'm' {
                            let params = &line[index + 2..params_end];
                            self.apply_sgr(params);
                        }
                        index = next_index;
                        plain_start = index;
                        continue;
                    }
                    // Malformed CSI: skip ESC + '['.
                    index += 2;
                    plain_start = index;
                }
                b']' => {
                    // OSC ... (BEL or ST)
                    if let Some((content_start, content_end, next_index)) =
                        parse_osc(bytes, index + 2)
                    {
                        let content = &line[content_start..content_end];
                        self.apply_osc(content);
                        index = next_index;
                        plain_start = index;
                        continue;
                    }
                    // Malformed OSC: skip ESC + ']'.
                    index += 2;
                    plain_start = index;
                }
                _ => {
                    // Unknown escape: skip ESC + one byte.
                    index += 2;
                    plain_start = index;
                }
            }
        }

        // Flush trailing plain text.
        if plain_start < bytes.len() {
            let plain = &line[plain_start..];
            if !plain.is_empty() {
                self.append_to_text(&mut out, plain);
            }
        }

        out
    }

    fn style_for_text(&self) -> Option<Style> {
        if self.style.is_null() && self.link.is_none() {
            None
        } else {
            Some(self.style)
        }
    }

    /// Get the current StyleMeta (for hyperlink), if any.
    fn meta_for_text(&self) -> Option<StyleMeta> {
        self.link.as_ref().map(|url| StyleMeta {
            link: Some(url.clone()),
            link_id: self.link_id.clone(),
            meta: None,
        })
    }

    /// Append text to a Text object with current style and meta.
    fn append_to_text(&self, out: &mut Text, plain: &str) {
        let style = self.style_for_text();
        let meta = self.meta_for_text();
        if let Some(meta) = meta {
            // Use spans with meta for hyperlinks
            let start = out.len();
            out.append(plain.to_string(), style);
            // Manually add meta to the last span
            if let Some(last_span) = out.spans_mut().last_mut() {
                if last_span.start == start {
                    last_span.meta = Some(meta);
                }
            }
        } else {
            out.append(plain.to_string(), style);
        }
    }

    fn apply_osc(&mut self, content: &str) {
        // Handle OSC 8 hyperlinks: "8;params;url" (url may be empty to clear).
        if let Some(after_8) = content.strip_prefix("8;") {
            // Split: "8;params;url"
            if let Some((params, url)) = after_8.split_once(';') {
                if url.is_empty() {
                    // Clear hyperlink
                    self.link = None;
                    self.link_id = None;
                } else {
                    self.link = Some(Arc::from(url));
                    self.link_id = parse_osc8_link_id(params);
                }
            }
        }
    }

    fn apply_sgr(&mut self, params: &str) {
        // Translate to semi-colon separated codes. Be lenient and ignore invalid codes.
        //
        // Python Rich: codes are int(min(255, int(code))) if code.isdigit() or code == "".
        let mut codes: Vec<u16> = Vec::new();
        for part in params.split(';') {
            if part.is_empty() {
                codes.push(0);
                continue;
            }
            if !part.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let parsed: u16 = part.parse::<u16>().unwrap_or(0).min(255);
            codes.push(parsed);
        }

        if codes.is_empty() {
            // `\x1b[m` is equivalent to reset.
            codes.push(0);
        }

        let mut iter = codes.into_iter();
        while let Some(code) = iter.next() {
            match code {
                0 => {
                    // reset
                    self.style = Style::new();
                }
                1 => self.style.bold = Some(true),
                2 => self.style.dim = Some(true),
                3 => self.style.italic = Some(true),
                4 => self.style.underline = Some(true),
                5 => self.style.blink = Some(true),
                6 => self.style.blink2 = Some(true),
                7 => self.style.reverse = Some(true),
                8 => self.style.conceal = Some(true),
                9 => self.style.strike = Some(true),
                21 => self.style.underline2 = Some(true),

                22 => {
                    // not dim not bold
                    self.style.bold = None;
                    self.style.dim = None;
                }
                23 => self.style.italic = None,
                24 => {
                    // resets both underline and underline2
                    self.style.underline = None;
                    self.style.underline2 = None;
                }
                25 => {
                    // resets both blink and blink2
                    self.style.blink = None;
                    self.style.blink2 = None;
                }
                27 => self.style.reverse = None,
                28 => self.style.conceal = None,
                29 => self.style.strike = None,

                51 => self.style.frame = Some(true),
                52 => self.style.encircle = Some(true),
                53 => self.style.overline = Some(true),
                54 => {
                    // resets both frame and encircle
                    self.style.frame = None;
                    self.style.encircle = None;
                }
                55 => self.style.overline = None,

                30..=37 => self.style.color = Some(SimpleColor::Standard((code - 30) as u8)),
                39 => self.style.color = None,
                40..=47 => self.style.bgcolor = Some(SimpleColor::Standard((code - 40) as u8)),
                49 => self.style.bgcolor = None,

                90..=97 => self.style.color = Some(SimpleColor::Standard((code - 90 + 8) as u8)),
                100..=107 => {
                    self.style.bgcolor = Some(SimpleColor::Standard((code - 100 + 8) as u8))
                }

                38 => {
                    // Foreground extended color.
                    if let Some(color_type) = iter.next() {
                        match color_type {
                            5 => {
                                if let Some(n) = iter.next() {
                                    self.style.color = Some(SimpleColor::EightBit(n as u8));
                                }
                            }
                            2 => {
                                let (Some(r), Some(g), Some(b)) =
                                    (iter.next(), iter.next(), iter.next())
                                else {
                                    continue;
                                };
                                self.style.color = Some(SimpleColor::Rgb {
                                    r: r as u8,
                                    g: g as u8,
                                    b: b as u8,
                                });
                            }
                            _ => {}
                        }
                    }
                }
                48 => {
                    // Background extended color.
                    if let Some(color_type) = iter.next() {
                        match color_type {
                            5 => {
                                if let Some(n) = iter.next() {
                                    self.style.bgcolor = Some(SimpleColor::EightBit(n as u8));
                                }
                            }
                            2 => {
                                let (Some(r), Some(g), Some(b)) =
                                    (iter.next(), iter.next(), iter.next())
                                else {
                                    continue;
                                };
                                self.style.bgcolor = Some(SimpleColor::Rgb {
                                    r: r as u8,
                                    g: g as u8,
                                    b: b as u8,
                                });
                            }
                            _ => {}
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

fn splitlines_like_python(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut out: Vec<&str> = Vec::new();
    let mut start: usize = 0;
    let mut i: usize = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                out.push(&s[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                out.push(&s[start..i]);
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    i += 2;
                } else {
                    i += 1;
                }
                start = i;
            }
            _ => i += 1,
        }
    }

    if start <= bytes.len() {
        if start < bytes.len() {
            out.push(&s[start..]);
        } else if !s.is_empty() {
            // Trailing newline: Python splitlines() returns a final empty string only when
            // keepends=True (default keepends=False). We mirror keepends=False here and
            // do not push an empty last line.
        }
    }

    if out.is_empty() && !s.is_empty() {
        out.push(s);
    }

    out
}

fn parse_csi(bytes: &[u8], start: usize) -> Option<(u8, usize, usize)> {
    // Scan for the final byte in the CSI sequence.
    // Final byte range is 0x40..=0x7e (see ANSI X3.64).
    let mut idx = start;
    while idx < bytes.len() {
        let b = bytes[idx];
        if (0x40..=0x7e).contains(&b) {
            // params_end is the start of final byte.
            return Some((b, idx, idx + 1));
        }
        idx += 1;
    }
    None
}

fn parse_osc(bytes: &[u8], start: usize) -> Option<(usize, usize, usize)> {
    // OSC can be terminated by BEL (0x07) or ST (ESC \).
    let mut idx = start;
    while idx < bytes.len() {
        match bytes[idx] {
            0x07 => return Some((start, idx, idx + 1)), // BEL
            0x1b => {
                if idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' {
                    return Some((start, idx, idx + 2)); // ST
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

fn parse_osc8_link_id(params: &str) -> Option<Arc<str>> {
    // OSC 8 params are colon-separated key/value tokens, e.g. "id=abc:foo=bar".
    params
        .split(':')
        .find_map(|token| token.strip_prefix("id="))
        .and_then(|id| {
            if id.is_empty() {
                None
            } else {
                Some(Arc::<str>::from(id))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_line_strips_ansi_and_adds_spans() {
        let mut decoder = AnsiDecoder::new();
        let text = decoder.decode_line("\x1b[1mBold\x1b[0m Normal");
        assert_eq!(text.plain_text(), "Bold Normal");
        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 0);
        assert_eq!(text.spans()[0].end, 4);
        assert_eq!(text.spans()[0].style.bold, Some(true));
    }

    #[test]
    fn test_decode_line_extended_truecolor() {
        let mut decoder = AnsiDecoder::new();
        let text = decoder.decode_line("\x1b[38;2;255;0;0mRed\x1b[0m");
        assert_eq!(text.plain_text(), "Red");
        assert_eq!(text.spans().len(), 1);
        assert_eq!(
            text.spans()[0].style.color,
            Some(SimpleColor::Rgb { r: 255, g: 0, b: 0 })
        );
    }

    #[test]
    fn test_decode_persists_style_across_lines() {
        let mut decoder = AnsiDecoder::new();
        let lines = decoder.decode("\x1b[31mred\nstill");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "red");
        assert_eq!(lines[1].plain_text(), "still");
        assert_eq!(lines[0].spans().len(), 1);
        assert_eq!(lines[1].spans().len(), 1);
        assert_eq!(
            lines[1].spans()[0].style.color,
            Some(SimpleColor::Standard(1))
        );
    }

    #[test]
    fn test_decode_line_after_carriage_return() {
        let mut decoder = AnsiDecoder::new();
        let text = decoder.decode_line("abc\rdef");
        assert_eq!(text.plain_text(), "def");
    }

    #[test]
    fn test_decode_splits_on_carriage_return_like_python() {
        let mut decoder = AnsiDecoder::new();
        let lines = decoder.decode("abc\rdef");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "abc");
        assert_eq!(lines[1].plain_text(), "def");
    }

    #[test]
    fn test_decode_new_sgr_codes() {
        let mut decoder = AnsiDecoder::new();
        // SGR 6 = blink2
        let text = decoder.decode_line("\x1b[6mRapid\x1b[0m");
        assert_eq!(text.spans()[0].style.blink2, Some(true));

        let mut decoder = AnsiDecoder::new();
        // SGR 8 = conceal
        let text = decoder.decode_line("\x1b[8mHidden\x1b[0m");
        assert_eq!(text.spans()[0].style.conceal, Some(true));

        let mut decoder = AnsiDecoder::new();
        // SGR 21 = underline2
        let text = decoder.decode_line("\x1b[21mDouble\x1b[0m");
        assert_eq!(text.spans()[0].style.underline2, Some(true));

        let mut decoder = AnsiDecoder::new();
        // SGR 53 = overline
        let text = decoder.decode_line("\x1b[53mOver\x1b[0m");
        assert_eq!(text.spans()[0].style.overline, Some(true));

        let mut decoder = AnsiDecoder::new();
        // SGR 51 = frame
        let text = decoder.decode_line("\x1b[51mFramed\x1b[0m");
        assert_eq!(text.spans()[0].style.frame, Some(true));

        let mut decoder = AnsiDecoder::new();
        // SGR 52 = encircle
        let text = decoder.decode_line("\x1b[52mCircle\x1b[0m");
        assert_eq!(text.spans()[0].style.encircle, Some(true));
    }

    #[test]
    fn test_decode_sgr_reset_codes() {
        let mut decoder = AnsiDecoder::new();
        // Set overline, then reset with SGR 55
        let text = decoder.decode_line("\x1b[53mOver\x1b[55mNormal\x1b[0m");
        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].style.overline, Some(true));
        // After reset, "Normal" should have overline=None
    }

    #[test]
    fn test_decode_osc8_hyperlink() {
        let mut decoder = AnsiDecoder::new();
        // OSC 8 hyperlink: ESC ] 8 ; params ; url BEL
        let text = decoder.decode_line("\x1b]8;;https://example.com\x07Link\x1b]8;;\x07");
        assert_eq!(text.plain_text(), "Link");
        assert_eq!(text.spans().len(), 1);
        // Check meta has the link
        let span = &text.spans()[0];
        assert!(span.meta.is_some());
        let meta = span.meta.as_ref().unwrap();
        assert_eq!(meta.link.as_deref(), Some("https://example.com"));
        assert_eq!(meta.link_id, None);
    }

    #[test]
    fn test_decode_osc8_hyperlink_with_id_and_sgr_reset_semantics() {
        let mut decoder = AnsiDecoder::new();
        let text =
            decoder.decode_line("\x1b]8;id=src42;https://example.com\x07L\x1b[0mi\x1b]8;;\x07N");
        assert_eq!(text.plain_text(), "LiN");
        assert_eq!(text.spans().len(), 2);

        // Link metadata persists across SGR reset and clears only on OSC 8 close.
        for span in text.spans() {
            let meta = span.meta.as_ref().expect("expected link metadata");
            assert_eq!(meta.link.as_deref(), Some("https://example.com"));
            assert_eq!(meta.link_id.as_deref(), Some("src42"));
        }
    }
}

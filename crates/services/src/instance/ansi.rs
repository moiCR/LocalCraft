use std::{ops::Range, sync::Arc};

use gpui::{FontStyle, FontWeight, HighlightStyle, SharedString, UnderlineStyle, px, rgb};

pub const MAX_LINE_BYTES: usize = 8_192;
const MAX_SPANS: usize = 128;

#[derive(Debug)]
pub struct ConsoleText {
    pub text: SharedString,
    pub search_text: SharedString,
    pub highlights: Vec<(Range<usize>, HighlightStyle)>,
}

impl ConsoleText {
    pub fn plain(text: String) -> Arc<Self> {
        Self::new(text, Vec::new())
    }

    fn new(text: String, highlights: Vec<(Range<usize>, HighlightStyle)>) -> Arc<Self> {
        let search_text = text.to_lowercase();
        let text: SharedString = text.into();
        let search_text = if search_text == text.as_ref() {
            text.clone()
        } else {
            search_text.into()
        };
        Arc::new(Self {
            text,
            search_text,
            highlights,
        })
    }
}

#[derive(Default)]
pub(super) struct AnsiParser {
    style: HighlightStyle,
}

impl AnsiParser {
    pub fn reset(&mut self) {
        self.style = HighlightStyle::default();
    }

    pub fn parse(&mut self, input: &str) -> Arc<ConsoleText> {
        let mut text = String::with_capacity(input.len().min(MAX_LINE_BYTES));
        let mut highlights = Vec::new();
        let mut start = 0;
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\u{1b}' | '\u{9b}' => {
                    let introducer = if ch == '\u{9b}' {
                        Some('[')
                    } else {
                        chars.next()
                    };
                    match introducer {
                        Some('[') => {
                            let mut parameters = String::new();
                            let mut final_byte = None;
                            for ch in chars.by_ref() {
                                if ('@'..='~').contains(&ch) {
                                    final_byte = Some(ch);
                                    break;
                                }
                                if parameters.len() < 256 {
                                    parameters.push(ch);
                                }
                            }
                            if final_byte == Some('m') {
                                push_span(&mut highlights, start..text.len(), self.style);
                                start = text.len();
                                self.sgr(&parameters);
                            }
                        }
                        Some(']' | 'P' | 'X' | '^' | '_') => {
                            while let Some(ch) = chars.next() {
                                if ch == '\u{7}'
                                    || (ch == '\u{1b}' && chars.next_if_eq(&'\\').is_some())
                                {
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                '\r' => {
                    text.clear();
                    highlights.clear();
                    start = 0;
                }
                '\t' if text.len() + 4 <= MAX_LINE_BYTES => text.push_str("    "),
                ch if !ch.is_control() && text.len() + ch.len_utf8() <= MAX_LINE_BYTES => {
                    text.push(ch)
                }
                _ => {}
            }
        }
        push_span(&mut highlights, start..text.len(), self.style);
        ConsoleText::new(text, highlights)
    }

    fn sgr(&mut self, parameters: &str) {
        let mut values = parameters.split(';').map(|value| {
            if value.is_empty() {
                Some(0)
            } else {
                value.parse::<u16>().ok()
            }
        });
        while let Some(code) = values.next() {
            match code {
                Some(0) => self.reset(),
                Some(1) => self.style.font_weight = Some(FontWeight::BOLD),
                Some(2) => self.style.fade_out = Some(0.35),
                Some(3) => self.style.font_style = Some(FontStyle::Italic),
                Some(4) => {
                    self.style.underline = Some(UnderlineStyle {
                        thickness: px(1.),
                        color: None,
                        wavy: false,
                    })
                }
                Some(22) => {
                    self.style.font_weight = None;
                    self.style.fade_out = None;
                }
                Some(23) => self.style.font_style = None,
                Some(24) => self.style.underline = None,
                Some(30..=37) => {
                    self.style.color = code
                        .and_then(|n| ansi_color(n - 30))
                        .map(|color| rgb(color).into())
                }
                Some(40..=47) => {
                    self.style.background_color = code
                        .and_then(|n| ansi_color(n - 40))
                        .map(|color| rgb(color).into())
                }
                Some(90..=97) => {
                    self.style.color = code
                        .and_then(|n| ansi_color(n - 90 + 8))
                        .map(|color| rgb(color).into())
                }
                Some(100..=107) => {
                    self.style.background_color = code
                        .and_then(|n| ansi_color(n - 100 + 8))
                        .map(|color| rgb(color).into())
                }
                Some(38 | 48) => {
                    let color = match values.next().flatten() {
                        Some(5) => values.next().flatten().and_then(ansi_color),
                        Some(2) => {
                            let red = values.next().flatten().filter(|n| *n <= 255);
                            let green = values.next().flatten().filter(|n| *n <= 255);
                            let blue = values.next().flatten().filter(|n| *n <= 255);
                            red.zip(green).zip(blue).map(|((r, g), b)| {
                                (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
                            })
                        }
                        _ => None,
                    };
                    if let Some(color) = color {
                        if code == Some(38) {
                            self.style.color = Some(rgb(color).into());
                        } else {
                            self.style.background_color = Some(rgb(color).into());
                        }
                    }
                }
                Some(39) => self.style.color = None,
                Some(49) => self.style.background_color = None,
                _ => {}
            }
        }
    }
}

fn push_span(
    spans: &mut Vec<(Range<usize>, HighlightStyle)>,
    range: Range<usize>,
    style: HighlightStyle,
) {
    if !range.is_empty() && style != HighlightStyle::default() && spans.len() < MAX_SPANS {
        spans.push((range, style));
    }
}

fn ansi_color(index: u16) -> Option<u32> {
    const BASIC: [u32; 16] = [
        0x000000, 0xaa0000, 0x00aa00, 0xaa5500, 0x0000aa, 0xaa00aa, 0x00aaaa, 0xaaaaaa, 0x555555,
        0xff5555, 0x55ff55, 0xffff55, 0x5555ff, 0xff55ff, 0x55ffff, 0xffffff,
    ];
    match index {
        0..=15 => BASIC.get(usize::from(index)).copied(),
        16..=231 => {
            let n = u32::from(index - 16);
            let component = |n| if n == 0 { 0 } else { 55 + n * 40 };
            Some((component(n / 36) << 16) | (component(n / 6 % 6) << 8) | component(n % 6))
        }
        232..=255 => Some(u32::from(8 + (index - 232) * 10) * 0x010101),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_rgb_styles_with_utf8_byte_ranges_and_plain_copy_text() {
        let line = AnsiParser::default().parse("prefix \x1b[38;2;255;85;85mé猫\x1b[0m suffix");
        assert_eq!(line.text.as_ref(), "prefix é猫 suffix");
        assert_eq!(line.highlights.len(), 1);
        assert_eq!(
            line.highlights.first().map(|span| span.0.clone()),
            Some(7..12)
        );
        assert_eq!(
            line.highlights.first().and_then(|span| span.1.color),
            Some(rgb(0xff5555).into())
        );
    }

    #[test]
    fn styles_persist_across_lines_and_osc_controls_are_hidden() {
        let mut parser = AnsiParser::default();
        parser.parse("\x1b[1;38;5;196mred");
        let line = parser
            .parse("\x1b]8;;https://example.invalid\x1b\\link\x1b]8;;\x1b\\\x1b[0m plain [42]");
        assert_eq!(line.text.as_ref(), "link plain [42]");
        assert_eq!(
            line.highlights.first().and_then(|span| span.1.font_weight),
            Some(FontWeight::BOLD)
        );
        assert!(parser.parse("plain").highlights.is_empty());
    }

    #[test]
    fn malformed_codes_and_excessive_styles_are_bounded() {
        let mut parser = AnsiParser::default();
        let line = parser.parse(&"\x1b[31mx\x1b[0m".repeat(10_000));
        assert!(line.text.len() <= MAX_LINE_BYTES);
        assert!(line.highlights.len() <= MAX_SPANS);
        assert_eq!(
            parser
                .parse("ok\x1b[38;2;999;0;0mbad\x1b[unfinished")
                .text
                .as_ref(),
            "okbadnfinished"
        );
    }
}

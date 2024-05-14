use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
};

use crate::theme::{
    DOS_BLACK, DOS_BLUE, DOS_BROWN, DOS_CYAN, DOS_DARK_GRAY, DOS_GREEN, DOS_LIGHT_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_LIGHT_GREEN, DOS_LIGHT_MAGENTA,
    DOS_LIGHT_RED, DOS_MAGENTA, DOS_RED, DOS_WHITE, DOS_YELLOW,
};

#[derive(Debug)]
enum PcbState {
    Default,
    GotAt,
    ReadColor1,
    ReadColor2,
}
pub fn get_styled_pcb_line<'a>(txt: &str) -> Line<'a> {
    get_styled_pcb_line_with_highlight(txt, false)
}

pub fn get_styled_pcb_line_with_highlight<'a>(txt: &str, highlight: bool) -> Line<'a> {
    let mut spans = Vec::new();

    let mut span_builder = String::new();
    let mut last_fg = None;
    let mut last_bg = None;

    let mut cur_fg_color = None;
    let mut cur_bg_color = None;
    let mut state = PcbState::Default;

    for ch in txt.chars() {
        match state {
            PcbState::ReadColor1 => {
                match ch.to_ascii_uppercase() {
                    '0' => cur_bg_color = Some(DOS_BLACK),
                    '1' => cur_bg_color = Some(DOS_BLUE),
                    '2' => cur_bg_color = Some(DOS_GREEN),
                    '3' => cur_bg_color = Some(DOS_CYAN),
                    '4' => cur_bg_color = Some(DOS_RED),
                    '5' => cur_bg_color = Some(DOS_MAGENTA),
                    '6' => cur_bg_color = Some(DOS_BROWN),
                    '7' => cur_bg_color = Some(DOS_LIGHT_GRAY),

                    '8' => cur_bg_color = Some(DOS_BLACK),
                    '9' => cur_bg_color = Some(DOS_BLUE),
                    'A' => cur_bg_color = Some(DOS_GREEN),
                    'B' => cur_bg_color = Some(DOS_CYAN),
                    'C' => cur_bg_color = Some(DOS_RED),
                    'D' => cur_bg_color = Some(DOS_MAGENTA),
                    'E' => cur_bg_color = Some(DOS_BROWN),
                    'F' => cur_bg_color = Some(DOS_LIGHT_GRAY),

                    _ => {
                        span_builder.push('@');
                        span_builder.push('X');
                        span_builder.push(ch);
                        state = PcbState::Default;
                        continue;
                    }
                }
                state = PcbState::ReadColor2;
            }

            PcbState::ReadColor2 => {
                match ch.to_ascii_uppercase() {
                    '0' => cur_fg_color = Some(DOS_BLACK),
                    '1' => cur_fg_color = Some(DOS_BLUE),
                    '2' => cur_fg_color = Some(DOS_GREEN),
                    '3' => cur_fg_color = Some(DOS_CYAN),
                    '4' => cur_fg_color = Some(DOS_RED),
                    '5' => cur_fg_color = Some(DOS_MAGENTA),
                    '6' => cur_fg_color = Some(DOS_BROWN),
                    '7' => cur_fg_color = Some(DOS_LIGHT_GRAY),

                    '8' => cur_fg_color = Some(DOS_DARK_GRAY),
                    '9' => cur_fg_color = Some(DOS_LIGHT_BLUE),
                    'A' => cur_fg_color = Some(DOS_LIGHT_GREEN),
                    'B' => cur_fg_color = Some(DOS_LIGHT_CYAN),
                    'C' => cur_fg_color = Some(DOS_LIGHT_RED),
                    'D' => cur_fg_color = Some(DOS_LIGHT_MAGENTA),
                    'E' => cur_fg_color = Some(DOS_YELLOW),
                    'F' => cur_fg_color = Some(DOS_WHITE),

                    _ => {
                        span_builder.push('@');
                        span_builder.push('X');
                        span_builder.push('0');
                        span_builder.push(ch);
                    }
                }
                state = PcbState::Default;
            }

            PcbState::GotAt => {
                if ch.to_ascii_uppercase() == 'X' {
                    state = PcbState::ReadColor1;
                } else {
                    span_builder.push('@');
                    span_builder.push(ch);
                    state = PcbState::Default;
                }
            }

            PcbState::Default => {
                if last_fg != cur_fg_color || last_bg != cur_bg_color {
                    if !span_builder.is_empty() {
                        let mut s = if let (Some(fg), Some(bg)) = (last_bg, last_fg) {
                            Span::styled(span_builder.clone(), Style::default().fg(fg).fg(bg))
                        } else {
                            Span::raw(span_builder.clone())
                        };
                        if highlight {
                            s = s.bold();
                        }
                        spans.push(s);

                        span_builder.clear();
                    }
                    last_fg = cur_fg_color;
                    last_bg = cur_bg_color;
                }

                if ch == '@' {
                    state = PcbState::GotAt;
                } else {
                    span_builder.push(ch);
                }
            }
        }
    }

    if !span_builder.is_empty() {
        let mut s = if let (Some(fg), Some(bg)) = (cur_fg_color, cur_bg_color) {
            Span::styled(span_builder.clone(), Style::default().fg(fg).bg(bg))
        } else {
            Span::raw(span_builder.clone())
        };
        if highlight {
            s = s.bold();
        }
        spans.push(s);

        span_builder.clear();
    }
    Line::from(spans)
}

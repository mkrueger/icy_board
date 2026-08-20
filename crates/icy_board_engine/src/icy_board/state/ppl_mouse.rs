use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub const MOUSE_MODE_TEXT: i32 = 0;
pub const MOUSE_MODE_GRAPHICS: i32 = 1;

pub const MOUSE_EVENT_NONE: i32 = 0;
pub const MOUSE_EVENT_PRESS: i32 = 1;
pub const MOUSE_EVENT_RELEASE: i32 = 2;
pub const MOUSE_EVENT_MOTION: i32 = 3;
pub const MOUSE_EVENT_WHEEL: i32 = 4;

pub const MOUSE_OFF_SEQUENCE: &[u8] = b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1016l";

const MAX_EVENTS: usize = 256;
const MAX_SEQUENCE: usize = 64;
const ESCAPE_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PplMouseEvent {
    pub event_type: i32,
    pub x: i32,
    pub y: i32,
    pub button: i32,
    pub modifiers: i32,
}

#[derive(Default)]
pub struct PplMouseState {
    enabled: bool,
    graphics_mode: bool,
    pixel_mode: bool,
    pending: Vec<u8>,
    pending_since: Option<Instant>,
    events: VecDeque<PplMouseEvent>,
    current: PplMouseEvent,
}

impl PplMouseState {
    pub fn enable(&mut self, mode: i32, tracking: i32) -> bool {
        if !matches!(mode, MOUSE_MODE_TEXT | MOUSE_MODE_GRAPHICS) {
            return false;
        }
        if !(0..=2).contains(&tracking) {
            return false;
        }
        self.enabled = true;
        self.graphics_mode = mode == MOUSE_MODE_GRAPHICS;
        self.pixel_mode = false;
        self.pending.clear();
        self.pending_since = None;
        self.events.clear();
        self.current = PplMouseEvent::default();
        true
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.graphics_mode = false;
        self.pixel_mode = false;
        self.pending.clear();
        self.pending_since = None;
        self.events.clear();
        self.current = PplMouseEvent::default();
    }

    pub fn is_graphics_mode(&self) -> bool {
        self.graphics_mode
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    pub fn pixels(&self) -> bool {
        self.pixel_mode
    }

    pub fn feed(&mut self, byte: u8) -> Vec<u8> {
        if !self.enabled {
            return vec![byte];
        }
        if self.pending.is_empty() {
            if byte == 0x1B {
                self.pending.push(byte);
                self.pending_since = Some(Instant::now());
                return Vec::new();
            }
            return vec![byte];
        }

        self.pending.push(byte);
        if self.pending.len() == 2 && byte != b'[' {
            return self.take_pending();
        }
        if self.pending.len() == 3 && byte != b'<' && byte != b'?' {
            return self.take_pending();
        }
        if self.pending.len() > MAX_SEQUENCE {
            return self.take_pending();
        }
        if self.pending.len() <= 3 {
            return Vec::new();
        }
        if self.pending[2] == b'?' {
            if byte == b'y' {
                let sequence = std::mem::take(&mut self.pending);
                self.pending_since = None;
                if let Some(enabled) = parse_pixel_mode_report(&sequence) {
                    self.pixel_mode = enabled;
                    return Vec::new();
                }
                return sequence;
            }
            if !byte.is_ascii_digit() && byte != b';' && byte != b'$' {
                return self.take_pending();
            }
            return Vec::new();
        }
        if byte == b'M' || byte == b'm' {
            let sequence = std::mem::take(&mut self.pending);
            self.pending_since = None;
            if let Some(event) = parse_sgr_event(&sequence) {
                if self.events.len() == MAX_EVENTS {
                    self.events.pop_front();
                }
                self.events.push_back(event);
                return Vec::new();
            }
            return sequence;
        }
        if !byte.is_ascii_digit() && byte != b';' {
            return self.take_pending();
        }
        Vec::new()
    }

    pub fn take_stale_keyboard(&mut self) -> Vec<u8> {
        if self.pending_since.is_some_and(|started| started.elapsed() >= ESCAPE_TIMEOUT) {
            self.take_pending()
        } else {
            Vec::new()
        }
    }

    pub fn poll(&mut self) -> i32 {
        let Some(event) = self.events.pop_front() else {
            return MOUSE_EVENT_NONE;
        };
        self.current = event;
        event.event_type
    }

    pub fn current(&self) -> PplMouseEvent {
        self.current
    }

    pub fn enable_sequence(&self, tracking: i32) -> Vec<u8> {
        let mut sequence = b"\x1b[?1000l\x1b[?1002l\x1b[?1003l".to_vec();
        sequence.extend_from_slice(match tracking {
            0 => b"\x1b[?1000h".as_slice(),
            1 => b"\x1b[?1002h".as_slice(),
            _ => b"\x1b[?1003h".as_slice(),
        });
        sequence.extend_from_slice(b"\x1b[?1006h");
        if self.graphics_mode {
            sequence.extend_from_slice(b"\x1b[?1016h\x1b[?1016$p");
        } else {
            sequence.extend_from_slice(b"\x1b[?1016l");
        }
        sequence
    }

    fn take_pending(&mut self) -> Vec<u8> {
        self.pending_since = None;
        std::mem::take(&mut self.pending)
    }
}

fn parse_pixel_mode_report(sequence: &[u8]) -> Option<bool> {
    let body = std::str::from_utf8(sequence.strip_prefix(b"\x1b[?1016;")?).ok()?;
    let status = body.strip_suffix("$y")?.parse::<u8>().ok()?;
    Some(matches!(status, 1 | 3))
}

fn parse_sgr_event(sequence: &[u8]) -> Option<PplMouseEvent> {
    let body = std::str::from_utf8(sequence.strip_prefix(b"\x1b[<")?).ok()?;
    let final_byte = *body.as_bytes().last()?;
    let values = &body[..body.len() - 1];
    let mut parts = values.split(';');
    let code = parts.next()?.parse::<u16>().ok()?;
    let x = parts.next()?.parse::<i32>().ok()?.checked_sub(1)?;
    let y = parts.next()?.parse::<i32>().ok()?.checked_sub(1)?;
    if parts.next().is_some() || x < 0 || y < 0 {
        return None;
    }

    let modifiers = i32::from(code & 4 != 0) | (i32::from(code & 8 != 0) << 1) | (i32::from(code & 16 != 0) << 2);
    let wheel = code & 64 != 0;
    let motion = code & 32 != 0;
    let base_button = i32::from(code & 3);
    let (event_type, button) = if motion {
        (MOUSE_EVENT_MOTION, if base_button == 3 { -1 } else { base_button })
    } else if wheel {
        (MOUSE_EVENT_WHEEL, if code & 1 == 0 { 3 } else { 4 })
    } else if final_byte == b'm' {
        (MOUSE_EVENT_RELEASE, if base_button == 3 { -1 } else { base_button })
    } else {
        (MOUSE_EVENT_PRESS, if base_button == 3 { -1 } else { base_button })
    };
    Some(PplMouseEvent {
        event_type,
        x,
        y,
        button,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sgr_press_motion_release_and_wheel() {
        let mut mouse = PplMouseState::default();
        assert!(mouse.enable(MOUSE_MODE_TEXT, 2));
        for byte in b"\x1b[<0;11;6M\x1b[<36;12;7M\x1b[<0;12;7m\x1b[<65;12;7M" {
            assert!(mouse.feed(*byte).is_empty());
        }

        assert_eq!(mouse.poll(), MOUSE_EVENT_PRESS);
        assert_eq!(
            mouse.current(),
            PplMouseEvent {
                event_type: 1,
                x: 10,
                y: 5,
                button: 0,
                modifiers: 0
            }
        );
        assert_eq!(mouse.poll(), MOUSE_EVENT_MOTION);
        assert_eq!(mouse.current().modifiers, 1);
        assert_eq!(mouse.poll(), MOUSE_EVENT_RELEASE);
        assert_eq!(mouse.poll(), MOUSE_EVENT_WHEEL);
        assert_eq!(mouse.current().button, 4);
        assert_eq!(mouse.poll(), MOUSE_EVENT_NONE);
    }

    #[test]
    fn replays_non_mouse_escape_sequences() {
        let mut mouse = PplMouseState::default();
        assert!(mouse.enable(MOUSE_MODE_TEXT, 2));
        let mut replayed = Vec::new();
        for byte in b"\x1b[A" {
            replayed.extend(mouse.feed(*byte));
        }
        assert_eq!(replayed, b"\x1b[A");
    }

    #[test]
    fn legacy_syncterm_96_and_97_are_motion_not_wheel() {
        let mut mouse = PplMouseState::default();
        assert!(mouse.enable(MOUSE_MODE_TEXT, 2));
        for byte in b"\x1b[<96;11;6M\x1b[<97;12;7M" {
            assert!(mouse.feed(*byte).is_empty());
        }

        assert_eq!(mouse.poll(), MOUSE_EVENT_MOTION);
        assert_eq!(mouse.poll(), MOUSE_EVENT_MOTION);
        assert_eq!(mouse.poll(), MOUSE_EVENT_NONE);
    }

    #[test]
    fn pixel_mode_requires_terminal_confirmation() {
        let mut mouse = PplMouseState::default();
        assert!(mouse.enable(MOUSE_MODE_GRAPHICS, 2));
        assert!(!mouse.pixels());
        for byte in b"\x1b[?1016;1$y" {
            assert!(mouse.feed(*byte).is_empty());
        }
        assert!(mouse.pixels());
    }

    #[test]
    fn pixel_confirmation_can_be_followed_by_a_click_in_one_read() {
        let mut mouse = PplMouseState::default();
        assert!(mouse.enable(MOUSE_MODE_GRAPHICS, 2));
        for byte in b"\x1b[?1016;1$y\x1b[<0;101;51M" {
            assert!(mouse.feed(*byte).is_empty());
        }
        assert!(mouse.pixels());
        assert_eq!(mouse.poll(), MOUSE_EVENT_PRESS);
        assert_eq!((mouse.current().x, mouse.current().y), (100, 50));
    }
}

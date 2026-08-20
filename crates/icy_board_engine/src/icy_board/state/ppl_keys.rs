use std::collections::VecDeque;

const MAX_EVENTS: usize = 256;
const MAX_SEQUENCE: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PplKeyEvent {
    pub code: i32,
    pub pressed: bool,
}

#[derive(Default)]
pub struct PplKeyState {
    enabled: bool,
    pending: Vec<u8>,
    events: VecDeque<PplKeyEvent>,
    current: PplKeyEvent,
}

impl PplKeyState {
    pub fn enable(&mut self) {
        self.enabled = true;
        self.pending.clear();
        self.events.clear();
        self.current = PplKeyEvent::default();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.pending.clear();
        self.events.clear();
        self.current = PplKeyEvent::default();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    pub fn current(&self) -> PplKeyEvent {
        self.current
    }

    pub fn poll(&mut self) -> bool {
        let Some(event) = self.events.pop_front() else {
            return false;
        };
        self.current = event;
        true
    }

    pub fn feed(&mut self, byte: u8) -> Vec<u8> {
        if !self.enabled {
            return vec![byte];
        }
        if self.pending.is_empty() {
            if byte == 0x1b {
                self.pending.push(byte);
                return Vec::new();
            }
            return vec![byte];
        }
        self.pending.push(byte);
        if self.pending.len() == 2 {
            if byte != b'[' {
                return std::mem::take(&mut self.pending);
            }
            return Vec::new();
        }
        if self.pending.len() == 3 {
            if byte != b'=' {
                return std::mem::take(&mut self.pending);
            }
            return Vec::new();
        }
        if self.pending.len() > MAX_SEQUENCE {
            return std::mem::take(&mut self.pending);
        }
        if byte != b'K' && byte != b'k' {
            if (0x40..=0x7e).contains(&byte) {
                return std::mem::take(&mut self.pending);
            }
            return Vec::new();
        }

        let sequence = std::mem::take(&mut self.pending);
        let Some(body) = sequence.strip_prefix(b"\x1b[=").and_then(|body| body.get(..body.len().saturating_sub(1))) else {
            return sequence;
        };
        let Ok(body) = std::str::from_utf8(body) else {
            return sequence;
        };
        let pressed = byte == b'K';
        let codes = body.split(';').map(str::parse::<i32>).collect::<Result<Vec<_>, _>>();
        let Ok(codes) = codes else {
            return sequence;
        };
        for code in codes {
            if self.events.len() == MAX_EVENTS {
                self.events.pop_front();
            }
            self.events.push_back(PplKeyEvent { code, pressed });
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_press_and_release_edges_and_preserves_other_input() {
        let mut keys = PplKeyState::default();
        keys.enable();
        let mut passed = Vec::new();
        for byte in b"a\x1b[=30;31K\x1b[=30k" {
            passed.extend(keys.feed(*byte));
        }
        assert_eq!(passed, b"a");
        assert!(keys.poll());
        assert_eq!(keys.current(), PplKeyEvent { code: 30, pressed: true });
        assert!(keys.poll());
        assert_eq!(keys.current(), PplKeyEvent { code: 31, pressed: true });
        assert!(keys.poll());
        assert_eq!(keys.current(), PplKeyEvent { code: 30, pressed: false });
    }

    #[test]
    fn preserves_unrelated_cterm_state_reports() {
        let mut keys = PplKeyState::default();
        keys.enable();
        let mut passed = Vec::new();
        for byte in b"\x1b[=7;2;1n" {
            passed.extend(keys.feed(*byte));
        }
        assert_eq!(passed, b"\x1b[=7;2;1n");
        assert!(!keys.poll());
    }
}

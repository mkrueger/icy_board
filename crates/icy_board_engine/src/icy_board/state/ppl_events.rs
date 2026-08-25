use async_trait::async_trait;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use super::{KeyChar, ppl_keys::PplKeyEvent, ppl_mouse::PplMouseEvent};
use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::EVENT_ID,
};

pub const EVENT_NONE: i32 = 0;
pub const EVENT_KEY: i32 = 1;
pub const EVENT_KEY_EDGE: i32 = 2;
pub const EVENT_MOUSE: i32 = 3;
pub const EVENT_OVERFLOW: i32 = 4;
pub const EVENT_SOUND: i32 = 5;

pub const KEY_UP: i32 = 0x11_0001;
pub const KEY_DOWN: i32 = 0x11_0002;
pub const KEY_RIGHT: i32 = 0x11_0003;
pub const KEY_LEFT: i32 = 0x11_0004;
pub const KEY_HOME: i32 = 0x11_0005;
pub const KEY_END: i32 = 0x11_0006;
pub const KEY_PAGE_UP: i32 = 0x11_0007;
pub const KEY_PAGE_DOWN: i32 = 0x11_0008;
pub const KEY_INSERT: i32 = 0x11_0009;

pub static KIND: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Kind".to_string()));
pub static CODE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Code".to_string()));
pub static SCAN_CODE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ScanCode".to_string()));
pub static TEXT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Text".to_string()));
pub static PRESSED: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Pressed".to_string()));
pub static X: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("X".to_string()));
pub static Y: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Y".to_string()));
pub static BUTTON: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Button".to_string()));
pub static MODIFIERS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Modifiers".to_string()));
pub static PIXELS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Pixels".to_string()));
pub static REPEATED: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Repeated".to_string()));
pub static BUTTONS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Buttons".to_string()));
pub static WHEEL_X: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("WheelX".to_string()));
pub static WHEEL_Y: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("WheelY".to_string()));
pub static TIME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Time".to_string()));
pub static ACTION: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Action".to_string()));
pub static CHANNEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Channel".to_string()));
pub static DROPPED: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Dropped".to_string()));
pub static LEFT_DOWN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("LeftDown".to_string()));
pub static MIDDLE_DOWN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MiddleDown".to_string()));
pub static RIGHT_DOWN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("RightDown".to_string()));
pub static SHIFT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Shift".to_string()));
pub static ALT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Alt".to_string()));
pub static CTRL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Ctrl".to_string()));
pub static META: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Meta".to_string()));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PplEvent {
    pub event_type: i32,
    pub code: i32,
    pub text: String,
    pub pressed: bool,
    pub x: i32,
    pub y: i32,
    pub button: i32,
    pub modifiers: i32,
    pub pixels: bool,
    pub repeated: bool,
    pub buttons: i32,
    pub wheel_x: i32,
    pub wheel_y: i32,
    pub time: u64,
}

impl PplEvent {
    pub fn value(self) -> VariableValue {
        user_data_value(self, EVENT_ID)
    }

    pub fn key(key: KeyChar) -> Self {
        Self {
            event_type: EVENT_KEY,
            code: key.ch as i32,
            text: key.ch.to_string(),
            pressed: true,
            ..Default::default()
        }
    }

    pub fn key_edge(key: PplKeyEvent) -> Self {
        Self {
            event_type: EVENT_KEY_EDGE,
            code: key.code,
            pressed: key.pressed,
            repeated: key.repeated,
            ..Default::default()
        }
    }

    pub fn mouse(mouse: PplMouseEvent) -> Self {
        Self {
            event_type: EVENT_MOUSE,
            code: mouse.event_type,
            x: mouse.x,
            y: mouse.y,
            button: mouse.button,
            modifiers: mouse.modifiers,
            buttons: mouse.buttons,
            wheel_x: mouse.wheel_x,
            wheel_y: mouse.wheel_y,
            ..Default::default()
        }
    }

    pub fn overflow(dropped: usize) -> Self {
        Self {
            event_type: EVENT_OVERFLOW,
            code: dropped.min(i32::MAX as usize) as i32,
            ..Default::default()
        }
    }

    /// A channel the terminal reports as drained, named the way `SNDPLAY` named it.
    pub fn sound(channel: i32) -> Self {
        Self {
            event_type: EVENT_SOUND,
            code: channel,
            ..Default::default()
        }
    }
}

impl UserData for PplEvent {
    const TYPE_NAME: &'static str = "Event";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        use crate::parser::{EVENT_KIND_ENUM_ID, MOUSE_ACTION_ENUM_ID, MOUSE_BUTTON_ENUM_ID};

        registry.add_property(KIND.clone(), VariableType::UserData(EVENT_KIND_ENUM_ID), false);
        registry.add_property(CODE.clone(), VariableType::Integer, false);
        registry.add_property(SCAN_CODE.clone(), VariableType::Integer, false);
        registry.add_property(TEXT.clone(), VariableType::String, false);
        registry.add_property(PRESSED.clone(), VariableType::Boolean, false);
        registry.add_property(X.clone(), VariableType::Integer, false);
        registry.add_property(Y.clone(), VariableType::Integer, false);
        registry.add_property(BUTTON.clone(), VariableType::UserData(MOUSE_BUTTON_ENUM_ID), false);
        registry.add_property(PIXELS.clone(), VariableType::Boolean, false);
        registry.add_property(REPEATED.clone(), VariableType::Boolean, false);
        registry.add_property(WHEEL_X.clone(), VariableType::Integer, false);
        registry.add_property(WHEEL_Y.clone(), VariableType::Integer, false);
        registry.add_property(TIME.clone(), VariableType::Unsigned, false);

        // What Code used to stand for depends on the kind, so each meaning says its own name.
        registry.add_property(ACTION.clone(), VariableType::UserData(MOUSE_ACTION_ENUM_ID), false);
        registry.add_property(CHANNEL.clone(), VariableType::Integer, false);
        registry.add_property(DROPPED.clone(), VariableType::Integer, false);
        registry.add_property(LEFT_DOWN.clone(), VariableType::Boolean, false);
        registry.add_property(MIDDLE_DOWN.clone(), VariableType::Boolean, false);
        registry.add_property(RIGHT_DOWN.clone(), VariableType::Boolean, false);
        registry.add_property(SHIFT.clone(), VariableType::Boolean, false);
        registry.add_property(ALT.clone(), VariableType::Boolean, false);
        registry.add_property(CTRL.clone(), VariableType::Boolean, false);
        registry.add_property(META.clone(), VariableType::Boolean, false);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplEvent {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *KIND {
            return Ok(VariableValue::new_int(self.event_type));
        }
        if *name == *CODE {
            // A translated key and a physical one are different numbering, so each
            // reports only for its own kind.
            let code = if self.event_type == EVENT_KEY { self.code } else { 0 };
            return Ok(VariableValue::new_int(code));
        }
        if *name == *SCAN_CODE {
            let scan_code = if self.event_type == EVENT_KEY_EDGE { self.code } else { 0 };
            return Ok(VariableValue::new_int(scan_code));
        }
        if *name == *TEXT {
            return Ok(VariableValue::new_string(self.text.clone()));
        }
        if *name == *PRESSED {
            return Ok(VariableValue::new_bool(self.pressed));
        }
        if *name == *X {
            return Ok(VariableValue::new_int(self.x));
        }
        if *name == *Y {
            return Ok(VariableValue::new_int(self.y));
        }
        if *name == *BUTTON {
            return Ok(VariableValue::new_int(self.button));
        }
        if *name == *PIXELS {
            return Ok(VariableValue::new_bool(self.pixels));
        }
        if *name == *REPEATED {
            return Ok(VariableValue::new_bool(self.repeated));
        }
        if *name == *WHEEL_X {
            return Ok(VariableValue::new_int(self.wheel_x));
        }
        if *name == *WHEEL_Y {
            return Ok(VariableValue::new_int(self.wheel_y));
        }
        if *name == *TIME {
            return Ok(VariableValue::new_unsigned(self.time));
        }
        if *name == *ACTION {
            // Only a mouse event acts; anything else reports no action rather than a number
            // that would read as one.
            let action = if self.event_type == EVENT_MOUSE { self.code } else { 0 };
            return Ok(VariableValue::new_int(action));
        }
        if *name == *CHANNEL {
            let channel = if self.event_type == EVENT_SOUND { self.code } else { -1 };
            return Ok(VariableValue::new_int(channel));
        }
        if *name == *DROPPED {
            let dropped = if self.event_type == EVENT_OVERFLOW { self.code } else { 0 };
            return Ok(VariableValue::new_int(dropped));
        }
        if *name == *LEFT_DOWN {
            return Ok(VariableValue::new_bool(self.buttons & 1 != 0));
        }
        if *name == *MIDDLE_DOWN {
            return Ok(VariableValue::new_bool(self.buttons & 2 != 0));
        }
        if *name == *RIGHT_DOWN {
            return Ok(VariableValue::new_bool(self.buttons & 4 != 0));
        }
        if *name == *SHIFT {
            return Ok(VariableValue::new_bool(self.modifiers & 1 != 0));
        }
        if *name == *ALT {
            return Ok(VariableValue::new_bool(self.modifiers & 2 != 0));
        }
        if *name == *CTRL {
            return Ok(VariableValue::new_bool(self.modifiers & 4 != 0));
        }
        if *name == *META {
            return Ok(VariableValue::new_bool(self.modifiers & 8 != 0));
        }
        Err("Invalid EVENT property".into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("EVENT properties are read-only".into())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        _name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err("EVENT has no functions".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("EVENT has no methods".into())
    }
}

const ESCAPE_TIMEOUT: Duration = Duration::from_millis(50);

/// Longest reply the channel watch is willing to collect before giving it back.
const MAX_NOTIFY_SEQUENCE: usize = 64;

/// Catches the `CSI = 7 ; channel ; 0 n` a terminal sends once a channel it was asked
/// to watch has drained. Everything else is handed back untouched.
#[derive(Default)]
pub struct AudioNotifyState {
    watching: bool,
    pending: Vec<u8>,
    pending_since: Option<Instant>,
    drained: VecDeque<i32>,
}

impl AudioNotifyState {
    /// Only a running channel can report a drain, so nothing else may hold back the
    /// escape a cursor key starts with.
    pub fn set_watching(&mut self, watching: bool) {
        self.watching = watching;
    }

    pub fn feed(&mut self, byte: u8) -> Vec<u8> {
        if self.pending.is_empty() {
            if byte == 0x1b && self.watching {
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
        if self.pending.len() == 3 && byte != b'=' {
            return self.take_pending();
        }
        if self.pending.len() > MAX_NOTIFY_SEQUENCE {
            return self.take_pending();
        }
        if self.pending.len() < 4 || !(0x40..=0x7e).contains(&byte) {
            return Vec::new();
        }
        let sequence = self.take_pending();
        match drained_channel(&sequence) {
            Some(channel) => {
                self.drained.push_back(channel);
                Vec::new()
            }
            None => sequence,
        }
    }

    pub fn poll(&mut self) -> Option<i32> {
        self.drained.pop_front()
    }

    pub fn take_stale_keyboard(&mut self) -> Vec<u8> {
        if self.pending_since.is_some_and(|started| started.elapsed() >= ESCAPE_TIMEOUT) {
            self.take_pending()
        } else {
            Vec::new()
        }
    }

    pub fn take_pending_bytes(&mut self) -> Vec<u8> {
        self.take_pending()
    }

    fn take_pending(&mut self) -> Vec<u8> {
        self.pending_since = None;
        std::mem::take(&mut self.pending)
    }
}

/// The logical channel a drain report names, or nothing when the reply is another answer.
fn drained_channel(sequence: &[u8]) -> Option<i32> {
    let body = std::str::from_utf8(sequence).ok()?.strip_prefix("\x1b[=")?.strip_suffix('n')?;
    let parts: Vec<&str> = body.split(';').collect();
    if parts.len() != 3 || parts[0] != "7" || parts[2] != "0" {
        return None;
    }
    let channel = parts[1].parse::<i32>().ok()?;
    // The board hands out APC channels from two, because CTerm keeps the first two.
    (2..16).contains(&channel).then_some(channel - 2)
}

pub struct LogicalKeyState {
    started: Instant,
    pending: Vec<KeyChar>,
    pending_since: Option<Instant>,
    ready: VecDeque<PplEvent>,
}

impl Default for LogicalKeyState {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            pending: Vec::new(),
            pending_since: None,
            ready: VecDeque::new(),
        }
    }
}

impl LogicalKeyState {
    pub fn feed(&mut self, key: KeyChar) {
        if self.pending.is_empty() && key.ch != '\x1b' {
            self.ready.push_back(self.named_key(key.ch));
            return;
        }
        if self.pending.is_empty() {
            self.pending_since = Some(Instant::now());
        }
        self.pending.push(key);
        if self.pending.len() == 2 && self.pending[1].ch != '[' {
            self.flush_pending();
            return;
        }
        if self.pending.len() < 3 || self.pending[1].ch != '[' {
            return;
        }
        let final_key = self.pending.last().unwrap().ch;
        if !('@'..='~').contains(&final_key) {
            return;
        }
        if let Some((code, text, modifiers)) = self.parse_csi_key(final_key) {
            self.ready.push_back(self.special_key(code, text, modifiers));
            self.clear_pending();
        } else {
            self.flush_pending();
        }
    }

    pub fn poll(&mut self) -> Option<PplEvent> {
        self.flush_stale();
        self.ready.pop_front()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.pending_since = None;
        self.ready.clear();
    }

    fn named_key(&self, ch: char) -> PplEvent {
        let text = if ch as u32 == 127 { "DEL".to_string() } else { ch.to_string() };
        let mut event = PplEvent {
            event_type: EVENT_KEY,
            code: ch as i32,
            text,
            pressed: true,
            ..Default::default()
        };
        event.time = self.elapsed_ms();
        event
    }

    fn special_key(&self, code: i32, text: &str, modifiers: i32) -> PplEvent {
        PplEvent {
            event_type: EVENT_KEY,
            code,
            text: text.to_string(),
            pressed: true,
            modifiers,
            time: self.elapsed_ms(),
            ..Default::default()
        }
    }

    fn parse_csi_key(&self, final_key: char) -> Option<(i32, &'static str, i32)> {
        let body: String = self.pending[2..self.pending.len() - 1].iter().map(|key| key.ch).collect();
        let parameters: Vec<i32> = body.split(';').filter_map(|value| value.parse().ok()).collect();
        let modifiers = parameters.get(1).copied().map_or(0, |value| (value - 1).max(0));
        let key = match final_key {
            'A' => (KEY_UP, "UP"),
            'B' => (KEY_DOWN, "DOWN"),
            'C' => (KEY_RIGHT, "RIGHT"),
            'D' => (KEY_LEFT, "LEFT"),
            'H' => (KEY_HOME, "HOME"),
            'F' => (KEY_END, "END"),
            'V' => (KEY_PAGE_UP, "PGUP"),
            'U' => (KEY_PAGE_DOWN, "PGDN"),
            '@' => (KEY_INSERT, "INS"),
            '~' => match parameters.first().copied()? {
                1 | 7 => (KEY_HOME, "HOME"),
                2 => (KEY_INSERT, "INS"),
                3 => (127, "DEL"),
                4 | 8 => (KEY_END, "END"),
                5 => (KEY_PAGE_UP, "PGUP"),
                6 => (KEY_PAGE_DOWN, "PGDN"),
                _ => return None,
            },
            _ => return None,
        };
        Some((key.0, key.1, modifiers))
    }

    fn flush_stale(&mut self) {
        if self.pending_since.is_some_and(|started| started.elapsed() >= ESCAPE_TIMEOUT) {
            self.flush_pending();
        }
    }

    fn flush_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        self.pending_since = None;
        for key in pending {
            self.ready.push_back(self.named_key(key.ch));
        }
    }

    fn clear_pending(&mut self) {
        self.pending.clear();
        self.pending_since = None;
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::state::KeySource;

    #[test]
    fn ansi_keys_become_one_logical_event() {
        let mut keys = LogicalKeyState::default();
        for ch in "\x1b[A".chars() {
            keys.feed(KeyChar::new(KeySource::User, ch));
        }
        let event = keys.poll().unwrap();
        assert_eq!((event.event_type, event.code, event.text.as_str()), (EVENT_KEY, KEY_UP, "UP"));
        assert!(keys.poll().is_none());
    }

    #[test]
    fn delete_keeps_inkey_text_and_a_named_code() {
        let mut keys = LogicalKeyState::default();
        keys.feed(KeyChar::new(KeySource::User, '\x7f'));
        let event = keys.poll().unwrap();
        assert_eq!((event.code, event.text.as_str()), (127, "DEL"));
    }

    #[test]
    fn parameterized_keys_include_xterm_modifiers() {
        let mut keys = LogicalKeyState::default();
        for ch in "\x1b[1;5A\x1b[2~".chars() {
            keys.feed(KeyChar::new(KeySource::User, ch));
        }
        let up = keys.poll().unwrap();
        assert_eq!((up.code, up.modifiers), (KEY_UP, 4));
        let insert = keys.poll().unwrap();
        assert_eq!((insert.code, insert.text.as_str()), (KEY_INSERT, "INS"));
    }

    #[test]
    fn audio_notify_releases_a_lone_escape_after_sequence_timeout() {
        let mut notify = AudioNotifyState::default();
        notify.set_watching(true);
        assert!(notify.feed(0x1b).is_empty());

        std::thread::sleep(ESCAPE_TIMEOUT);

        assert_eq!(notify.take_stale_keyboard(), vec![0x1b]);
        assert!(notify.take_stale_keyboard().is_empty());
    }

    #[test]
    fn audio_notify_passes_escapes_on_when_no_channel_is_watched() {
        let mut notify = AudioNotifyState::default();

        assert_eq!(notify.feed(0x1b), vec![0x1b]);
        assert_eq!(notify.feed(b'['), vec![b'[']);
        assert!(notify.take_stale_keyboard().is_empty());
    }
}

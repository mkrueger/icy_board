use codepages::tables::UNICODE_TO_CP437;

use crate::vm::TerminalTarget;

pub const MACRO_SLOTS: usize = 64;
const MAX_MACRO_BYTES: usize = 512 * 1024;

pub struct FinishedMacro {
    pub slot: usize,
    pub user_bytes: Vec<u8>,
    pub sysop_bytes: Vec<u8>,
    pub overflowed: bool,
}

struct MacroRecording {
    slot: usize,
    user_bytes: Vec<u8>,
    sysop_bytes: Vec<u8>,
    overflowed: bool,
}

pub struct PplTerminalControl {
    update_depth: usize,
    recording: Option<MacroRecording>,
    defined_macros: [bool; MACRO_SLOTS],
}

impl Default for PplTerminalControl {
    fn default() -> Self {
        Self {
            update_depth: 0,
            recording: None,
            defined_macros: [false; MACRO_SLOTS],
        }
    }
}

impl PplTerminalControl {
    pub fn begin_update(&mut self) -> bool {
        let outermost = self.update_depth == 0;
        self.update_depth = self.update_depth.saturating_add(1);
        outermost
    }

    pub fn end_update(&mut self) -> Option<bool> {
        if self.update_depth == 0 {
            return None;
        }
        self.update_depth -= 1;
        Some(self.update_depth == 0)
    }

    pub fn take_update_depth(&mut self) -> usize {
        std::mem::take(&mut self.update_depth)
    }

    pub fn start_recording(&mut self, slot: usize) -> bool {
        if self.recording.is_some() {
            return false;
        }
        self.recording = Some(MacroRecording {
            slot,
            user_bytes: Vec::new(),
            sysop_bytes: Vec::new(),
            overflowed: false,
        });
        true
    }

    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn record(&mut self, target: TerminalTarget, data: &[char], user_is_utf8: bool, is_sysop: bool, has_no_user: bool) -> bool {
        let Some(recording) = &mut self.recording else {
            return false;
        };
        let mut utf8 = [0; 4];
        for ch in data {
            if target != TerminalTarget::Sysop || is_sysop || has_no_user {
                if user_is_utf8 {
                    push_bytes(recording, true, ch.encode_utf8(&mut utf8).as_bytes());
                } else {
                    push_bytes(recording, true, &[UNICODE_TO_CP437.get(ch).copied().unwrap_or(b'.')]);
                }
            }
            if target != TerminalTarget::User {
                push_bytes(recording, false, &[UNICODE_TO_CP437.get(ch).copied().unwrap_or(b'.')]);
            }
        }
        true
    }

    pub fn finish_recording(&mut self) -> Option<FinishedMacro> {
        let recording = self.recording.take()?;
        Some(FinishedMacro {
            slot: recording.slot,
            user_bytes: recording.user_bytes,
            sysop_bytes: recording.sysop_bytes,
            overflowed: recording.overflowed,
        })
    }

    pub fn is_defined(&self, slot: usize) -> bool {
        self.defined_macros[slot]
    }

    pub fn mark_defined(&mut self, slot: usize) {
        self.defined_macros[slot] = true;
    }

    pub fn mark_deleted(&mut self, slot: usize) {
        self.defined_macros[slot] = false;
    }

    pub fn clear_defined(&mut self) -> bool {
        self.recording = None;
        let had_macros = self.defined_macros.contains(&true);
        self.defined_macros.fill(false);
        had_macros
    }

    pub fn take_defined_slots(&mut self) -> Vec<usize> {
        self.recording = None;
        let slots = self
            .defined_macros
            .iter()
            .enumerate()
            .filter_map(|(slot, defined)| defined.then_some(slot))
            .collect();
        self.defined_macros.fill(false);
        slots
    }
}

fn push_bytes(recording: &mut MacroRecording, user: bool, bytes: &[u8]) {
    let target = if user { &mut recording.user_bytes } else { &mut recording.sysop_bytes };
    let Some(new_len) = target.len().checked_add(bytes.len()) else {
        recording.overflowed = true;
        return;
    };
    if new_len > MAX_MACRO_BYTES {
        recording.overflowed = true;
        return;
    }
    target.extend_from_slice(bytes);
}

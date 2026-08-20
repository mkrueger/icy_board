//! Channel bookkeeping for PPL's dBase opcodes.
//!
//! Every operation reports failure the way PPL does, with `true` meaning an error, so the
//! statement and function forms of an opcode can share one implementation.

pub mod file;
pub mod index;
pub mod ops;

use std::path::{Path, PathBuf};

use codepages::tables::get_utf8;

use self::{
    file::{DbaseFile, FieldInfo, MAX_DBASE_CHANNELS, TYPE_CHARACTER, TYPE_DATE, TYPE_FLOAT, TYPE_LOGICAL, TYPE_NUMERIC, to_cp437},
    index::DbaseIndex,
};
use crate::executable::{VariableType, VariableValue};

/// What DSELECT answers when no channel carries the alias.
pub const NO_CHANNEL: i32 = MAX_DBASE_CHANNELS as i32;

struct DbaseChannel {
    db: DbaseFile,
    alias: String,
    indexes: Vec<DbaseIndex>,
    active_index: Option<usize>,
    error: bool,
    bof: bool,
    eof: bool,
    /// DNEW has opened a buffer that DADD has still to write out.
    pending: bool,
}

#[derive(Default)]
pub struct DbaseState {
    channels: [Option<Box<DbaseChannel>>; MAX_DBASE_CHANNELS],
}

impl DbaseState {
    fn slot(&mut self, channel: i32) -> Option<&mut Box<DbaseChannel>> {
        let index = usize::try_from(channel).ok()?;
        self.channels.get_mut(index)?.as_mut()
    }

    fn get(&self, channel: i32) -> Option<&DbaseChannel> {
        let index = usize::try_from(channel).ok()?;
        self.channels.get(index)?.as_deref()
    }

    /// Records an error against a channel and hands back PPL's `true` for failure.
    fn fail(&mut self, channel: i32) -> bool {
        if let Some(slot) = self.slot(channel) {
            slot.error = true;
        }
        true
    }

    fn ok(&mut self, channel: i32) -> bool {
        if let Some(slot) = self.slot(channel) {
            slot.error = false;
        }
        false
    }

    pub fn is_open(&self, channel: i32) -> bool {
        self.get(channel).is_some()
    }

    pub fn error(&self, channel: i32) -> bool {
        self.get(channel).is_none_or(|c| c.error)
    }

    /// The lowest channel with nothing open on it. It is not reserved, matching `PCBoard`.
    pub fn next_free(&self) -> i32 {
        self.channels.iter().position(std::option::Option::is_none).map_or(NO_CHANNEL, |i| i as i32)
    }

    pub fn select(&self, alias: &str) -> i32 {
        self.channels
            .iter()
            .position(|c| c.as_ref().is_some_and(|c| c.alias.eq_ignore_ascii_case(alias)))
            .map_or(NO_CHANNEL, |i| i as i32)
    }

    pub fn alias(&self, channel: i32) -> String {
        self.get(channel).map_or_else(String::new, |c| c.alias.clone())
    }

    pub fn set_alias(&mut self, channel: i32, alias: &str) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        slot.alias = alias.to_string();
        self.ok(channel)
    }

    // -- opening and closing ------------------------------------------------------------

    pub fn create(&mut self, channel: i32, path: &Path, fields: &[FieldInfo]) -> bool {
        let Ok(db) = DbaseFile::create(path, fields) else {
            return self.fail(channel);
        };
        self.install(channel, db)
    }

    pub fn open(&mut self, channel: i32, path: &Path) -> bool {
        let Ok(db) = DbaseFile::open(path) else {
            return self.fail(channel);
        };
        self.install(channel, db)
    }

    fn install(&mut self, channel: i32, mut db: DbaseFile) -> bool {
        let Ok(index) = usize::try_from(channel) else {
            return true;
        };
        if index >= MAX_DBASE_CHANNELS {
            return true;
        }
        // dBase opens a table positioned on its first record.
        let _ = db.goto(1);
        let alias = db.path().file_stem().map_or_else(String::new, |s| s.to_string_lossy().to_uppercase());
        self.channels[index] = Some(Box::new(DbaseChannel {
            db,
            alias,
            indexes: Vec::new(),
            active_index: None,
            error: false,
            bof: false,
            eof: false,
            pending: false,
        }));
        false
    }

    pub fn close(&mut self, channel: i32) -> bool {
        let Ok(index) = usize::try_from(channel) else {
            return true;
        };
        let Some(slot) = self.channels.get_mut(index) else {
            return true;
        };
        let Some(mut channel) = slot.take() else {
            return true;
        };
        channel.db.flush().is_err()
    }

    pub fn close_all(&mut self) -> bool {
        let mut failed = false;
        for index in 0..MAX_DBASE_CHANNELS {
            if self.channels[index].is_some() {
                failed |= self.close(index as i32);
            }
        }
        failed
    }

    // -- field metadata -----------------------------------------------------------------

    pub fn field_count(&self, channel: i32) -> i32 {
        self.get(channel).map_or(0, |c| c.db.fields().len() as i32)
    }

    pub fn field_name(&self, channel: i32, number: i32) -> String {
        let Some(slot) = self.get(channel) else {
            return String::new();
        };
        usize::try_from(number)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| slot.db.fields().get(n))
            .map_or_else(String::new, |f| f.name.clone())
    }

    fn field(&self, channel: i32, name: &str) -> Option<&FieldInfo> {
        let slot = self.get(channel)?;
        let index = slot.db.field_index(name)?;
        slot.db.fields().get(index)
    }

    /// `PCBoard` reports a float field as numeric even though the header keeps `F`.
    pub fn field_type(&self, channel: i32, name: &str) -> String {
        self.field(channel, name).map_or_else(String::new, |f| {
            let reported = if f.field_type == TYPE_FLOAT { TYPE_NUMERIC } else { f.field_type };
            (reported as char).to_string()
        })
    }

    pub fn field_length(&self, channel: i32, name: &str) -> i32 {
        self.field(channel, name).map_or(0, |f| f.length as i32)
    }

    pub fn field_decimals(&self, channel: i32, name: &str) -> i32 {
        self.field(channel, name).map_or(0, |f| i32::from(f.decimals))
    }

    // -- navigation ---------------------------------------------------------------------

    pub fn record_count(&self, channel: i32) -> i32 {
        self.get(channel).map_or(0, |c| c.db.record_count() as i32)
    }

    pub fn record_no(&self, channel: i32) -> i32 {
        self.get(channel).map_or(0, |c| c.db.record_no() as i32)
    }

    /// A channel with nothing open on it reports both ends of the file at once, so a
    /// record loop written the usual way stops on its first test.
    pub fn bof(&self, channel: i32) -> bool {
        self.get(channel).is_none_or(|c| c.bof)
    }

    pub fn eof(&self, channel: i32) -> bool {
        self.get(channel).is_none_or(|c| c.eof)
    }

    pub fn changed(&self, channel: i32) -> bool {
        self.get(channel).is_some_and(|c| c.db.dirty())
    }

    pub fn deleted(&self, channel: i32) -> bool {
        self.get(channel).is_some_and(|c| c.db.is_deleted())
    }

    pub fn go(&mut self, channel: i32, record_no: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        let count = slot.db.record_count() as i32;
        let target = record_no.clamp(0, count.saturating_add(1));
        if slot.db.goto(target.max(0) as usize).is_err() {
            return self.fail(channel);
        }
        let Some(slot) = self.slot(channel) else {
            return true;
        };
        slot.bof = record_no < 1;
        slot.eof = record_no > count;
        self.ok(channel)
    }

    pub fn top(&mut self, channel: i32) -> bool {
        self.go(channel, 1)
    }

    pub fn bottom(&mut self, channel: i32) -> bool {
        let count = self.record_count(channel);
        self.go(channel, count.max(1))
    }

    /// Walking off either end parks the position just past it, as dBase does.
    pub fn skip(&mut self, channel: i32, count: i32) -> bool {
        let Some(slot) = self.get(channel) else {
            return self.fail(channel);
        };
        let target = slot.db.record_no() as i32 + count;
        self.go(channel, target)
    }

    // -- field values -------------------------------------------------------------------

    /// The raw padded bytes of a field, which is exactly what a PPE expects to see.
    pub fn get_field(&mut self, channel: i32, name: &str) -> String {
        let Some(slot) = self.get(channel) else {
            self.fail(channel);
            return String::new();
        };
        let Some(index) = slot.db.field_index(name) else {
            self.fail(channel);
            return String::new();
        };
        let text = get_utf8(slot.db.get_field(index));
        self.ok(channel);
        text
    }

    pub fn put_field(&mut self, channel: i32, name: &str, value: &VariableValue) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        let Some(index) = slot.db.field_index(name) else {
            return self.fail(channel);
        };
        let field = slot.db.fields()[index].clone();
        let Some(bytes) = format_value(&field, value) else {
            return self.fail(channel);
        };
        slot.db.set_field(index, &bytes);
        self.ok(channel)
    }

    pub fn blank_field(&mut self, channel: i32, name: &str) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        let Some(index) = slot.db.field_index(name) else {
            return self.fail(channel);
        };
        slot.db.blank_field(index);
        self.ok(channel)
    }

    pub fn copy_field(&mut self, from: i32, from_name: &str, to: i32, to_name: &str) -> bool {
        let Some(source) = self.get(from) else {
            return self.fail(from);
        };
        let Some(index) = source.db.field_index(from_name) else {
            return self.fail(from);
        };
        let bytes = source.db.get_field(index).to_vec();

        let Some(target) = self.slot(to) else {
            return self.fail(to);
        };
        let Some(index) = target.db.field_index(to_name) else {
            return self.fail(to);
        };
        target.db.set_field(index, &bytes);
        self.ok(to)
    }

    // -- record lifecycle ---------------------------------------------------------------

    pub fn blank_record(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        slot.db.blank_record();
        self.ok(channel)
    }

    pub fn new_record(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        if slot.db.begin_new().is_err() {
            return self.fail(channel);
        }
        let Some(slot) = self.slot(channel) else {
            return true;
        };
        slot.pending = true;
        self.ok(channel)
    }

    pub fn add_record(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        if slot.db.append().is_err() {
            return self.fail(channel);
        }
        let Some(slot) = self.slot(channel) else {
            return true;
        };
        slot.pending = false;
        slot.bof = false;
        slot.eof = false;
        self.ok(channel)
    }

    pub fn append_record(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        if slot.db.append_blank().is_err() {
            return self.fail(channel);
        }
        let Some(slot) = self.slot(channel) else {
            return true;
        };
        slot.bof = false;
        slot.eof = false;
        self.ok(channel)
    }

    pub fn set_deleted(&mut self, channel: i32, deleted: bool) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        if slot.db.record_no() < 1 || slot.db.record_no() > slot.db.record_count() {
            return self.fail(channel);
        }
        slot.db.set_deleted(deleted);
        if slot.db.flush().is_err() {
            return self.fail(channel);
        }
        self.ok(channel)
    }

    pub fn pack(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        if slot.db.pack().is_err() {
            return self.fail(channel);
        }
        self.ok(channel)
    }

    // -- indexes ------------------------------------------------------------------------

    pub fn create_index(&mut self, channel: i32, name: &str, path: PathBuf, expression: &str) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        match DbaseIndex::create(name.to_uppercase(), path, expression.to_uppercase(), &mut slot.db) {
            Ok(index) => {
                slot.indexes.push(index);
                slot.active_index = Some(slot.indexes.len() - 1);
                self.ok(channel)
            }
            Err(_) => self.fail(channel),
        }
    }

    pub fn open_index(&mut self, channel: i32, name: &str, path: PathBuf) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        match DbaseIndex::open(name.to_uppercase(), path, &mut slot.db) {
            Ok(index) => {
                slot.indexes.push(index);
                slot.active_index = Some(slot.indexes.len() - 1);
                self.ok(channel)
            }
            Err(_) => self.fail(channel),
        }
    }

    pub fn close_index(&mut self, channel: i32, name: &str) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        let wanted = name.to_uppercase();
        let Some(at) = slot
            .indexes
            .iter()
            .position(|i| i.name.eq_ignore_ascii_case(&wanted) || i.path.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&wanted)))
        else {
            return self.fail(channel);
        };
        slot.indexes.remove(at);
        slot.active_index = if slot.indexes.is_empty() { None } else { Some(0) };
        self.ok(channel)
    }

    pub fn close_all_indexes(&mut self, channel: i32) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        slot.indexes.clear();
        slot.active_index = None;
        self.ok(channel)
    }

    pub fn tag(&mut self, channel: i32, name: &str) -> bool {
        let Some(slot) = self.slot(channel) else {
            return self.fail(channel);
        };
        let Some(at) = slot.indexes.iter().position(|i| i.name.eq_ignore_ascii_case(name)) else {
            return self.fail(channel);
        };
        slot.active_index = Some(at);
        self.ok(channel)
    }

    /// `PCBoard` answers 1 for an exact or leading match and 0 for anything else, leaving
    /// the position just past the end when it finds nothing.
    pub fn seek(&mut self, channel: i32, search: &str) -> i32 {
        let Some(slot) = self.get(channel) else {
            self.fail(channel);
            return 0;
        };
        let Some(active) = slot.active_index else {
            self.fail(channel);
            return 0;
        };
        let found = slot.indexes[active].seek(&to_cp437(search));
        if let Some(record_no) = found {
            self.go(channel, record_no as i32);
            1
        } else {
            let past = self.record_count(channel) + 1;
            self.go(channel, past);
            0
        }
    }
}

/// Renders a PPL value the way the field's type stores it on disk.
fn format_value(field: &FieldInfo, value: &VariableValue) -> Option<Vec<u8>> {
    let text = match field.field_type {
        TYPE_CHARACTER => value.as_string(),
        TYPE_NUMERIC | TYPE_FLOAT => {
            let rendered = format!("{:.*}", field.decimals as usize, value.as_double());
            format!("{rendered:>width$}", width = field.length)
        }
        TYPE_DATE => {
            // dBase keeps a date as CCYYMMDD text, which is how a DDATE spells itself.
            let ddate = value.clone().convert_to(VariableType::DDate);
            if ddate.as_int() == 0 { String::new() } else { ddate.as_string() }
        }
        TYPE_LOGICAL => if value.as_bool() { "1" } else { "0" }.to_string(),
        _ => return None,
    };
    Some(to_cp437(&text))
}

/// Appends `.DBF` when the PPE left the extension off.
pub fn table_path(name: &str) -> String {
    if Path::new(name).extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.DBF")
    }
}

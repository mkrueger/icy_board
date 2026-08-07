//! The dBase III+ layer behind PPL's `D*` opcodes.
//!
//! PPL hands a PPE the raw fixed width bytes of a field rather than a typed value, so
//! everything here works on the record image and leaves interpretation to the caller.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use codepages::tables::{UNICODE_TO_CP437, get_utf8};

use crate::Res;

pub const MAX_DBASE_CHANNELS: usize = 8;

const HEADER_SIZE: usize = 32;
const DESCRIPTOR_SIZE: usize = 32;
const HEADER_TERMINATOR: u8 = 0x0D;
const FILE_TERMINATOR: u8 = 0x1A;

const NOT_DELETED: u8 = b' ';
const DELETED: u8 = b'*';

pub const TYPE_CHARACTER: u8 = b'C';
pub const TYPE_NUMERIC: u8 = b'N';
pub const TYPE_FLOAT: u8 = b'F';
pub const TYPE_DATE: u8 = b'D';
pub const TYPE_LOGICAL: u8 = b'L';
pub const TYPE_MEMO: u8 = b'M';

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: u8,
    pub length: usize,
    pub decimals: u8,
    /// Where the field starts in the record image, past the deletion flag.
    offset: usize,
}

/// An open `.DBF` together with the record the channel is currently sitting on.
pub struct DbaseFile {
    file: File,
    path: PathBuf,
    fields: Vec<FieldInfo>,
    record_count: usize,
    record_size: usize,
    first_record: u64,
    /// The record image, deletion flag included.
    record: Vec<u8>,
    /// 1 based; 0 means no record is loaded.
    record_no: usize,
    dirty: bool,
}

impl DbaseFile {
    pub fn open(path: &Path) -> Res<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;

        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;
        let record_count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let first_record = u16::from_le_bytes([header[8], header[9]]) as u64;
        let record_size = u16::from_le_bytes([header[10], header[11]]) as usize;

        if first_record as usize <= HEADER_SIZE || record_size == 0 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "not a dBase file")));
        }

        let descriptor_bytes = first_record as usize - HEADER_SIZE - 1;
        let mut descriptors = vec![0u8; descriptor_bytes];
        file.read_exact(&mut descriptors)?;

        let mut fields = Vec::new();
        let mut offset = 0;
        for chunk in descriptors.chunks_exact(DESCRIPTOR_SIZE) {
            if chunk[0] == HEADER_TERMINATOR || chunk[0] == 0 {
                break;
            }
            let name_end = chunk[..11].iter().position(|c| *c == 0).unwrap_or(11);
            let length = chunk[16] as usize;
            fields.push(FieldInfo {
                name: get_utf8(&chunk[..name_end]).to_uppercase(),
                field_type: chunk[11],
                length,
                decimals: chunk[17],
                offset,
            });
            offset += length;
        }

        Ok(Self {
            file,
            path: path.to_path_buf(),
            fields,
            record_count,
            record_size,
            first_record,
            record: vec![NOT_DELETED; record_size],
            record_no: 0,
            dirty: false,
        })
    }

    pub fn create(path: &Path, fields: &[FieldInfo]) -> Res<Self> {
        let mut fields = fields.to_vec();
        let mut offset = 0;
        for field in &mut fields {
            field.offset = offset;
            offset += field.length;
        }
        let record_size = offset + 1;
        let first_record = (HEADER_SIZE + DESCRIPTOR_SIZE * fields.len() + 1) as u64;

        let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path)?;
        let mut header = [0u8; HEADER_SIZE];
        header[0] = 0x03;
        let today = chrono::Local::now().date_naive();
        header[1] = (chrono::Datelike::year(&today) - 1900) as u8;
        header[2] = chrono::Datelike::month(&today) as u8;
        header[3] = chrono::Datelike::day(&today) as u8;
        header[8..10].copy_from_slice(&(first_record as u16).to_le_bytes());
        header[10..12].copy_from_slice(&(record_size as u16).to_le_bytes());
        file.write_all(&header)?;

        for field in &fields {
            let mut descriptor = [0u8; DESCRIPTOR_SIZE];
            let name = to_cp437(&field.name);
            let len = name.len().min(11);
            descriptor[..len].copy_from_slice(&name[..len]);
            descriptor[11] = field.field_type;
            descriptor[16] = field.length as u8;
            descriptor[17] = field.decimals;
            file.write_all(&descriptor)?;
        }
        file.write_all(&[HEADER_TERMINATOR, FILE_TERMINATOR])?;

        Ok(Self {
            file,
            path: path.to_path_buf(),
            fields,
            record_count: 0,
            record_size,
            first_record,
            record: vec![NOT_DELETED; record_size],
            record_no: 0,
            dirty: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn fields(&self) -> &[FieldInfo] {
        &self.fields
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn record_no(&self) -> usize {
        self.record_no
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    fn record_offset(&self, record_no: usize) -> u64 {
        self.first_record + (record_no - 1) as u64 * self.record_size as u64
    }

    /// Loads a record into the buffer. Out of range numbers leave the buffer alone, which
    /// is what lets a PPE read back the value it just wrote to a field that does not exist.
    pub fn goto(&mut self, record_no: usize) -> Res<bool> {
        if record_no < 1 || record_no > self.record_count {
            self.record_no = record_no;
            return Ok(false);
        }
        self.flush()?;
        let offset = self.record_offset(record_no);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut self.record)?;
        self.record_no = record_no;
        Ok(true)
    }

    pub fn flush(&mut self) -> Res<()> {
        if !self.dirty || self.record_no < 1 || self.record_no > self.record_count {
            return Ok(());
        }
        let offset = self.record_offset(self.record_no);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&self.record)?;
        self.file.flush()?;
        self.dirty = false;
        Ok(())
    }

    pub fn get_field(&self, index: usize) -> &[u8] {
        let field = &self.fields[index];
        let start = 1 + field.offset;
        &self.record[start..start + field.length]
    }

    pub fn set_field(&mut self, index: usize, value: &[u8]) {
        let field = self.fields[index].clone();
        let start = 1 + field.offset;
        let slot = &mut self.record[start..start + field.length];
        slot.fill(b' ');
        let len = value.len().min(field.length);
        slot[..len].copy_from_slice(&value[..len]);
        self.dirty = true;
    }

    pub fn blank_field(&mut self, index: usize) {
        let field = &self.fields[index];
        let start = 1 + field.offset;
        let length = field.length;
        self.record[start..start + length].fill(b' ');
        self.dirty = true;
    }

    pub fn blank_record(&mut self) {
        self.record.fill(b' ');
        self.record[0] = NOT_DELETED;
        self.dirty = true;
    }

    pub fn is_deleted(&self) -> bool {
        self.record[0] == DELETED
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        self.record[0] = if deleted { DELETED } else { NOT_DELETED };
        self.dirty = true;
    }

    /// Starts the fresh buffer DNEW hands to a PPE. The position is dropped so that
    /// filling the buffer in cannot write over the record the channel was sitting on.
    pub fn begin_new(&mut self) -> Res<()> {
        self.flush()?;
        self.record.fill(b' ');
        self.record[0] = NOT_DELETED;
        self.record_no = 0;
        self.dirty = false;
        Ok(())
    }

    /// Writes the buffer out as a brand new record and positions on it.
    pub fn append(&mut self) -> Res<()> {
        self.record_count += 1;
        self.record_no = self.record_count;
        let offset = self.record_offset(self.record_no);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&self.record)?;
        self.file.write_all(&[FILE_TERMINATOR])?;
        self.dirty = false;
        self.write_record_count()
    }

    pub fn append_blank(&mut self) -> Res<()> {
        self.begin_new()?;
        self.append()
    }

    fn write_record_count(&mut self) -> Res<()> {
        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&(self.record_count as u32).to_le_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    /// Drops every record whose deletion flag is set and renumbers the rest.
    pub fn pack(&mut self) -> Res<()> {
        self.flush()?;
        let mut kept: Vec<Vec<u8>> = Vec::new();
        let mut buffer = vec![0u8; self.record_size];
        for no in 1..=self.record_count {
            self.file.seek(SeekFrom::Start(self.record_offset(no)))?;
            self.file.read_exact(&mut buffer)?;
            if buffer[0] != DELETED {
                kept.push(buffer.clone());
            }
        }

        self.file.seek(SeekFrom::Start(self.first_record))?;
        for record in &kept {
            self.file.write_all(record)?;
        }
        self.file.write_all(&[FILE_TERMINATOR])?;
        let end = self.first_record + (kept.len() * self.record_size) as u64 + 1;
        self.file.set_len(end)?;

        self.record_count = kept.len();
        self.dirty = false;
        self.write_record_count()?;
        self.goto(1)?;
        Ok(())
    }
}

pub fn to_cp437(text: &str) -> Vec<u8> {
    text.chars().map(|c| *UNICODE_TO_CP437.get(&c).unwrap_or(&b'.')).collect()
}

/// Parses one `"Name,Type,Length,Decimals"` entry of a DCREATE field list.
pub fn parse_field_info(spec: &str) -> Option<FieldInfo> {
    let mut parts = spec.split(',');
    let name = parts.next()?.trim().to_uppercase();
    let field_type = parts.next()?.trim().to_uppercase().bytes().next()?;
    let length: usize = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let decimals: u8 = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);

    if name.is_empty() || length == 0 || length > 255 {
        return None;
    }
    if !matches!(field_type, TYPE_CHARACTER | TYPE_NUMERIC | TYPE_FLOAT | TYPE_DATE | TYPE_LOGICAL) {
        return None;
    }
    Some(FieldInfo {
        name,
        field_type,
        length,
        decimals,
        offset: 0,
    })
}

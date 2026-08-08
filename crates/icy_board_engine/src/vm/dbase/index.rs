//! The `.NDX` index behind DNOPEN, DNCREATE and DSEEK.
//!
//! The keys are kept sorted in memory, which is what DSEEK searches. The file on disk
//! exists so a later DNOPEN can recover the key expression and so dBase tools can read it;
//! only a single root page is written, which covers the table sizes a BBS deals in.

use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use codepages::tables::get_utf8;

use super::file::{DbaseFile, to_cp437};
use crate::Res;

const PAGE_SIZE: usize = 512;
const EXPRESSION_OFFSET: usize = 0x18;
const EXPRESSION_MAX: usize = 256;

pub struct DbaseIndex {
    pub name: String,
    pub path: PathBuf,
    pub expression: String,
    /// Sorted `(key, record number)` pairs, the key being the raw padded field bytes.
    entries: Vec<(Vec<u8>, usize)>,
    key_length: usize,
}

impl DbaseIndex {
    /// Builds an index over a field of an already open table.
    pub fn create(name: String, path: PathBuf, expression: String, db: &mut DbaseFile) -> Res<Self> {
        let Some(field) = db.field_index(&expression) else {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "no such index field")));
        };
        let key_length = db.fields()[field].length;

        let restore = db.record_no();
        let mut entries = Vec::with_capacity(db.record_count());
        for no in 1..=db.record_count() {
            if db.goto(no)? {
                entries.push((db.get_field(field).to_vec(), no));
            }
        }
        db.goto(restore)?;
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let index = Self {
            name,
            path,
            expression,
            entries,
            key_length,
        };
        index.write()?;
        Ok(index)
    }

    /// Reads an existing index, taking only the key expression from the file and
    /// rebuilding the keys from the table so they cannot be stale.
    pub fn open(name: String, path: PathBuf, db: &mut DbaseFile) -> Res<Self> {
        let mut file = OpenOptions::new().read(true).open(&path)?;
        let mut header = [0u8; PAGE_SIZE];
        file.read_exact(&mut header)?;

        let tail = &header[EXPRESSION_OFFSET..EXPRESSION_OFFSET + EXPRESSION_MAX];
        let end = tail.iter().position(|c| *c == 0).unwrap_or(tail.len());
        let expression = get_utf8(&tail[..end]).trim().to_string();

        Self::create(name, path, expression, db)
    }

    fn write(&self) -> Res<()> {
        let record_size = 8 + self.key_length;
        let per_page = (PAGE_SIZE - 4) / record_size;

        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&self.path)?;

        let mut header = [0u8; PAGE_SIZE];
        header[0..4].copy_from_slice(&1u32.to_le_bytes());
        header[4..8].copy_from_slice(&2u32.to_le_bytes());
        header[0x0C..0x0E].copy_from_slice(&(self.key_length as u16).to_le_bytes());
        header[0x0E..0x10].copy_from_slice(&(per_page as u16).to_le_bytes());
        header[0x12..0x14].copy_from_slice(&(record_size as u16).to_le_bytes());
        let expression = to_cp437(&self.expression);
        let len = expression.len().min(EXPRESSION_MAX - 1);
        header[EXPRESSION_OFFSET..EXPRESSION_OFFSET + len].copy_from_slice(&expression[..len]);
        file.write_all(&header)?;

        let written = self.entries.len().min(per_page);
        let mut page = vec![0u8; PAGE_SIZE];
        page[0..4].copy_from_slice(&(written as u32).to_le_bytes());
        for (slot, (key, record_no)) in self.entries.iter().take(written).enumerate() {
            let at = 4 + slot * record_size;
            page[at + 4..at + 8].copy_from_slice(&(*record_no as u32).to_le_bytes());
            let len = key.len().min(self.key_length);
            page[at + 8..at + 8 + len].copy_from_slice(&key[..len]);
        }
        file.seek(SeekFrom::Start(PAGE_SIZE as u64))?;
        file.write_all(&page)?;
        Ok(())
    }

    /// The record number of the first key starting with `search`, in index order.
    pub fn seek(&self, search: &[u8]) -> Option<usize> {
        self.entries.iter().find(|(key, _)| key.starts_with(search)).map(|(_, no)| *no)
    }
}

/// Appends `.NDX` when the PPE left the extension off.
pub fn index_path(name: &str) -> String {
    if Path::new(name).extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.NDX")
    }
}

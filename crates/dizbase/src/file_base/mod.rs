use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use file_header::FileAttributes;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;
use twox_hash::XxHash3_64;

use crate::file_base_scanner::scan_file;

use self::{
    file_header::FileHeader,
    metadata::{MetadataHeader, MetadataType},
};

pub mod file_header;
pub mod metadata;
pub mod pattern;

/// Bumped whenever the schema changes so that `migrate` knows what to apply.
const SCHEMA_VERSION: i32 = 1;

/// Holds the file base's own files, out of the way of anything a user can upload.
pub const STATE_DIR: &str = ".icy";

#[derive(Error, Debug)]
pub enum FileBaseError {
    #[error("Invalid header signature (needs to start with 'ICFB')")]
    InvalidHeaderSignature,

    #[error("Invalid search token")]
    InvalidSearchToken,

    #[error("Directory {0} is not a directory")]
    DirIsNoDir(PathBuf),

    #[error("Can't open metadata file")]
    CantOpenMetadata,

    #[error("File {0} not found")]
    FileNotFound(String),

    #[error("File {0} is already in the file base")]
    FileAlreadyExists(String),

    #[error("No extension found")]
    NoExtension,
}

pub struct FileBase {
    connection: Connection,
    dir: PathBuf,
    /// Names that live in the directory but are not downloadable files.
    reserved_names: HashSet<String>,
    name_map: HashMap<String, usize>,
    file_headers: Vec<FileHeader>,
}

impl Deref for FileBase {
    type Target = Vec<FileHeader>;
    fn deref(&self) -> &Self::Target {
        &self.file_headers
    }
}

impl DerefMut for FileBase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file_headers
    }
}

impl FileBase {
    /// `meta_data_path` names the index; the database and its write-ahead log go into a
    /// `.icy` directory beside it, where a directory scan will never look.
    pub fn database_path<P: AsRef<Path>>(meta_data_path: P) -> PathBuf {
        let meta_data_path = meta_data_path.as_ref();
        let name = meta_data_path.file_name().unwrap_or(OsStr::new("dir"));
        meta_data_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(STATE_DIR)
            .join(name)
            .with_extension("db")
    }

    pub fn open<P: AsRef<Path>>(dir: &Path, meta_data_path: P) -> crate::Result<Self> {
        let db_path = Self::database_path(&meta_data_path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&db_path)?;
        Self::configure(&connection)?;
        Self::migrate(&connection)?;

        let mut res = Self {
            connection,
            dir: dir.to_path_buf(),
            reserved_names: Self::reserved_names(&meta_data_path),
            name_map: HashMap::new(),
            file_headers: Vec::new(),
        };
        res.load_headers()?;
        if let Err(err) = res.scan_path() {
            log::error!("Filebase error scanning path: {}", err);
        }
        Ok(res)
    }

    /// An index left behind by the old binary format sits in the directory it describes.
    fn reserved_names<P: AsRef<Path>>(meta_data_path: P) -> HashSet<String> {
        let mut names = HashSet::new();
        if let Some(index) = meta_data_path.as_ref().file_name().and_then(|n| n.to_str()) {
            names.insert(format!("{}.fmd", index));
            names.insert(index.to_string());
        }
        names
    }

    /// WAL lets the other nodes keep reading while one of them writes.
    fn configure(connection: &Connection) -> crate::Result<()> {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(15))?;
        Ok(())
    }

    fn migrate(connection: &Connection) -> crate::Result<()> {
        let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id         INTEGER PRIMARY KEY,
                name       TEXT    NOT NULL UNIQUE,
                date       INTEGER NOT NULL,
                size       INTEGER NOT NULL,
                dl_counter INTEGER NOT NULL DEFAULT 0,
                attribute  INTEGER NOT NULL DEFAULT 0,
                scanned    INTEGER NOT NULL DEFAULT 0,
                authored   INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS metadata (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                type    INTEGER NOT NULL,
                data    BLOB    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS metadata_file_id ON metadata(file_id);",
        )?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    fn load_headers(&mut self) -> crate::Result<()> {
        let mut statement = self
            .connection
            .prepare("SELECT id, name, date, size, dl_counter, attribute FROM files ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok(FileHeader {
                id: row.get(0)?,
                name: row.get(1)?,
                date: DateTime::from_timestamp_millis(row.get(2)?).unwrap_or_else(Utc::now),
                size: row.get::<_, i64>(3)? as u64,
                dl_counter: row.get::<_, i64>(4)? as u64,
                attribute: FileAttributes::from_bits_truncate(row.get::<_, i64>(5)? as u8),
            })
        })?;

        self.file_headers.clear();
        self.name_map.clear();
        for header in rows {
            let header = header?;
            self.name_map.insert(header.name.clone(), self.file_headers.len());
            self.file_headers.push(header);
        }
        Ok(())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Files dropped into the directory out of band are picked up here; entries whose file
    /// has gone are kept so that their download counts survive a temporarily offline volume.
    fn scan_path(&mut self) -> crate::Result<()> {
        if !self.dir.is_dir() {
            return Err(FileBaseError::DirIsNoDir(self.dir.clone()).into());
        }
        let mut new_files = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                log::warn!("Skipping file with a non utf-8 name: {}", path.display());
                continue;
            };
            if self.name_map.contains_key(file_name) || self.reserved_names.contains(file_name) {
                continue;
            }
            let (date, size) = Self::file_stats(&path);
            new_files.push((file_name.to_string(), date, size));
        }
        if new_files.is_empty() {
            return Ok(());
        }

        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare("INSERT OR IGNORE INTO files (name, date, size) VALUES (?1, ?2, ?3)")?;
            for (name, date, size) in &new_files {
                statement.execute(params![name, date.timestamp_millis(), *size as i64])?;
            }
        }
        transaction.commit()?;
        self.load_headers()
    }

    fn file_stats(path: &Path) -> (DateTime<Utc>, u64) {
        let mut date = Utc::now();
        let mut size = 0;
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(system_time) = metadata.modified() {
                date = system_time.into();
            }
            size = metadata.len();
        }
        (date, size)
    }

    fn header_index(&self, path: &Path) -> crate::Result<usize> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| FileBaseError::FileNotFound(path.display().to_string()))?;
        self.name_map
            .get(file_name)
            .copied()
            .ok_or_else(|| FileBaseError::FileNotFound(file_name.to_string()).into())
    }

    pub fn add_file(&mut self, path: &Path, metadata: Vec<MetadataHeader>) -> crate::Result<()> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| FileBaseError::FileNotFound(path.display().to_string()))?
            .to_string();
        if self.name_map.contains_key(&file_name) {
            return Err(FileBaseError::FileAlreadyExists(file_name).into());
        }
        let (date, size) = Self::file_stats(path);

        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO files (name, date, size, scanned) VALUES (?1, ?2, ?3, 1)",
            params![file_name, date.timestamp_millis(), size as i64],
        )?;
        let id = transaction.last_insert_rowid();
        Self::insert_metadata(&transaction, id, &metadata)?;
        transaction.commit()?;

        self.name_map.insert(file_name.clone(), self.file_headers.len());
        self.file_headers.push(FileHeader {
            id,
            name: file_name,
            date,
            size,
            dl_counter: 0,
            attribute: FileAttributes::NONE,
        });
        Ok(())
    }

    /// Reads the stored metadata, deriving it from the file itself the first time it is asked for.
    pub fn read_metadata(&mut self, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
        let index = self.header_index(path)?;
        let id = self.file_headers[index].id;
        let scanned: bool = self
            .connection
            .query_row("SELECT scanned FROM files WHERE id = ?1", params![id], |row| row.get(0))
            .optional()?
            .unwrap_or(false);
        if !scanned {
            return self.scan_into_database(id, path);
        }

        let mut statement = self.connection.prepare("SELECT type, data FROM metadata WHERE file_id = ?1")?;
        let rows = statement.query_map(params![id], |row| {
            Ok(MetadataHeader {
                metadata_type: MetadataType::from_data(row.get::<_, i64>(0)? as u8),
                data: row.get(1)?,
            })
        })?;
        let mut result = Vec::new();
        for header in rows {
            result.push(header?);
        }
        Ok(result)
    }

    pub fn write_metadata(&mut self, path: &Path, metadata: Vec<MetadataHeader>) -> crate::Result<()> {
        let index = self.header_index(path)?;
        let id = self.file_headers[index].id;

        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM metadata WHERE file_id = ?1", params![id])?;
        Self::insert_metadata(&transaction, id, &metadata)?;
        transaction.execute("UPDATE files SET scanned = 1 WHERE id = ?1", params![id])?;
        transaction.commit()?;
        Ok(())
    }

    fn scan_into_database(&mut self, id: i64, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
        let mut metadata = match scan_file(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                log::error!("Error scanning file {}: {}", path.display(), err);
                Vec::new()
            }
        };
        // A description someone wrote outweighs whatever the archive carries.
        if let Some(authored) = Self::authored_description(&self.connection, id)? {
            metadata.retain(|header| header.get_type() != MetadataType::FileID);
            metadata.push(authored);
        }
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM metadata WHERE file_id = ?1", params![id])?;
        Self::insert_metadata(&transaction, id, &metadata)?;
        transaction.execute("UPDATE files SET scanned = 1 WHERE id = ?1", params![id])?;
        transaction.commit()?;
        Ok(metadata)
    }

    fn authored_description(connection: &Connection, id: i64) -> crate::Result<Option<MetadataHeader>> {
        let data: Option<Vec<u8>> = connection
            .query_row(
                "SELECT data FROM metadata WHERE file_id = ?1 AND type = ?2 AND (SELECT authored FROM files WHERE id = ?1) = 1",
                params![id, MetadataType::FileID.to_data() as i64],
                |row| row.get(0),
            )
            .optional()?;
        Ok(data.map(|data| MetadataHeader::new(MetadataType::FileID, data)))
    }

    /// Replaces the description without touching derived metadata such as the hash, and
    /// marks it so that a later scan will not overwrite it.
    pub fn set_description(&mut self, path: &Path, description: &str) -> crate::Result<()> {
        let index = self.header_index(path)?;
        let id = self.file_headers[index].id;

        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM metadata WHERE file_id = ?1 AND type = ?2",
            params![id, MetadataType::FileID.to_data() as i64],
        )?;
        if !description.is_empty() {
            transaction.execute(
                "INSERT INTO metadata (file_id, type, data) VALUES (?1, ?2, ?3)",
                params![id, MetadataType::FileID.to_data() as i64, description.as_bytes()],
            )?;
        }
        transaction.execute("UPDATE files SET authored = ?2 WHERE id = ?1", params![id, !description.is_empty()])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn description(&mut self, path: &Path) -> crate::Result<Option<String>> {
        Ok(self
            .read_metadata(path)?
            .iter()
            .find(|header| header.get_type() == MetadataType::FileID)
            .map(|header| String::from_utf8_lossy(&header.data).to_string()))
    }

    pub fn is_authored(&self, path: &Path) -> crate::Result<bool> {
        let id = self.file_headers[self.header_index(path)?].id;
        Ok(self
            .connection
            .query_row("SELECT authored FROM files WHERE id = ?1", params![id], |row| row.get(0))
            .optional()?
            .unwrap_or(false))
    }

    /// Throws the derived metadata away so the next read rebuilds it; `force` drops an
    /// authored description as well.
    pub fn rescan(&mut self, path: &Path, force: bool) -> crate::Result<()> {
        let id = self.file_headers[self.header_index(path)?].id;
        if force {
            self.connection.execute("UPDATE files SET scanned = 0, authored = 0 WHERE id = ?1", params![id])?;
        } else {
            self.connection.execute("UPDATE files SET scanned = 0 WHERE id = ?1", params![id])?;
        }
        self.read_metadata(path)?;
        Ok(())
    }

    pub fn remove_file(&mut self, path: &Path) -> crate::Result<()> {
        let id = self.file_headers[self.header_index(path)?].id;
        self.connection.execute("DELETE FROM files WHERE id = ?1", params![id])?;
        self.load_headers()
    }

    fn insert_metadata(connection: &Connection, id: i64, metadata: &[MetadataHeader]) -> crate::Result<()> {
        let mut statement = connection.prepare("INSERT INTO metadata (file_id, type, data) VALUES (?1, ?2, ?3)")?;
        for header in metadata {
            statement.execute(params![id, header.get_type().to_data() as i64, header.data])?;
        }
        Ok(())
    }

    /// Writes back whatever the caller changed on the headers it got through `DerefMut`.
    pub fn save(&mut self) -> crate::Result<()> {
        let transaction = self.connection.transaction()?;
        {
            let mut statement =
                transaction.prepare("UPDATE files SET date = ?2, size = ?3, dl_counter = ?4, attribute = ?5 WHERE id = ?1")?;
            for header in &self.file_headers {
                statement.execute(params![
                    header.id,
                    header.date.timestamp_millis(),
                    header.size as i64,
                    header.dl_counter as i64,
                    header.attribute.bits() as i64
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn get_hash(path: &Path) -> crate::Result<u64> {
        let data = fs::read(&path)?;
        let hash = XxHash3_64::oneshot(&data);
        Ok(hash)
    }

    pub fn full_path(&self, entry: &FileHeader) -> PathBuf {
        self.dir.join(&entry.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn base(dir: &TempDir) -> FileBase {
        FileBase::open(dir.path(), dir.path().join("dir")).unwrap()
    }

    fn write(dir: &TempDir, name: &str, contents: &[u8]) {
        fs::write(dir.path().join(name), contents).unwrap();
    }

    fn names(base: &FileBase) -> Vec<String> {
        let mut names: Vec<String> = base.iter().map(|h| h.name.clone()).collect();
        names.sort();
        names
    }

    #[test]
    fn test_files_on_disk_are_picked_up() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        write(&dir, "BETA.TXT", b"beta");

        assert_eq!(names(&base(&dir)), vec!["ALPHA.TXT", "BETA.TXT"]);
    }

    #[test]
    fn test_the_database_does_not_list_itself() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");

        drop(base(&dir));
        assert_eq!(names(&base(&dir)), vec!["ALPHA.TXT"]);
    }

    #[test]
    fn test_the_database_lives_beside_the_files_not_among_them() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        drop(base(&dir));

        assert!(dir.path().join(".icy/dir.db").is_file());
        // A file of that name can be uploaded without shadowing anything.
        write(&dir, "dir.db", b"not the database");
        assert_eq!(names(&base(&dir)), vec!["ALPHA.TXT", "dir.db"]);
    }

    #[test]
    fn test_a_metadata_path_outside_the_area_is_honoured() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");

        let mut base = FileBase::open(dir.path(), elsewhere.path().join("dir01")).unwrap();
        assert!(elsewhere.path().join(".icy/dir01.db").is_file());
        assert_eq!(base.len(), 1);
        assert!(base.description(&dir.path().join("ALPHA.TXT")).is_ok());
    }

    #[test]
    fn test_an_index_from_the_old_format_is_not_listed() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        write(&dir, "dir", b"a leftover binary index");
        write(&dir, "dir.fmd", b"leftover metadata");

        assert_eq!(names(&base(&dir)), vec!["ALPHA.TXT"]);
    }

    #[test]
    fn test_a_rescan_does_not_duplicate_entries() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");

        for _ in 0..3 {
            assert_eq!(base(&dir).len(), 1);
        }
    }

    #[test]
    fn test_counters_and_flags_survive_a_reopen() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");

        let mut first = base(&dir);
        first[0].dl_counter = 7;
        first[0].set_free(true);
        first.save().unwrap();
        drop(first);

        let second = base(&dir);
        assert_eq!(second[0].dl_counter, 7);
        assert!(second[0].is_free());
    }

    #[test]
    fn test_metadata_is_derived_once_and_then_read_back() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        let derived = base.read_metadata(&path).unwrap();
        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].get_type(), MetadataType::Hash);

        let reread = base.read_metadata(&path).unwrap();
        assert_eq!(reread.len(), 1);
        assert_eq!(reread[0].data, derived[0].data);
    }

    #[test]
    fn test_writing_metadata_replaces_what_was_there() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        base.write_metadata(&path, vec![MetadataHeader::new(MetadataType::Uploader, b"sysop".to_vec())])
            .unwrap();
        base.write_metadata(&path, vec![MetadataHeader::new(MetadataType::Uploader, b"someone".to_vec())])
            .unwrap();

        let stored = base.read_metadata(&path).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].data, b"someone");
    }

    #[test]
    fn test_an_authored_description_survives_a_scan() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        base.set_description(&path, "written by hand").unwrap();

        assert_eq!(base.description(&path).unwrap().as_deref(), Some("written by hand"));
        assert!(base.is_authored(&path).unwrap());
        // The scan still ran, so the derived hash is there next to the description.
        assert!(base.read_metadata(&path).unwrap().iter().any(|m| m.get_type() == MetadataType::Hash));
    }

    #[test]
    fn test_a_plain_rescan_keeps_an_authored_description() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        base.set_description(&path, "written by hand").unwrap();
        base.rescan(&path, false).unwrap();
        assert_eq!(base.description(&path).unwrap().as_deref(), Some("written by hand"));

        base.rescan(&path, true).unwrap();
        assert_eq!(base.description(&path).unwrap(), None);
    }

    #[test]
    fn test_a_removed_file_is_forgotten() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        base.remove_file(&path).unwrap();
        assert!(base.is_empty());
    }

    #[test]
    fn test_metadata_survives_a_reopen() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut first = base(&dir);
        first
            .write_metadata(&path, vec![MetadataHeader::new(MetadataType::FileID, b"a description".to_vec())])
            .unwrap();
        drop(first);

        let stored = base(&dir).read_metadata(&path).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].data, b"a description");
    }

    #[test]
    fn test_a_file_cannot_be_added_twice() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        let path = dir.path().join("ALPHA.TXT");

        let mut base = base(&dir);
        assert!(base.add_file(&path, Vec::new()).is_err());
    }

    #[test]
    fn test_an_entry_whose_file_vanished_keeps_its_counter() {
        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");

        let mut first = base(&dir);
        first[0].dl_counter = 3;
        first.save().unwrap();
        drop(first);
        fs::remove_file(dir.path().join("ALPHA.TXT")).unwrap();

        let second = base(&dir);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].dl_counter, 3);
    }

    /// Two nodes hold the same area open at once, which the old format could not survive.
    #[test]
    fn test_two_open_bases_can_both_write() {        let dir = TempDir::new().unwrap();
        write(&dir, "ALPHA.TXT", b"alpha");
        write(&dir, "BETA.TXT", b"beta");

        let mut node1 = base(&dir);
        let mut node2 = base(&dir);

        node1
            .write_metadata(&dir.path().join("ALPHA.TXT"), vec![MetadataHeader::new(MetadataType::Uploader, b"node1".to_vec())])
            .unwrap();
        node2
            .write_metadata(&dir.path().join("BETA.TXT"), vec![MetadataHeader::new(MetadataType::Uploader, b"node2".to_vec())])
            .unwrap();

        assert_eq!(node2.read_metadata(&dir.path().join("ALPHA.TXT")).unwrap()[0].data, b"node1");
        assert_eq!(node1.read_metadata(&dir.path().join("BETA.TXT")).unwrap()[0].data, b"node2");
    }
}


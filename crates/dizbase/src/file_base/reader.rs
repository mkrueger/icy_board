use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, params};

use super::{FileBase, file_header::FileAttributes};

pub const MAX_PAGE_SIZE: usize = 100;
pub const MAX_SCAN_ROWS: usize = 1024;
pub const MAX_DESCRIPTION_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub description_truncated: bool,
    pub size: i64,
    pub date: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct FilePage {
    pub entries: Vec<FileEntry>,
    pub next_after: i64,
    pub has_more: bool,
}

pub fn valid_file_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 1024 && name != "." && name != ".." && !name.contains(['/', '\\', ':']) && !name.chars().any(char::is_control)
}

pub fn resolve_file(dir: &Path, metadata_path: &Path, name: &str) -> crate::Result<Option<PathBuf>> {
    if !valid_file_name(name) {
        return Err("invalid file name".into());
    }
    let connection = Connection::open_with_flags(FileBase::database_path(metadata_path), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_secs(1))?;
    let mut statement = connection.prepare("SELECT name, attribute FROM files WHERE name = ?1 COLLATE NOCASE LIMIT 2")?;
    let mut rows = statement.query([name])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let stored_name: String = row.get(0)?;
    let attributes = FileAttributes::from_bits_truncate(row.get::<_, i64>(1)? as u8);
    if rows.next()?.is_some() {
        return Err("ambiguous file name in filebase".into());
    }
    let path = dir.join(&stored_name);
    if !valid_file_name(&stored_name)
        || attributes.contains(FileAttributes::DELETED)
        || !std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file())
    {
        return Ok(None);
    }
    Ok(Some(path))
}

pub fn read_page(dir: &Path, metadata_path: &Path, query: &str, after: i64, limit: usize) -> crate::Result<FilePage> {
    if after < 0 || !(1..=MAX_PAGE_SIZE).contains(&limit) || query.len() > 1024 {
        return Err("invalid filebase page arguments".into());
    }
    let connection = Connection::open_with_flags(FileBase::database_path(metadata_path), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_secs(1))?;
    let mut statement = connection.prepare(
        "SELECT id, substr(name, 1, 1025), date, size, attribute,
            (SELECT substr(data, 1, ?3 + 1) FROM metadata WHERE file_id = files.id AND type = 4 ORDER BY rowid LIMIT 1)
         FROM files WHERE id > ?1 ORDER BY id LIMIT ?2",
    )?;
    let mut rows = statement.query(params![after, (MAX_SCAN_ROWS + 1) as i64, MAX_DESCRIPTION_BYTES as i64])?;
    let query = query.to_lowercase();
    let mut page = FilePage {
        next_after: after,
        ..Default::default()
    };
    let mut scanned = 0;
    while let Some(row) = rows.next()? {
        if page.entries.len() == limit || scanned == MAX_SCAN_ROWS {
            page.has_more = true;
            break;
        }
        scanned += 1;
        page.next_after = row.get(0)?;
        let name: String = row.get(1)?;
        let attributes = FileAttributes::from_bits_truncate(row.get::<_, i64>(4)? as u8);
        if attributes.contains(FileAttributes::DELETED)
            || name.is_empty()
            || name.len() > 1024
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', ':'])
            || !std::fs::symlink_metadata(dir.join(&name)).is_ok_and(|metadata| metadata.file_type().is_file())
        {
            continue;
        }
        let mut data = row.get::<_, Option<Vec<u8>>>(5)?.unwrap_or_default();
        let description_truncated = data.len() > MAX_DESCRIPTION_BYTES;
        if description_truncated {
            data.truncate(MAX_DESCRIPTION_BYTES);
            while std::str::from_utf8(&data).is_err_and(|error| error.error_len().is_none()) {
                data.pop();
            }
        }
        let description = String::from_utf8(data)?;
        if !name.to_lowercase().contains(&query) && !description.to_lowercase().contains(&query) {
            continue;
        }
        let size: i64 = row.get(3)?;
        if size < 0 {
            return Err("negative filebase size".into());
        }
        let date = DateTime::from_timestamp_millis(row.get(2)?).ok_or("invalid filebase date")?;
        page.entries.push(FileEntry {
            id: page.next_after,
            name,
            description,
            description_truncated,
            size,
            date,
        });
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_file_resolution_is_area_scoped_and_read_only() {
        let root = tempfile::tempdir().unwrap();
        for area in ["first", "second"] {
            let dir = root.path().join(area);
            std::fs::create_dir(&dir).unwrap();
            let metadata = dir.join("dir");
            std::fs::write(dir.join("shared.txt"), area).unwrap();
            std::fs::write(dir.join("deleted.txt"), b"deleted").unwrap();
            let mut base = FileBase::open(&dir, &metadata).unwrap();
            for header in base.iter_mut() {
                if header.name() == "deleted.txt" {
                    header.set_deleted(true);
                }
            }
            base.save().unwrap();
            drop(base);
            std::fs::write(dir.join("unpublished.txt"), b"unpublished").unwrap();
            assert_eq!(resolve_file(&dir, &metadata, "SHARED.TXT").unwrap(), Some(dir.join("shared.txt")));
            for name in ["*.txt", "shared.?xt", "deleted.txt", "unpublished.txt", "absent"] {
                assert_eq!(resolve_file(&dir, &metadata, name).unwrap(), None, "{name}");
            }
            for name in [
                "",
                ".",
                "..",
                "../shared.txt",
                "first/shared.txt",
                "first\\shared.txt",
                "C:shared.txt",
                "bad\nname",
            ] {
                assert!(resolve_file(&dir, &metadata, name).is_err(), "{name}");
            }
            std::fs::remove_file(dir.join("shared.txt")).unwrap();
            assert_eq!(resolve_file(&dir, &metadata, "shared.txt").unwrap(), None);
            assert!(resolve_file(&dir, &dir.join("missing"), "shared.txt").is_err());
            assert!(!FileBase::database_path(dir.join("missing")).exists());
        }
    }

    #[test]
    fn pages_read_published_metadata_without_scanning_or_writing() {
        let root = tempfile::tempdir().unwrap();
        let metadata = root.path().join("dir");
        for name in ["one.zip", "two.zip", "deleted.zip"] {
            std::fs::write(root.path().join(name), b"data").unwrap();
        }
        let large = std::fs::File::create(root.path().join("large.zip")).unwrap();
        large.set_len(2_147_483_648).unwrap();
        let mut base = FileBase::open(root.path(), &metadata).unwrap();
        base.set_description(&root.path().join("one.zip"), "Gr\u{fc}\u{df}e\nTools").unwrap();
        base.set_description(&root.path().join("two.zip"), "Tools").unwrap();
        for header in base.iter_mut() {
            if header.name() == "deleted.zip" {
                header.set_deleted(true);
            }
        }
        base.save().unwrap();
        drop(base);
        std::fs::write(root.path().join("unpublished.zip"), b"not indexed").unwrap();
        let first = read_page(root.path(), &metadata, "TOOLS", 0, 1).unwrap();
        assert_eq!(first.entries.len(), 1);
        assert!(first.has_more);
        let second = read_page(root.path(), &metadata, "TOOLS", first.next_after, 1).unwrap();
        assert_eq!(second.entries.len(), 1);
        assert_ne!(first.entries[0].id, second.entries[0].id);
        let all = read_page(root.path(), &metadata, "", 0, 100).unwrap();
        assert_eq!(all.entries.len(), 3);
        assert!(!all.has_more);
        assert_eq!(all.entries.iter().find(|entry| entry.name == "large.zip").unwrap().size, 2_147_483_648);
        assert_eq!(read_page(root.path(), &metadata, "GR\u{dc}SSE", 0, 10).unwrap().entries.len(), 0);
        let unicode = read_page(root.path(), &metadata, "GR\u{dc}\u{df}E", 0, 10).unwrap();
        assert_eq!(unicode.entries[0].description, "Gr\u{fc}\u{df}e\nTools");
        let connection = Connection::open_with_flags(FileBase::database_path(&metadata), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(connection.query_row("SELECT count(*) FROM files", [], |row| row.get::<_, i64>(0)).unwrap(), 4);
    }

    #[test]
    fn missing_index_is_not_created_and_invalid_pages_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let metadata = root.path().join("dir");
        assert!(read_page(root.path(), &metadata, "", 0, 10).is_err());
        assert!(!FileBase::database_path(&metadata).exists());
        for (after, limit) in [(-1, 10), (0, 0), (0, MAX_PAGE_SIZE + 1)] {
            assert!(read_page(root.path(), &metadata, "", after, limit).is_err());
        }
    }

    #[test]
    fn scan_budget_advances_past_nonmatches_and_truncates_utf8_safely() {
        let root = tempfile::tempdir().unwrap();
        let metadata = root.path().join("dir");
        drop(FileBase::open(root.path(), &metadata).unwrap());
        let mut connection = Connection::open(FileBase::database_path(&metadata)).unwrap();
        let transaction = connection.transaction().unwrap();
        for id in 1..=MAX_SCAN_ROWS + 1 {
            transaction
                .execute(
                    "INSERT INTO files(id, name, date, size) VALUES (?1, ?2, 0, 1)",
                    params![id as i64, format!("file-{id}.zip")],
                )
                .unwrap();
        }
        transaction.commit().unwrap();
        let last_id = (MAX_SCAN_ROWS + 1) as i64;
        std::fs::write(root.path().join(format!("file-{last_id}.zip")), b"x").unwrap();
        let description = format!("{}\u{fc}tail", "x".repeat(MAX_DESCRIPTION_BYTES - 1));
        connection
            .execute(
                "INSERT INTO metadata(file_id, type, data) VALUES (?1, 4, ?2)",
                params![last_id, description.as_bytes()],
            )
            .unwrap();
        let first = read_page(root.path(), &metadata, "", 0, 15).unwrap();
        assert!(first.entries.is_empty());
        assert!(first.has_more);
        assert_eq!(first.next_after, MAX_SCAN_ROWS as i64);
        let last = read_page(root.path(), &metadata, "", first.next_after, 15).unwrap();
        assert_eq!(last.entries.len(), 1);
        assert!(!last.has_more);
        assert_eq!(last.next_after, last_id);
        assert_eq!(last.entries[0].description.len(), MAX_DESCRIPTION_BYTES - 1);
        assert!(last.entries[0].description_truncated);
        assert!(read_page(root.path(), &metadata, "tail", first.next_after, 15).unwrap().entries.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn links_and_unsafe_index_names_are_not_exposed() {
        let root = tempfile::tempdir().unwrap();
        let metadata = root.path().join("dir");
        std::fs::write(root.path().join("real.zip"), b"x").unwrap();
        drop(FileBase::open(root.path(), &metadata).unwrap());
        std::os::unix::fs::symlink(root.path().join("real.zip"), root.path().join("link.zip")).unwrap();
        let connection = Connection::open(FileBase::database_path(&metadata)).unwrap();
        for name in ["link.zip", "../real.zip", "sub/real.zip", "..\\real.zip"] {
            connection.execute("INSERT INTO files(name, date, size) VALUES (?1, 0, 1)", [name]).unwrap();
        }
        let page = read_page(root.path(), &metadata, "", 0, 100).unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].name, "real.zip");
        assert_eq!(resolve_file(root.path(), &metadata, "link.zip").unwrap(), None);
    }
}

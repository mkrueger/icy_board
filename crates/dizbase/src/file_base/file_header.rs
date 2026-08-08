use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct FileHeader {
    /// Row id in the `files` table, 0 until the row is inserted.
    pub id: i64,
    /// File name (up to 255 bytes long)
    pub name: String,
    /// unix utc timestamp
    pub date: DateTime<Utc>,
    /// size of the file in bytes
    pub size: u64,
    /// # times of download
    pub dl_counter: u64,
    /// Attributes of the file
    pub attribute: FileAttributes,
}

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    pub struct FileAttributes : u8 {
        const NONE = 0b0000_0000;

        /// File is free - no dl costs
        const FREE = 0b0000_0001;
        /// File has tags to scan for
        const HAS_TAGS = 0b0000_0010;
        /// PW protected - it's in the metadata
        const PASSWORD = 0b0001_0000;
        /// File can't be deleted
        const LOCKED = 0b0100_0000;
        /// File is deleted
        const DELETED = 0b1000_0000;
    }
}

impl FileHeader {
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn date(&self) -> DateTime<Utc> {
        self.date
    }
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn dl_counter(&self) -> u64 {
        self.dl_counter
    }

    pub fn is_free(&self) -> bool {
        self.attribute.contains(FileAttributes::FREE)
    }
    pub fn needs_password(&self) -> bool {
        self.attribute.contains(FileAttributes::PASSWORD)
    }
    pub fn is_locked(&self) -> bool {
        self.attribute.contains(FileAttributes::LOCKED)
    }
    pub fn is_deleted(&self) -> bool {
        self.attribute.contains(FileAttributes::DELETED)
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.attribute |= FileAttributes::FREE;
        } else {
            self.attribute &= !FileAttributes::FREE;
        }
    }

    pub fn set_password(&mut self, password: bool) {
        if password {
            self.attribute |= FileAttributes::PASSWORD;
        } else {
            self.attribute &= !FileAttributes::PASSWORD;
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        if locked {
            self.attribute |= FileAttributes::LOCKED;
        } else {
            self.attribute &= !FileAttributes::LOCKED;
        }
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        if deleted {
            self.attribute |= FileAttributes::DELETED;
        } else {
            self.attribute &= !FileAttributes::DELETED;
        }
    }
}

use std::{
    collections::HashSet,
    io::{Read, Write},
};

use thiserror::Error;

pub const MAGIC: &[u8; 8] = b"ICYPPE\0\0";
pub const HEADER_SIZE: usize = 64;
pub const DIRECTORY_ENTRY_SIZE: usize = 48;
pub const REQUIRED: u16 = 1;
pub const SECTION_KINDS: [[u8; 4]; 9] = [*b"CONS", *b"TYPE", *b"VARS", *b"ROUT", *b"IMPT", *b"CODE", *b"META", *b"DBUG", *b"IDEN"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    None,
    Zstd,
}

#[derive(Clone, Copy, Debug)]
pub struct LoadLimits {
    pub file_bytes: u64,
    pub section_bytes: u64,
    pub decoded_bytes: u64,
    pub sections: u32,
    pub zstd_window_log: u32,
}

impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            file_bytes: 64 * 1024 * 1024,
            section_bytes: 32 * 1024 * 1024,
            decoded_bytes: 64 * 1024 * 1024,
            sections: 64,
            zstd_window_log: 25,
        }
    }
}

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Invalid PPE 400 container: {0}")]
    Invalid(&'static str),
    #[error("Unsupported PPE 400 requirement: {0}")]
    Unsupported(String),
    #[error("PPE 400 resource limit exceeded: {0}")]
    Limit(&'static str),
    #[error("PPE 400 compression error: {0}")]
    Compression(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub kind: [u8; 4],
    pub schema: u16,
    pub flags: u16,
    pub entries: u32,
    pub data: Vec<u8>,
}

impl Section {
    pub fn new(kind: [u8; 4], entries: u32, data: Vec<u8>) -> Self {
        Self {
            kind,
            schema: 1,
            flags: REQUIRED,
            entries,
            data,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub runtime: u32,
    pub entry_routine: u32,
    pub sections: Vec<Section>,
}

fn integer<const SIZE: usize>(data: &[u8], offset: usize) -> Result<[u8; SIZE], ContainerError> {
    data.get(offset..offset.checked_add(SIZE).ok_or(ContainerError::Invalid("offset overflow"))?)
        .ok_or(ContainerError::Invalid("truncated structure"))?
        .try_into()
        .map_err(|_| ContainerError::Invalid("integer width"))
}

impl Container {
    pub fn encode(&self, compression: Compression, limits: &LoadLimits) -> Result<Vec<u8>, ContainerError> {
        let count = u32::try_from(self.sections.len()).map_err(|_| ContainerError::Limit("section count"))?;
        if count > limits.sections {
            return Err(ContainerError::Limit("section count"));
        }
        let directory_bytes = self
            .sections
            .len()
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(ContainerError::Limit("directory"))?;
        let start = HEADER_SIZE.checked_add(directory_bytes).ok_or(ContainerError::Limit("directory"))?;
        let mut output = vec![0; start];
        output[..8].copy_from_slice(MAGIC);
        output[8..10].copy_from_slice(&1u16.to_le_bytes());
        output[12..16].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        output[16..20].copy_from_slice(&self.runtime.to_le_bytes());
        output[20..24].copy_from_slice(&1u32.to_le_bytes());
        output[24..32].copy_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
        output[32..36].copy_from_slice(&count.to_le_bytes());
        output[36..40].copy_from_slice(&(DIRECTORY_ENTRY_SIZE as u32).to_le_bytes());
        output[40..44].copy_from_slice(&self.entry_routine.to_le_bytes());
        let mut total = 0u64;
        let mut kinds = HashSet::new();
        for (index, section) in self.sections.iter().enumerate() {
            if !kinds.insert(section.kind) || section.flags & !REQUIRED != 0 {
                return Err(ContainerError::Invalid("duplicate section or unknown flags"));
            }
            let decoded = section.data.len() as u64;
            total = total.checked_add(decoded).ok_or(ContainerError::Limit("decoded bytes"))?;
            if decoded > limits.section_bytes || total > limits.decoded_bytes {
                return Err(ContainerError::Limit("decoded bytes"));
            }
            let mut packed = Vec::new();
            if compression == Compression::Zstd && !section.data.is_empty() {
                let mut encoder = zstd::stream::Encoder::new(Vec::new(), 3)?;
                encoder.include_checksum(true)?;
                encoder.include_contentsize(true)?;
                encoder.set_pledged_src_size(Some(decoded))?;
                encoder.window_log(limits.zstd_window_log)?;
                encoder.write_all(&section.data)?;
                packed = encoder.finish()?;
            }
            let compressed = !packed.is_empty() && packed.len() < section.data.len();
            let stored = if compressed { packed.as_slice() } else { section.data.as_slice() };
            let offset = output.len() as u64;
            if offset.checked_add(stored.len() as u64).is_none_or(|size| size > limits.file_bytes) {
                return Err(ContainerError::Limit("file bytes"));
            }
            let directory = &mut output[HEADER_SIZE + index * DIRECTORY_ENTRY_SIZE..][..DIRECTORY_ENTRY_SIZE];
            directory[..4].copy_from_slice(&section.kind);
            directory[4..6].copy_from_slice(&section.schema.to_le_bytes());
            directory[6..8].copy_from_slice(&section.flags.to_le_bytes());
            directory[8..12].copy_from_slice(&u32::from(compressed).to_le_bytes());
            directory[16..24].copy_from_slice(&offset.to_le_bytes());
            directory[24..32].copy_from_slice(&(stored.len() as u64).to_le_bytes());
            directory[32..40].copy_from_slice(&decoded.to_le_bytes());
            directory[40..44].copy_from_slice(&section.entries.to_le_bytes());
            output.extend_from_slice(stored);
        }
        let size = output.len() as u64;
        output[48..56].copy_from_slice(&size.to_le_bytes());
        Ok(output)
    }

    pub fn decode(data: &[u8], limits: &LoadLimits) -> Result<Self, ContainerError> {
        if data.len() as u64 > limits.file_bytes {
            return Err(ContainerError::Limit("file bytes"));
        }
        if data.len() < HEADER_SIZE || !data.starts_with(MAGIC) {
            return Err(ContainerError::Invalid("header"));
        }
        if u16::from_le_bytes(integer(data, 8)?) != 1 || u32::from_le_bytes(integer(data, 20)?) != 1 {
            return Err(ContainerError::Unsupported("container or bytecode version".into()));
        }
        let runtime = u32::from_le_bytes(integer(data, 16)?);
        if runtime != 400 {
            return Err(ContainerError::Unsupported(format!("runtime {runtime}")));
        }
        if u32::from_le_bytes(integer(data, 12)?) != HEADER_SIZE as u32
            || u64::from_le_bytes(integer(data, 24)?) != HEADER_SIZE as u64
            || u32::from_le_bytes(integer(data, 36)?) != DIRECTORY_ENTRY_SIZE as u32
            || u64::from_le_bytes(integer(data, 48)?) != data.len() as u64
            || data[44..48].iter().chain(&data[56..64]).any(|value| *value != 0)
        {
            return Err(ContainerError::Invalid("header fields"));
        }
        let count = u32::from_le_bytes(integer(data, 32)?);
        if count > limits.sections {
            return Err(ContainerError::Limit("section count"));
        }
        let end = (count as usize)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .and_then(|size| size.checked_add(HEADER_SIZE))
            .ok_or(ContainerError::Invalid("directory overflow"))?;
        let directory = data.get(HEADER_SIZE..end).ok_or(ContainerError::Invalid("truncated directory"))?;
        let mut kinds = HashSet::new();
        let mut ranges = Vec::new();
        let mut descriptors = Vec::new();
        let mut total = 0u64;
        for entry in directory.chunks_exact(DIRECTORY_ENTRY_SIZE) {
            let kind = integer::<4>(entry, 0)?;
            let schema = u16::from_le_bytes(integer(entry, 4)?);
            let flags = u16::from_le_bytes(integer(entry, 6)?);
            let compression = u32::from_le_bytes(integer(entry, 8)?);
            let offset = u64::from_le_bytes(integer(entry, 16)?);
            let stored = u64::from_le_bytes(integer(entry, 24)?);
            let decoded = u64::from_le_bytes(integer(entry, 32)?);
            let entries = u32::from_le_bytes(integer(entry, 40)?);
            if !kinds.insert(kind) || flags & !REQUIRED != 0 || entry[12..16].iter().chain(&entry[44..48]).any(|value| *value != 0) {
                return Err(ContainerError::Invalid("section flags, reserved bytes or duplicate"));
            }
            let range_end = offset.checked_add(stored).ok_or(ContainerError::Invalid("section overflow"))?;
            if offset < end as u64 || range_end > data.len() as u64 {
                return Err(ContainerError::Invalid("section outside payload"));
            }
            if stored > 0 {
                ranges.push((offset, range_end));
            }
            // Compression is a container-version-1 mechanic, so an unknown code is
            // malformed rather than a section this loader may skip.
            if compression > 1 {
                return Err(ContainerError::Invalid("section compression"));
            }
            if (!SECTION_KINDS.contains(&kind) || schema != 1) && flags & REQUIRED != 0 {
                return Err(ContainerError::Unsupported(format!(
                    "section {:?}, schema {schema}",
                    String::from_utf8_lossy(&kind)
                )));
            }
            total = total.checked_add(decoded).ok_or(ContainerError::Limit("decoded bytes"))?;
            if decoded > limits.section_bytes || total > limits.decoded_bytes {
                return Err(ContainerError::Limit("decoded bytes"));
            }
            descriptors.push((kind, schema, flags, compression, offset, range_end, decoded, entries));
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(ContainerError::Invalid("overlapping sections"));
        }
        let mut sections = Vec::new();
        for (kind, schema, flags, compression, offset, end, decoded, entries) in descriptors {
            let stored = &data[usize::try_from(offset).map_err(|_| ContainerError::Limit("host address space"))?
                ..usize::try_from(end).map_err(|_| ContainerError::Limit("host address space"))?];
            let bytes = if compression == 0 {
                if stored.len() as u64 != decoded {
                    return Err(ContainerError::Invalid("uncompressed size"));
                }
                stored.to_vec()
            } else {
                if !stored.starts_with(&[0x28, 0xb5, 0x2f, 0xfd])
                    || stored.get(4).is_none_or(|descriptor| descriptor & 4 == 0)
                    || zstd::zstd_safe::get_frame_content_size(stored).ok().flatten() != Some(decoded)
                    || zstd::zstd_safe::find_frame_compressed_size(stored).ok() != Some(stored.len())
                    || zstd::zstd_safe::get_dict_id_from_frame(stored).is_some()
                {
                    return Err(ContainerError::Invalid("Zstd frame profile"));
                }
                let mut decoder = zstd::stream::read::Decoder::with_buffer(stored)?.single_frame();
                decoder.window_log_max(limits.zstd_window_log)?;
                let mut bytes = Vec::new();
                decoder
                    .take(decoded.checked_add(1).ok_or(ContainerError::Limit("decoded bytes"))?)
                    .read_to_end(&mut bytes)?;
                if bytes.len() as u64 != decoded {
                    return Err(ContainerError::Invalid("Zstd decoded size"));
                }
                bytes
            };
            sections.push(Section {
                kind,
                schema,
                flags,
                entries,
                data: bytes,
            });
        }
        Ok(Self {
            runtime,
            entry_routine: u32::from_le_bytes(integer(data, 40)?),
            sections,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Container {
        Container {
            runtime: 400,
            entry_routine: 1,
            sections: vec![Section::new(*b"CODE", 1024, vec![0; 8192]), Section::new(*b"CONS", 1, b"hello".to_vec())],
        }
    }

    #[test]
    fn compressed_and_uncompressed_roundtrip() {
        let limits = LoadLimits::default();
        let program = sample();
        let plain = program.encode(Compression::None, &limits).unwrap();
        let packed = program.encode(Compression::Zstd, &limits).unwrap();
        assert!(packed.len() < plain.len());
        assert_eq!(program, Container::decode(&plain, &limits).unwrap());
        assert_eq!(program, Container::decode(&packed, &limits).unwrap());
        assert_eq!(plain, program.encode(Compression::None, &limits).unwrap());
        assert_eq!(packed, program.encode(Compression::Zstd, &limits).unwrap());
    }

    #[test]
    fn unknown_optional_is_preserved_but_required_is_rejected() {
        let limits = LoadLimits::default();
        let mut program = sample();
        let mut extension = Section::new(*b"FUTR", 1, vec![42]);
        extension.flags = 0;
        program.sections.push(extension);
        assert_eq!(
            Container::decode(&program.encode(Compression::None, &limits).unwrap(), &limits).unwrap(),
            program
        );
        program.sections.last_mut().unwrap().flags = REQUIRED;
        assert!(matches!(
            Container::decode(&program.encode(Compression::None, &limits).unwrap(), &limits),
            Err(ContainerError::Unsupported(_))
        ));
    }

    #[test]
    fn rejects_truncation_overflow_overlap_and_limits() {
        let limits = LoadLimits::default();
        let bytes = sample().encode(Compression::None, &limits).unwrap();
        for length in 0..bytes.len() {
            assert!(Container::decode(&bytes[..length], &limits).is_err());
        }
        for offset in [0, u32::MAX as u64 + 1, u64::MAX] {
            let mut broken = bytes.clone();
            broken[80..88].copy_from_slice(&offset.to_le_bytes());
            assert!(Container::decode(&broken, &limits).is_err());
        }
        let mut overlap = bytes.clone();
        overlap[128..136].copy_from_slice(&bytes[80..88]);
        assert!(Container::decode(&overlap, &limits).is_err());
        assert!(Container::decode(&bytes, &LoadLimits { section_bytes: 8191, ..limits }).is_err());
        assert!(Container::decode(&bytes, &LoadLimits { section_bytes: 8192, ..limits }).is_ok());
        assert!(Container::decode(&bytes, &LoadLimits { decoded_bytes: 8196, ..limits }).is_err());
    }

    #[test]
    fn rejects_corrupt_compressed_payload() {
        let limits = LoadLimits::default();
        let mut bytes = sample().encode(Compression::Zstd, &limits).unwrap();
        let offset = u64::from_le_bytes(integer(&bytes, 80).unwrap()) as usize;
        let size = u64::from_le_bytes(integer(&bytes, 88).unwrap()) as usize;
        bytes[offset + size - 1] ^= 1;
        assert!(Container::decode(&bytes, &limits).is_err());
    }
}

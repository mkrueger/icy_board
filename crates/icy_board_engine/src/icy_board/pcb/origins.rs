//! Reads `ORIGINS.DAT`, which holds the origin lines `PCBoard` hands out per
//! conference.

use std::path::Path;

use crate::Res;

/// `PCBoard` writes its version as a 16 bit word in front of the records.
const HEADER_LEN: usize = 2;

/// `struct ORIGIN`: `char origin[70]; char ranges[70]; char reserved[10];`
const RECORD_LEN: usize = 150;
const FIELD_LEN: usize = 70;

/// An origin line and the conferences it speaks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbOrigin {
    pub origin: String,
    pub conferences: Vec<u16>,
}

impl PcbOrigin {
    /// Reads the file, skipping the records `PCBoard` blanked out on delete.
    pub fn import_pcboard(path: &Path) -> Res<Vec<PcbOrigin>> {
        let data = std::fs::read(path)?;
        let mut result = Vec::new();
        let Some(records) = data.get(HEADER_LEN..) else {
            return Ok(result);
        };
        for record in records.chunks_exact(RECORD_LEN) {
            if record.iter().all(|byte| *byte == 0) {
                continue;
            }
            let origin = field(&record[..FIELD_LEN]);
            if origin.is_empty() {
                continue;
            }
            result.push(PcbOrigin {
                origin,
                conferences: parse_ranges(&field(&record[FIELD_LEN..FIELD_LEN * 2])),
            });
        }
        Ok(result)
    }
}

fn field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// Turns `1-200 203 205 250-100` into the conferences it names. `PCBoard` reads a
/// range in whichever direction it was written.
fn parse_ranges(ranges: &str) -> Vec<u16> {
    let mut result = Vec::new();
    for part in ranges.split_whitespace() {
        let (low, high) = match part.split_once('-') {
            Some((low, high)) => (low, high),
            None => (part, part),
        };
        let (Ok(low), Ok(high)) = (low.trim().parse::<u16>(), high.trim().parse::<u16>()) else {
            continue;
        };
        let (low, high) = if low <= high { (low, high) } else { (high, low) };
        result.extend(low..=high);
    }
    result.sort_unstable();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(origin: &str, ranges: &str) -> Vec<u8> {
        let mut record = vec![0u8; RECORD_LEN];
        record[..origin.len()].copy_from_slice(origin.as_bytes());
        record[FIELD_LEN..FIELD_LEN + ranges.len()].copy_from_slice(ranges.as_bytes());
        record
    }

    #[test]
    fn test_a_range_written_backwards_names_the_same_conferences() {
        assert_eq!(parse_ranges("5-3"), vec![3, 4, 5]);
    }

    #[test]
    fn test_single_conferences_and_ranges_can_share_a_line() {
        assert_eq!(parse_ranges("1-3 7 9-10"), vec![1, 2, 3, 7, 9, 10]);
    }

    #[test]
    fn test_a_blanked_out_record_is_not_an_origin() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ORIGINS.DAT");
        let mut data = vec![3u8, 0];
        data.extend(vec![0u8; RECORD_LEN]);
        data.extend(record("A board (2)", "4"));
        std::fs::write(&path, data).unwrap();

        let origins = PcbOrigin::import_pcboard(&path).unwrap();

        assert_eq!(
            origins,
            vec![PcbOrigin {
                origin: "A board (2)".to_string(),
                conferences: vec![4]
            }]
        );
    }

    #[test]
    fn test_a_file_holding_nothing_but_the_version_has_no_origins() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ORIGINS.DAT");
        std::fs::write(&path, [3u8, 0]).unwrap();

        assert!(PcbOrigin::import_pcboard(&path).unwrap().is_empty());
    }
}

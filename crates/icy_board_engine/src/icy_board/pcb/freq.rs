//! Reads the three FREQ files `PCBoard` keeps beside its fido configuration:
//! the paths a request may be answered from, the magic names, and the nodes
//! that get nothing.

use std::path::Path;

use crate::Res;

/// `PCBoard` writes its version as a 16 bit word in front of the records.
const HEADER_LEN: usize = 2;

/// `MAXFLEN`, the DOS path length every one of these records is built on.
const MAXFLEN: usize = 66;
const PASSWORD_LEN: usize = 10;
const RESERVED_LEN: usize = 10;
const MAGIC_NAME_LEN: usize = 20;

/// `struct NFREQ_PATH`: `char Path[66]; char Password[10]; char reserved[10];`
const PATH_RECORD_LEN: usize = MAXFLEN + PASSWORD_LEN + RESERVED_LEN;

/// `struct NFREQ_MAGIC`: `char MagicName[20]; char RealName[66];
/// char Password[10]; char reserved[10];`
const MAGIC_RECORD_LEN: usize = MAGIC_NAME_LEN + MAXFLEN + PASSWORD_LEN + RESERVED_LEN;

/// `struct NADDRESS`: four 16 bit numbers, three flags, a link kind, a range
/// and the reserved tail.
const ADDRESS_RECORD_LEN: usize = 8 + 3 + 1 + 70 + RESERVED_LEN;

/// `PCBFIDO.CFG` holds a version word, then `DIRECTORIES`, `EMSI_DATA`,
/// `FREQ_INFO` and `ARCHIVERS`, each written where the one before it ends.
const DIRECTORIES_LEN: usize = 9 * MAXFLEN;
const EMSI_DATA_LEN: usize = 60 + 30 + 30 + 50 + 10 + 50;
const FREQ_INFO_OFFSET: usize = HEADER_LEN + DIRECTORIES_LEN + EMSI_DATA_LEN;
const FREQ_INFO_LEN: usize = 15;

/// The limits of the `Fido FREQ restrictions` screen. `PCBoard` names the two
/// size fields bytes but counts them in kilobytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PcbFreqInfo {
    pub session_minutes: u16,
    pub daily_minutes: u16,
    pub session_kbytes: u32,
    pub daily_kbytes: u32,
    pub allowed_nodes: char,
    pub min_baud: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbFreqPath {
    pub path: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbFreqMagic {
    pub name: String,
    pub file: String,
    pub password: String,
}

/// A denied node, as zone, net, node and point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PcbFreqDeny {
    pub zone: u16,
    pub net: u16,
    pub node: u16,
    pub point: u16,
}

fn records(path: &Path, length: usize) -> Res<Vec<Vec<u8>>> {
    let data = std::fs::read(path)?;
    let Some(body) = data.get(HEADER_LEN..) else {
        return Ok(Vec::new());
    };
    Ok(body
        .chunks_exact(length)
        .filter(|record| record.iter().any(|byte| *byte != 0))
        .map(<[u8]>::to_vec)
        .collect())
}

fn field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn long(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

impl PcbFreqInfo {
    pub fn import_pcboard(path: &Path) -> Res<Option<PcbFreqInfo>> {
        let data = std::fs::read(path)?;
        let Some(record) = data.get(FREQ_INFO_OFFSET..FREQ_INFO_OFFSET + FREQ_INFO_LEN) else {
            return Ok(None);
        };
        Ok(Some(PcbFreqInfo {
            session_minutes: word(record, 0),
            daily_minutes: word(record, 2),
            session_kbytes: long(record, 4),
            daily_kbytes: long(record, 8),
            allowed_nodes: (record[12] as char).to_ascii_uppercase(),
            min_baud: word(record, 13),
        }))
    }
}

impl PcbFreqPath {
    pub fn import_pcboard(path: &Path) -> Res<Vec<PcbFreqPath>> {
        Ok(records(path, PATH_RECORD_LEN)?
            .into_iter()
            .map(|record| PcbFreqPath {
                path: field(&record[..MAXFLEN]),
                password: field(&record[MAXFLEN..MAXFLEN + PASSWORD_LEN]),
            })
            .filter(|entry| !entry.path.is_empty())
            .collect())
    }
}

impl PcbFreqMagic {
    pub fn import_pcboard(path: &Path) -> Res<Vec<PcbFreqMagic>> {
        Ok(records(path, MAGIC_RECORD_LEN)?
            .into_iter()
            .map(|record| PcbFreqMagic {
                name: field(&record[..MAGIC_NAME_LEN]),
                file: field(&record[MAGIC_NAME_LEN..MAGIC_NAME_LEN + MAXFLEN]),
                password: field(&record[MAGIC_NAME_LEN + MAXFLEN..MAGIC_NAME_LEN + MAXFLEN + PASSWORD_LEN]),
            })
            .filter(|entry| !entry.name.is_empty())
            .collect())
    }
}

impl PcbFreqDeny {
    pub fn import_pcboard(path: &Path) -> Res<Vec<PcbFreqDeny>> {
        Ok(records(path, ADDRESS_RECORD_LEN)?
            .into_iter()
            .map(|record| PcbFreqDeny {
                zone: word(&record, 0),
                net: word(&record, 2),
                node: word(&record, 4),
                point: word(&record, 6),
            })
            .filter(|entry| entry.zone != 0 || entry.net != 0 || entry.node != 0)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(directory: &Path, name: &str, records: &[Vec<u8>]) -> std::path::PathBuf {
        let path = directory.join(name);
        let mut data = vec![3u8, 0];
        for record in records {
            data.extend(record);
        }
        std::fs::write(&path, data).unwrap();
        path
    }

    fn padded(parts: &[(&str, usize)]) -> Vec<u8> {
        let mut record = Vec::new();
        for (text, length) in parts {
            let mut field = vec![0u8; *length];
            field[..text.len()].copy_from_slice(text.as_bytes());
            record.extend(field);
        }
        record
    }

    #[test]
    fn test_a_path_record_carries_its_directory_and_password() {
        let directory = tempfile::tempdir().unwrap();
        let path = file(
            directory.path(),
            "FREQPATH.DAT",
            &[padded(&[("C:\\PCB\\GEN", MAXFLEN), ("secret", PASSWORD_LEN), ("", RESERVED_LEN)])],
        );

        let paths = PcbFreqPath::import_pcboard(&path).unwrap();

        assert_eq!(
            paths,
            vec![PcbFreqPath {
                path: "C:\\PCB\\GEN".to_string(),
                password: "secret".to_string()
            }]
        );
    }

    #[test]
    fn test_a_magic_record_names_the_file_behind_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = file(
            directory.path(),
            "MAGICNAM.DAT",
            &[padded(&[
                ("FILES", MAGIC_NAME_LEN),
                ("C:\\PCB\\LIST.ZIP", MAXFLEN),
                ("", PASSWORD_LEN),
                ("", RESERVED_LEN),
            ])],
        );

        let magic = PcbFreqMagic::import_pcboard(&path).unwrap();

        assert_eq!(magic[0].name, "FILES");
        assert_eq!(magic[0].file, "C:\\PCB\\LIST.ZIP");
        assert!(magic[0].password.is_empty());
    }

    #[test]
    fn test_a_denied_node_is_read_as_an_address() {
        let directory = tempfile::tempdir().unwrap();
        let mut record = vec![0u8; ADDRESS_RECORD_LEN];
        record[..8].copy_from_slice(&[21, 0, 1, 0, 2, 0, 0, 0]);
        let path = file(directory.path(), "FREQDENY.DAT", &[record]);

        let denied = PcbFreqDeny::import_pcboard(&path).unwrap();

        assert_eq!(
            denied,
            vec![PcbFreqDeny {
                zone: 21,
                net: 1,
                node: 2,
                point: 0
            }]
        );
    }

    #[test]
    fn test_a_blanked_out_record_is_skipped() {
        let directory = tempfile::tempdir().unwrap();
        let path = file(
            directory.path(),
            "FREQPATH.DAT",
            &[
                vec![0u8; PATH_RECORD_LEN],
                padded(&[("C:\\PCB\\GEN", MAXFLEN), ("", PASSWORD_LEN), ("", RESERVED_LEN)]),
            ],
        );

        assert_eq!(PcbFreqPath::import_pcboard(&path).unwrap().len(), 1);
    }

    #[test]
    fn test_a_file_holding_nothing_but_the_version_has_no_records() {
        let directory = tempfile::tempdir().unwrap();
        let path = file(directory.path(), "FREQPATH.DAT", &[]);

        assert!(PcbFreqPath::import_pcboard(&path).unwrap().is_empty());
    }

    #[test]
    fn test_the_restrictions_are_read_from_where_pcbfido_keeps_them() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("PCBFIDO.CFG");
        let mut data = vec![0u8; FREQ_INFO_OFFSET];
        data[0] = 3;
        data.extend([30, 0]);
        data.extend([60, 0]);
        data.extend(1024u32.to_le_bytes());
        data.extend(4096u32.to_le_bytes());
        data.push(b'L');
        data.extend([0x60, 0x09]);
        std::fs::write(&path, data).unwrap();

        let info = PcbFreqInfo::import_pcboard(&path).unwrap().unwrap();

        assert_eq!(info.session_kbytes, 1024);
        assert_eq!(info.daily_kbytes, 4096);
        assert_eq!(info.allowed_nodes, 'L');
        assert_eq!(info.min_baud, 2400);
    }

    #[test]
    fn test_a_truncated_configuration_has_no_restrictions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("PCBFIDO.CFG");
        std::fs::write(&path, [3u8, 0]).unwrap();

        assert_eq!(PcbFreqInfo::import_pcboard(&path).unwrap(), None);
    }
}

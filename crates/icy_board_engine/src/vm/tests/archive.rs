use std::io::{Cursor, Write};

fn zip_fixture() -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("folder/first.txt", options).unwrap();
    zip.write_all(b"first").unwrap();
    zip.start_file("../second.txt", options).unwrap();
    zip.write_all(b"second").unwrap();
    zip.add_directory("empty/", options).unwrap();
    zip.finish().unwrap().into_inner()
}

fn tar_fixture() -> Vec<u8> {
    let mut archive = Vec::new();
    for (name, kind, target, contents) in [
        ("first.txt", b'0', "", &b"first"[..]),
        ("second.txt", b'0', "", &b"second"[..]),
        ("link", b'2', "../outside", &b""[..]),
        ("hard", b'1', "first.txt", &b""[..]),
        ("directory/", b'5', "", &b""[..]),
        ("pipe", b'6', "", &b""[..]),
    ] {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(format!("{:011o}\0", contents.len()).as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].fill(b' ');
        header[156] = kind;
        header[157..157 + target.len()].copy_from_slice(target.as_bytes());
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum: u32 = header.iter().map(|b| *b as u32).sum();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
        archive.extend_from_slice(&header);
        archive.extend_from_slice(contents);
        archive.resize(archive.len().div_ceil(512) * 512, 0);
    }
    archive.resize(archive.len() + 1024, 0);
    archive
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    assert!(bytes.len() < 65536);
    let mut gzip = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3, 1];
    let len = bytes.len() as u16;
    gzip.extend_from_slice(&len.to_le_bytes());
    gzip.extend_from_slice(&(!len).to_le_bytes());
    gzip.extend_from_slice(bytes);
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32 & 0u32.wrapping_sub(crc & 1));
        }
    }

    gzip.extend_from_slice(&(!crc).to_le_bytes());
    gzip.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    gzip
}

fn arc_fixture() -> Vec<u8> {
    let contents = b"ARC content";
    let mut crc = 0u16;
    for byte in contents {
        crc ^= *byte as u16;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xa001u16 & 0u16.wrapping_sub(crc & 1));
        }
    }
    let mut arc = vec![0x1a, 2];
    let mut name = [0u8; 13];
    name[..9].copy_from_slice(b"ENTRY.TXT");
    arc.extend_from_slice(&name);
    arc.extend_from_slice(&(contents.len() as u32).to_le_bytes());
    arc.extend_from_slice(&0u32.to_le_bytes());
    arc.extend_from_slice(&crc.to_le_bytes());
    arc.extend_from_slice(&(contents.len() as u32).to_le_bytes());
    arc.extend_from_slice(contents);
    arc.extend_from_slice(&[0x1a, 0]);
    arc
}

fn ace_header(output: &mut Vec<u8>, data: &[u8]) {
    output.extend_from_slice(&(!crc32fast::hash(data) as u16).to_le_bytes());
    output.extend_from_slice(&(data.len() as u16).to_le_bytes());
    output.extend_from_slice(data);
}

fn solid_ace_fixture(stored_prefix: bool) -> Vec<u8> {
    fn bits(output: &mut Vec<bool>, value: u32, width: u32) {
        output.extend((0..width).rev().map(|shift| value & (1 << shift) != 0));
    }
    fn tree(output: &mut Vec<bool>, symbol: u32) {
        bits(output, symbol, 9);
        bits(output, 0, 4);
        bits(output, 2, 4);
        for width in [1, 1, 0] {
            bits(output, width, 3);
        }
        for position in 0..=symbol {
            bits(output, u32::from(position != symbol), 1);
        }
    }
    fn payload(symbol: u32, size: usize) -> Vec<u8> {
        let mut output = Vec::new();
        tree(&mut output, symbol);
        tree(&mut output, 0);
        bits(&mut output, if symbol == 65 { size as u32 } else { 1 }, 15);
        if symbol == 260 {
            bits(&mut output, 0, 1);
            bits(&mut output, 0, 1);
        } else {
            for _ in 0..size {
                bits(&mut output, 0, 1);
            }
        }
        output
            .chunks(32)
            .flat_map(|chunk| {
                let word = chunk
                    .iter()
                    .enumerate()
                    .fold(0u32, |word, (index, bit)| word | (u32::from(*bit) << (31 - index)));
                word.to_le_bytes()
            })
            .collect()
    }
    let mut archive = Vec::new();
    let mut main = vec![0, 0, 0x80];
    main.extend_from_slice(b"**ACE**");
    main.extend_from_slice(&[10, 10, 0, 0]);
    main.resize(26, 0);
    ace_header(&mut archive, &main);
    // The second and third LZ77 members copy from the previous member's dictionary.
    for (name, contents, symbol) in [("first.txt", &b"A"[..], 65), ("second.txt", &b"AA"[..], 260), ("third.txt", &b"AA"[..], 260)] {
        let stored = stored_prefix && symbol == 65;
        let compressed = if stored { contents.to_vec() } else { payload(symbol, contents.len()) };
        let mut file = vec![1, 1, 0];
        file.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        file.extend_from_slice(&(contents.len() as u32).to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(!crc32fast::hash(contents)).to_le_bytes());
        file.extend_from_slice(&[u8::from(!stored), 0, 0, 0, 0, 0]);
        file.extend_from_slice(&(name.len() as u16).to_le_bytes());
        file.extend_from_slice(name.as_bytes());
        ace_header(&mut archive, &file);
        archive.extend_from_slice(&compressed);
    }
    archive.extend_from_slice(&[0; 4]);
    archive
}

fn nonsolid_ace_fixture(flags: u16) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut main = vec![0];
    main.extend_from_slice(&flags.to_le_bytes());
    main.extend_from_slice(b"**ACE**");
    main.extend_from_slice(&[10, 10, 0, 0]);
    main.resize(26, 0);
    ace_header(&mut archive, &main);
    for (name, contents) in [("large.bin", vec![0; 16 * 1024 * 1024 + 1]), ("small.txt", b"ok".to_vec())] {
        let mut file = vec![1, 1, 0];
        file.extend_from_slice(&(contents.len() as u32).to_le_bytes());
        file.extend_from_slice(&(contents.len() as u32).to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&(!crc32fast::hash(&contents)).to_le_bytes());
        file.extend_from_slice(&[0; 6]);
        file.extend_from_slice(&(name.len() as u16).to_le_bytes());
        file.extend_from_slice(name.as_bytes());
        ace_header(&mut archive, &file);
        archive.extend_from_slice(&contents);
    }
    archive.extend_from_slice(&[0; 4]);
    archive
}

#[test]
fn archive_zip_metadata_consumption_eof_and_atomic_extract() {
    let root = tempfile::tempdir_in(".").unwrap();
    let output_path = root.path().canonicalize().unwrap().join("extracted.txt");
    let fixture = zip_fixture();
    let output = super::run_ppl_with_files(
        &format!(
            r#"
ARCHIVEREADER reader = Archive.Open("input.bin")
PRINTLN reader.Valid, ":", reader.Format, ":", Error.Last().OK
PRINTLN reader.Next()
ARCHIVEENTRY first = reader.Entry
PRINTLN first.Valid, ":", first.Index, ":", first.Name, ":", first.FileName, ":", first.Size
PRINTLN first.Kind = ArchiveEntryKind.File, ":", first.IsDirectory, ":", first.IsLink, ":", first.HasLinkTarget
PRINTLN reader.Next(), ":", reader.Entry.Index
PRINTLN reader.Extract("{}"), ":", Error.Last().OK
PRINTLN reader.ReadText() = "", ":", Error.Last().Code = ErrCode.Invalid
PRINTLN reader.Next(), ":", reader.Entry.IsDirectory
PRINTLN reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Invalid
PRINTLN reader.Next(), ":", reader.Entry.Valid, ":", Error.Last().OK
PRINTLN reader.Next(), ":", Error.Last().OK
PRINTLN reader.Close(), ":", reader.Close(), ":", reader.Valid
PRINTLN first.Valid, ":", first.Name, ":", first.Size
EXIT
"#,
            output_path.display()
        ),
        &[("input.bin", &fixture)],
    );
    assert_eq!(
        output,
        "1:ZIP:1\n1\n1:0:folder/first.txt:first.txt:5\n1:0:0:0\n1:1\n1:1\n1:1\n1:1\n0:1\n0:0:1\n0:1\n1:1:0\n1:folder/first.txt:5\n"
    );
    assert_eq!(std::fs::read(&output_path).unwrap(), b"second");
    assert!(!root.path().join("second.txt").exists());
}

#[test]
fn archive_tar_and_tgz_preserve_sequential_position_and_links() {
    let tar = tar_fixture();
    let gz = gzip(&tar);
    for (name, bytes) in [("input.tar", tar.as_slice()), ("input.tgz", gz.as_slice())] {
        let output = super::run_ppl_with_files(
            &format!(
                r#"
ARCHIVEREADER reader = Archive.Open("{name}")
PRINTLN reader.Valid, ":", reader.Next()
PRINTLN reader.Next(), ":", reader.ReadText()
PRINTLN reader.Next(), ":", reader.Entry.Kind = ArchiveEntryKind.SymbolicLink
PRINTLN reader.Entry.IsLink, ":", reader.Entry.HasLinkTarget, ":", reader.Entry.LinkTarget
PRINTLN reader.ReadText() = "", ":", Error.Last().Code = ErrCode.Unsupported
PRINTLN reader.Next(), ":", reader.Entry.Kind = ArchiveEntryKind.HardLink
PRINTLN reader.Next(), ":", reader.Entry.IsDirectory
PRINTLN reader.Next(), ":", reader.Entry.Kind = ArchiveEntryKind.Special
PRINTLN reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Unsupported
PRINTLN reader.Next(), ":", Error.Last().OK
EXIT
"#
            ),
            &[(name, bytes)],
        );
        assert_eq!(output, "1:1\n1:second\n1:1\n1:1:../outside\n1:1\n1:1\n1:1\n1:1\n0:1\n0:1\n", "{name}");
    }
}

#[test]
fn archive_options_snapshot_and_cumulative_rewind_budgets() {
    let fixture = zip_fixture();
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEOPTIONS options = Archive.Options()
PRINTLN options.Format = "", ":", options.MaxEntryBytes, ":", options.MaxTotalBytes, ":", options.MaxEntries
options.MaxEntryBytes = 6
options.MaxTotalBytes = 10
options.MaxEntries = 3
ARCHIVEREADER reader = Archive.Open("input.zip", options)
options.MaxEntryBytes = 1
PRINTLN reader.Next(), ":", reader.ReadText()
PRINTLN reader.Rewind(), ":", reader.Next(), ":", reader.ReadText()
PRINTLN reader.Rewind(), ":", reader.Next(), ":", reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit, ":", reader.Valid
options.MaxEntryBytes = 0
PRINTLN Error.Last().Code = ErrCode.Invalid, ":", options.MaxEntryBytes
options.MaxEntries = -1
PRINTLN Error.Last().Code = ErrCode.Invalid, ":", options.MaxEntries
options.MaxTotalBytes = 268435457
PRINTLN Error.Last().Code = ErrCode.Limit
options.MaxEntries = 100001
PRINTLN Error.Last().Code = ErrCode.Limit
options.Format = "not-a-format"
PRINTLN Error.Last().Code = ErrCode.Invalid
options.Format = "zIp"
options.MaxEntryBytes = 6
options.MaxTotalBytes = 64
options.MaxEntries = 1
reader = Archive.Open("input.zip", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", Error.Last().Code = ErrCode.Limit
reader = Archive.Open("input.zip", options)
PRINTLN reader.Next(), ":", reader.Rewind(), ":", reader.Next(), ":", Error.Last().Code = ErrCode.Limit
options.MaxEntries = 10
options.MaxTotalBytes = 10
reader = Archive.Open("input.zip", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
PRINTLN reader.Rewind(), ":", reader.Next(), ":", reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit
options.MaxEntryBytes = 4
reader = Archive.Open("input.zip", options)
PRINTLN reader.Next(), ":", reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit
EXIT
"#,
        &[("input.zip", &fixture)],
    );
    assert_eq!(
        output,
        "1:16777216:67108864:10000\n1:first\n1:1:first\n1:1:0:1:0\n1:1\n1:3\n1\n1\n1\n1:0:1\n1:1:0:1\n1:1:second:1\n1:1:0:1\n1:0:1\n"
    );
}

#[test]
fn archive_empty_defaults_errors_and_readonly_contract() {
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader
ARCHIVEENTRY entry
PRINTLN reader.Valid, ":", reader.Format = "", ":", reader.Entry.Valid
PRINTLN entry.Valid, ":", entry.Index, ":", entry.Size, ":", entry.CompressedSize
PRINTLN entry.Name = "", ":", entry.LinkTarget = "", ":", entry.Kind = ArchiveEntryKind.Unknown
PRINTLN entry.Date.IsEmpty, ":", entry.Time.IsEmpty, ":", entry.IsEncrypted
PRINTLN reader.Next(), ":", Error.Last().Code = ErrCode.Invalid
PRINTLN reader.Close(), ":", Error.Last().OK
reader = Archive.Open("missing.zip")
PRINTLN reader.Valid, ":", Error.Last().Code = ErrCode.Unavailable, ":", Error.Last().Kind = ErrKind.File
reader = Archive.Open("broken.zip")
PRINTLN reader.Valid, ":", Error.Last().Code = ErrCode.Format
PRINTLN Archive.Formats().Len() > 10
EXIT
"#,
        &[("broken.zip", b"not an archive")],
    );
    assert_eq!(output, "0:1:0\n0:0:0:0\n1:1:1\n1:1:0\n0:1\n1:1\n0:1:1\n0:1\n1\n");
    for source in ["ARCHIVEENTRY entry\nentry.Name = \"changed\"", "ARCHIVEREADER reader\nreader.Valid = TRUE"] {
        assert!(!super::compile_errors(source).is_empty());
    }
}

#[test]
fn archive_explicit_destinations_reject_source_and_preserve_existing_files() {
    let root = tempfile::tempdir_in(".").unwrap();
    let root = root.path().canonicalize().unwrap();
    let source = root.join("input.zip");
    let destination = root.join("existing.txt");
    let fixture = zip_fixture();
    std::fs::write(&source, &fixture).unwrap();
    std::fs::write(&destination, b"old").unwrap();
    let output = super::run_ppl(&format!(
        r#"
ARCHIVEREADER reader = Archive.Open("{}")
PRINTLN reader.Next()
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().Code = ErrCode.Denied
PRINTLN reader.Extract("{}"), ":", Error.Last().Code = ErrCode.Invalid
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().OK
EXIT
"#,
        source.display(),
        source.display(),
        destination.display(),
        destination.display()
    ));
    assert_eq!(output, "1\n0:1\n0:1\n1:1\n");
    assert_eq!(std::fs::read(source).unwrap(), fixture);
    assert_eq!(std::fs::read(destination).unwrap(), b"first");
}

#[test]
fn archive_stored_arc_and_gzip_single_file() {
    for (name, bytes, expected) in [("input.arc", arc_fixture(), "ARC content"), ("input.gz", gzip(b"gzip content"), "gzip content")] {
        let output = super::run_ppl_with_files(
            &format!(
                r#"
ARCHIVEREADER reader = Archive.Open("{name}")
PRINTLN reader.Valid, ":", reader.Next(), ":", reader.Entry.Kind = ArchiveEntryKind.File
PRINTLN reader.ReadText()
PRINTLN reader.Next(), ":", Error.Last().OK
EXIT
"#
            ),
            &[(name, &bytes)],
        );
        assert_eq!(output, format!("1:1:1\n{expected}\n0:1\n"));
    }
}

#[test]
fn archive_encrypted_zip_password_errors_and_success() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .with_aes_encryption(zip::AesMode::Aes256, "Correct");
    zip.start_file("secret.txt", options).unwrap();
    zip.write_all(b"secret content").unwrap();
    let fixture = zip.finish().unwrap().into_inner();
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("encrypted.zip")
PRINTLN reader.Next(), ":", reader.Next(), ":", Error.Last().OK
reader = Archive.Open("encrypted.zip")
PRINTLN reader.Next(), ":", reader.Entry.IsEncrypted
PRINTLN reader.ReadText() = "", ":", Error.Last().Code = ErrCode.Denied, ":", reader.Valid
ARCHIVEOPTIONS options = Archive.Options()
options.Password = "WrongSecretMixed"
reader = Archive.Open("encrypted.zip", options)
PRINTLN reader.Next()
PRINTLN reader.ReadText() = "", ":", Error.Last().Code = ErrCode.Denied, ":", Error.Last().Message
options.Password = "Correct"
reader = Archive.Open("encrypted.zip", options)
PRINTLN reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
STRING credential = "Correct"
options.Password = credential
PRINTLN options.Password
reader = Archive.Open("encrypted.zip", options)
PRINTLN reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
EXIT
"#,
        &[("encrypted.zip", &fixture)],
    );
    assert_eq!(
        output,
        "1:0:1\n1:1\n1:1:0\n1\n1:1:Archive password is missing or incorrect\n1:secret content:1\n******\n1:secret content:1\n"
    );
    let secret = crate::executable::VariableValue::new_password(icy_board_ppl::password::Password::Protected("MixedSecret".to_string()));
    assert!(!format!("{secret:?}").contains("MixedSecret"));
    assert_eq!(secret.as_string(), "******");
}

#[test]
fn archive_text_utf8_eof_cp437_and_unbounded_payload() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("utf8", &b"\xef\xbb\xbfUTF8\x1a\x95tail"[..]),
        ("cp437", &b"Gr\x81\xe1e"[..]),
        ("nul", &b"hello\0\x95tail"[..]),
        ("large", &vec![b'x'; 1024][..]),
    ] {
        zip.start_file(name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let fixture = zip.finish().unwrap().into_inner();
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("text.zip")
PRINTLN reader.Next(), ":", reader.ReadText() = "UTF8"
PRINTLN reader.Next(), ":", reader.ReadText() = "Grüße"
PRINTLN reader.Next(), ":", reader.ReadText() = "hello"
PRINTLN reader.Next(), ":", reader.ReadText().Len()
EXIT
"#,
        &[("text.zip", &fixture)],
    );
    assert_eq!(output, "1:1\n1:1\n1:1\n1:1024\n");
}

#[cfg(unix)]
#[test]
fn archive_rejects_symlink_paths_and_source_hardlinks() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir_in(".").unwrap();
    let root = root.path().canonicalize().unwrap();
    let source = root.join("source.zip");
    let alias = root.join("alias.zip");
    let hardlink = root.join("hardlink.zip");
    let output = root.join("existing.txt");
    let directory_link = root.join("linked");
    std::fs::write(&source, zip_fixture()).unwrap();
    std::fs::write(&output, b"old").unwrap();
    symlink(&source, &alias).unwrap();
    std::fs::hard_link(&source, &hardlink).unwrap();
    symlink(&root, &directory_link).unwrap();
    let symlink_output = root.join("symlink.txt");
    symlink(&output, &symlink_output).unwrap();
    let actual = super::run_ppl(&format!(
        r#"
ARCHIVEREADER reader = Archive.Open("{}")
PRINTLN reader.Valid, ":", Error.Last().Code = ErrCode.Denied
reader = Archive.Open("{}")
PRINTLN reader.Next()
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().Code = ErrCode.Denied
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().Code = ErrCode.Denied
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().Code = ErrCode.Denied
PRINTLN reader.ReadText()
reader = Archive.Open("{}")
PRINTLN reader.Valid, ":", reader.Next(), ":", reader.Extract("{}"), ":", Error.Last().OK
EXIT
"#,
        alias.display(),
        source.display(),
        hardlink.display(),
        symlink_output.display(),
        directory_link.join("source.zip").display(),
        directory_link.join("source.zip").display(),
        directory_link.join("new.txt").display()
    ));
    assert_eq!(actual, "0:1\n1\n0:1\n0:1\n0:1\nfirst\n1:1:1:1\n");
    assert_eq!(std::fs::read(output).unwrap(), b"old");
    assert_eq!(std::fs::read(&source).unwrap(), zip_fixture());
    assert_eq!(std::fs::read(root.join("new.txt")).unwrap(), b"first");
    assert!(std::fs::symlink_metadata(&directory_link).unwrap().file_type().is_symlink());
}

#[test]
fn archive_corrupt_payload_invalidates_reader_but_keeps_entry_snapshot() {
    let mut fixture = arc_fixture();
    let offset = fixture.len() - 3;
    fixture[offset] ^= 1;
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("corrupt.arc")
PRINTLN reader.Next()
ARCHIVEENTRY saved = reader.Entry
PRINTLN reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Format, ":", reader.Valid
PRINTLN saved.Valid, ":", saved.Name, ":", saved.Size
PRINTLN reader.Next(), ":", Error.Last().Code = ErrCode.Invalid
PRINTLN reader.Rewind(), ":", Error.Last().Code = ErrCode.Invalid
EXIT
"#,
        &[("corrupt.arc", &fixture)],
    );
    assert_eq!(output, "1\n0:1:0\n1:ENTRY.TXT:11\n0:1\n0:1\n");
}

#[test]
fn archive_rewind_keeps_original_open_file_after_path_replacement() {
    let root = tempfile::tempdir_in(".").unwrap();
    let source = root.path().canonicalize().unwrap().join("source.zip");
    std::fs::write(&source, zip_fixture()).unwrap();
    let output = super::run_ppl(&format!(
        r#"
ARCHIVEREADER reader = Archive.Open("{}")
PRINTLN reader.Next(), ":", reader.ReadText()
ZIPWRITER replacement = Zip.Create("{}", TRUE)
PRINTLN replacement.Finish()
PRINTLN reader.Rewind(), ":", reader.Next(), ":", reader.ReadText()
PRINTLN reader.Extract("{}", TRUE), ":", Error.Last().Code = ErrCode.Denied
PRINTLN reader.Close()
reader = Archive.Open("{}")
PRINTLN reader.Valid, ":", reader.Next(), ":", Error.Last().OK
EXIT
"#,
        source.display(),
        source.display(),
        source.display(),
        source.display()
    ));
    assert_eq!(output, "1:first\n1\n1:1:first\n0:1\n1\n1:0:1\n");
}

#[test]
fn archive_metadata_walk_skips_members_above_default_read_limit() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("large.bin", options).unwrap();
    zip.write_all(&vec![0; 16 * 1024 * 1024 + 1]).unwrap();
    zip.start_file("after.txt", options).unwrap();
    zip.write_all(b"after").unwrap();
    let fixture = zip.finish().unwrap().into_inner();
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("large.zip")
PRINTLN reader.Next(), ":", reader.Entry.Size, ":", Error.Last().OK
PRINTLN reader.Next(), ":", reader.Entry.Name, ":", reader.ReadText()
PRINTLN reader.Next(), ":", Error.Last().OK
PRINTLN reader.Rewind(), ":", reader.Next()
PRINTLN reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit, ":", reader.Valid
EXIT
"#,
        &[("large.zip", &fixture)],
    );
    assert_eq!(output, "1:16777217:1\n1:after.txt:after\n0:1\n1:1\n0:1:0\n");
}

#[test]
fn archive_solid_ace_replays_skipped_history_and_charges_decode_budget() {
    use unarc_rs::unified::ArchiveFormat;
    let fixture = solid_ace_fixture(false);
    let mut raw = ArchiveFormat::Ace.open(Cursor::new(&fixture)).unwrap();
    let first = raw.next_entry().unwrap().unwrap();
    raw.skip(&first).unwrap();
    let second = raw.next_entry().unwrap().unwrap();
    assert!(raw.read(&second).is_err(), "the fixture must require the skipped dictionary");
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("solid.ace")
PRINTLN reader.Valid, ":", reader.Next(), ":", reader.Next()
PRINTLN reader.ReadText(), ":", Error.Last().OK
PRINTLN reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
PRINTLN reader.Next(), ":", Error.Last().OK
ARCHIVEOPTIONS options = Archive.Options()
options.MaxTotalBytes = 4
reader = Archive.Open("solid.ace", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", reader.ReadText()
PRINTLN reader.Next(), ":", reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit
options.MaxTotalBytes = 5
reader = Archive.Open("solid.ace", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", reader.ReadText()
PRINTLN reader.Rewind(), ":", reader.Next(), ":", reader.Next()
PRINTLN reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit
EXIT
"#,
        &[("solid.ace", &fixture)],
    );
    assert_eq!(output, "1:1:1\nAA:1\n1:AA:1\n0:1\n1:1:AA\n1:0:1\n1:1:AA\n1:1:1\n0:1\n");
}

#[test]
fn archive_nonsolid_ace_skips_without_spending_decode_budget() {
    for flags in [0, 0x10] {
        let fixture = nonsolid_ace_fixture(flags);
        let output = super::run_ppl_with_files(
            r#"
ARCHIVEOPTIONS options = Archive.Options()
options.MaxTotalBytes = 2
ARCHIVEREADER reader = Archive.Open("nonsolid.ace", options)
PRINTLN reader.Valid, ":", reader.Next(), ":", reader.Entry.Size, ":", reader.Next(), ":", reader.ReadText()
PRINTLN reader.Next(), ":", Error.Last().OK
EXIT
"#,
            &[("nonsolid.ace", &fixture)],
        );
        assert_eq!(output, "1:1:16777217:1:ok\n0:1\n", "main flags {flags:#x}");
    }
}

#[test]
fn archive_solid_ace_replays_stored_prefix_before_lz77_read() {
    let fixture = solid_ace_fixture(true);
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEREADER reader = Archive.Open("stored-solid.ace")
PRINTLN reader.Next(), ":", reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
PRINTLN reader.Next(), ":", reader.ReadText(), ":", Error.Last().OK
ARCHIVEOPTIONS options = Archive.Options()
options.MaxTotalBytes = 2
reader = Archive.Open("stored-solid.ace", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", reader.ReadBytes().Len(), ":", Error.Last().Code = ErrCode.Limit
EXIT
"#,
        &[("stored-solid.ace", &fixture)],
    );
    assert_eq!(output, "1:1:AA:1\n1:AA:1\n1:1:0:1\n");
}

#[test]
fn archive_entry_limit_allows_clean_eof_at_exact_boundary() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for name in ["first", "second"] {
        zip.start_file(name, options).unwrap();
        zip.write_all(b"x").unwrap();
    }
    let fixture = zip.finish().unwrap().into_inner();
    let output = super::run_ppl_with_files(
        r#"
ARCHIVEOPTIONS options = Archive.Options()
options.MaxEntries = 2
ARCHIVEREADER reader = Archive.Open("two.zip", options)
PRINTLN reader.Next(), ":", reader.Next()
PRINTLN reader.Next(), ":", Error.Last().OK, ":", reader.Valid
PRINTLN reader.Next(), ":", Error.Last().OK
PRINTLN reader.Rewind(), ":", reader.Next(), ":", Error.Last().Code = ErrCode.Limit
options.MaxEntries = 1
reader = Archive.Open("two.zip", options)
PRINTLN reader.Next(), ":", reader.Next(), ":", Error.Last().Code = ErrCode.Limit
EXIT
"#,
        &[("two.zip", &fixture)],
    );
    assert_eq!(output, "1:1\n0:1:1\n0:1\n1:0:1\n1:0:1\n");
}

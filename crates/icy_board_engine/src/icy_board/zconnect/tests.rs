use super::super::{ZconnectArea, ZconnectLink};
use super::*;

// Independent wire fixture, deliberately not produced by encode_local().
const FIXTURE: &[u8] = b"abs: alice@origin.example (Alice)\r\nBET: Independent fixture\r\nMID: fixture-1@origin.example\r\nEDA: 20260906123456S+2\r\nEMP: /PUBLIC/TEST\r\nROT: hub.example!origin.example\r\nBEZ: older@origin.example\r\nBEZ: parent@origin.example\r\nX-OPAQUE: preserve me\r\nLEN: 7\r\n\r\nHello\r\n";

fn config() -> ZconnectConfig {
    ZconnectConfig {
        enabled: true,
        local_system: "leaf.example".into(),
        links: vec![ZconnectLink {
            id: "HUB".into(),
            areas: vec![ZconnectArea {
                remote_board: "/PUBLIC/TEST".into(),
                local_area: "test-area".into(),
                read_only: false,
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn packet(root: &Path, name: &str, data: &[u8]) -> PathBuf {
    let path = root.join(name);
    atomic_write(&path, &make_archive(data).unwrap()).unwrap();
    path
}

fn custom_message(headers: &str, body: &[u8], id: &str) -> Vec<u8> {
    let mut bytes = format!(
        "ABS: bob@origin.example\r\nBET: Custom\r\nMID: {id}@origin.example\r\nEDA: 20260906000000W-9:30\r\nROT: origin.example\r\n{headers}LEN: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn local_message(root: &Path, path: &str, flags: u32, text: &str) {
    let path = root.join(path);
    let mut base = if path.with_extension("jhr").exists() {
        JamMessageBase::open(path).unwrap()
    } else {
        JamMessageBase::create(path).unwrap()
    };
    base.write_message(
        &JamMessage::default()
            .with_from(BString::from("Alice Smith"))
            .with_subject(BString::from("Local subject"))
            .with_date_time(parse_date("20260906000000W+0").unwrap())
            .with_reply_id(BString::from("parent@origin.example"))
            .with_attributes(flags)
            .with_text(BString::from(text)),
    )
    .unwrap();
    base.write_jhr_header().unwrap();
    base.sync().unwrap();
}

fn write_identity_message(base: &mut JamMessageBase, id: Option<&[u8]>, reply: Option<&[u8]>, reply_to: u32, flags: u32) -> u32 {
    let mut message = JamMessage::default()
        .with_from(BString::from("Alice"))
        .with_subject(BString::from("Identity test"))
        .with_date_time(parse_date("20260906000000W+0").unwrap())
        .with_attributes(flags)
        .with_reply_to(reply_to)
        .with_text(BString::from("Local identity\r"));
    if let Some(id) = id {
        message = message.with_msg_id(BString::from(id.to_vec()));
    }
    if let Some(reply) = reply {
        message = message.with_reply_id(BString::from(reply.to_vec()));
    }
    base.write_message(&message).unwrap();
    base.write_jhr_header().unwrap();
    base.sync().unwrap();
    base.highest_message_number()
}

#[test]
fn identity_mapping_preserves_valid_ids_and_hashes_exact_foreign_bytes() {
    let c = config();
    for raw in ["Mixed.Case@Origin.example", "<Mixed.Case@Origin.example>"] {
        assert_eq!(mapped_id(&c, raw.as_bytes()), "Mixed.Case@Origin.example");
    }
    let foreign: &[&[u8]] = &[
        b"2:240/100 abcd",
        b"2:240/100 ABCD",
        b" 2:240/100 abcd",
        b"2:240/100 abcd ",
        b"<2:240/100 abcd>",
        b"<<Mixed.Case@Origin.example>>",
        b" <Mixed.Case@Origin.example>",
        b"<Mixed.Case@Origin.example> ",
        b"one@origin.example two@origin.example",
        b"bad\r\nEMP: secret@private.example",
        b"bad\0id",
        b"bad\xffid",
        b"bad\xfeid",
        b"bad\xef\xbf\xbdid",
        b"",
    ];
    let mut mapped = HashSet::new();
    for raw in foreign {
        let id = mapped_id(&c, raw);
        assert_eq!(id, format!("jam-sha256-{}@leaf.example", digest(raw)));
        assert!(valid_mid(&id));
        assert_eq!(mapped_id(&c, id.as_bytes()), id, "mapped IDs must survive subsequent imports/exports");
        assert!(mapped.insert(id), "distinct exact byte strings must not be normalized together");
    }
}

#[test]
fn foreign_jam_ids_and_references_round_trip_stably_across_retries_and_links() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config();
    let mut other = c.links[0].clone();
    other.id = "OTHER".into();
    other.areas[0].remote_board = "/SECOND".into();
    c.links.push(other);
    let ids: [&[u8]; 5] = [
        b"2:240/100 abcd",
        b"2:240/100 ef01",
        b"<RFC.Parent@origin.example>",
        b"unsafe\xff\r\nEMP: secret@private.example",
        b"unchanged@origin.example",
    ];
    let area = root.path().join("test-area");
    let mut base = JamMessageBase::create(&area).unwrap();
    for (index, id) in ids.iter().enumerate() {
        let reply = (index > 0).then(|| ids[index - 1]);
        write_identity_message(&mut base, Some(id), reply, 0, attributes::MSG_LOCAL);
    }
    let first = scan(&c, root.path(), "HUB").unwrap();
    assert_eq!(first.messages, ids.len());
    let path = first.packet.unwrap();
    let archive = read_limited(&path, MAX_ARCHIVE).unwrap();
    let messages = parse_archive(&archive).unwrap();
    assert_eq!(scan(&c, root.path(), "hub").unwrap().packet.unwrap(), path);
    assert_eq!(read_limited(&path, MAX_ARCHIVE).unwrap(), archive);
    let other_path = scan(&c, root.path(), "OTHER").unwrap().packet.unwrap();
    let other_messages = parse_archive(&read_limited(&other_path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(other_messages.len(), ids.len());
    for (index, raw) in ids.iter().enumerate() {
        let message = &messages[index];
        assert_eq!(message.text("MID").unwrap(), mapped_id(&c, raw));
        assert_eq!(message.value("MID"), other_messages[index].value("MID"));
        assert_eq!(message.value("BEZ"), other_messages[index].value("BEZ"));
        assert_eq!(message.values("EMP").collect::<Vec<_>>(), vec![b"/PUBLIC/TEST".as_slice()]);
        assert_eq!(other_messages[index].text("EMP").unwrap(), "/SECOND");
        assert_eq!(message.body, b"Local identity\r\n");
        let source = base.read_header(base.lowest_message_number() + index as u32).unwrap();
        assert_eq!(field(&source, SubfieldType::MsgID), Some(*raw), "export must not rewrite original JAM IDs");
        if index > 0 {
            assert_eq!(message.value("BEZ"), messages[index - 1].value("MID"));
            assert_eq!(field(&source, SubfieldType::ReplyID), Some(ids[index - 1]));
        } else {
            assert!(!message.has("BEZ"));
        }
    }
    let mut receiver = config();
    receiver.local_system = "receiver.example".into();
    receiver.links[0].areas[0].local_area = "received".into();
    receiver.outbound = "receiver-spool".into();
    assert_eq!(toss(&receiver, root.path(), "HUB", &path).unwrap().imported, ids.len());
    assert_eq!(toss(&receiver, root.path(), "HUB", &path).unwrap().duplicates, ids.len());
    let received = JamMessageBase::open(root.path().join("received")).unwrap();
    for (index, message) in messages.iter().enumerate() {
        let header = received.read_header(received.lowest_message_number() + index as u32).unwrap();
        assert_eq!(field(&header, SubfieldType::MsgID), message.value("MID"));
        assert_eq!(field(&header, SubfieldType::ReplyID), message.value("BEZ"));
        assert_eq!(received.read_message_text(&header).unwrap(), "Local identity\r");
    }
    assert_eq!(scan(&receiver, root.path(), "HUB").unwrap().messages, 0);
}

#[test]
fn native_local_reply_uses_the_original_missing_id_fallback_even_after_ack() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config();
    let mut other = c.links[0].clone();
    other.id = "OTHER".into();
    c.links.push(other);
    let area = root.path().join("test-area");
    let mut base = JamMessageBase::create(&area).unwrap();
    let parent_number = write_identity_message(&mut base, None, None, 0, attributes::MSG_LOCAL);
    let parent = base.read_header(parent_number).unwrap();
    assert!(field(&parent, SubfieldType::MsgID).is_none());
    let key = format!("{}\0{}\0{}", area.display(), parent_number, parent.date_written);
    let expected = format!("{}@leaf.example", digest(key.as_bytes()));
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let original = parse_archive(&read_limited(&path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(original[0].text("MID").unwrap(), expected);
    acknowledge_outbound(&c, root.path(), "HUB").unwrap();
    let child_number = write_identity_message(&mut base, None, None, parent_number, attributes::MSG_LOCAL);
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let reply = parse_archive(&read_limited(&path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].text("BEZ").unwrap(), expected);
    assert_ne!(reply[0].value("MID"), reply[0].value("BEZ"));
    let other_path = scan(&c, root.path(), "OTHER").unwrap().packet.unwrap();
    let both = parse_archive(&read_limited(&other_path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(both.len(), 2);
    assert_eq!(both[0].value("MID"), both[1].value("BEZ"));
    assert_eq!(both[1].value("MID"), reply[0].value("MID"));
    assert!(field(&base.read_header(child_number).unwrap(), SubfieldType::ReplyID).is_none());
}

#[test]
fn explicit_imported_bez_is_not_replaced_by_native_reply_fallback() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    let fixture = packet(root.path(), "fixture.zip", FIXTURE);
    assert_eq!(toss(&c, root.path(), "HUB", &fixture).unwrap().imported, 1);
    let mut base = JamMessageBase::open(root.path().join("test-area")).unwrap();
    let imported = base.read_header(base.lowest_message_number()).unwrap();
    let native_parent = write_identity_message(&mut base, None, None, 0, attributes::MSG_LOCAL);
    let bez = field(&imported, SubfieldType::ReplyID).unwrap();
    write_identity_message(&mut base, None, Some(bez), native_parent, attributes::MSG_LOCAL);
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let messages = parse_archive(&read_limited(&path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(messages.len(), 2, "the imported message must not be re-exported");
    assert_eq!(messages[1].value("BEZ"), Some(bez));
    assert_eq!(messages[1].text("BEZ").unwrap(), "parent@origin.example");
    assert_ne!(messages[1].value("BEZ"), messages[0].value("MID"));
}

#[test]
fn native_reply_lookup_does_not_disclose_private_or_missing_parents() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    let mut base = JamMessageBase::create(root.path().join("test-area")).unwrap();
    for id in [None, Some(b"private-parent@origin.example".as_slice())] {
        let parent = write_identity_message(&mut base, id, None, 0, attributes::MSG_LOCAL | attributes::MSG_PRIVATE);
        write_identity_message(&mut base, None, None, parent, attributes::MSG_LOCAL);
    }
    write_identity_message(&mut base, None, None, u32::MAX, attributes::MSG_LOCAL);
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let messages = parse_archive(&read_limited(&path, MAX_ARCHIVE).unwrap()).unwrap();
    assert_eq!(messages.len(), 3);
    assert!(messages.iter().all(|message| !message.has("BEZ")));
}

#[test]
fn config_contract_defaults_and_validation() {
    let empty: ZconnectConfig = toml::from_str("").unwrap();
    assert_eq!(empty, ZconnectConfig::default());
    assert!(!empty.enabled);
    assert_eq!(empty.inbound, PathBuf::from("zconnect/inbound"));
    assert_eq!(empty.outbound, PathBuf::from("zconnect/outbound"));
    assert_eq!(empty.local_user, "sysop");
    assert!(empty.validate().is_ok());
    let c: ZconnectConfig = toml::from_str(
        "enabled = true\nlocal_system = 'leaf.example'\n[[link]]\nid = 'HUB'\n[[link.area]]\nremote_board = '/PUBLIC/TEST'\nlocal_area = 'test-area'\n",
    )
    .unwrap();
    assert_eq!(c, config());
    assert_eq!(c.link("hub").unwrap().port, 23);
    assert_eq!(c.link("hub").unwrap().timeout_secs, 60);
    assert_eq!(c.link("hub").unwrap().login, "zconnect");
    assert!(c.validate().is_ok(), "offline host must be allowed");
    assert_eq!(toml::from_str::<ZconnectConfig>(&toml::to_string(&c).unwrap()).unwrap(), c);
    for id in ["../escape", ".", "A/B", "a\\b", "x\nBAD", "bad.dot"] {
        let mut invalid = c.clone();
        invalid.links[0].id = id.into();
        assert!(invalid.validate().is_err(), "{id}");
    }
    for system in ["localhost", "a..b", "-a.example", "a.example\r\n", "a/b.example"] {
        let mut invalid = c.clone();
        invalid.local_system = system.into();
        assert!(invalid.validate().is_err(), "{system}");
    }
    for board in ["NO/SLASH", "/LOWER/case", "/END/", "/TWO//PARTS", "/../BAD", "/BOARD@private.example"] {
        let mut invalid = c.clone();
        invalid.links[0].areas[0].remote_board = board.into();
        assert!(invalid.validate().is_err(), "{board}");
    }
    let mut invalid = c.clone();
    let mut duplicate = invalid.links[0].clone();
    duplicate.id = "hub".into();
    invalid.links.push(duplicate);
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].areas.push(c.links[0].areas[0].clone());
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].port = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].timeout_secs = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].login = "shell".into();
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].password = "secret\rcommand".into();
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.links[0].areas[0].local_area = "../escape".into();
    assert!(invalid.validate().is_err());
    assert!(!pending_packet(&c, Path::new("/board"), "../../escape").to_string_lossy().contains(".."));
}

#[test]
fn independent_fixture_import_preserves_headers_body_route_and_reply() {
    let root = tempfile::tempdir().unwrap();
    let path = packet(root.path(), "fixture.zip", FIXTURE);
    let c = config();
    assert_eq!(toss(&c, root.path(), "hub", &path).unwrap().imported, 1);
    assert_eq!(read_limited(&path, MAX_ARCHIVE).unwrap(), make_archive(FIXTURE).unwrap());
    let base = JamMessageBase::open(root.path().join("test-area")).unwrap();
    let header = base.read_header(base.lowest_message_number()).unwrap();
    assert_eq!(base.read_message_text(&header).unwrap(), "Hello\r");
    assert_eq!(header.subject().unwrap(), "Independent fixture");
    assert_eq!(field(&header, SubfieldType::MsgID).unwrap(), b"fixture-1@origin.example");
    assert_eq!(field(&header, SubfieldType::ReplyID).unwrap(), b"parent@origin.example");
    assert_eq!(field(&header, SubfieldType::Address0).unwrap(), b"leaf.example!hub.example!origin.example");
    assert_eq!(field(&header, WIRE_BODY).unwrap(), b"Hello\r\n");
    let original_header = field(&header, WIRE_HEADER).unwrap();
    assert!(original_header.windows(b"X-OPAQUE: preserve me".len()).any(|s| s == b"X-OPAQUE: preserve me"));
    assert_eq!(&FIXTURE[..FIXTURE.len() - 7], original_header);
    assert_eq!(header.date_written as i64, parse_date("20260906123456W+0").unwrap().timestamp());
    assert_eq!(toss(&c, root.path(), "HUB", &path).unwrap().duplicates, 1);
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
}

#[test]
fn len_not_blank_lines_or_embedded_headers_frames_messages() {
    let body = b"First\r\n\r\nLEN: 0\r\nMID: not-a-header\r\n";
    let mut bytes = custom_message("EMP: /PUBLIC/TEST\r\n", body, "first");
    bytes.extend_from_slice(FIXTURE);
    let mut messages = Vec::new();
    parse_messages(&bytes, &mut messages).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].body, body);
    assert_eq!(messages[1].body, b"Hello\r\n");
    let zero = custom_message("EMP: /PUBLIC/TEST\r\n", b"", "empty");
    let mut messages = Vec::new();
    parse_messages(&zero, &mut messages).unwrap();
    assert!(messages[0].body.is_empty());
}

#[test]
fn malformed_tail_never_imports_the_valid_prefix() {
    let root = tempfile::tempdir().unwrap();
    let mut bytes = FIXTURE.to_vec();
    bytes.extend_from_slice(b"LEN: 4\r\n\r\nabc");
    let path = packet(root.path(), "bad.zip", &bytes);
    assert!(toss(&config(), root.path(), "HUB", &path).is_err());
    assert!(!root.path().join("test-area.jhr").exists());
    assert!(path.exists());
    for bad in [
        String::from_utf8(FIXTURE.to_vec()).unwrap().replace("LEN: 7", "LEN: +7"),
        String::from_utf8(FIXTURE.to_vec()).unwrap().replace("LEN: 7", "LEN: 7\r\nlen: 7"),
        String::from_utf8(FIXTURE.to_vec())
            .unwrap()
            .replace("LEN: 7", "LEN: 999999999999999999999999999"),
        String::from_utf8(FIXTURE.to_vec()).unwrap().replace("LEN: 7", "LEN: 8"),
        String::from_utf8(FIXTURE.to_vec())
            .unwrap()
            .replace("EDA: 20260906123456S+2", "EDA: 20260230123456S+2"),
        String::from_utf8(FIXTURE.to_vec()).unwrap().replace("\r\nBET:", "\nBET:"),
        String::from_utf8(FIXTURE.to_vec()).unwrap().replace("MID: fixture-1", "MID: <fixture-1"),
    ] {
        assert!(parse_messages(bad.as_bytes(), &mut Vec::new()).is_err());
    }
}

#[test]
fn charset_iso1_and_legacy_ascii_are_lossless_other_bytes_are_retained() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    let mut bytes = custom_message("EMP: /PUBLIC/TEST\r\nCHARSET: iso1\r\n", b"Gr\xfc\xdfe\r\n", "iso");
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\n", b"Legacy ASCII\r\n", "ascii"));
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\nCHARSET: UNICODE\r\n", b"\xff\xfeA\0", "unicode"));
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\n", b"\x81", "ambiguous-legacy"));
    let path = packet(root.path(), "charsets.zip", &bytes);
    let report = toss(&c, root.path(), "HUB", &path).unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(report.unsupported, 2);
    let base = JamMessageBase::open(root.path().join("test-area")).unwrap();
    let header = base.read_header(base.lowest_message_number()).unwrap();
    assert_eq!(base.read_message_text(&header).unwrap(), "Grüße\r");
    let original = read_limited(&path, MAX_ARCHIVE).unwrap();
    let retained = root.path().join("zconnect/inbound/HUB/retained").join(format!("{}.zip", digest(&original)));
    assert_eq!(read_limited(&retained, MAX_ARCHIVE).unwrap(), original);
}

#[test]
fn crossposts_deduplicate_per_area_not_globally_and_keep_private_content() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config();
    c.links[0].areas.push(ZconnectArea {
        remote_board: "/SECOND".into(),
        local_area: "second".into(),
        read_only: true,
    });
    let bytes = custom_message(
        "EMP: /PUBLIC/TEST\r\nEMP: /SECOND\r\nEMP: /SECOND\r\nEMP: /UNKNOWN\r\nEMP: user@private.example\r\n",
        b"Crosspost\r\n",
        "cross",
    );
    let path = packet(root.path(), "cross.zip", &bytes);
    let report = toss(&c, root.path(), "HUB", &path).unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(report.duplicates, 0);
    assert_eq!(report.unknown_boards, 1);
    assert_eq!(report.unsupported, 1);
    assert_eq!(toss(&c, root.path(), "HUB", &path).unwrap().duplicates, 2);
    // Mapping the previously unknown board later must deliver it despite the MID
    // already being present in two other areas.
    c.links[0].areas.push(ZconnectArea {
        remote_board: "/UNKNOWN".into(),
        local_area: "third".into(),
        read_only: false,
    });
    let retry = toss(&c, root.path(), "HUB", &path).unwrap();
    assert_eq!(retry.imported, 1);
    assert_eq!(retry.duplicates, 2);
}

#[test]
fn private_binary_and_unknown_members_are_not_classified_by_extension() {
    let root = tempfile::tempdir().unwrap();
    let mut bytes = custom_message("EMP: /PUBLIC/TEST@private.example\r\n", b"Secret\r\n", "private");
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\nTYP: BIN\r\n", b"\0\xff\r\nLEN: 99", "binary"));
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\nCRYPT: PGP\r\n", b"secret", "crypt"));
    let path = packet(root.path(), "misleading.zip", &bytes); // Internal MAIL.BRT!
    let report = toss(&config(), root.path(), "HUB", &path).unwrap();
    assert_eq!(report.imported, 0);
    assert_eq!(report.unsupported, 3);
    assert!(path.exists());
    assert!(!root.path().join("test-area.jhr").exists());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("ODD.PRV", SimpleFileOptions::default()).unwrap();
    zip.write_all(FIXTURE).unwrap();
    let archive = zip.finish().unwrap().into_inner();
    let path = root.path().join("public-in-prv.zip");
    atomic_write(&path, &archive).unwrap();
    assert_eq!(toss(&config(), root.path(), "HUB", &path).unwrap().imported, 1);
}

#[test]
fn route_loop_uses_full_case_insensitive_system_name() {
    let root = tempfile::tempdir().unwrap();
    let bytes = String::from_utf8(FIXTURE.to_vec())
        .unwrap()
        .replace("hub.example!origin.example", "other.example!LEAF.EXAMPLE!origin.example");
    let path = packet(root.path(), "loop.zip", bytes.as_bytes());
    let report = toss(&config(), root.path(), "HUB", &path).unwrap();
    assert_eq!(report.loops, 1);
    assert_eq!(report.imported, 0);
    assert!(!root.path().join("test-area.jhr").exists());
}

#[test]
fn full_ids_not_casefolded_jam_crc_determine_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let lower = BString::from("message@origin.example");
    let upper = BString::from("MESSAGE@origin.example");
    assert_eq!(JamMessageBase::crc(&lower), JamMessageBase::crc(&upper));
    let mut bytes = custom_message("EMP: /PUBLIC/TEST\r\n", b"First\r\n", "message");
    bytes.extend_from_slice(&custom_message("EMP: /PUBLIC/TEST\r\n", b"Second\r\n", "MESSAGE"));
    let path = packet(root.path(), "same-crc.zip", &bytes);
    let c = config();
    assert_eq!(toss(&c, root.path(), "HUB", &path).unwrap().imported, 2);
    assert_eq!(toss(&c, root.path(), "HUB", &path).unwrap().duplicates, 2);
}

#[test]
fn held_local_message_blocks_checkpoint_and_read_only_area_is_not_exported() {
    let root = tempfile::tempdir().unwrap();
    let mut c = config();
    local_message(root.path(), "test-area", attributes::MSG_LOCAL | attributes::MSG_HOLD, "Held\r");
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "Later\r");
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
    let journal = load_journal(&root.path().join("zconnect/outbound/HUB")).unwrap();
    assert_eq!(journal.committed.get("/PUBLIC/TEST").copied().unwrap_or(0), 0);
    c.links[0].areas[0].local_area = "readonly".into();
    c.links[0].areas[0].read_only = true;
    local_message(root.path(), "readonly", attributes::MSG_LOCAL, "Read only\r");
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
}

#[test]
fn operation_lock_is_shared_by_case_insensitive_link_lookup() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    let (dir, guard) = operation_lock(&c, root.path(), checked_link(&c, "hub").unwrap()).unwrap();
    let contender = OpenOptions::new().read(true).write(true).open(dir.join("operation.lock")).unwrap();
    assert!(FileExt::try_lock(&contender).is_err());
    drop(guard);
    FileExt::try_lock(&contender).unwrap();
}

#[test]
fn outbound_round_trip_retry_and_ack_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "Grüße\rSecond line\r");
    let first = scan(&c, root.path(), "hub").unwrap();
    assert_eq!(first.messages, 1);
    let path = first.packet.unwrap();
    assert_eq!(path, root.path().join("zconnect/outbound/HUB/mail.zip"));
    let original = read_limited(&path, MAX_ARCHIVE).unwrap();
    let messages = parse_archive(&original).unwrap();
    assert_eq!(messages[0].body, b"Gr\xfc\xdfe\r\nSecond line\r\n");
    assert_eq!(messages[0].text("ABS").unwrap(), "sysop@leaf.example (Alice Smith)");
    assert_eq!(messages[0].text("BEZ").unwrap(), "parent@origin.example");
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "New mail\r");
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 1);
    assert_eq!(read_limited(&path, MAX_ARCHIVE).unwrap(), original, "failed transport must retry exact archive");
    let mut receiver = c.clone();
    receiver.local_system = "receiver.example".into();
    receiver.links[0].areas[0].local_area = "received".into();
    receiver.outbound = "receiver-spool".into();
    assert_eq!(toss(&receiver, root.path(), "HUB", &path).unwrap().imported, 1);
    let received = JamMessageBase::open(root.path().join("received")).unwrap();
    let header = received.read_header(received.lowest_message_number()).unwrap();
    assert_eq!(received.read_message_text(&header).unwrap(), "Grüße\rSecond line\r");
    acknowledge_outbound(&c, root.path(), "HUB").unwrap();
    assert!(!path.exists());
    acknowledge_outbound(&c, root.path(), "HUB").unwrap();
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 1, "only the unsent new message");
    acknowledge_outbound(&c, root.path(), "HUB").unwrap();
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
    assert_eq!(scan(&receiver, root.path(), "HUB").unwrap().messages, 0, "imports must not be re-exported");
}

#[test]
fn excludes_private_binary_and_nonlocal_jam_even_if_local_flag_is_set() {
    let root = tempfile::tempdir().unwrap();
    for flags in [
        attributes::MSG_PRIVATE,
        attributes::MSG_TYPENET,
        attributes::MSG_TYPELOCAL,
        attributes::MSG_FILEATTACH,
        attributes::MSG_ENCRYPT,
        attributes::MSG_NODISP,
    ] {
        local_message(root.path(), "test-area", attributes::MSG_LOCAL | flags, "must stay local");
    }
    local_message(root.path(), "test-area", attributes::MSG_TYPEECHO, "imported elsewhere");
    assert_eq!(scan(&config(), root.path(), "HUB").unwrap().messages, 0);
}

#[test]
fn unrepresentable_local_text_never_advances_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "valid first\r");
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "Unicode snowman: ☃\r");
    assert!(scan(&c, root.path(), "HUB").is_err());
    let dir = root.path().join("zconnect/outbound/HUB");
    assert!(load_journal(&dir).unwrap().committed.is_empty());
    assert!(!dir.join("mail.zip").exists());
}

#[test]
fn crash_recovery_promotes_prepared_and_retires_already_committed_packet() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "Crash test\r");
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let dir = path.parent().unwrap();
    fs::rename(&path, dir.join("prepared.zip")).unwrap();
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 1);
    assert!(path.exists());
    let mut journal = load_journal(dir).unwrap();
    let pending = journal.pending.take().unwrap();
    journal.committed = pending.next;
    journal.retired = Some(pending.sha256);
    save_journal(dir, &journal).unwrap(); // Simulated power loss before unlink.
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
    assert!(!path.exists());
    assert!(load_journal(dir).unwrap().retired.is_none());
}

#[test]
fn missing_or_modified_pending_archive_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let c = config();
    local_message(root.path(), "test-area", attributes::MSG_LOCAL, "Keep me\r");
    let path = scan(&c, root.path(), "HUB").unwrap().packet.unwrap();
    let original = read_limited(&path, MAX_ARCHIVE).unwrap();
    atomic_write(&path, b"damaged").unwrap();
    assert!(acknowledge_outbound(&c, root.path(), "HUB").is_err());
    assert!(scan(&c, root.path(), "HUB").is_err());
    fs::remove_file(&path).unwrap();
    assert!(scan(&c, root.path(), "HUB").is_err());
    assert!(load_journal(path.parent().unwrap()).unwrap().committed.is_empty());
    atomic_write(&path, &original).unwrap();
    acknowledge_outbound(&c, root.path(), "HUB").unwrap();
    assert_eq!(scan(&c, root.path(), "HUB").unwrap().messages, 0);
}

#[test]
fn archive_traversal_oversize_and_duplicate_members_are_rejected() {
    for names in [vec!["../escape"], vec!["/absolute"], vec!["a\\b"], vec!["MAIL.BRT", "mail.brt"]] {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in &names {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(FIXTURE).unwrap();
        }
        assert!(parse_archive(&zip.finish().unwrap().into_inner()).is_err(), "accepted {names:?}");
    }
    let bytes = custom_message("EMP: /PUBLIC/TEST\r\n", &vec![b'x'; MAX_BODY + 1], "huge");
    assert!(parse_messages(&bytes, &mut Vec::new()).is_err());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..=MAX_ENTRIES {
        zip.start_file(format!("{i}.BRT"), SimpleFileOptions::default()).unwrap();
    }
    assert!(parse_archive(&zip.finish().unwrap().into_inner()).is_err());
}

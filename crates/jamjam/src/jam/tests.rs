use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_open_base() {
    let base = JamMessageBase::open("data/jam/general").unwrap();

    assert_eq!(base.active_messages(), 4);
    assert_eq!(base.base_messagenumber(), 1);
    assert_eq!(base.mod_counter(), 4);
    assert!(!base.needs_password());
}

#[test]
fn test_read_headers() {
    let base = JamMessageBase::open("data/jam/general").unwrap();
    let headers = base.read_headers().unwrap();
    assert_eq!(base.active_messages(), headers.len() as u32);
}

#[test]
fn test_get_header() {
    let base = JamMessageBase::open("data/jam/general").unwrap();
    let header = base.read_header(3).unwrap();
    assert_eq!(header.get_from().unwrap(), "omnibrain");
    assert_eq!(header.get_subject().unwrap(), "Re: Hello All");
    assert_eq!(header.reply_to, 2);
}

#[test]
fn test_get_text() {
    let base = JamMessageBase::open("data/jam/general").unwrap();
    let header = base.read_header(4).unwrap();
    let txt = base.read_msg_text(&header).unwrap();
    assert_eq!(txt, "private message\r\r... Multitasking: Reading in the bathroom\r");
}

#[test]
fn test_iter_ra() {
    let base = JamMessageBase::open("data/jam/ra").unwrap();
    for msg in base.iter() {
        let msg = msg.unwrap();
        for sub in msg.sub_fields.iter() {
            if *sub.get_type() == SubfieldType::MsgID {
                assert_eq!(JamMessageBase::get_crc(sub.get_string()), msg.msgid_crc);
            }
        }
    }
    assert_eq!(3, base.iter().count());
}

#[test]
fn test_iter_mystic() {
    let base = JamMessageBase::open("data/jam/general").unwrap();
    for msg in base.iter() {
        let msg = msg.unwrap();
        println!("{} {} {}", msg.reply1st, msg.reply_to, msg.replynext);
        for sub in msg.sub_fields.iter() {
            if *sub.get_type() == SubfieldType::MsgID {
                assert_eq!(JamMessageBase::get_crc(sub.get_string()), msg.msgid_crc);
            }
        }
    }
    assert_eq!(4, base.iter().count());
}

#[test]
fn test_iter_elebbs() {
    let base = JamMessageBase::open("data/jam/elebbs").unwrap();
    for msg in base.iter() {
        let msg = msg.unwrap();
        for sub in msg.sub_fields.iter() {
            if *sub.get_type() == SubfieldType::MsgID {
                assert_eq!(JamMessageBase::get_crc(sub.get_string()), msg.msgid_crc);
            }
        }
    }
    assert_eq!(4, base.iter().count());
}

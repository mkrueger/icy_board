use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use crate::{
    jam::{
        self,
        msg_header::{MessageSubfield, SubfieldType},
        JamMessage, JamMessageBase,
    },
    pcboard::{message_header::ExtendedHeaderInformation, PCBoardMessage, PCBoardMessageBase},
    util::echmoail::EchomailAddress,
};

fn get_jam_attributes(msg: &PCBoardMessage) -> u32 {
    let mut result = 0;
    match msg.get_status() {
        crate::pcboard::MessageStatus::Private | crate::pcboard::MessageStatus::CommentToSysop => {
            result |= jam::attributes::MSG_PRIVATE
        }
        _ => {}
    }

    if msg.is_read() {
        result |= jam::attributes::MSG_READ;
    }

    if msg.is_deleted() {
        result |= jam::attributes::MSG_DELETED;
    }

    if msg.header.has_attach() {
        result |= jam::attributes::MSG_FILEATTACH;
    }

    if msg.header.has_reqrr() {
        result |= jam::attributes::MSG_RECEIPTREQ;
    }

    result
}

pub fn convert_pcboard_to_jam(
    pcboard_path: &Path,
    jam_dest_path: &Path,
    aka: &EchomailAddress,
) -> crate::Result<()> {
    let pcb_base = PCBoardMessageBase::open(pcboard_path)?;

    let mut jam_messages = BTreeMap::new();
    let mut message_ids = HashMap::new();

    for msg_result in pcb_base.iter() {
        let msg = msg_result?;
        let attribute = get_jam_attributes(&msg);
        let time = msg.header.date_time();

        let mut to = msg.header.to_field;
        let mut from = msg.header.from_field;
        let mut subj = msg.header.subj_field;
        let mut sub_fields = Vec::new();
        for header in msg.extended_header {
            match header.info {
                ExtendedHeaderInformation::To => {
                    to.clone_from(&header.content);
                }
                ExtendedHeaderInformation::From => {
                    from.clone_from(&header.content);
                }
                ExtendedHeaderInformation::Subject => {
                    subj.clone_from(&header.content);
                }
                ExtendedHeaderInformation::To2 => {
                    to.append(&mut header.content.clone());
                }
                ExtendedHeaderInformation::From2 => {
                    from.append(&mut header.content.clone());
                }
                ExtendedHeaderInformation::Attach => {
                    sub_fields.push(MessageSubfield::new(SubfieldType::EnclFile, header.content));
                }

                ExtendedHeaderInformation::List
                | ExtendedHeaderInformation::Route
                | ExtendedHeaderInformation::Origin
                | ExtendedHeaderInformation::Reqrr
                | ExtendedHeaderInformation::Ackrr
                | ExtendedHeaderInformation::Ackname
                | ExtendedHeaderInformation::Packout
                | ExtendedHeaderInformation::Forward
                | ExtendedHeaderInformation::Ufollow
                | ExtendedHeaderInformation::Unewsgr => {
                    // ignore
                    continue;
                }
            }
        }

        let new_msg = JamMessage::new(msg.header.msg_number, aka)
            .with_reply_to(msg.header.reply_to)
            .with_date_time(time)
            .with_text(msg.text)
            .with_attributes(attribute)
            .with_password(&msg.header.password)
            .with_from(from)
            .with_to(to.clone())
            .with_subject(subj.clone());
        message_ids.insert(msg.header.msg_number, new_msg.get_msgid_crc());
        jam_messages.insert(msg.header.msg_number, new_msg);
    }

    // Setting reply information for each message.
    // TODO: reply_next
    // Does it even make any sense at all?

    let mut reply_next = Vec::new();
    for (i, msg) in &mut jam_messages {
        let reply_to = msg.get_reply_to();
        if reply_to != 0 && reply_to != *i {
            if let Some(crc) = message_ids.get_mut(&reply_to) {
                msg.set_reply_crc(*crc);
                reply_next.push((reply_to, *i));
            }
        }
    }

    for (reply_to, rep1st) in reply_next {
        if let Some(msg) = jam_messages.get_mut(&reply_to) {
            if msg.get_reply1st() == 0 {
                msg.set_reply1st(rep1st);
            }
        }
    }

    // write jam message base
    let mut jam_base = JamMessageBase::create(jam_dest_path)?;
    for msg in jam_messages.values() {
        jam_base.write_message(msg)?;
    }
    jam_base.write_jhr_header()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_read_message() {
        let tmpdir = TempDir::with_prefix_in("jamtest", ".").unwrap();
        let converted = tmpdir.path().join("jambase");
        convert_pcboard_to_jam(
            &PathBuf::from("data/pcboard/test"),
            &converted,
            &EchomailAddress::default(),
        )
        .unwrap();

        let pcb = PCBoardMessageBase::open("data/pcboard/test").unwrap();
        let jam = JamMessageBase::open(&converted).unwrap();

        assert_eq!(pcb.active_messages(), jam.active_messages());

        for i in 1..=pcb.active_messages() {
            let pcb_msg = pcb.read_message(i).unwrap();
            let jam_msg = jam.read_header(i).unwrap();
            let jam_txt = jam.read_msg_text(&jam_msg).unwrap();

            assert!(pcb_msg.header.msg_number == jam_msg.message_number);
            assert!(pcb_msg.header.to_field == *jam_msg.get_to().unwrap());
            assert!(pcb_msg.header.from_field == *jam_msg.get_from().unwrap());
            assert!(pcb_msg.header.subj_field == *jam_msg.get_subject().unwrap());
            assert!(pcb_msg.text == jam_txt);
        }
    }
}

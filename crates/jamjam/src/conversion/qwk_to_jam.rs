use crate::{
    jam::{JamMessage, JamMessageBase},
    qwk::qwk_message::QWKMessage,
    util::echmoail::EchomailAddress,
};

pub fn convert_qwk_to_jam(qwk_mail: &[QWKMessage], jam_base: &mut JamMessageBase) -> crate::Result<()> {
    for mail in qwk_mail {
        let mut jam_msg = JamMessage::new(mail.msg_number, &EchomailAddress::default());

        jam_msg = jam_msg
            .with_from(mail.from.clone())
            .with_to(mail.to.clone())
            .with_subject(mail.subj.clone())
            .with_date_time(mail.date_time().and_utc())
            .with_is_deleted(mail.is_deleted());

        jam_base.write_message(&jam_msg)?;
    }
    jam_base.write_jhr_header()?;

    Ok(())
}
/*

#[cfg(test)]
mod tests {
    use crate::qwk::QwkMessageBase;

    use super::*;
    use pretty_assertions::assert_eq;
    use std::{path::PathBuf, time::SystemTime};
    use tempfile::TempDir;

    #[test]
    fn test_read_message_reader() {
        let mut msg_base = QwkMessageBase::open("/home/mkrueger/Downloads/large", true).unwrap();
        msg_base.index_offset_bug = true;

        let mail: Vec<QWKMessage> = msg_base.iter().flatten().collect();
        convert_qwk_to_jam(&mail, &mut JamMessageBase::create(PathBuf::from("/home/mkrueger/Downloads/jam_base")).unwrap()).unwrap();
    }
}

*/

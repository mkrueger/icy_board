//! PCBoard sender passwords protect modification, whereas group passwords
//! protect reading. Untagged JAM passwords retain the conservative group rule.
use bstr::BString;
use jamjam::jam::{attributes, msg_header::{JamMessageHeader, MessageSubfield, SubfieldType}};

const SENDER: &[u8] = b"ICYBOARD-SECURITY: S";

pub(crate) fn requires_read_password(header: &JamMessageHeader, may_read_all: bool) -> bool {
    !may_read_all && header.needs_password() && !header.sub_fields.iter().any(|field|
        field.field_type() == SubfieldType::FTSKludge && field.content().as_slice() == SENDER)
}

pub(crate) fn may_read_header(header: &JamMessageHeader, user: &str, alias: &str, may_read_all: bool) -> bool {
    if header.is_deleted() || header.attributes & attributes::MSG_NODISP != 0 { return false; }
    !header.is_private() || may_read_all || [header.to(), header.from()].into_iter().flatten().any(|name| {
        let name = name.to_string();
        name.trim().eq_ignore_ascii_case(user) || (!alias.is_empty() && name.trim().eq_ignore_ascii_case(alias))
    })
}

pub(crate) fn set_security_kind(header: &mut JamMessageHeader, sender_password: bool) {
    header.sub_fields.retain(|field| !(field.field_type() == SubfieldType::FTSKludge
        && field.content().starts_with(b"ICYBOARD-SECURITY:")));
    if sender_password {
        header.sub_fields.push(MessageSubfield::new(SubfieldType::FTSKludge, BString::from(SENDER)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jamjam::jam::JamMessage;
    #[test]
    fn sender_group_and_sysop_have_distinct_read_rules() {
        let message = JamMessage::default().with_password(&BString::from("secret"));
        let mut header = message.header().clone();
        assert!(requires_read_password(&header, false));
        assert!(!requires_read_password(&header, true));
        set_security_kind(&mut header, true);
        assert!(!requires_read_password(&header, false));
        set_security_kind(&mut header, false);
        assert!(requires_read_password(&header, false));
    }
    #[test]
    fn private_deleted_and_hidden_headers_fail_closed() {
        let message = JamMessage::default().with_from(BString::from("AUTHOR")).with_to(BString::from("READER"))
            .with_attributes(attributes::MSG_PRIVATE);
        let mut header = message.header().clone();
        assert!(may_read_header(&header, "reader", "", false));
        assert!(!may_read_header(&header, "stranger", "", false));
        header.attributes |= attributes::MSG_DELETED;
        assert!(!may_read_header(&header, "reader", "", true));
    }
}
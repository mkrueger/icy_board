//! Deciding which messages a read command actually wants to see.
//! `PCBoard` skips over anything that fails one of these, without prompting.

use jamjam::jam::msg_header::JamMessageHeader;
use regex::Regex;

use crate::icy_board::state::Session;

use super::message_security::{may_read_header, requires_read_password};
use super::read_command::{ReadCommand, user_search};

/// The default lets every message through.
#[derive(Clone)]
pub struct MessageFilter {
    any_msgs: bool,
    your_msgs: bool,
    from_msgs: bool,
    msgs_to_all: bool,
    unread_only: bool,
    /// Messages older than this are skipped.
    written_after: Option<u32>,
    user_search: u8,
    name_to: String,
    name_from: String,
    /// The subject a thread is following, without any `Re: `.
    thread_subject: Option<String>,
    text: Option<Regex>,
    /// The reader's own name and alias, upper cased.
    own_names: Vec<String>,
    /// `PCBoard`'s `SEC_READALLMAIL`, which opens receiver-only mail to the sysop.
    may_read_all_mail: bool,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            any_msgs: true,
            your_msgs: false,
            from_msgs: false,
            msgs_to_all: false,
            unread_only: false,
            written_after: None,
            user_search: user_search::NONE,
            name_to: String::new(),
            name_from: String::new(),
            thread_subject: None,
            text: None,
            own_names: Vec::new(),
            may_read_all_mail: true,
        }
    }
}

impl MessageFilter {
    pub fn new(cmd: &ReadCommand, session: &Session, may_read_all_mail: bool) -> Self {
        let mut own_names = vec![session.user_name.to_ascii_uppercase()];
        if !session.alias_name.is_empty() {
            own_names.push(session.alias_name.to_ascii_uppercase());
        }
        Self {
            any_msgs: cmd.any_msgs,
            your_msgs: cmd.your_msgs,
            from_msgs: cmd.from_msgs,
            msgs_to_all: cmd.msgs_to_all,
            unread_only: cmd.unread_only,
            written_after: cmd.new_date.as_deref().and_then(parse_mmddyy),
            user_search: cmd.user_search,
            name_to: cmd.user_name_to.to_ascii_uppercase(),
            name_from: cmd.user_name_from.to_ascii_uppercase(),
            thread_subject: if cmd.threading { Some(strip_re(&cmd.search_text).to_string()) } else { None },
            // The regex itself lives on the session so found text gets highlighted.
            text: if cmd.do_text_search { session.search_pattern.clone() } else { None },
            own_names,
            may_read_all_mail,
        }
    }

    /// True when nothing is being filtered, so a message can be shown without reading its body.
    pub fn is_empty(&self) -> bool {
        self.any_msgs
            && !self.unread_only
            && self.written_after.is_none()
            && self.user_search == user_search::NONE
            && self.thread_subject.is_none()
            && self.text.is_none()
    }

    pub fn matches(&self, header: &JamMessageHeader, body: &str, _last_read: u32) -> bool {
        if !self.may_read(header) || !self.may_search(header, self.may_read_all_mail) {
            return false;
        }
        let to = field(header.to());
        let from = field(header.from());
        let subject = field(header.subject());

        if !(self.any_msgs || self.your_msgs && self.is_own(&to) || self.from_msgs && self.is_own(&from) || self.msgs_to_all && to == "ALL") {
            return false;
        }
        if self.unread_only && header.is_read() {
            return false;
        }
        if let Some(after) = self.written_after
            && header.date_written < after
        {
            return false;
        }
        if self.user_search & user_search::TO != 0 && !to.contains(&self.name_to) {
            return false;
        }
        if self.user_search & user_search::FROM != 0 && !from.contains(&self.name_from) {
            return false;
        }
        if self.user_search == user_search::USER && !to.contains(&self.name_to) && !from.contains(&self.name_to) {
            return false;
        }
        if let Some(thread) = &self.thread_subject
            && !strip_re(&subject).eq_ignore_ascii_case(thread)
        {
            return false;
        }
        if let Some(text) = &self.text {
            // PCBoard searches the To..Subject block first, then the body.
            if !text.is_match(&to) && !text.is_match(&from) && !text.is_match(&subject) && !text.is_match(body) {
                return false;
            }
        }
        true
    }

    /// Searches have no password authorization. Skip group-password mail before
    /// testing even the public fields: a hit-dependent password prompt leaks
    /// whether the protected body contains the search text. Sender passwords
    /// protect modification only; read-all-mail privilege still bypasses both.
    pub(super) fn may_search(&self, header: &JamMessageHeader, may_read_all_mail: bool) -> bool {
        self.text.is_none() || !requires_read_password(header, may_read_all_mail)
    }

    /// `readstatus` in `PCBoard`: a receiver-only message belongs to its two ends,
    /// and to whoever may read all mail.
    pub fn may_read(&self, header: &JamMessageHeader) -> bool {
        !header.is_deleted()
            && may_read_header(
                header,
                self.own_names.first().map(String::as_str).unwrap_or_default(),
                self.own_names.get(1).map(String::as_str).unwrap_or_default(),
                self.may_read_all_mail,
            )
    }

    fn is_own(&self, name: &str) -> bool {
        self.own_names.iter().any(|own| own == name)
    }
}

fn field(value: Option<&bstr::BString>) -> String {
    value.map(|v| v.to_string().trim().to_ascii_uppercase()).unwrap_or_default()
}

fn strip_re(subject: &str) -> &str {
    let subject = subject.trim();
    if subject.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("re: ")) {
        subject[4..].trim_start()
    } else {
        subject
    }
}

/// The date field of the new-message scan is a bare MMDDYY.
fn parse_mmddyy(input: &str) -> Option<u32> {
    if input.len() != 6 || !input.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let month = input[0..2].parse::<u32>().ok()?;
    let day = input[2..4].parse::<u32>().ok()?;
    let year = input[4..6].parse::<i32>().ok()?;
    let year = if year < 80 { 2000 + year } else { 1900 + year };
    chrono::NaiveDate::from_ymd_opt(year, month, day)?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp()
        .try_into()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(to: &str, from: &str, subject: &str) -> JamMessageHeader {
        use jamjam::jam::msg_header::{MessageSubfield, SubfieldType};
        JamMessageHeader {
            sub_fields: vec![
                MessageSubfield::new(SubfieldType::RecvName, bstr::BString::from(to)),
                MessageSubfield::new(SubfieldType::SenderName, bstr::BString::from(from)),
                MessageSubfield::new(SubfieldType::Subject, bstr::BString::from(subject)),
            ],
            ..Default::default()
        }
    }

    fn filter(setup: impl Fn(&mut MessageFilter)) -> MessageFilter {
        let mut filter = MessageFilter {
            any_msgs: true,
            ..Default::default()
        };
        setup(&mut filter);
        filter
    }

    #[test]
    fn an_empty_filter_takes_everything() {
        let filter = filter(|_| {});
        assert!(filter.is_empty());
        assert!(filter.matches(&header("ALL", "SYSOP", "Hello"), "body", 0));
    }

    #[test]
    fn a_receiver_only_message_belongs_to_its_two_ends() {
        let private = |to: &str, from: &str| {
            let mut header = header(to, from, "Hello");
            header.attributes |= jamjam::jam::attributes::MSG_PRIVATE;
            header
        };
        let reader = |f: &mut MessageFilter| {
            f.may_read_all_mail = false;
            f.own_names = vec!["TEST USER".to_string()];
        };

        assert!(!filter(reader).matches(&private("SOMEONE ELSE", "REMOTE USER"), "body", 0));
        assert!(filter(reader).matches(&private("TEST USER", "REMOTE USER"), "body", 0));
        assert!(filter(reader).matches(&private("REMOTE USER", "TEST USER"), "body", 0));
        assert!(filter(|f| f.may_read_all_mail = true).matches(&private("SOMEONE ELSE", "REMOTE USER"), "body", 0));
        assert!(filter(reader).matches(&header("SOMEONE ELSE", "REMOTE USER", "Hello"), "body", 0));
    }

    #[test]
    fn your_messages_are_the_ones_addressed_to_you() {
        let filter = filter(|f| {
            f.any_msgs = false;
            f.your_msgs = true;
            f.own_names = vec!["TEST USER".to_string()];
        });
        assert!(filter.matches(&header("TEST USER", "SYSOP", ""), "", 0));
        assert!(!filter.matches(&header("ALL", "SYSOP", ""), "", 0));
    }

    #[test]
    fn ya_also_takes_the_messages_to_all() {
        let filter = filter(|f| {
            f.any_msgs = false;
            f.your_msgs = true;
            f.msgs_to_all = true;
            f.own_names = vec!["TEST USER".to_string()];
        });
        assert!(filter.matches(&header("ALL", "SYSOP", ""), "", 0));
    }

    #[test]
    fn unread_only_uses_recipient_status_not_the_scan_pointer() {
        let filter = filter(|f| f.unread_only = true);
        let mut msg = header("ALL", "SYSOP", "");
        msg.message_number = 5;
        assert!(filter.matches(&msg, "", 100));
        assert!(filter.matches(&msg, "", 4));
        msg.attributes |= jamjam::jam::attributes::MSG_READ;
        assert!(!filter.matches(&msg, "", 0));
    }

    #[test]
    fn a_sender_search_matches_part_of_the_name() {
        let filter = filter(|f| {
            f.user_search = user_search::FROM;
            f.name_from = "SYS".to_string();
        });
        assert!(filter.matches(&header("ALL", "SYSOP", ""), "", 0));
        assert!(!filter.matches(&header("ALL", "TEST USER", ""), "", 0));
    }

    #[test]
    fn a_user_search_looks_at_both_ends() {
        let filter = filter(|f| {
            f.user_search = user_search::USER;
            f.name_to = "SYSOP".to_string();
        });
        assert!(filter.matches(&header("ALL", "SYSOP", ""), "", 0));
        assert!(filter.matches(&header("SYSOP", "TEST USER", ""), "", 0));
        assert!(!filter.matches(&header("ALL", "TEST USER", ""), "", 0));
    }

    #[test]
    fn a_thread_follows_the_subject_through_the_replies() {
        let filter = filter(|f| f.thread_subject = Some("HELLO".to_string()));
        assert!(filter.matches(&header("ALL", "SYSOP", "Hello"), "", 0));
        assert!(filter.matches(&header("ALL", "SYSOP", "Re: Hello"), "", 0));
        assert!(!filter.matches(&header("ALL", "SYSOP", "Goodbye"), "", 0));
        assert!(!filter.matches(&header("ALL", "SYSOP", "日付"), "", 0));
    }

    #[test]
    fn a_text_search_looks_at_the_header_and_the_body() {
        let filter = filter(|f| f.text = Some(Regex::new("(?i)needle").unwrap()));
        assert!(filter.matches(&header("ALL", "SYSOP", "a needle"), "", 0));
        assert!(filter.matches(&header("ALL", "SYSOP", ""), "hay needle hay", 0));
        assert!(!filter.matches(&header("ALL", "SYSOP", ""), "only hay", 0));
    }

    #[test]
    fn text_search_skips_group_password_mail_without_testing_for_a_hit() {
        let search = filter(|f| {
            f.text = Some(Regex::new("(?i)needle").unwrap());
            f.may_read_all_mail = false;
        });
        let mut protected = header("ALL", "SYSOP", "ordinary subject");
        protected.password_crc = jamjam::jam::JamMessageBase::crc(&bstr::BString::from("SECRET"));
        assert!(!search.may_search(&protected, false));
        assert!(!search.matches(&protected, "hidden needle", 0));
        assert!(!search.matches(&protected, "only hay", 0));
        protected.set_subject(bstr::BString::from("needle"));
        assert!(!search.matches(&protected, "only hay", 0));

        // Ordinary reading still reaches the reader's password prompt.
        assert!(filter(|f| f.may_read_all_mail = false).matches(&protected, "hidden needle", 0));
        // A caller-supplied privileged filter cannot bypass the actual session.
        let privileged = filter(|f| f.text = search.text.clone());
        assert!(!privileged.may_search(&protected, false));
        assert!(privileged.matches(&protected, "hidden needle", 0));

        super::super::message_security::set_security_kind(&mut protected, true);
        assert!(search.may_search(&protected, false));
        assert!(search.matches(&protected, "hidden needle", 0));
    }

    #[test]
    fn the_new_message_date_is_mmddyy() {
        assert_eq!(parse_mmddyy("013099"), Some(917_654_400));
        assert_eq!(parse_mmddyy("1"), None);
        assert_eq!(parse_mmddyy("991301"), None);
    }
}

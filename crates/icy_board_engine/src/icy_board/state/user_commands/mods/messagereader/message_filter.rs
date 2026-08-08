//! Deciding which messages a read command actually wants to see.
//! PCBoard skips over anything that fails one of these, without prompting.

use jamjam::jam::msg_header::JamMessageHeader;
use regex::Regex;

use crate::icy_board::state::Session;

use super::read_command::{ReadCommand, user_search};

/// The default lets every message through.
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
        }
    }
}

impl MessageFilter {
    pub fn new(cmd: &ReadCommand, session: &Session) -> Self {
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

    pub fn matches(&self, header: &JamMessageHeader, body: &str, last_read: u32) -> bool {
        let to = field(header.get_to());
        let from = field(header.get_from());
        let subject = field(header.get_subject());

        if !self.any_msgs && !(self.your_msgs && self.is_own(&to) || self.from_msgs && self.is_own(&from) || self.msgs_to_all && to == "ALL") {
            return false;
        }
        if self.unread_only && header.message_number <= last_read {
            return false;
        }
        if let Some(after) = self.written_after {
            if header.date_written < after {
                return false;
            }
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
        if let Some(thread) = &self.thread_subject {
            if !strip_re(&subject).eq_ignore_ascii_case(thread) {
                return false;
            }
        }
        if let Some(text) = &self.text {
            // PCBoard searches the To..Subject block first, then the body.
            if !text.is_match(&to) && !text.is_match(&from) && !text.is_match(&subject) && !text.is_match(body) {
                return false;
            }
        }
        true
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
    if subject.len() >= 4 && subject[..4].eq_ignore_ascii_case("re: ") {
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
    fn unread_only_skips_what_the_pointer_has_passed() {
        let filter = filter(|f| f.unread_only = true);
        let mut msg = header("ALL", "SYSOP", "");
        msg.message_number = 5;
        assert!(!filter.matches(&msg, "", 5));
        assert!(filter.matches(&msg, "", 4));
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
    }

    #[test]
    fn a_text_search_looks_at_the_header_and_the_body() {
        let filter = filter(|f| f.text = Some(Regex::new("(?i)needle").unwrap()));
        assert!(filter.matches(&header("ALL", "SYSOP", "a needle"), "", 0));
        assert!(filter.matches(&header("ALL", "SYSOP", ""), "hay needle hay", 0));
        assert!(!filter.matches(&header("ALL", "SYSOP", ""), "only hay", 0));
    }

    #[test]
    fn the_new_message_date_is_mmddyy() {
        assert_eq!(parse_mmddyy("013099"), Some(917654400));
        assert_eq!(parse_mmddyy("1"), None);
        assert_eq!(parse_mmddyy("991301"), None);
    }
}

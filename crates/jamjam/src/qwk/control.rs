use bstr::{BString, ByteSlice};

use super::QwkError;

pub struct Conference {
    /// Conference number
    /// Note: Qwk limits conferences to u16
    pub number: u16,
    /// Conference name
    pub name: bstr::BString,
}

pub struct ControlDat {
    /// BBS name
    pub bbs_name: bstr::BString,

    /// BBS city and state
    pub bbs_city_and_state: bstr::BString,

    /// BBS phone number
    pub bbs_phone_number: bstr::BString,

    /// BBS Sysop name
    pub bbs_sysop_name: bstr::BString,

    /// Mail door registration #, BBSID
    pub bbs_id: bstr::BString,

    /// Serial number of the bbs_id
    pub serial_number: i32,

    /// Mail packet creation time
    pub creation_time: bstr::BString,

    /// User name (upper case)
    pub qmail_user_name: bstr::BString,

    /// Name of menu for Qmail, blank if none
    /// usually blank
    pub qmail_menu_name: bstr::BString,

    /// Seem to be always zero.  A few doors, such as DCQWK for TAG is using this
    /// field to indicate the conference where Fido NetMail should be placed.  This
    /// gives the reader program the ability easily send NetMail.
    ///
    /// I leave this field in in case it is useful for someone.
    pub zero_line: bstr::BString,

    /// Total number of messages in packet
    pub message_count: u32,

    /// Conferences in the packet
    pub conferences: Vec<Conference>,

    /// Welcome screen file
    pub welcome_screen: bstr::BString,
    /// BBS news file
    pub news_screen: bstr::BString,
    /// Log off screen
    pub logoff_screen: bstr::BString,
}
const EOL: &[u8; 1] = b"\n";

impl ControlDat {
    pub fn read(s: &[u8]) -> crate::Result<Self> {
        let mut lines = s.lines();
        let bbs_name = BString::from(lines.next().unwrap_or_default());
        let bbs_city_and_state = BString::from(lines.next().unwrap_or_default());
        let bbs_phone_number = BString::from(lines.next().unwrap_or_default());
        let bbs_sysop_name = BString::from(lines.next().unwrap_or_default());

        let id_line = lines.next().unwrap_or_default();
        let mut bbs_id = BString::from(id_line);
        let mut serial_number = 0;
        for i in 0..id_line.len() {
            if id_line[i] == b',' {
                let (serial, id) = id_line.split_at(i);
                bbs_id = BString::from(&id[1..]);
                if let Ok(num) = serial.to_str()?.parse::<i32>() {
                    serial_number = num;
                }
                break;
            }
        }

        let creation_time = BString::from(lines.next().unwrap_or_default());
        let qmail_user_name = BString::from(lines.next().unwrap_or_default());
        let qmail_menu_name = BString::from(lines.next().unwrap_or_default());
        let zero_line = BString::from(lines.next().unwrap_or_default());

        let num_messages_txt = lines.next().unwrap_or_default();
        let message_count = if let Ok(num) = num_messages_txt.to_str()?.parse::<u32>() {
            num
        } else {
            return Err(QwkError::CantParseMessageNumbers(num_messages_txt.into()).into());
        };

        let num_conferences = lines.next().unwrap_or_default();
        let mut conferences = Vec::new();
        if let Ok(num) = num_conferences.to_str()?.parse::<u16>() {
            for _ in 0..num {
                let number_txt = lines.next().unwrap_or_default();
                let name = BString::from(lines.next().unwrap_or_default());

                if let Ok(number) = number_txt.to_str()?.parse::<u16>() {
                    let conference = Conference { number, name };
                    conferences.push(conference);
                } else {
                    return Err(QwkError::CantParseConferenceNumber(num_conferences.into()).into());
                }
            }
        } else {
            return Err(QwkError::CantParseConferenceNumber(num_conferences.into()).into());
        }

        let welcome_screen = BString::from(lines.next().unwrap_or_default());
        let news_screen = BString::from(lines.next().unwrap_or_default());
        let logoff_screen = BString::from(lines.next().unwrap_or_default());
        // TODO: parse optional data?
        /*
        Some mail doors, such as MarkMail, will send additional information
        about the user from here on.

        0                             ?
        25                            Number of lines that follow this one
        JANE DOE                      User name in uppercase
        Jane                          User first name in mixed case
        NEW YORK, NY                  User city information
        718 555-1212                  User data phone number
        718 555-1212                  User home phone number
        108                           Security level
        00-00-00                      Expiration date
        01-01-91                      Last log on date
        23:59                         Last log on time
        999                           Log on count
        0                             Current conference number on the BBS
        0                             Total KB downloaded
        999                           Download count
        0                             Total KB uploaded
        999                           Upload count
        999                           Minutes per day
        999                           Minutes remaining today
        999                           Minutes used this call
        32767                         Max. download KB per day
        32767                         Remaining KB today
        0                             KB downloaded today
        23:59                         Current time on BBS
        01-01-91                      Current date on BBS
        My BBS                        BBS network tag-line
        0                             ?
        */
        Ok(Self {
            bbs_name,
            bbs_city_and_state,
            bbs_phone_number,
            bbs_sysop_name,
            bbs_id,
            serial_number,
            creation_time,
            qmail_user_name,
            qmail_menu_name,
            zero_line,
            message_count,
            conferences,
            welcome_screen,
            news_screen,
            logoff_screen,
        })
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend(self.bbs_name.bytes());
        s.extend(EOL);
        s.extend(self.bbs_city_and_state.bytes());
        s.extend(EOL);
        s.extend(self.bbs_phone_number.bytes());
        s.extend(EOL);
        s.extend(self.bbs_sysop_name.bytes());
        s.extend(EOL);

        s.extend(format!("{},", self.serial_number).as_bytes());
        s.extend(self.bbs_id.bytes());
        s.extend(EOL);
        s.extend(self.creation_time.bytes());
        s.extend(EOL);
        s.extend(self.qmail_user_name.bytes());
        s.extend(EOL);
        s.extend(self.qmail_menu_name.bytes());
        s.extend(EOL);
        s.push(b'0');
        s.extend(EOL);
        s.extend(self.message_count.to_string().as_bytes());
        s.extend(EOL);
        s.extend((self.conferences.len() as u16).to_string().as_bytes());
        s.extend(EOL);
        for conference in &self.conferences {
            s.extend(conference.number.to_string().as_bytes());
            s.extend(EOL);
            s.extend(conference.name.bytes());
            s.extend(EOL);
        }
        s.extend(self.welcome_screen.bytes());
        s.extend(EOL);
        s.extend(self.news_screen.bytes());
        s.extend(EOL);
        s.extend(self.logoff_screen.bytes());
        s
    }
}

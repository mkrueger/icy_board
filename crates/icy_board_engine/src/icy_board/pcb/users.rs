use std::{fs, path::Path};

use jamjam::util::basic_real::BasicReal;

use crate::{
    Res,
    datetime::{IcbDate, IcbTime},
    tables::import_cp437_string,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbUserRecord {
    pub name: String,
    pub city: String,
    pub password: String,

    pub bus_data_phone: String,
    pub home_voice_phone: String,

    pub last_date_on: IcbDate,
    pub last_time_on: IcbTime,

    pub expert_mode: bool,

    /// Protocol (A->Z)
    pub protocol: char,

    pub is_dirty: bool,
    pub msg_clear: bool,
    pub has_mail: bool,
    pub dont_ask_fse: bool,
    pub use_fsedefault: bool,
    pub scroll_msg_body: bool,
    pub long_msg_header: bool,
    pub short_file_descr: bool,

    pub wide_editor: bool,
    pub is_chat_available: bool,

    ///  Date for Last DIR Scan (most recent file)
    pub date_last_dir_read: IcbDate,
    pub security_level: u8,

    /// Expired security level
    pub exp_security_level: u8,

    /// Number of times the caller has connected
    pub num_times_on: usize,

    /// Page length when display data on the screen
    pub page_len: u8,

    pub num_uploads: i32,
    pub num_downloads: i32,

    pub daily_downloaded_bytes: usize,

    pub user_comment: String,
    pub sysop_comment: String,

    /// Number of minutes online
    pub elapsed_time_on: u16,

    /// Julian date for Registration Expiration Date
    pub exp_date: IcbDate,
    // unsigned short LastConference;     ///  Number of the conference the caller was in
    pub delete_flag: bool,
    pub rec_num: u32,

    pub last_conference: u16,

    /// Conference Registration Flags (conf 0-39)
    pub conf_reg_flags: [u8; 5],
    /// Expired Registration Conference Flags (conf 0-39)
    pub conf_exp_flags: [u8; 5],
    /// User Selected Conference Flags (conf 0-39)
    pub conf_usr_flags: [u8; 5],

    pub last_message_read_ptr: Vec<i32>,

    pub ul_tot_dnld_bytes: u64,
    pub ul_tot_upld_bytes: u64,
}

impl PcbUserRecord {
    pub const RECORD_SIZE: u64 = 0x190;

    /// # Errors
    pub fn read_users(path: &Path) -> Res<Vec<PcbUserRecord>> {
        let mut users = Vec::new();

        let data = fs::read(path)?;

        let mut data = &data[..];
        while !data.is_empty() {
            let name = import_cp437_string(&data[..25], true);
            data = &data[25..];
            let city = import_cp437_string(&data[..24], true);
            data = &data[24..];
            let password = import_cp437_string(&data[..12], true);
            data = &data[12..];
            let bus_data_phone = import_cp437_string(&data[..13], true);
            data = &data[13..];
            let home_voice_phone = import_cp437_string(&data[..13], true);
            data = &data[13..];

            let last_date_on = import_cp437_string(&data[..6], true);
            data = &data[6..];
            let last_date_on = IcbDate::parse(&last_date_on);

            let last_time_on = import_cp437_string(&data[..5], true);
            data = &data[5..];
            let last_time_on = IcbTime::parse(&last_time_on);

            let expert_mode = data[0] == b'Y';
            let protocol = data[1] as char;

            let packet_flags = data[2];

            /*
             Bit 0 = Dirty Flag (used to indicate another process updated the record)
             Bit 1 = Clear Screen Between Messages
             Bit 2 = Has Mail Flag
             Bit 3 = Don't Ask for Full Screen Editor Use
             Bit 4 = Full Screen Editor Default
             Bit 5 = Scroll Message Body
             Bit 6 = Use Short Message Headers
             Bit 7 = Use Wide (79-column) Editor
            */
            let is_dirty = (packet_flags & (1 << 0)) != 0;
            let msg_clear = (packet_flags & (1 << 1)) != 0;
            let has_mail = (packet_flags & (1 << 2)) != 0;
            let dont_ask_fse = (packet_flags & (1 << 3)) != 0;
            let use_fsedefault = (packet_flags & (1 << 4)) != 0;
            let scroll_msg_body = (packet_flags & (1 << 5)) != 0;
            let long_msg_header = (packet_flags & (1 << 6)) == 0;
            let wide_editor = (packet_flags & (1 << 7)) != 0;
            data = &data[3..];

            let date_last_dir_read = import_cp437_string(&data[..6], true);
            data = &data[6..];
            let date_last_dir_read = IcbDate::parse(&date_last_dir_read);

            let security_level = data[0];
            data = &data[1..];

            let num_times_on: u16 = u16::from_le_bytes([data[0], data[1]]);
            data = &data[2..];
            let page_len = data[0];
            data = &data[1..];

            let num_uploads = u16::from_le_bytes([data[0], data[1]]);
            data = &data[2..];

            let num_downloads = u16::from_le_bytes([data[0], data[1]]);
            data = &data[2..];

            let daily_downloaded_bytes = import_cp437_string(&data[..8], true);
            data = &data[8..];
            let daily_downloaded_bytes = daily_downloaded_bytes.parse::<u32>().unwrap_or_default();

            let user_comment = import_cp437_string(&data[..30], true);
            data = &data[30..];
            let sysop_comment = import_cp437_string(&data[..30], true);
            data = &data[30..];

            let elapsed_time_on = u16::from_le_bytes([data[0], data[1]]);
            data = &data[2..];

            let reg_exp_date = import_cp437_string(&data[..6], true);
            data = &data[6..];
            let reg_exp_date = IcbDate::parse(&reg_exp_date);

            let exp_security_level = data[0];
            data = &data[1..];
            let _last_conference_old = data[0];
            data = &data[1..];

            let mut conf_reg_flags = [0; 5];
            conf_reg_flags.clone_from_slice(&data[0..5]);
            data = &data[5..];

            let mut conf_exp_flags = [0; 5];
            conf_exp_flags.clone_from_slice(&data[0..5]);
            data = &data[5..];

            let mut conf_sel_flags = [0; 5];
            conf_sel_flags.clone_from_slice(&data[0..5]);
            data = &data[5..];

            let ul_tot_dnld_bytes = import_cp437_string(&data[..8], true);
            data = &data[8..];
            let ul_tot_dnld_bytes = ul_tot_dnld_bytes.parse::<u32>().unwrap_or_default();

            let ul_tot_upld_bytes = import_cp437_string(&data[..8], true);
            data = &data[8..];
            let ul_tot_upld_bytes = ul_tot_upld_bytes.parse::<u32>().unwrap_or_default();

            let delete_flag = data[0] == b'Y';
            data = &data[1..];

            // Last Message Read pointer - skip that
            let mut last_message_read_ptr = vec![];
            for _ in 0..40 {
                let real = BasicReal::from([data[0], data[1], data[2], data[3]]);
                last_message_read_ptr.push(real.into());
                data = &data[4..];
            }

            let rec_num = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) - 1;
            data = &data[4..];

            // flags 2
            let flags2 = data[0];
            data = &data[1..];

            // Bit 0 = Chat Status - OFF=Available, ON=unavailable
            let is_chat_available = (flags2 & (1 << 0)) == 0;
            // Bit 1 = Short File Description
            let short_file_descr = (flags2 & (1 << 1)) == 0;

            // resevered
            data = &data[8..];
            let last_conference = u16::from_le_bytes([data[0], data[1]]);
            data = &data[2..];

            let user = PcbUserRecord {
                name,
                city,
                password,
                bus_data_phone,
                home_voice_phone,
                last_date_on,
                last_time_on,
                expert_mode,
                protocol: protocol as char,

                is_dirty,
                msg_clear,
                has_mail,
                dont_ask_fse,
                use_fsedefault,
                scroll_msg_body,
                long_msg_header,
                wide_editor,
                is_chat_available,
                date_last_dir_read,
                security_level,
                num_times_on: num_times_on as usize,
                page_len,
                num_uploads: num_uploads as i32,
                num_downloads: num_downloads as i32,
                daily_downloaded_bytes: daily_downloaded_bytes as usize,
                user_comment,
                sysop_comment,
                elapsed_time_on,
                exp_date: reg_exp_date,
                exp_security_level,
                last_conference,

                conf_reg_flags,
                conf_exp_flags,
                conf_usr_flags: conf_sel_flags,

                last_message_read_ptr,

                ul_tot_dnld_bytes: ul_tot_dnld_bytes as u64,
                ul_tot_upld_bytes: ul_tot_upld_bytes as u64,
                delete_flag,
                rec_num,
                short_file_descr,
            };

            users.push(user);
        }
        Ok(users)
    }
}

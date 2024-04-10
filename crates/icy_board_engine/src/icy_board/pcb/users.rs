use std::{
    fs,
    io::{Cursor, Read},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};
use icy_ppe::{datetime::IcbDate, Res};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbUserRecord {
    pub name: String,
    pub city: String,
    pub password: String,

    pub bus_data_phone: String,
    pub home_voice_phone: String,

    pub last_date_on: IcbDate,
    pub last_time_on: String,

    pub expert_mode: bool,

    /// Protocol (A->Z)
    pub protocol: char,

    pub is_dirty: bool,
    pub msg_clear: bool,
    pub has_mail: bool,
    pub dont_ask_fse: bool,
    pub use_fsedefault: bool,
    pub scroll_msg_body: bool,
    pub short_header: bool,
    pub wide_editor: bool,

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
    pub rec_num: usize,

    pub last_conference: u16,
    pub ul_tot_dnld_bytes: u64,
    pub ul_tot_upld_bytes: u64,
}

impl PcbUserRecord {
    /// # Errors
    pub fn read_users(path: &Path) -> Res<Vec<PcbUserRecord>> {
        const RECORD_SIZE: u64 = 0x190;
        let mut users = Vec::new();

        let data = fs::read(path)?;
        let mut cursor = Cursor::new(data);
        while cursor.position() + RECORD_SIZE <= cursor.get_ref().len() as u64 {
            let mut name = [0u8; 25];
            cursor.read_exact(&mut name)?;

            let mut city = [0u8; 24];
            cursor.read_exact(&mut city)?;

            let mut password = [0u8; 12];
            cursor.read_exact(&mut password)?;

            let mut data_phone = [0u8; 13];
            cursor.read_exact(&mut data_phone)?;

            let mut voice_phone = [0u8; 13];
            cursor.read_exact(&mut voice_phone)?;

            let mut last_date_on = [0u8; 6];
            cursor.read_exact(&mut last_date_on)?;

            let last_date_on = IcbDate::parse(&String::from_utf8_lossy(&last_date_on));
            let mut last_time_on = [0u8; 5];

            cursor.read_exact(&mut last_time_on)?;

            let expert_mode = cursor.read_u8()?;
            let protocol = cursor.read_u8()?;

            let packet_flags = cursor.read_u8()?;

            let is_dirty = (packet_flags & (1 << 0)) != 0;
            let msg_clear = (packet_flags & (1 << 1)) != 0;
            let has_mail = (packet_flags & (1 << 2)) != 0;
            let dont_ask_fse = (packet_flags & (1 << 3)) != 0;
            let use_fsedefault = (packet_flags & (1 << 4)) != 0;
            let scroll_msg_body = (packet_flags & (1 << 5)) != 0;
            let short_header = (packet_flags & (1 << 6)) != 0;
            let wide_editor = (packet_flags & (1 << 7)) != 0;

            let mut date_last_dir_read = [0u8; 6];
            cursor.read_exact(&mut date_last_dir_read)?;
            let date_last_dir_read = IcbDate::parse(&String::from_utf8_lossy(&date_last_dir_read));

            let security_level = cursor.read_u8()?;
            let num_times_on = cursor.read_u16::<LittleEndian>()?;
            let page_len = cursor.read_u8()?;
            let num_uploads = cursor.read_u16::<LittleEndian>()?;
            let num_downloads = cursor.read_u16::<LittleEndian>()?;

            let daily_downloaded_bytes = cursor.read_u64::<LittleEndian>()?;

            let mut cmt1 = [0u8; 30];
            cursor.read_exact(&mut cmt1)?;
            let mut cmt2 = [0u8; 30];
            cursor.read_exact(&mut cmt2)?;

            let elapsed_time_on = cursor.read_u16::<LittleEndian>()?;

            let mut reg_exp_date = [0u8; 6];
            cursor.read_exact(&mut reg_exp_date)?;
            let reg_exp_date = IcbDate::parse(&String::from_utf8_lossy(&reg_exp_date));

            let exp_security_level = cursor.read_u8()?;
            let last_conference = cursor.read_u8()? as u16;

            cursor.set_position(cursor.position() + 15);

            let ul_tot_dnld_bytes = cursor.read_u64::<LittleEndian>()?;
            let ul_tot_upld_bytes = cursor.read_u64::<LittleEndian>()?;
            let delete_flag = cursor.read_u8()? != 0;
            cursor.set_position(cursor.position() + 40 * 4);

            let rec_num = cursor.read_u32::<LittleEndian>()? as usize;
            // flags byte ?
            cursor.read_u8()?;
            // reseved bytes 2
            cursor.set_position(cursor.position() + 8);
            let _last_conference2 = cursor.read_u16::<LittleEndian>()? as i32;

            let user = PcbUserRecord {
                name: String::from_utf8_lossy(&name).trim().to_string(),
                city: String::from_utf8_lossy(&city).trim().to_string(),
                password: String::from_utf8_lossy(&password).trim().to_string(),
                bus_data_phone: String::from_utf8_lossy(&data_phone).trim().to_string(),
                home_voice_phone: String::from_utf8_lossy(&voice_phone).trim().to_string(),
                last_date_on,
                last_time_on: String::from_utf8_lossy(&last_time_on).trim().to_string(),
                expert_mode: expert_mode == b'Y',
                protocol: protocol as char,

                is_dirty,
                msg_clear,
                has_mail,
                dont_ask_fse,
                use_fsedefault,
                scroll_msg_body,
                short_header,
                wide_editor,

                date_last_dir_read,
                security_level,
                num_times_on: num_times_on as usize,
                page_len,
                num_uploads: num_uploads as i32,
                num_downloads: num_downloads as i32,
                daily_downloaded_bytes: daily_downloaded_bytes as usize,
                user_comment: String::from_utf8_lossy(&cmt1).trim().to_string(),
                sysop_comment: String::from_utf8_lossy(&cmt2).trim().to_string(),
                elapsed_time_on,
                exp_date: reg_exp_date,
                exp_security_level,
                last_conference,
                ul_tot_dnld_bytes,
                ul_tot_upld_bytes,
                delete_flag,
                rec_num,
            };

            users.push(user);
        }
        Ok(users)
    }
}

use std::{fs::File, io::BufReader, path::Path};

use icy_ppe::{
    parser::Encoding,
    tables::{import_cp437_string, UNICODE_TO_CP437},
    Res,
};

use crate::icy_board::pcboard_data::{
    append_bool, append_int, append_line, read_bool, read_int, read_line,
};

/// Only for compatibility with very old PCBoard PPEs
/// Even pcboard itself only generates that for compatiblity purposes
#[derive(Default, Clone, Debug)]
pub struct PcbLegacyConferenceHeader {
    /// Conference name - 14 chars
    pub name: String,

    pub public_conf: bool,
    pub auto_rejoin: bool,
    pub view_members: bool,
    pub priv_uplds: bool,
    pub priv_msgs: bool,
    pub echo_mail: bool,

    pub req_sec_level: u16,
    pub add_sec: u16,
    pub add_time: u16,

    pub msg_blocks: u8,

    pub msg_file: String,
    pub user_menu: String,
    pub sysop_menu: String,
    pub news_file: String,

    pub pub_upld_sort: u8,
    pub upld_dir: String,
    pub pub_upld_loc: String,
    pub prv_upld_sort: u8,

    pub priv_dir: String,
    pub prv_upld_loc: String,

    pub drs_menu: String,
    pub drs_file: String,

    pub blt_menu: String,
    pub blt_name_loc: String,

    pub scr_menu: String,
    pub scr_name_loc: String,

    pub dir_menu: String,
    pub dir_name_loc: String,

    pub pth_name_loc: String,
}

impl PcbLegacyConferenceHeader {
    pub const HEADER_SIZE: usize = 0x224;

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();

        append_cp437(&mut res, &self.name, 14);
        res.push(if self.public_conf { 1 } else { 0 });
        res.push(if self.auto_rejoin { 1 } else { 0 });
        res.push(if self.view_members { 1 } else { 0 });
        res.push(if self.priv_uplds { 1 } else { 0 });
        res.push(if self.priv_msgs { 1 } else { 0 });
        res.push(if self.echo_mail { 1 } else { 0 });
        res.extend(&self.req_sec_level.to_le_bytes());
        res.extend(&self.add_sec.to_le_bytes());
        res.extend(&self.add_time.to_le_bytes());
        res.push(self.msg_blocks);
        append_cp437(&mut res, &self.msg_file, 32);
        append_cp437(&mut res, &self.user_menu, 32);
        append_cp437(&mut res, &self.sysop_menu, 32);
        append_cp437(&mut res, &self.news_file, 32);

        res.push(self.pub_upld_sort);
        append_cp437(&mut res, &self.upld_dir, 29);
        append_cp437(&mut res, &self.pub_upld_loc, 26);

        res.push(self.prv_upld_sort);
        append_cp437(&mut res, &self.priv_dir, 29);
        append_cp437(&mut res, &self.prv_upld_loc, 26);

        append_cp437(&mut res, &self.drs_menu, 29);
        append_cp437(&mut res, &self.drs_file, 33);

        append_cp437(&mut res, &self.blt_menu, 29);
        append_cp437(&mut res, &self.blt_name_loc, 33);

        append_cp437(&mut res, &self.scr_menu, 29);
        append_cp437(&mut res, &self.scr_name_loc, 33);

        append_cp437(&mut res, &self.dir_menu, 29);
        append_cp437(&mut res, &self.dir_name_loc, 33);

        append_cp437(&mut res, &self.pth_name_loc, 33);

        res
    }
}

fn append_cp437(res: &mut Vec<u8>, name: &str, arg: i32) {
    for i in 0..arg {
        if (i as usize) < name.len() {
            if let Some(ch) = UNICODE_TO_CP437.get(&name.chars().nth(i as usize).unwrap()) {
                res.push(*ch);
            } else {
                res.push(b'*');
            }
        } else {
            res.push(0);
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct PcbConferenceHeader {
    pub name: String,
    pub auto_rejoin: bool,
    pub view_members: bool,
    pub private_uploads: bool,
    pub private_msgs: bool,
    pub echo_mail: bool,
    pub add_conference_security: i32,
    pub add_conference_time: i32,
    pub message_blocks: i32,
    pub message_file: String,
    pub users_menu: String,
    pub sysop_menu: String,
    pub news_file: String,

    /// Sort type for public upload DIR file
    pub pub_upload_sort: u8,
    pub pub_upload_dirfile: String,
    pub pub_upload_location: String,

    pub private_upload_sort: u8,
    pub private_upload_dirfile: String,
    pub private_upload_location: String,

    pub public_conference: bool,

    pub doors_menu: String,
    pub doors_file: String,
    pub required_security: u8,

    pub blt_menu: String,
    pub blt_file: String,
    // empty string
    pub script_menu: String,
    pub script_file: String,
    // empty string
    pub dir_menu: String,
    pub dir_file: String,
    // empty string
    pub dlpth_list_file: String,
}

impl PcbConferenceHeader {
    #[allow(clippy::field_reassign_with_default)]
    pub fn deserialize(reader: &mut BufReader<File>) -> Res<Self> {
        let encoding = Encoding::CP437;
        let mut ret = Self::default();
        ret.name = read_line(reader, encoding)?;
        ret.auto_rejoin = read_bool(reader, encoding)?;
        ret.view_members = read_bool(reader, encoding)?;
        ret.private_uploads = read_bool(reader, encoding)?;
        ret.private_msgs = read_bool(reader, encoding)?;
        ret.echo_mail = read_bool(reader, encoding)?;

        ret.add_conference_security = read_int(reader, encoding)?;
        ret.add_conference_time = read_int(reader, encoding)?;
        ret.message_blocks = read_int(reader, encoding)?;

        ret.message_file = read_line(reader, encoding)?;
        ret.users_menu = read_line(reader, encoding)?;
        ret.sysop_menu = read_line(reader, encoding)?;
        ret.news_file = read_line(reader, encoding)?;
        ret.pub_upload_sort = read_int(reader, encoding)? as u8;
        ret.pub_upload_dirfile = read_line(reader, encoding)?;
        ret.pub_upload_location = read_line(reader, encoding)?;
        ret.private_upload_sort = read_int(reader, encoding)? as u8;
        ret.private_upload_dirfile = read_line(reader, encoding)?;
        ret.private_upload_location = read_line(reader, encoding)?;
        ret.public_conference = read_bool(reader, encoding)?;
        ret.doors_menu = read_line(reader, encoding)?;
        ret.doors_file = read_line(reader, encoding)?;
        ret.required_security = read_int(reader, encoding)? as u8;
        ret.blt_menu = read_line(reader, encoding)?;
        ret.blt_file = read_line(reader, encoding)?;
        read_line(reader, encoding)?;
        ret.script_menu = read_line(reader, encoding)?;
        ret.script_file = read_line(reader, encoding)?;
        read_line(reader, encoding)?;
        ret.dir_menu = read_line(reader, encoding)?;
        ret.dir_file = read_line(reader, encoding)?;
        read_line(reader, encoding)?;
        ret.dlpth_list_file = read_line(reader, encoding)?;
        Ok(ret)
    }

    pub fn import_pcboard<P: AsRef<Path>>(path: &P, len: usize) -> Res<Vec<Self>> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut ret = Vec::new();
        for _ in 0..=len {
            ret.push(Self::deserialize(&mut reader)?);
        }

        Ok(ret)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();
        let encoding = Encoding::CP437;
        append_line(&mut res, encoding, &self.name);
        append_bool(&mut res, encoding, self.auto_rejoin);
        append_bool(&mut res, encoding, self.view_members);
        append_bool(&mut res, encoding, self.private_uploads);
        append_bool(&mut res, encoding, self.private_msgs);
        append_bool(&mut res, encoding, self.echo_mail);

        append_int(&mut res, encoding, self.add_conference_security);
        append_int(&mut res, encoding, self.add_conference_time);
        append_int(&mut res, encoding, self.message_blocks);

        append_line(&mut res, encoding, &self.message_file);
        append_line(&mut res, encoding, &self.users_menu);
        append_line(&mut res, encoding, &self.sysop_menu);
        append_line(&mut res, encoding, &self.news_file);

        append_line(&mut res, encoding, &self.pub_upload_dirfile);
        append_line(&mut res, encoding, &self.pub_upload_location);
        append_int(&mut res, encoding, self.pub_upload_sort as i32);
        append_line(&mut res, encoding, &self.private_upload_dirfile);
        append_line(&mut res, encoding, &self.private_upload_location);
        append_int(&mut res, encoding, self.private_upload_sort as i32);
        append_bool(&mut res, encoding, self.public_conference);
        append_line(&mut res, encoding, &self.doors_menu);
        append_line(&mut res, encoding, &self.doors_file);
        append_int(&mut res, encoding, self.required_security as i32);
        append_line(&mut res, encoding, &self.blt_menu);
        append_line(&mut res, encoding, &self.blt_file);
        res.extend(b"\r\n");
        append_line(&mut res, encoding, &self.script_menu);
        append_line(&mut res, encoding, &self.script_file);
        res.extend(b"\r\n");
        append_line(&mut res, encoding, &self.dir_menu);
        append_line(&mut res, encoding, &self.dir_file);
        res.extend(b"\r\n");
        append_line(&mut res, encoding, &self.dlpth_list_file);
        res
    }
}

#[derive(Default, Clone, Debug)]
pub struct PcbAdditionalConferenceHeader {
    pub force_echo: bool,
    pub read_only: bool, //*
    pub no_private_msgs: bool,
    pub ret_receipt_level: u8,
    pub record_origin: bool,
    pub prompt_for_routing: bool,
    pub allow_aliases: bool, // *
    pub show_intro_on_ra: bool,
    pub req_level_to_enter: u8, // *
    pub password: String,       // *
    pub intro: String,          // *
    pub attach_loc: String,     // *
    pub reg_flags: u32,
    pub attach_level: u8,
    pub carbon_limit: u8,
    pub cmd_lst: String, // *
    pub old_index: bool,
    pub long_to_names: bool,
    pub carbon_level: u8,
    pub conf_type: u8,
    pub export_ptr: u32,
    pub charge_time: f32,
    pub charge_msg_read: f32,
    pub charge_msg_write: f32,
}

impl PcbAdditionalConferenceHeader {
    const RECORD_SIZE: usize = 256;

    pub fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Vec<Self>> {
        let mut res = Vec::new();
        let data = &std::fs::read(path)?;
        let mut data = &data[..];
        while !data.is_empty() {
            let force_echo = data[0] != 0;
            let read_only = data[1] != 0;
            let no_private_msgs = data[2] != 0;
            let ret_receipt_level = data[3];
            let record_origin = data[4] != 0;
            let prompt_for_routing = data[5] != 0;
            let allow_aliases = data[6] != 0;
            let show_intro_on_ra = data[7] != 0;
            let req_level_to_enter = data[8];
            let password = import_cp437_string(&data[9..22], true);
            let intro = import_cp437_string(&data[22..54], true);
            let attach_loc = import_cp437_string(&data[54..86], true);
            let reg_flags = u32::from_le_bytes(data[86..90].try_into()?);
            let attach_level = data[90];
            let carbon_limit = data[91];
            let cmd_lst = import_cp437_string(&data[92..124], true);
            let old_index = data[124] != 0;
            let long_to_names = data[125] != 0;
            let carbon_level = data[126];
            let conf_type = data[127];
            let export_ptr = u32::from_le_bytes(data[128..132].try_into()?);
            let charge_time = f32::from_le_bytes(data[132..136].try_into()?);
            let charge_msg_read = f32::from_le_bytes(data[136..140].try_into()?);
            let charge_msg_write = f32::from_le_bytes(data[140..144].try_into()?);

            res.push(Self {
                force_echo,
                read_only,
                no_private_msgs,
                ret_receipt_level,
                record_origin,
                prompt_for_routing,
                allow_aliases,
                show_intro_on_ra,
                req_level_to_enter,
                password,
                intro,
                attach_loc,
                reg_flags,
                attach_level,
                carbon_limit,
                cmd_lst,
                old_index,
                long_to_names,
                carbon_level,
                conf_type,
                export_ptr,
                charge_time,
                charge_msg_read,
                charge_msg_write,
            });
            data = &data[Self::RECORD_SIZE..];
        }
        Ok(res)
    }

    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();
        res.push(if self.force_echo { 1 } else { 0 });
        res.push(if self.read_only { 1 } else { 0 });
        res.push(if self.no_private_msgs { 1 } else { 0 });
        res.push(self.ret_receipt_level);
        res.push(if self.record_origin { 1 } else { 0 });
        res.push(if self.prompt_for_routing { 1 } else { 0 });
        res.push(if self.allow_aliases { 1 } else { 0 });
        res.push(if self.show_intro_on_ra { 1 } else { 0 });
        res.push(self.req_level_to_enter);
        res.extend(self.password.as_bytes());
        res.extend(&vec![0; 13 - self.password.len()]);
        res.extend(self.intro.as_bytes());
        res.extend(&vec![0; 32 - self.intro.len()]);
        res.extend(self.attach_loc.as_bytes());
        res.extend(&vec![0; 32 - self.attach_loc.len()]);
        res.extend(&self.reg_flags.to_le_bytes());
        res.push(self.attach_level);
        res.push(self.carbon_limit);
        res.extend(self.cmd_lst.as_bytes());
        res.extend(&vec![0; 32 - self.cmd_lst.len()]);
        res.push(if self.old_index { 1 } else { 0 });
        res.push(if self.long_to_names { 1 } else { 0 });
        res.push(self.carbon_level);
        res.push(self.conf_type);
        res.extend(&self.export_ptr.to_le_bytes());
        res.extend(&self.charge_time.to_le_bytes());
        res.extend(&self.charge_msg_read.to_le_bytes());
        res.extend(&self.charge_msg_write.to_le_bytes());

        while res.len() < Self::RECORD_SIZE {
            res.push(0);
        }
        res
    }
}

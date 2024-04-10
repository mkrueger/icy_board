use std::{collections::HashMap, path::PathBuf};

use icy_ppe::Res;
use qfile::{QFilePath, QTraitSync};

use self::{
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader},
    pcboard_data::PcbBoardData,
    user_inf::PcbUserInf,
    users::PcbUserRecord,
};

use super::{icb_text::IcbTextFile, IcyBoardError};

pub mod pcbconferences;
pub mod pcboard_data;
pub mod user_inf;
pub mod users;

#[derive(Clone)]
pub struct PcbUser {
    pub user: PcbUserRecord,
    pub inf: PcbUserInf,
}

impl PcbUser {
    pub fn get_first_name(&self) -> String {
        if let Some(idx) = self.user.name.find(' ') {
            self.user.name[..idx].to_string()
        } else {
            self.user.name.clone()
        }
    }
    pub fn get_last_name(&self) -> String {
        if let Some(idx) = self.user.name.find(' ') {
            self.user.name[idx + 1..].to_string()
        } else {
            String::new()
        }
    }
}

pub struct PcbBoard {
    pub file_name: String,
    pub users: Vec<PcbUser>,
    pub data: PcbBoardData,
    pub conferences: Vec<PcbConferenceHeader>,
    pub add_conferences: Vec<PcbAdditionalConferenceHeader>,
    pub display_text: IcbTextFile,

    // TODO: proper board statistics.
    pub num_callers: usize,

    pub paths: HashMap<String, String>,
}

impl PcbBoard {
    pub fn new() -> Self {
        let users = Vec::new();
        let conferences = Vec::new();
        let data = PcbBoardData::default();
        let display_text = IcbTextFile::default();
        let paths = HashMap::new();

        PcbBoard {
            file_name: String::new(),
            num_callers: 0,
            users,
            data,
            conferences,
            add_conferences: Vec::new(),
            display_text,
            paths,
        }
    }

    pub fn resolve_file(&self, file: &str) -> String {
        let mut s: String = file
            .chars()
            .map(|x| match x {
                '\\' => '/',
                _ => x,
            })
            .collect();

        for (k, v) in &self.paths {
            if s.starts_with(k) {
                s = v.clone() + &s[k.len()..];
            }
        }

        if let Ok(mut file_path) = QFilePath::add_path(s.clone()) {
            if let Ok(file) = file_path.get_path_buf() {
                return file.to_string_lossy().to_string();
            }
        }
        s
    }

    pub fn load(file: &str) -> Res<PcbBoard> {
        let data = PcbBoardData::import_pcboard(file)?;
        let users = Vec::new();
        let conferences = Vec::new();
        let mut paths = HashMap::new();

        let file_path = PathBuf::from(file);
        let mut help = data.path.help_loc.clone();
        if help.ends_with('\\') {
            help.pop();
        }

        help = help.replace('\\', "/");

        let help_loc = PathBuf::from(&help);
        let mut path = file_path.parent().unwrap().to_path_buf();
        path.push(help_loc.file_name().unwrap());
        if !path.exists() {
            return Err(Box::new(IcyBoardError::Error(
                "Can't resolve C: file".to_string(),
            )));
        }

        //let len = to_str().unwrap().len();
        let k = help_loc.parent().unwrap().to_str().unwrap().to_string();
        let v = file_path
            .parent()
            .unwrap()
            .to_path_buf()
            .to_str()
            .unwrap()
            .to_string();
        paths.insert(k, v);
        let mut res = PcbBoard {
            file_name: file.to_string(),
            num_callers: 0,
            users,
            data,
            conferences,
            add_conferences: Vec::new(),
            display_text: IcbTextFile::default(),
            paths,
        };

        let r = res.resolve_file(&res.data.path.usr_file);
        let users = PcbUserRecord::read_users(&PathBuf::from(&r))?;

        let r = res.resolve_file(&res.data.path.inf_file);
        let user_inf = PcbUserInf::read_users(&PathBuf::from(&r))?;
        for user in users {
            let inf = user_inf[user.rec_num - 1].clone();
            res.users.push(PcbUser { user, inf });
        }

        let r = res.resolve_file(&res.data.path.conference_file);
        let max_conferences = res.data.num_conf as usize;
        let conferences = PcbConferenceHeader::import_pcboard(&r, max_conferences)?;
        res.conferences = conferences;

        let r = res.resolve_file(&(res.data.path.conference_file.to_string() + ".ADD"));
        let add_conferences = PcbAdditionalConferenceHeader::import_pcboard(&r)?;
        res.add_conferences = add_conferences;

        let pcb_text = res.resolve_file(&res.data.path.text_loc) + "/PCBTEXT";
        res.display_text = IcbTextFile::load(&pcb_text)?;
        /*
                let r = res.resolve_file(&res.conferences[0].message_file);
                let hdr = MessageBaseHeader::deserialize(&mut Cursor::new(&fs::read(r)?))?;
                res.num_callers = hdr.num_callers as usize;
        */
        Ok(res)
    }
}

impl Default for PcbBoard {
    fn default() -> Self {
        Self::new()
    }
}

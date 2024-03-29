use std::{collections::HashMap, fs, path::PathBuf};

use call_wait_screen::App;
use clap::Parser;
use icy_board_engine::icy_board::{
    conferences::ConferenceHeader, data::IcyBoardData, text_messages::DisplayText,
    user_inf::UserInf, users::UserRecord, User,
};
use icy_ppe::Res;
use qfile::{QFilePath, QTraitSync};
use semver::Version;
use thiserror::Error;

pub mod call_stat;
mod call_wait_screen;

#[derive(clap::Parser)]
#[command(version="", about="IcyBoard BBS", long_about = None)]
struct Cli {
    /// PCBOARD.DAT file to run
    file: String,
}

#[derive(Error, Debug)]
enum IcyBoardError {
    #[error("Error: {0}")]
    Error(String),
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

pub struct IcyBoard {
    pub users: Vec<User>,
    pub data: IcyBoardData,
    pub conferences: Vec<ConferenceHeader>,
    pub display_text: DisplayText,

    paths: HashMap<String, String>,
}

impl IcyBoard {
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

    fn load(file: &str) -> Res<IcyBoard> {
        let data = IcyBoardData::deserialize(file)?;
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
        let mut res = IcyBoard {
            users,
            data,
            conferences,
            display_text: DisplayText::default(),
            paths,
        };

        let r = res.resolve_file(&res.data.path.usr_file);
        let users = UserRecord::read_users(&PathBuf::from(&r))?;

        let r = res.resolve_file(&res.data.path.inf_file);
        let user_inf = UserInf::read_users(&PathBuf::from(&r))?;
        for user in users {
            let inf = user_inf[user.rec_num].clone();
            res.users.push(User { user, inf });
        }

        let r = res.resolve_file(&res.data.path.conference_file);
        let max_conferences = res.data.num_conf as usize;
        let conferences = ConferenceHeader::load(&r, max_conferences)?;
        res.conferences = conferences;
        let txt = fs::read(res.resolve_file(&res.data.path.text_loc) + "/PCBTEXT")?;
        res.display_text = DisplayText::parse_file(&txt)?;

        Ok(res)
    }
}

fn main() {
    let arguments = Cli::parse();

    match IcyBoard::load(&arguments.file) {
        Ok(icy_board) => {
            if let Err(err) = App::run(icy_board) {
                println!("Error: {}", err);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

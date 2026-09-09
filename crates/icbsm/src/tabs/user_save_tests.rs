use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    user_base::{User, UserBase},
};

pub(crate) struct Fixture {
    pub dir: PathBuf,
    pub board: Arc<Mutex<IcyBoard>>,
    pub persisted: Vec<u8>,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("icbsm-user-save-{}-{:016x}", std::process::id(), fastrand::u64(..)));
        std::fs::create_dir(&dir).unwrap();
        let mut board = IcyBoard::default();
        board.root_path = dir.clone();
        board.config.paths.user_file = dir.join("users.toml");
        for name in ["Sysop", "Charlie", "Bob"] {
            board.users.new_user(User {
                name: name.into(),
                city_or_state: "Berlin".into(),
                home_voice_phone: "(123) 456-7890".into(),
                security_level: 10,
                exp_security_level: 20,
                ..Default::default()
            });
        }
        board.edit_users(|_| Ok(())).unwrap();
        let persisted = std::fs::read(&board.config.paths.user_file).unwrap();
        Self {
            dir,
            board: Arc::new(Mutex::new(board)),
            persisted,
        }
    }

    pub fn snapshot(&self) -> Vec<User> {
        self.board.lock().unwrap().users.iter().cloned().collect()
    }

    pub fn assert_unchanged(&self, before: &[User]) {
        let board = self.board.lock().unwrap();
        assert_eq!(board.users.len(), before.len());
        for (live, before) in board.users.iter().zip(before) {
            assert!(super::user_editor::same_user_edit(live, before), "live user changed after failed save");
        }
        assert!(std::fs::read(self.dir.join("users.toml")).unwrap() == self.persisted);
    }

    pub fn disk_users(&self) -> UserBase {
        UserBase::load(&self.dir.join("users.toml")).unwrap()
    }

    #[cfg(unix)]
    pub fn fail_serialization(&self) -> Vec<User> {
        use std::os::unix::ffi::OsStringExt;
        let mut board = self.board.lock().unwrap();
        // Backup remains readable; staged serialization fails after security normalization.
        board.users[0].path = Some(std::ffi::OsString::from_vec(vec![0xff]).into());
        board.users[0].email = "pending-normalization@example.invalid".into();
        drop(board);
        self.snapshot()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

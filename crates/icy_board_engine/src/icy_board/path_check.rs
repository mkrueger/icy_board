//! Walking the paths a board is configured with, the way PCBSetup did on a full
//! save. See `writefile` in DATAWRIT.C and `checkexistence` in CHKEXIST.C.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::{IcyBoard, IcyBoardSerializer, bulletins::BullettinList, file_directory::DirectoryList, lookup_case_insensitive, surveys::SurveyList};

/// Extensions a display file is found under. See `find_file_with_extension`.
const DISPLAY_EXTENSIONS: [&str; 5] = ["pcb", "ans", "avt", "rip", "asc"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathKind {
    File,
    Directory,
    /// Written without an extension - the graphics, security and language
    /// variants all count as the file being there.
    DisplayFile,
    /// A display file that a PPE of the same name may stand in for.
    Menu,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PathProblem {
    Missing,
    /// There under a different spelling. DOS did not care, this does.
    WrongCase(PathBuf),
    ExpectedFileFoundDirectory,
    ExpectedDirectoryFoundFile,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PathReport {
    /// Where the path is written down, in the words the sysop sees.
    pub context: String,
    pub path: PathBuf,
    pub kind: PathKind,
    pub problem: PathProblem,
}

impl std::fmt::Display for PathReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.problem {
            PathProblem::Missing => write!(f, "{}: {} does not exist", self.context, self.path.display()),
            PathProblem::WrongCase(found) => write!(f, "{}: {} is spelled {} on disk", self.context, self.path.display(), found.display()),
            PathProblem::ExpectedFileFoundDirectory => write!(f, "{}: {} is a directory, a file was expected", self.context, self.path.display()),
            PathProblem::ExpectedDirectoryFoundFile => write!(f, "{}: {} is a file, a directory was expected", self.context, self.path.display()),
        }
    }
}

impl IcyBoard {
    /// Every path in the configuration that does not lead where it says.
    ///
    /// Paths the board creates as it runs - logs, answer files, message bases,
    /// file base metadata - are left alone, as is anything left blank.
    pub fn check_paths(&self) -> Vec<PathReport> {
        let mut reports = Vec::new();
        let mut seen_lists = HashSet::new();

        for (context, path, kind) in self.configured_paths() {
            self.check(context, path, kind, &mut reports);
        }

        for (number, conference) in self.conferences.iter().enumerate() {
            let name = format!("Conference {} ({})", number, conference.name);
            for (label, path, kind) in conference_paths(conference) {
                self.check(format!("{name}, {label}"), path.clone(), kind, &mut reports);
            }

            self.check_bulletins(&name, &conference.blt_file, &mut seen_lists, &mut reports);
            self.check_directories(&name, &conference.dir_file, &mut seen_lists, &mut reports);
            self.check_surveys(&name, &conference.survey_file, &mut seen_lists, &mut reports);
        }

        reports
    }

    fn check(&self, context: String, path: PathBuf, kind: PathKind, reports: &mut Vec<PathReport>) {
        if path.as_os_str().is_empty() {
            return;
        }
        let resolved = if path.is_absolute() { path.clone() } else { self.root_path.join(&path) };
        if let Some(problem) = check_path(&resolved, kind) {
            reports.push(PathReport { context, path, kind, problem });
        }
    }

    fn configured_paths(&self) -> Vec<(String, PathBuf, PathKind)> {
        let paths = &self.config.paths;
        let mut list = vec![
            ("System files, Conferences", paths.conferences.clone(), PathKind::File),
            ("System files, User file", paths.user_file.clone(), PathKind::File),
            ("System files, Group file", paths.group_file.clone(), PathKind::File),
            ("System files, Display text", paths.icbtext.clone(), PathKind::File),
            ("System files, Command file", paths.command_file.clone(), PathKind::File),
            ("System files, Language file", paths.language_file.clone(), PathKind::File),
            ("System files, Protocol file", paths.protocol_data_file.clone(), PathKind::File),
            ("System files, Security levels", paths.pwrd_sec_level_file.clone(), PathKind::File),
            ("System files, Help files", paths.help_path.clone(), PathKind::Directory),
            ("System files, Security messages", paths.security_file_path.clone(), PathKind::Directory),
            ("System files, Command display files", paths.command_display_path.clone(), PathKind::Directory),
            ("System files, Temporary work directory", paths.tmp_work_path.clone(), PathKind::Directory),
            ("Configuration files, Trashcan uploads", paths.trashcan_upload_files.clone(), PathKind::File),
            ("Configuration files, Trashcan users", paths.trashcan_user.clone(), PathKind::File),
            ("Configuration files, Trashcan passwords", paths.trashcan_passwords.clone(), PathKind::File),
            ("Configuration files, Trashcan e-mail", paths.trashcan_email.clone(), PathKind::File),
            ("Configuration files, VIP users", paths.vip_users.clone(), PathKind::File),
            ("Display files, Welcome", paths.welcome.clone(), PathKind::DisplayFile),
            ("Display files, New user", paths.newuser.clone(), PathKind::DisplayFile),
            ("Display files, Closed board", paths.closed.clone(), PathKind::DisplayFile),
            ("Display files, Expire warning", paths.expire_warning.clone(), PathKind::DisplayFile),
            ("Display files, Expired", paths.expired.clone(), PathKind::DisplayFile),
            ("Display files, Conference join menu", paths.conf_join_menu.clone(), PathKind::Menu),
            ("Display files, Chat intro", paths.chat_intro_file.clone(), PathKind::DisplayFile),
            ("Display files, Chat menu", paths.chat_menu.clone(), PathKind::Menu),
            ("Display files, Chat actions menu", paths.chat_actions_menu.clone(), PathKind::Menu),
            ("Display files, No ANSI warning", paths.no_ansi.clone(), PathKind::DisplayFile),
            ("New user files, Newask survey", paths.newask_survey.clone(), PathKind::DisplayFile),
            ("New user files, Logon survey", paths.logon_survey.clone(), PathKind::DisplayFile),
            ("New user files, Logoff survey", paths.logoff_survey.clone(), PathKind::DisplayFile),
        ];
        if !paths.ftn_file.as_os_str().is_empty() {
            list.push(("Message networking, FTN configuration", paths.ftn_file.clone(), PathKind::File));
        }
        list.into_iter().map(|(label, path, kind)| (label.to_string(), path, kind)).collect()
    }

    fn check_bulletins(&self, conference: &str, list_file: &Path, seen: &mut HashSet<PathBuf>, reports: &mut Vec<PathReport>) {
        let Some(list) = self.load_list::<BullettinList>(list_file, seen) else {
            return;
        };
        for (number, bulletin) in list.iter().enumerate() {
            self.check(
                format!("{conference}, bulletin {}", number + 1),
                bulletin.path.clone(),
                PathKind::DisplayFile,
                reports,
            );
        }
    }

    fn check_directories(&self, conference: &str, list_file: &Path, seen: &mut HashSet<PathBuf>, reports: &mut Vec<PathReport>) {
        let Some(list) = self.load_list::<DirectoryList>(list_file, seen) else {
            return;
        };
        for directory in list.iter() {
            self.check(
                format!("{conference}, file area '{}'", directory.name),
                directory.path.clone(),
                PathKind::Directory,
                reports,
            );
        }
    }

    fn check_surveys(&self, conference: &str, list_file: &Path, seen: &mut HashSet<PathBuf>, reports: &mut Vec<PathReport>) {
        let Some(list) = self.load_list::<SurveyList>(list_file, seen) else {
            return;
        };
        for (number, survey) in list.iter().enumerate() {
            self.check(
                format!("{conference}, survey {}", number + 1),
                survey.survey_file.clone(),
                PathKind::DisplayFile,
                reports,
            );
        }
    }

    /// Reads a list file once, however many conferences point at it.
    fn load_list<T: IcyBoardSerializer>(&self, list_file: &Path, seen: &mut HashSet<PathBuf>) -> Option<T> {
        if list_file.as_os_str().is_empty() {
            return None;
        }
        let resolved = self.resolve_file(&list_file);
        if !seen.insert(resolved.clone()) {
            return None;
        }
        T::load(&resolved).ok()
    }
}

fn conference_paths(conference: &super::conferences::Conference) -> Vec<(&'static str, &PathBuf, PathKind)> {
    vec![
        ("users menu", &conference.users_menu, PathKind::Menu),
        ("sysop menu", &conference.sysop_menu, PathKind::Menu),
        ("news file", &conference.news_file, PathKind::DisplayFile),
        ("intro file", &conference.intro_file, PathKind::DisplayFile),
        ("attachment location", &conference.attachment_location, PathKind::Directory),
        ("public upload location", &conference.pub_upload_location, PathKind::Directory),
        ("private upload location", &conference.private_upload_location, PathKind::Directory),
        ("command file", &conference.command_file, PathKind::File),
        ("doors menu", &conference.doors_menu, PathKind::Menu),
        ("doors file", &conference.doors_file, PathKind::File),
        ("bulletin menu", &conference.blt_menu, PathKind::Menu),
        ("bulletin list", &conference.blt_file, PathKind::File),
        ("survey menu", &conference.survey_menu, PathKind::Menu),
        ("survey list", &conference.survey_file, PathKind::File),
        ("directory menu", &conference.dir_menu, PathKind::Menu),
        ("directory list", &conference.dir_file, PathKind::File),
        ("area menu", &conference.area_menu, PathKind::Menu),
        ("area list", &conference.area_file, PathKind::File),
    ]
}

/// What is wrong with a resolved path, if anything.
pub fn check_path(resolved: &Path, kind: PathKind) -> Option<PathProblem> {
    match kind {
        PathKind::DisplayFile | PathKind::Menu => {
            if display_file_exists(resolved, kind, Case::Exact) {
                return None;
            }
            match display_file_case(resolved, kind) {
                Some(found) => Some(PathProblem::WrongCase(found)),
                None => Some(PathProblem::Missing),
            }
        }
        PathKind::File | PathKind::Directory => {
            if resolved.exists() {
                return kind_mismatch(resolved, kind);
            }
            let corrected = lookup_case_insensitive(resolved);
            if corrected != resolved && corrected.exists() {
                return Some(PathProblem::WrongCase(corrected));
            }
            Some(PathProblem::Missing)
        }
    }
}

fn kind_mismatch(resolved: &Path, kind: PathKind) -> Option<PathProblem> {
    match kind {
        PathKind::File if resolved.is_dir() => Some(PathProblem::ExpectedFileFoundDirectory),
        PathKind::Directory if resolved.is_file() => Some(PathProblem::ExpectedDirectoryFoundFile),
        _ => None,
    }
}

/// The board appends extensions to a name and asks the file system, so a name
/// that only matches when case is ignored does not answer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Exact,
    Ignored,
}

fn display_file_exists(resolved: &Path, kind: PathKind, case: Case) -> bool {
    find_display_variant(resolved, kind, case).is_some()
}

/// The file that answers for a name written without an extension.
fn find_display_variant(resolved: &Path, kind: PathKind, case: Case) -> Option<PathBuf> {
    if resolved.is_file() {
        return Some(resolved.to_path_buf());
    }
    if kind == PathKind::Menu && resolved.with_extension("ppe").is_file() {
        return Some(resolved.with_extension("ppe"));
    }
    let (Some(directory), Some(stem)) = (resolved.parent(), resolved.file_name().and_then(|name| name.to_str())) else {
        return None;
    };
    let directory = if directory.as_os_str().is_empty() { Path::new(".") } else { directory };
    std::fs::read_dir(directory)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().to_str().is_some_and(|name| is_display_variant(name, stem, case)))
        .map(|entry| entry.path())
}

/// The same search once more, ignoring how the path is spelled.
fn display_file_case(resolved: &Path, kind: PathKind) -> Option<PathBuf> {
    find_display_variant(&lookup_case_insensitive(resolved), kind, Case::Ignored)
}

/// Whether a file name is one of the forms a display file is looked up under:
/// the name, an optional security level, an optional graphics letter, an
/// optional language and one of the known extensions.
fn is_display_variant(name: &str, stem: &str, case: Case) -> bool {
    let Some((rest, extension)) = name.rsplit_once('.') else {
        return false;
    };
    let known_extension = match case {
        Case::Exact => DISPLAY_EXTENSIONS.contains(&extension),
        Case::Ignored => DISPLAY_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)),
    };
    if !known_extension {
        return false;
    }
    if matches_stem(rest, stem, case) {
        return true;
    }
    matches!(rest.rsplit_once('.'), Some((before_language, _)) if matches_stem(before_language, stem, case))
}

fn matches_stem(rest: &str, stem: &str, case: Case) -> bool {
    if rest.len() < stem.len() || !rest.is_char_boundary(stem.len()) {
        return false;
    }
    let (head, tail) = rest.split_at(stem.len());
    let same_name = match case {
        Case::Exact => head == stem,
        Case::Ignored => head.eq_ignore_ascii_case(stem),
    };
    if !same_name {
        return false;
    }
    let graphics: &[char] = match case {
        Case::Exact => &['g', 'r', 'v'],
        Case::Ignored => &['g', 'G', 'r', 'R', 'v', 'V'],
    };
    let tail = tail.strip_suffix(graphics).unwrap_or(tail);
    tail.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(files: &[&str], directories: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for directory in directories {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        for file in files {
            let path = root.path().join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        root
    }

    #[test]
    fn a_file_that_is_there_is_no_problem() {
        let root = board(&["gen/icbtext.toml"], &[]);
        assert_eq!(check_path(&root.path().join("gen/icbtext.toml"), PathKind::File), None);
    }

    #[test]
    fn a_file_that_is_not_there_is_missing() {
        let root = board(&[], &[]);
        assert_eq!(check_path(&root.path().join("gen/icbtext.toml"), PathKind::File), Some(PathProblem::Missing));
    }

    #[test]
    fn a_file_spelled_the_dos_way_is_reported_as_spelling() {
        let root = board(&["GEN/ICBTEXT.TOML"], &[]);
        let problem = check_path(&root.path().join("gen/icbtext.toml"), PathKind::File);
        assert!(matches!(problem, Some(PathProblem::WrongCase(_))), "{problem:?}");
    }

    #[test]
    fn a_directory_where_a_file_belongs_is_reported() {
        let root = board(&[], &["gen/icbtext.toml"]);
        assert_eq!(
            check_path(&root.path().join("gen/icbtext.toml"), PathKind::File),
            Some(PathProblem::ExpectedFileFoundDirectory)
        );
    }

    #[test]
    fn a_file_where_a_directory_belongs_is_reported() {
        let root = board(&["help"], &[]);
        assert_eq!(check_path(&root.path().join("help"), PathKind::Directory), Some(PathProblem::ExpectedDirectoryFoundFile));
    }

    #[test]
    fn a_display_file_is_found_under_the_extension_it_carries() {
        let root = board(&["art/welcome.pcb"], &[]);
        assert_eq!(check_path(&root.path().join("art/welcome"), PathKind::DisplayFile), None);
    }

    #[test]
    fn a_display_file_is_found_under_its_graphics_variant() {
        let root = board(&["art/welcomeg.ans"], &[]);
        assert_eq!(check_path(&root.path().join("art/welcome"), PathKind::DisplayFile), None);
    }

    #[test]
    fn a_display_file_is_found_under_its_security_variant() {
        let root = board(&["art/welcome10g.pcb"], &[]);
        assert_eq!(check_path(&root.path().join("art/welcome"), PathKind::DisplayFile), None);
    }

    #[test]
    fn a_display_file_is_found_under_its_language_variant() {
        let root = board(&["art/welcome.eng.pcb"], &[]);
        assert_eq!(check_path(&root.path().join("art/welcome"), PathKind::DisplayFile), None);
    }

    #[test]
    fn a_display_file_spelled_the_dos_way_is_reported_as_spelling() {
        let root = board(&["art/WELCOME.PCB"], &[]);
        let problem = check_path(&root.path().join("art/welcome"), PathKind::DisplayFile);
        let Some(PathProblem::WrongCase(found)) = problem else {
            panic!("{problem:?}");
        };
        assert_eq!(found.file_name().unwrap(), "WELCOME.PCB");
    }

    #[test]
    fn a_file_that_only_starts_the_same_is_not_the_display_file() {
        let root = board(&["art/welcome_old.pcb"], &[]);
        assert_eq!(check_path(&root.path().join("art/welcome"), PathKind::DisplayFile), Some(PathProblem::Missing));
    }

    #[test]
    fn a_menu_may_be_a_ppe_instead() {
        let root = board(&["menus/main.ppe"], &[]);
        assert_eq!(check_path(&root.path().join("menus/main"), PathKind::Menu), None);
        assert_eq!(check_path(&root.path().join("menus/main"), PathKind::DisplayFile), Some(PathProblem::Missing));
    }

    #[test]
    fn an_empty_path_is_not_checked() {
        let mut board = IcyBoard::default();
        board.config.paths.welcome = PathBuf::new();
        assert!(!board.check_paths().iter().any(|report| report.context.contains("Welcome")));
    }

    #[test]
    fn a_conference_says_which_field_of_which_conference_is_wrong() {
        let root = board(&[], &[]);
        let mut board = IcyBoard::default();
        board.root_path = root.path().to_path_buf();
        let mut conference = super::super::conferences::Conference::default();
        conference.name = "Sysop".to_string();
        conference.news_file = PathBuf::from("main/news");
        board.conferences.push(conference);

        let reports = board.check_paths();
        let report = reports.iter().find(|report| report.path == PathBuf::from("main/news")).expect("the news file is not reported");
        assert_eq!(report.context, "Conference 0 (Sysop), news file");
    }
}

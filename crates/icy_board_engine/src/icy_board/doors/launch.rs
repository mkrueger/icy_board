use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use super::Door;
use crate::Res;

/// Values a door may ask for in its command line arguments.
pub struct Placeholders {
    pub drop_file: Option<PathBuf>,
    pub node: usize,
    pub user_id: i32,
    pub user_name: String,
    pub time_left_seconds: i64,
    pub term_width: u16,
    pub term_height: u16,
    pub socket_handle: Option<i64>,
}

impl Placeholders {
    fn lookup(&self, name: &str) -> Res<String> {
        let drop_file = match self.drop_file.as_deref() {
            Some(path) => path,
            None if matches!(name, "dropFile" | "dropFilePath" | "dropFileDir") => {
                return Err(format!("{{{name}}} needs a drop file type on the door").into());
            }
            None => Path::new(""),
        };
        Ok(match name {
            "dropFile" => drop_file.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
            "dropFilePath" => drop_file.display().to_string(),
            "dropFileDir" => drop_file.parent().unwrap_or(Path::new("")).display().to_string(),
            "socketHandle" => match self.socket_handle {
                Some(handle) => handle.to_string(),
                None => return Err("{socketHandle} needs the door's socket connection option".into()),
            },
            "node" => (self.node + 1).to_string(),
            "userId" => self.user_id.to_string(),
            "userName" => self.user_name.clone(),
            "timeLeftSeconds" => self.time_left_seconds.to_string(),
            "termWidth" => self.term_width.to_string(),
            "termHeight" => self.term_height.to_string(),
            _ => return Err(format!("unknown door argument placeholder {{{name}}}").into()),
        })
    }
}

/// Expands one argument. Arguments are never joined into a command line, so an
/// expanded value cannot turn into extra arguments or shell syntax.
pub fn expand(template: &str, values: &Placeholders) -> Res<String> {
    let mut expanded = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        expanded.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(remainder) = rest.strip_prefix("{{") {
            expanded.push('{');
            rest = remainder;
            continue;
        }
        let Some(end) = rest.find('}') else {
            return Err(format!("unterminated placeholder in door argument '{template}'").into());
        };
        expanded.push_str(&values.lookup(&rest[1..end])?);
        rest = &rest[end + 1..];
    }
    expanded.push_str(rest);
    Ok(expanded)
}

/// Reports unknown placeholders and socket use without the socket option
/// without needing a caller session.
pub fn check_arguments(door: &Door) -> Res<()> {
    let probe = Placeholders {
        drop_file: door.drop_file.file_name(0).map(PathBuf::from),
        node: 0,
        user_id: 0,
        user_name: String::new(),
        time_left_seconds: 0,
        term_width: 80,
        term_height: 25,
        socket_handle: door.provide_socket_connection.then_some(0),
    };
    for argument in &door.args {
        expand(argument, &probe)?;
    }
    Ok(())
}

/// Editing convenience for a single input line; launching never builds a
/// command line out of these parts.
pub fn join_arguments(arguments: &[String]) -> String {
    shell_words::join(arguments.iter().map(String::as_str))
}

pub fn parse_arguments(line: &str) -> Res<Vec<String>> {
    Ok(shell_words::split(line)?)
}

static ACTIVE_DOORS: LazyLock<Mutex<HashMap<String, u32>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Counts one running door and releases it again when dropped.
pub struct ParallelGuard(String);

impl Drop for ParallelGuard {
    fn drop(&mut self) {
        let mut active = ACTIVE_DOORS.lock().unwrap();
        if let Some(count) = active.get_mut(&self.0) {
            *count -= 1;
            if *count == 0 {
                active.remove(&self.0);
            }
        }
    }
}

/// Returns `None` when `max_parallel` sessions of this door already run.
pub fn acquire(name: &str, max_parallel: u32) -> Option<ParallelGuard> {
    let key = name.to_uppercase();
    let mut active = ACTIVE_DOORS.lock().unwrap();
    let count = active.get(&key).copied().unwrap_or(0);
    if max_parallel > 0 && count >= max_parallel {
        return None;
    }
    active.insert(key.clone(), count + 1);
    Some(ParallelGuard(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> Placeholders {
        Placeholders {
            drop_file: Some(PathBuf::from("/tmp/icb doors/node1/door32.sys")),
            node: 2,
            user_id: 41,
            user_name: "Probe User".into(),
            time_left_seconds: 900,
            term_width: 132,
            term_height: 43,
            socket_handle: Some(7),
        }
    }

    #[test]
    fn placeholders_expand_without_splitting_or_quoting_values() {
        let values = values();
        assert_eq!(expand("{dropFilePath}", &values).unwrap(), "/tmp/icb doors/node1/door32.sys");
        assert_eq!(expand("{dropFile}", &values).unwrap(), "door32.sys");
        assert_eq!(expand("{dropFileDir}", &values).unwrap(), "/tmp/icb doors/node1");
        assert_eq!(expand("-N{node}-{userId}", &values).unwrap(), "-N3-41");
        assert_eq!(expand("{userName}", &values).unwrap(), "Probe User");
        assert_eq!(expand("{termWidth}x{termHeight}", &values).unwrap(), "132x43");
        assert_eq!(expand("{timeLeftSeconds}", &values).unwrap(), "900");
        assert_eq!(expand("{socketHandle}", &values).unwrap(), "7");
        assert_eq!(expand("{{node}", &values).unwrap(), "{node}");
        assert_eq!(expand("a } b", &values).unwrap(), "a } b");
        assert_eq!(expand("plain", &values).unwrap(), "plain");
    }

    #[test]
    fn unknown_and_unusable_placeholders_are_reported() {
        let mut values = values();
        assert!(expand("{srvPort}", &values).unwrap_err().to_string().contains("{srvPort}"));
        assert!(expand("{node", &values).is_err());
        values.socket_handle = None;
        assert!(expand("{socketHandle}", &values).unwrap_err().to_string().contains("socket connection"));
    }

    #[test]
    fn arguments_are_checked_against_the_door_configuration() {
        let mut door = Door {
            args: vec!["-D".into(), "{dropFilePath}".into(), "-H{socketHandle}".into()],
            drop_file: super::super::DropFile::Door32Sys,
            ..Door::default()
        };
        assert!(check_arguments(&door).is_err());
        door.provide_socket_connection = true;
        assert!(check_arguments(&door).is_ok());
        door.drop_file = super::super::DropFile::None;
        assert!(check_arguments(&door).unwrap_err().to_string().contains("drop file"));
        door.drop_file = super::super::DropFile::Door32Sys;
        door.args.push("{unknown}".into());
        assert!(check_arguments(&door).is_err());
    }

    #[test]
    fn parallel_guard_counts_per_door_and_releases_on_drop() {
        let first = acquire("BRE", 2).unwrap();
        let second = acquire("bre", 2).unwrap();
        assert!(acquire("BRE", 2).is_none());
        assert!(acquire("OTHER", 2).is_some());
        drop(second);
        let third = acquire("BRE", 2).unwrap();
        drop((first, third));
        assert!(acquire("BRE", 1).is_some());
        assert!(ACTIVE_DOORS.lock().unwrap().get("OTHER").is_none());
    }

    #[test]
    fn zero_keeps_a_door_unlimited() {
        let guards: Vec<_> = (0..8).map(|_| acquire("UNLIMITED", 0).unwrap()).collect();
        assert_eq!(guards.len(), 8);
    }
}

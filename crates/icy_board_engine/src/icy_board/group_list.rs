use std::{fmt::Display, fs, ops::Deref, path::Path, str::FromStr};

use crate::Res;

#[derive(Default)]
pub struct Group {
    pub name: String,
    pub description: String,
    pub members: Vec<String>,
}

#[derive(Default)]
pub struct GroupList {
    group: Vec<Group>,
}

impl Display for GroupList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for group in &self.group {
            write!(f, "{}:{}:", group.name, group.description)?;
            if !group.members.is_empty() {
                write!(f, " ")?;
                for (i, member) in group.members.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", member)?;
                }
            }
            write!(f, "\n")?;
            continue;
        }
        Ok(())
    }
}

impl Deref for GroupList {
    type Target = Vec<Group>;

    fn deref(&self) -> &Self::Target {
        &self.group
    }
}

enum ParseState {
    GroupName,
    Description,
    Comment,
    MemberStart,
    Member,
}

impl FromStr for GroupList {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut state = ParseState::GroupName;
        let mut res = GroupList::new();
        let mut grp = Group::default();
        let mut name = String::new();
        for ch in s.chars() {
            match state {
                ParseState::GroupName => {
                    if ch == '#' && grp.name.len() == 0 {
                        state = ParseState::Comment;
                    } else if ch == ':' {
                        state = ParseState::Description;
                    } else if !ch.is_whitespace() || grp.name.len() > 0 {
                        grp.name.push(ch);
                    }
                }
                ParseState::Description => {
                    if ch == ':' {
                        state = ParseState::MemberStart;
                    } else if !ch.is_whitespace() || grp.description.len() > 0 {
                        grp.description.push(ch);
                    }
                }
                ParseState::MemberStart => {
                    if ch == '\n' {
                        res.group.push(grp);
                        grp = Group::default();
                        state = ParseState::GroupName;
                    } else if !ch.is_whitespace() {
                        state = ParseState::Member;
                        name.push(ch);
                    }
                }
                ParseState::Member => {
                    if ch == ',' {
                        grp.members.push(name.trim_end().to_string());
                        name = String::new();
                        state = ParseState::MemberStart;
                    } else if ch == '\n' {
                        grp.members.push(name.trim_end().to_string());
                        name = String::new();
                        res.group.push(grp);
                        grp = Group::default();
                        state = ParseState::GroupName;
                    } else {
                        name.push(ch);
                    }
                }

                ParseState::Comment => {
                    if ch == '\n' {
                        state = ParseState::GroupName;
                    }
                }
            }
        }
        Ok(res)
    }
}

impl GroupList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group(&mut self, name: impl Into<String>, description: impl Into<String>) {
        self.group.push(Group {
            name: name.into(),
            description: description.into(),
            members: Vec::new(),
        });
    }

    pub fn add_member(&mut self, group_name: &str, member_name: &str) {
        if let Some(group) = self.group.iter_mut().find(|g| g.name == group_name) {
            group.members.push(member_name.to_string());
        }
    }

    pub fn get_groups(&self, user_name: &str) -> Vec<String> {
        self.group
            .iter()
            .filter(|g| g.members.contains(&user_name.to_string()))
            .map(|g| g.name.clone())
            .collect()
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let txt = fs::read_to_string(path)?;
        Ok(Self::from_str(&txt)?)
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        fs::write(path, self.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn gen_groups() -> GroupList {
        let mut group_list = GroupList::new();
        group_list.add_group("sysops", "System Operators");
        group_list.add_group("users", "Normal Users");
        group_list.add_group("read_mail", "Can read mail");
        group_list.add_group("write_mail", "Can write mail");
        group_list.add_member("sysops", "sysop");
        group_list.add_member("users", "guest");
        group_list.add_member("users", "user2");
        group_list.add_member("read_mail", "sysop");
        group_list.add_member("read_mail", "guest");
        group_list.add_member("read_mail", "user2");
        group_list.add_member("write_mail", "sysop");
        group_list.add_member("write_mail", "user2");
        group_list
    }

    #[test]
    fn test_group_list() {
        let group_list = gen_groups();
        assert_eq!(group_list.get_groups("sysop"), vec!["sysops", "read_mail", "write_mail"]);
        assert_eq!(group_list.get_groups("guest"), vec!["users", "read_mail"]);
        assert_eq!(group_list.get_groups("user2"), vec!["users", "read_mail", "write_mail"]);
    }

    #[test]
    fn test_to_string() {
        let group_list = gen_groups();
        let expected = "sysops:System Operators: sysop\nusers:Normal Users: guest, user2\nread_mail:Can read mail: sysop, guest, user2\nwrite_mail:Can write mail: sysop, user2\n";
        assert_eq!(group_list.to_string(), expected);
    }

    #[test]
    fn test_parse_string() {
        let group_list = gen_groups();
        let expected = "sysops:System Operators: sysop\nusers:Normal Users: guest, user2\nread_mail:Can read mail: sysop, guest, user2\nwrite_mail:Can write mail: sysop, user2\n";
        let expected = GroupList::from_str(expected).unwrap();
        assert_eq!(group_list.len(), expected.len());
        for i in 0..group_list.len() {
            assert_eq!(group_list[i].name, expected[i].name);
            assert_eq!(group_list[i].description, expected[i].description);
            assert_eq!(group_list[i].members, expected[i].members);
        }
    }

    #[test]
    fn test_get_groups() {
        let group_list = gen_groups();
        let groups = group_list.get_groups("sysop");
        assert_eq!(groups, vec!["sysops", "read_mail", "write_mail"]);

        let groups = group_list.get_groups("guest");
        assert_eq!(groups, vec!["users", "read_mail"]);

        let groups = group_list.get_groups("user2");
        assert_eq!(groups, vec!["users", "read_mail", "write_mail"]);
    }

    #[test]
    fn test_parse_comment() {
        let expected = "# Foo\nsysops:: sysop\n      #Bar\n";
        let group_list = GroupList::from_str(expected).unwrap();
        let groups = group_list.get_groups("sysop");
        assert_eq!(groups, vec!["sysops"]);
    }
}

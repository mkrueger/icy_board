use crate::{
    Connection, NetError,
    binkp::{BinkpCommand, Frame, cram::*},
};

/// What this board tells a remote about itself during the handshake.
#[derive(Clone, Debug)]
pub struct BinkpIdentity {
    /// The 5D addresses this side answers to, most specific one first.
    pub addresses: Vec<String>,
    pub system_name: String,
    pub sysop: String,
    pub location: String,
    pub mailer: String,
}

impl Default for BinkpIdentity {
    fn default() -> Self {
        Self {
            addresses: Vec::new(),
            system_name: String::new(),
            sysop: String::new(),
            location: String::new(),
            mailer: format!("IcyBoard/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl BinkpIdentity {
    fn greeting(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.system_name.is_empty() {
            lines.push(format!("SYS {}", self.system_name));
        }
        if !self.sysop.is_empty() {
            lines.push(format!("ZYZ {}", self.sysop));
        }
        if !self.location.is_empty() {
            lines.push(format!("LOC {}", self.location));
        }
        lines.push(format!("VER {} binkp/1.0", self.mailer));
        lines
    }
}

/// What the remote told us about itself before the batch started.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteInfo {
    pub addresses: Vec<String>,
    pub options: Vec<String>,
    pub system_name: String,
    pub sysop: String,
    pub location: String,
    pub mailer: String,
    pub secure: bool,
}

impl RemoteInfo {
    fn note(&mut self, argument: &str) {
        let Some((keyword, rest)) = argument.split_once(' ') else {
            return;
        };
        let rest = rest.trim();
        match keyword {
            "SYS" => self.system_name = rest.to_string(),
            "ZYZ" => self.sysop = rest.to_string(),
            "LOC" => self.location = rest.to_string(),
            "VER" => self.mailer = rest.to_string(),
            "OPT" => self.options.extend(rest.split_whitespace().map(str::to_string)),
            _ => {}
        }
    }

    /// The random bytes the remote wants to see hashed with the password.
    pub fn cram_challenge(&self) -> Option<Vec<u8>> {
        self.options
            .iter()
            .find_map(|option| option.to_ascii_uppercase().strip_prefix("CRAM-MD5-").and_then(from_hex))
    }

    fn password_answer(&self, password: &str) -> String {
        if password.is_empty() {
            return "-".to_string();
        }
        match self.cram_challenge() {
            Some(challenge) => format!("CRAM-MD5-{}", to_hex(&cram_md5(password.as_bytes(), &challenge))),
            None => password.to_string(),
        }
    }
}

/// Runs the session setup stage of FTS-1026 6.1.1 and leaves the connection at
/// the start of the file transfer stage.
pub async fn originate_session(connection: &mut dyn Connection, identity: &BinkpIdentity, called: &str, password: &str) -> crate::Result<RemoteInfo> {
    for line in identity.greeting() {
        Frame::command(BinkpCommand::Nul, line).send(connection).await?;
    }
    Frame::command(BinkpCommand::Adr, identity.addresses.join(" ")).send(connection).await?;

    let mut remote = RemoteInfo::default();
    let mut answered = false;
    loop {
        match Frame::read(connection).await? {
            Frame::Command(BinkpCommand::Nul, argument) => remote.note(&argument),

            Frame::Command(BinkpCommand::Adr, argument) => {
                remote.addresses = argument.split_whitespace().map(str::to_string).collect();
                if !called.is_empty() && !remote.addresses.iter().any(|address| is_same_system(address, called)) {
                    return abort(connection, NetError::BinkpWrongSystem(called.to_string(), argument)).await;
                }
                if !answered {
                    // Waiting for the address is what lets a challenge arrive first, which
                    // FTS-1026 6.1.1 allows as the one exception to answering immediately.
                    Frame::command(BinkpCommand::Pwd, remote.password_answer(password)).send(connection).await?;
                    answered = true;
                }
            }

            Frame::Command(BinkpCommand::Ok, argument) => {
                remote.secure = !password.is_empty() && argument.trim() != "non-secure";
                return Ok(remote);
            }

            Frame::Command(BinkpCommand::Err, argument) => return Err(NetError::BinkpRemoteError(argument).into()),
            Frame::Command(BinkpCommand::Bsy, argument) => return Err(NetError::BinkpRemoteBusy(argument).into()),

            Frame::Command(command, _) => {
                return abort(connection, NetError::BinkpUnexpectedFrame(command.to_string())).await;
            }
            Frame::Data(_) => {
                return abort(connection, NetError::BinkpUnexpectedFrame("data".to_string())).await;
            }
        }
    }
}

/// The remote is owed an explanation before the connection goes away.
async fn abort(connection: &mut dyn Connection, error: NetError) -> crate::Result<RemoteInfo> {
    let _ = Frame::command(BinkpCommand::Err, error.to_string()).send(connection).await;
    Err(error.into())
}

/// Addresses name the same system when they differ only in a point of zero or
/// in the case of the domain.
fn is_same_system(left: &str, right: &str) -> bool {
    fn normalize(address: &str) -> String {
        let (address, domain) = address.split_once('@').unwrap_or((address, ""));
        let address = address.strip_suffix(".0").unwrap_or(address);
        format!("{}@{}", address, domain.to_ascii_lowercase())
    }
    normalize(left) == normalize(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ChannelConnection;

    fn identity() -> BinkpIdentity {
        BinkpIdentity {
            addresses: vec!["21:1/100@fsxnet".to_string()],
            system_name: "Icy Board".to_string(),
            sysop: "Sysop".to_string(),
            location: "Somewhere".to_string(),
            ..Default::default()
        }
    }

    /// Plays the answering side, returning the frames the originator sent.
    fn answer(mut peer: ChannelConnection, greeting: Vec<Frame>, closing: Frame) -> tokio::task::JoinHandle<Vec<Frame>> {
        tokio::spawn(async move {
            let mut seen = Vec::new();
            for frame in greeting {
                frame.send(&mut peer).await.unwrap();
            }
            loop {
                let frame = Frame::read(&mut peer).await.unwrap();
                let done = matches!(frame, Frame::Command(BinkpCommand::Pwd, _));
                seen.push(frame);
                if done {
                    break;
                }
            }
            closing.send(&mut peer).await.unwrap();
            seen
        })
    }

    fn password_sent(frames: &[Frame]) -> String {
        frames
            .iter()
            .find_map(|frame| match frame {
                Frame::Command(BinkpCommand::Pwd, argument) => Some(argument.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[tokio::test]
    async fn test_a_password_is_sent_as_typed_when_no_challenge_was_offered() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet")],
            Frame::command(BinkpCommand::Ok, "secure"),
        );
        let remote = originate_session(&mut ours, &identity(), "21:1/1@fsxnet", "swordfish").await.unwrap();
        assert_eq!(password_sent(&peer.await.unwrap()), "swordfish");
        assert!(remote.secure);
    }

    #[tokio::test]
    async fn test_a_challenge_is_answered_with_the_digest_instead_of_the_password() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![
                Frame::command(BinkpCommand::Nul, "OPT CRAM-MD5-0123456789abcdef"),
                Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet"),
            ],
            Frame::command(BinkpCommand::Ok, "secure"),
        );
        originate_session(&mut ours, &identity(), "21:1/1@fsxnet", "swordfish").await.unwrap();

        let expected = to_hex(&cram_md5(b"swordfish", &from_hex("0123456789abcdef").unwrap()));
        assert_eq!(password_sent(&peer.await.unwrap()), format!("CRAM-MD5-{}", expected));
    }

    #[tokio::test]
    async fn test_having_no_password_is_announced_with_a_dash() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet")],
            Frame::command(BinkpCommand::Ok, "non-secure"),
        );
        let remote = originate_session(&mut ours, &identity(), "21:1/1@fsxnet", "").await.unwrap();
        assert_eq!(password_sent(&peer.await.unwrap()), "-");
        assert!(!remote.secure);
    }

    #[tokio::test]
    async fn test_the_greeting_carries_the_system_details_and_the_address() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet")],
            Frame::command(BinkpCommand::Ok, ""),
        );
        originate_session(&mut ours, &identity(), "", "").await.unwrap();

        let seen = peer.await.unwrap();
        assert!(seen.contains(&Frame::command(BinkpCommand::Nul, "SYS Icy Board")));
        assert!(seen.contains(&Frame::command(BinkpCommand::Nul, "ZYZ Sysop")));
        assert!(seen.contains(&Frame::command(BinkpCommand::Adr, "21:1/100@fsxnet")));
    }

    #[tokio::test]
    async fn test_what_the_remote_says_about_itself_is_kept() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![
                Frame::command(BinkpCommand::Nul, "SYS fsxNet Hub"),
                Frame::command(BinkpCommand::Nul, "VER binkd/1.1a binkp/1.1"),
                Frame::command(BinkpCommand::Adr, "21:1/1@fsxnet 21:1/0@fsxnet"),
            ],
            Frame::command(BinkpCommand::Ok, "secure"),
        );
        let remote = originate_session(&mut ours, &identity(), "21:1/1@fsxnet", "swordfish").await.unwrap();
        peer.await.unwrap();

        assert_eq!(remote.system_name, "fsxNet Hub");
        assert_eq!(remote.mailer, "binkd/1.1a binkp/1.1");
        assert_eq!(remote.addresses.len(), 2);
    }

    #[tokio::test]
    async fn test_a_system_that_is_not_the_one_that_was_called_is_refused() {
        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = answer(
            peer,
            vec![Frame::command(BinkpCommand::Adr, "21:1/2@fsxnet")],
            Frame::command(BinkpCommand::Ok, "secure"),
        );
        let error = originate_session(&mut ours, &identity(), "21:1/1@fsxnet", "swordfish").await.unwrap_err();
        assert!(error.to_string().contains("21:1/1@fsxnet"));
        peer.abort();
    }

    #[tokio::test]
    async fn test_a_busy_remote_ends_the_session() {
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            Frame::command(BinkpCommand::Bsy, "Too many servers").send(&mut peer).await.unwrap();
        });
        let error = originate_session(&mut ours, &identity(), "", "").await.unwrap_err();
        assert!(error.to_string().contains("Too many servers"));
    }

    #[test]
    fn test_a_zero_point_and_the_domain_case_do_not_make_another_system() {
        assert!(is_same_system("21:1/100.0@fsxnet", "21:1/100@FsxNet"));
        assert!(!is_same_system("21:1/100.1@fsxnet", "21:1/100@fsxnet"));
        assert!(!is_same_system("21:1/100@fsxnet", "21:1/100@micronet"));
    }
}

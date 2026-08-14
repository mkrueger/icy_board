use crate::datetime::IcbDate;
use crate::icy_board::conferences::ConferenceType;
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ALPHA, MASK_ASCII, MASK_PWD, display_flags},
    },
    user_base::ConferenceFlags,
};
use bstr::BString;
use chrono::{DateTime, Utc};
use jamjam::jam::{
    attributes,
    msg_header::{MessageSubfield, SubfieldType},
};

/// PCBoard treats conference types 3 and 4 as "internet" for the routing,
/// newsgroup and follow-up questions.
fn is_usenet(conference_type: &ConferenceType) -> bool {
    matches!(conference_type, ConferenceType::UsnetModeratedNewsgroup | ConferenceType::UsnetPublicNewsgroup)
}

/// Message options gathered from the security/return-receipt/echo prompts.
struct MessageOptions {
    attributes: u32,
    password: Option<String>,
    packout_date: Option<DateTime<Utc>>,
    sub_fields: Vec<MessageSubfield>,
}

impl IcyBoardState {
    pub async fn enter_message(&mut self) -> Res<()> {
        if self.session.current_conference.is_read_only {
            self.display_text(
                IceText::ConferenceIsReadOnly,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        // Per-conference security level required to enter a message (ReqLevelToEnter).
        let write_sec = self.session.current_conference.sec_write_message.clone();
        if !self.check_sec("E", &write_sec).await? {
            return Ok(());
        }
        self.set_activity(NodeStatus::EnterMessage).await;

        // PCBoard joins the arguments into the recipient field and then still
        // asks the question with that name pre-filled. Consuming the token
        // silently would swallow one answer of a PPE that stuffs the whole E
        // sequence.
        let mut to = String::new();
        while let Some(token) = self.session.tokens.pop_front() {
            if !to.is_empty() {
                to.push(' ');
            }
            to.push_str(&token);
        }
        to.truncate(25);
        let default_to = if to.trim().is_empty() { "ALL".to_string() } else { to.trim().to_string() };

        let Some(to) = self.get_message_recipient(IceText::MessageTo, default_to, false).await? else {
            return Ok(());
        };

        let subject = self
            .input_field(
                IceText::MessageSubject,
                54,
                &MASK_ASCII,
                "",
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if subject.is_empty() {
            return Ok(());
        };

        let to_all = to.eq_ignore_ascii_case("ALL");
        let options = self.get_message_options(to_all).await?;

        self.write_message(
            self.session.current_conference_number as i32,
            self.session.current_message_area as i32,
            &to,
            &subject,
            options.attributes,
            options.password,
            options.packout_date,
            options.sub_fields,
            IceText::SavingMessage,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn get_message_recipient(&mut self, prompt: IceText, mut default_to: String, empty_ends: bool) -> Res<Option<String>> {
        loop {
            let answer = self
                .input_field(
                    prompt,
                    54,
                    &MASK_ASCII,
                    "",
                    Some(default_to.clone()),
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
                )
                .await?;
            if empty_ends && answer.is_empty() {
                return Ok(None);
            }
            let to = if answer.is_empty() { default_to.clone() } else { answer };

            let validate = self.get_board().await.config.message.validate_to_name && !self.session.current_conference.echo_mail_in_conference;
            if !validate || to.eq_ignore_ascii_case("ALL") || to.eq_ignore_ascii_case("SYSOP") {
                return Ok(Some(to));
            }

            let conference = self.session.current_conference_number as usize;
            let (found, registered) = {
                let board = self.get_board().await;
                match board.users.find_by_name(&to) {
                    Some(index) => {
                        let registered = conference == 0
                            || board.users[index]
                                .conference_flags
                                .get(&conference)
                                .is_some_and(|flags| flags.contains(ConferenceFlags::Registered));
                        (true, registered)
                    }
                    None => (false, false),
                }
            };
            if found && registered {
                return Ok(Some(to));
            }

            self.session.op_text = to.clone();
            self.display_text(
                if found {
                    IceText::UserNotRegisteredInConference
                } else {
                    IceText::CouldntFindInUsers
                },
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
            let retry = self
                .input_field(
                    IceText::ReEnterUsersName,
                    1,
                    "CR",
                    "",
                    Some("R".to_string()),
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;
            if retry.eq_ignore_ascii_case("C") {
                return Ok(Some(to));
            }
            default_to = to;
        }
    }

    /// Prompts for message security, return receipt and echo flag, mirroring the
    /// original PCBoard flow.
    async fn get_message_options(&mut self, to_all: bool) -> Res<MessageOptions> {
        let mut options = MessageOptions {
            attributes: 0,
            password: None,
            packout_date: None,
            sub_fields: Vec::new(),
        };

        // Security (getsecurity). Only "receiver only" messages are marked private;
        // password protected messages carry a password but stay public.
        let is_email = self.session.current_conference.conference_type == ConferenceType::InternetEmail;
        let receiver_only = if self.session.current_conference.private_msgs || is_email {
            true
        } else if self.session.current_conference.disallow_private_msgs {
            false
        } else {
            self.get_message_security(to_all, &mut options).await?
        };

        if receiver_only {
            options.attributes |= attributes::MSG_PRIVATE;
        }

        // Return receipt (getretreceipt) - only for receiver-only messages not addressed
        // to ALL and only if the user's security level allows requesting one.
        if receiver_only && !to_all && self.session.current_conference.sec_request_rr.session_can_access(&self.session) && self.get_ret_receipt().await? {
            options.attributes |= attributes::MSG_RECEIPTREQ;
        }

        // Echo flag (getechoflag) - only asked in conferences that echo mail and don't force it.
        let mut echoed = self.session.current_conference.echo_mail_in_conference;
        if echoed && !self.session.current_conference.force_echomail {
            echoed = self.get_echo_flag().await?;
        }

        if echoed {
            self.get_routing_info(receiver_only, &mut options).await?;
            self.get_newsgroups(receiver_only, &mut options).await?;
        }

        Ok(options)
    }

    /// Public messages are routed in usenet
    /// style conferences, private ones everywhere else.
    async fn get_routing_info(&mut self, receiver_only: bool, options: &mut MessageOptions) -> Res<()> {
        if !self.session.current_conference.prompt_for_routing {
            return Ok(());
        }
        let wanted = if is_usenet(&self.session.current_conference.conference_type) {
            !receiver_only
        } else {
            receiver_only
        };
        if !wanted {
            return Ok(());
        }
        let route_to = self
            .input_field(IceText::RoutedTo, 60, &MASK_ASCII, "", None, display_flags::NEWLINE | display_flags::FIELDLEN)
            .await?;
        if !route_to.is_empty() {
            options.sub_fields.push(MessageSubfield::new(SubfieldType::AddressD, BString::from(route_to)));
        }
        Ok(())
    }

    /// The newsgroup and follow-up questions only
    /// apply to public messages in a usenet style conference.
    async fn get_newsgroups(&mut self, receiver_only: bool, options: &mut MessageOptions) -> Res<()> {
        if receiver_only || !is_usenet(&self.session.current_conference.conference_type) {
            return Ok(());
        }
        let newsgroup = self
            .input_field(
                IceText::DestNewsGroup,
                60,
                &MASK_ASCII,
                "",
                Some(self.session.current_conference.name.clone()),
                display_flags::NEWLINE | display_flags::FIELDLEN | display_flags::GUIDE,
            )
            .await?;
        if !newsgroup.is_empty() {
            options
                .sub_fields
                .push(MessageSubfield::new(SubfieldType::FTSKludge, BString::from(format!("NEWSGROUPS: {newsgroup}"))));
        }

        let followup = self
            .input_field(
                IceText::FollowupNewsGroup,
                60,
                &MASK_ASCII,
                "",
                None,
                display_flags::NEWLINE | display_flags::FIELDLEN,
            )
            .await?;
        if !followup.is_empty() {
            options
                .sub_fields
                .push(MessageSubfield::new(SubfieldType::FTSKludge, BString::from(format!("FOLLOWUP-TO: {followup}"))));
        }
        Ok(())
    }

    /// Returns `true` when the message is "receiver only" (private).
    async fn get_message_security(&mut self, to_all: bool, options: &mut MessageOptions) -> Res<bool> {
        loop {
            let input = self
                .input_field(
                    IceText::MessageSecurity,
                    1,
                    "GNRSDgnrsd",
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN,
                )
                .await?;

            match input.to_ascii_uppercase().chars().next().unwrap_or('N') {
                // Public message
                'N' => return Ok(false),
                // Receiver only (private)
                'R' => {
                    if to_all {
                        self.display_text(
                            IceText::CantProtectMessageToAll,
                            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
                        )
                        .await?;
                        continue;
                    }
                    return Ok(true);
                }
                // Sender password protected (public status, password required to read)
                'S' => {
                    let password = self.input_security_password().await?;
                    if password.is_empty() {
                        continue;
                    }
                    options.password = Some(password);
                    return Ok(false);
                }
                // Group password protected
                'G' => {
                    let confirm = self
                        .input_field(
                            IceText::CallerMustKnowPassword,
                            1,
                            &MASK_ALPHA,
                            "",
                            Some(self.session.no_char.to_string()),
                            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
                        )
                        .await?;
                    if confirm != self.session.yes_char.to_uppercase().to_string() {
                        continue;
                    }
                    let password = self.input_security_password().await?;
                    if password.is_empty() {
                        continue;
                    }
                    options.password = Some(password);
                    return Ok(false);
                }
                // Set pack-out date
                'D' => {
                    // PCBoard keeps asking until the date parses or the caller
                    // just presses Enter.
                    loop {
                        let date = self
                            .input_field(
                                IceText::EnterPackDate,
                                8,
                                "0123456789-/",
                                "",
                                None,
                                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
                            )
                            .await?;
                        if date.is_empty() {
                            break;
                        }
                        let parsed = IcbDate::parse(&date);
                        if parsed.year() != 0 {
                            options.packout_date = Some(parsed.to_utc_date_time());
                            break;
                        }
                    }
                    return Ok(false);
                }
                _ => continue,
            }
        }
    }

    async fn input_security_password(&mut self) -> Res<String> {
        self.input_field(
            IceText::SecurityPassword,
            12,
            &MASK_PWD,
            "",
            None,
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN,
        )
        .await
    }

    async fn get_echo_flag(&mut self) -> Res<bool> {
        let input = self
            .input_field(
                IceText::EchoMessage,
                1,
                &MASK_ALPHA,
                "",
                Some(self.session.yes_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;
        Ok(input.is_empty() || input == self.session.yes_char.to_uppercase().to_string())
    }
}

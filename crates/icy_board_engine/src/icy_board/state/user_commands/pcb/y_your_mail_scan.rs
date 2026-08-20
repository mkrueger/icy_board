use crate::{
    icy_board::{
        commands::CommandType,
        conferences::Conference,
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{MASK_ASCII, display_flags},
        user_base::ConferenceFlags,
    },
    vm::TerminalTarget,
};
use bstr::BString;
use jamjam::jam::{JamMessageBase, msg_header::JamMessageHeader};

use crate::{Res, icy_board::state::IcyBoardState};

#[derive(Default, Clone)]
struct YourMailScan {
    select_conf: bool,
    all_conf: bool,
    wait_conf: bool,
    since: bool,
    forward: bool,
    quick: bool,
    skip_zero: bool,
}

/// One message the scan turned up, and whether it has been read.
struct Found {
    number: u32,
    read: bool,
}

#[derive(Default)]
struct ScanResult {
    to_you: Vec<Found>,
    from_you: Vec<Found>,
    /// Everything the caller is allowed to see - the "Total Found" column.
    visible: u32,
}

impl IcyBoardState {
    pub async fn your_mail_scan(&mut self) -> Res<()> {
        let text = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::MessageScanPrompt,
                8,
                &MASK_ASCII,
                CommandType::YourMailScan.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::STACKED,
            )
            .await?
        };
        // An empty answer leaves without scanning anything.
        if text.trim().is_empty() {
            return Ok(());
        }
        self.session.push_tokens(&text);

        let mut scan = YourMailScan {
            quick: self.get_board().await.config.message.default_quick_personal_scan,
            ..Default::default()
        };
        while let Some(cmd) = self.session.tokens.pop_front() {
            apply_option(&mut scan, &cmd);
        }

        self.display_text(IceText::AbortKeys, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
        self.session.disp_options.abort_printout = false;

        if scan.quick {
            self.display_text(IceText::ScanHeader1, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.display_text(IceText::ScanHeader2, display_flags::NEWLINE).await?;
            self.display_text(IceText::ScanHeader3, display_flags::NEWLINE).await?;
        }

        let mut found = 0;
        if scan.all_conf {
            let confs = self.get_board().await.conferences.clone();
            for (number, conf) in confs.iter().enumerate() {
                if self.session.disp_options.abort_printout {
                    break;
                }
                if !self.may_scan(conf, number as u16, &scan) {
                    continue;
                }
                found += self.scan_and_report(number, conf, &scan).await?;
            }
        } else {
            let conf = self.session.current_conference.clone();
            let number = self.session.current_conference_number as usize;
            found = self.scan_and_report(number, &conf, &scan).await?;
        }

        // The private mail base has no conference of its own, so it is reported
        // on its own line - PCBoard had nowhere to put this.
        found += self.scan_and_report_email(&scan).await?;

        if scan.skip_zero && found == 0 {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
        }
        Ok(())
    }

    async fn scan_and_report_email(&mut self, scan: &YourMailScan) -> Res<u32> {
        // Scanning must not bring the base into being - an absent one holds nothing.
        let path = self.email_msgbase_path().await;
        if !path.with_extension("jhr").exists() {
            return Ok(0);
        }
        let mut msg_base = match JamMessageBase::open(&path) {
            Ok(msg_base) => msg_base,
            Err(err) => {
                log::error!("Personal mail scan could not open the e-mail base: {err}");
                return Ok(0);
            }
        };
        let name = self.session.user_name.clone();
        let mut result = ScanResult::default();
        let name = BString::from(name);
        let alias = BString::from(self.session.alias_name.clone());
        self.scan_base(&mut msg_base, scan, &name, &alias, &mut result);

        if scan.skip_zero && (result.visible == 0 || (scan.wait_conf && result.to_you.is_empty())) {
            return Ok(0);
        }
        let label = self.get_display_text(IceText::PersonalMailBase)?;
        if scan.quick {
            self.report_quick_line(&format!("      {label}"), &result).await?;
        } else {
            self.display_text(IceText::Scanning, display_flags::LFBEFORE).await?;
            self.println(TerminalTarget::Both, &label).await?;
            self.report_long_body(&result).await?;
        }
        Ok(result.visible)
    }

    /// The conferences a scan over all of them is allowed to touch.
    fn may_scan(&self, conf: &Conference, number: u16, scan: &YourMailScan) -> bool {
        if number != self.session.current_conference_number {
            if !self.is_registered(conf, number) {
                return false;
            }
            if scan.select_conf && !self.has_conference_flag(number, ConferenceFlags::Selected) {
                return false;
            }
        }
        // "W" narrows the run down to the conferences that flagged mail waiting.
        !scan.wait_conf || self.has_conference_flag(number, ConferenceFlags::MailWaiting)
    }

    fn has_conference_flag(&self, number: u16, flag: ConferenceFlags) -> bool {
        self.session
            .current_user
            .as_ref()
            .and_then(|user| user.conference_flags.get(&(number as usize)))
            .is_some_and(|flags| flags.contains(flag))
    }

    async fn scan_and_report(&mut self, number: usize, conf: &Conference, scan: &YourMailScan) -> Res<u32> {
        let result = self.scan_conference(conf, scan)?;
        // Without anything to show, a skipping scan does not name the conference.
        if scan.skip_zero && (result.visible == 0 || (scan.wait_conf && result.to_you.is_empty())) {
            return Ok(0);
        }
        if scan.quick {
            self.report_quick(number, conf, &result).await?;
        } else {
            self.report_long(number, conf, &result).await?;
        }
        Ok(result.visible)
    }

    /// One line per conference: the name, then what is waiting and what was found.
    async fn report_quick(&mut self, number: usize, conf: &Conference, result: &ScanResult) -> Res<()> {
        self.report_quick_line(&format!("{:>5} {}", number, conf.name), result).await
    }

    async fn report_quick_line(&mut self, name: &str, result: &ScanResult) -> Res<()> {
        self.reset_color(TerminalTarget::Both).await?;
        self.print(TerminalTarget::Both, name).await?;
        self.set_color(TerminalTarget::Both, IcbColor::Dos(8)).await?;
        for i in 0..66usize.saturating_sub(name.len()) {
            self.print(TerminalTarget::Both, if i % 2 == 1 { "." } else { " " }).await?;
        }
        self.show_count(result.to_you.len() as u32).await?;
        self.show_count(result.visible).await?;
        self.new_line().await?;
        Ok(())
    }

    async fn show_count(&mut self, count: u32) -> Res<()> {
        let color = if count == 0 { IcbColor::Dos(7) } else { IcbColor::dos_white() };
        self.set_color(TerminalTarget::Both, color).await?;
        self.print(TerminalTarget::Both, &format!("{count:>6}")).await
    }

    /// The long form lists the message numbers themselves.
    async fn report_long(&mut self, number: usize, conf: &Conference, result: &ScanResult) -> Res<()> {
        self.display_text(IceText::Scanning, display_flags::LFBEFORE).await?;
        self.println(TerminalTarget::Both, &format!("{} ({})", conf.name, number)).await?;
        self.report_long_body(result).await
    }

    async fn report_long_body(&mut self, result: &ScanResult) -> Res<()> {
        self.display_text(IceText::MessagesForYou, display_flags::DEFAULT).await?;
        self.print_message_numbers(&result.to_you).await?;

        self.display_text(IceText::MessagesFromYou, display_flags::DEFAULT).await?;
        self.print_message_numbers(&result.from_you).await?;

        self.display_text(IceText::TotalMessagesFound, display_flags::DEFAULT).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_red()).await?;
        self.println(TerminalTarget::Both, &format!(" {}", result.visible)).await?;
        self.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }

    /// The numbers in columns, each with a `+` when the message has been read.
    async fn print_message_numbers(&mut self, found: &[Found]) -> Res<()> {
        if found.is_empty() {
            self.display_text(IceText::None, display_flags::NEWLINE).await?;
            return Ok(());
        }
        self.reset_color(TerminalTarget::Both).await?;
        let width = found.iter().map(|f| f.number.to_string().len()).max().unwrap_or(1) + 2;
        let columns = (80 / width).max(1);
        for (i, entry) in found.iter().enumerate() {
            if i > 0 && i % columns == 0 {
                self.new_line().await?;
                self.print(TerminalTarget::Both, "                ").await?;
            }
            let marker = if entry.read { '+' } else { ' ' };
            self.print(TerminalTarget::Both, &format!("{:<1$}{marker} ", entry.number, width - 2)).await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }
        self.new_line().await?;
        Ok(())
    }

    fn scan_conference(&self, conf: &Conference, scan: &YourMailScan) -> Res<ScanResult> {
        let name = BString::from(self.session.user_name.clone());
        let alias = BString::from(self.session.alias_name.clone());
        let mut result = ScanResult::default();

        let areas: Vec<std::path::PathBuf> = conf
            .areas
            .as_ref()
            .map(|areas| areas.iter().map(|area| area.path.clone()).collect())
            .unwrap_or_default();
        for path in areas {
            let Ok(mut msg_base) = JamMessageBase::open(&path) else {
                log::error!("can't open message base: {}", path.display());
                continue;
            };
            self.scan_base(&mut msg_base, scan, &name, &alias, &mut result);
        }
        Ok(result)
    }

    /// Walks one base the way `PCBoard` does: forward from the last-read pointer
    /// when scanning since the last call, backwards from the top otherwise.
    fn scan_base(&self, msg_base: &mut JamMessageBase, scan: &YourMailScan, name: &BString, alias: &BString, result: &mut ScanResult) {
        let low = msg_base.lowest_message_number();
        let high = msg_base.highest_message_number();
        if high < low {
            return;
        }
        let last_read = if scan.since {
            msg_base
                .find_last_read(JamMessageBase::crc(name), self.session.cur_user_id as u32)
                .ok()
                .flatten()
                .map_or(0, |entry| entry.last_read_msg)
                .clamp(low.saturating_sub(1), high)
        } else {
            low.saturating_sub(1)
        };

        let numbers: Vec<u32> = if scan.forward {
            ((last_read + 1)..=high).collect()
        } else {
            ((last_read + 1)..=high).rev().collect()
        };

        for number in numbers {
            let Ok(header) = msg_base.read_header(number) else {
                continue;
            };
            if !self.may_read(&header, name, alias) {
                continue;
            }
            result.visible += 1;
            let entry = |number: u32, header: &JamMessageHeader| Found {
                number,
                read: header.is_read(),
            };
            if matches_user(header.to(), name, alias) {
                result.to_you.push(entry(number, &header));
            }
            if matches_user(header.from(), name, alias) {
                result.from_you.push(entry(number, &header));
            }
        }
    }

    /// A private message is only of interest to the two ends of it.
    fn may_read(&self, header: &JamMessageHeader, name: &BString, alias: &BString) -> bool {
        if !header.is_private() || self.session.is_sysop {
            return true;
        }
        matches_user(header.to(), name, alias) || matches_user(header.from(), name, alias)
    }
}

fn matches_user(field: Option<&BString>, name: &BString, alias: &BString) -> bool {
    field.is_some_and(|value| value.eq_ignore_ascii_case(name) || (!alias.is_empty() && value.eq_ignore_ascii_case(alias)))
}

fn apply_option(scan: &mut YourMailScan, cmd: &str) {
    if cmd.len() == 1 {
        match cmd {
            "A" => {
                scan.select_conf = true;
                scan.all_conf = true;
            }
            "C" => {
                scan.select_conf = false;
                scan.all_conf = false;
            }
            "*" | "S" => {
                scan.since = true;
                scan.forward = true;
            }
            "Q" => scan.quick = true,
            "L" => scan.quick = false,
            "+" => scan.forward = true,
            "-" => scan.forward = false,
            "W" => {
                scan.wait_conf = true;
                scan.all_conf = true;
                scan.since = true;
                scan.forward = true;
                scan.skip_zero = true;
            }
            "Z" => scan.skip_zero = true,
            _ => {}
        }
        return;
    }
    // The words PCBoard accepted next to the single letters.
    match cmd {
        "ALL" => {
            scan.all_conf = true;
            scan.select_conf = false;
        }
        "C-" => {
            scan.all_conf = false;
            scan.select_conf = false;
            scan.forward = false;
        }
        "C+" => {
            scan.all_conf = false;
            scan.select_conf = false;
            scan.forward = true;
        }
        _ => {}
    }
}

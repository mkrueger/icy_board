use bstr::BString;
use icy_board_engine::{
    icy_board::{
        commands::Command,
        conferences::Conference,
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{display_flags, MASK_ASCII},
    },
    vm::TerminalTarget,
};
use jamjam::jam::JamMessageBase;

use crate::{menu_runner::PcbBoardCommand, Res};

#[derive(Default)]
struct PersonalMailScan {
    select_conf: bool,
    all_conf: bool,
    wait_conf: bool,
    since: bool,
    forward: bool,
    quick: bool,
    skip_zero: bool,
}

struct ScanResult {
    pub msg_from: u32,
    pub msg_to: u32,
}

impl PcbBoardCommand {
    pub async fn personal_mail(&mut self, action: &Command) -> Res<()> {
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
                    IceText::MessageScanPrompt,
                    8,
                    &MASK_ASCII,
                    &action.help,
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::STACKED,
                )
                .await?
        };
        self.state.session.push_tokens(&text);
        let mut scan = PersonalMailScan::default();
        scan.quick = true;
        loop {
            let Some(cmd) = self.state.session.tokens.pop_front() else {
                break;
            };
            match cmd.as_str() {
                "A" => {
                    // ALL
                    scan.select_conf = true;
                    scan.all_conf = true;
                }
                "C" => {
                    // CURRENT
                    scan.select_conf = false;
                    scan.all_conf = false;
                }
                "*" | "S" => {
                    // SINCE
                    scan.since = true;
                    scan.forward = true;
                }
                "Q" => {
                    // QUICK
                    scan.quick = true;
                }
                "L" => {
                    // LONG
                    scan.quick = false;
                }
                "+" => {
                    // FORWARD
                    scan.forward = true;
                }
                "-" => {
                    // BACKWARD
                    scan.forward = false;
                }
                "W" => {
                    // WAIT
                    scan.wait_conf = true;
                    scan.all_conf = true;
                    scan.since = true;
                    scan.forward = true;
                }
                "Z" => {
                    // SKIP ZERO
                    scan.skip_zero = true;
                }
                _ => {}
            }
        }
        self.state
            .display_text(IceText::AbortKeys, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;

        if scan.quick {
            self.state
                .display_text(IceText::ScanHeader1, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.display_text(IceText::ScanHeader2, display_flags::NEWLINE).await?;
            self.state.display_text(IceText::ScanHeader3, display_flags::NEWLINE).await?;
        }
        let mut msgs = 0;

        if scan.all_conf {
            let confs = self.state.get_board().await.conferences.clone();
            for (i, conf) in confs.iter().enumerate() {
                let res = self.scan_conference(&conf, &scan)?;
                if !scan.skip_zero || (res.msg_from > 0 || res.msg_to > 0) {
                    self.display_result(i, &conf, &res).await?;
                    msgs += res.msg_from + res.msg_to;
                }
            }
        } else {
            let res = self.scan_conference(&self.state.session.current_conference, &scan)?;
            let num = self.state.session.current_conference_number as usize;
            let conf = self.state.session.current_conference.clone();
            if !scan.skip_zero || (res.msg_from > 0 || res.msg_to > 0) {
                self.display_result(num, &conf, &res).await?;
                msgs += res.msg_from + res.msg_to;
            }
        }

        if msgs == 0 && scan.skip_zero {
            self.state.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
        }
        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }

    async fn display_result(&mut self, num: usize, conf: &icy_board_engine::icy_board::conferences::Conference, res: &ScanResult) -> Res<()> {
        self.state.reset_color(TerminalTarget::Both).await?;
        self.state.print(TerminalTarget::Both, &format!("{:>5} ", num)).await?;
        self.state.print(TerminalTarget::Both, &format!("{}", conf.name)).await?;
        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(8)).await?;
        for i in 0..(60 - conf.name.len()) {
            if i % 2 == 1 {
                self.state.print(TerminalTarget::Both, ".").await?;
            } else {
                self.state.print(TerminalTarget::Both, " ").await?;
            }
        }
        if res.msg_to > 0 {
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(15)).await?;
        } else {
            self.state.reset_color(TerminalTarget::Both).await?;
        }
        self.state.print(TerminalTarget::Both, &format!("{:>6}", res.msg_to)).await?;

        if res.msg_from > 0 {
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(15)).await?;
        } else {
            self.state.reset_color(TerminalTarget::Both).await?;
        }
        self.state.print(TerminalTarget::Both, &format!("{:>6}", res.msg_from)).await?;
        self.state.new_line().await?;
        Ok(())
    }

    fn scan_conference(&self, conf: &Conference, _scan: &PersonalMailScan) -> Res<ScanResult> {
        let name = BString::from(self.state.session.user_name.clone());
        let alias = BString::from(self.state.session.alias_name.clone());
        let mut msg_from = 0;
        let mut msg_to = 0;
        for area in conf.areas.iter() {
            let path = self.state.resolve_path(&area.filename);
            let Ok(msg_base) = JamMessageBase::open(&path) else {
                log::info!("can't open message base: {:?}", path);
                continue;
            };

            for msg in msg_base.iter().flatten() {
                if let Some(from) = msg.get_from() {
                    if from.eq_ignore_ascii_case(&name) || from.eq_ignore_ascii_case(&alias) {
                        msg_from += 1;
                    }
                }
                if let Some(to) = msg.get_to() {
                    if to.eq_ignore_ascii_case(&name) || to.eq_ignore_ascii_case(&alias) {
                        msg_to += 1;
                    }
                }
            }
        }
        Ok(ScanResult { msg_from, msg_to })
    }
}

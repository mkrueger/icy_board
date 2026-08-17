use std::path::Path;

use crate::{Res, tables::import_cp437_string};
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};

use super::{IcyBoardSerializer, PCBoardBinImporter, PCBoardImport};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct UsageStatistics {
    pub calls: u64,
    pub messages: u64,
    pub uploads: u64,
    pub uploads_kb: u64,
    pub downloads: u64,
    pub downloads_kb: u64,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct LastCaller {
    pub user_name: String,

    /// Utc time in rfc3339 format
    pub time: String,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub last_callers: Vec<LastCaller>,
    /// The day `today` counts, so the daily figures can be cleared when the board rolls
    /// into the next one.
    #[serde(default)]
    pub today_date: String,
    pub today: UsageStatistics,
    pub total: UsageStatistics,
}

impl Statistics {
    /// Every call the board has ever taken - the number PCBoard writes to the caller log
    /// and PPL reports, which runs into the millions on an old system.
    pub fn cur_caller_number(&self) -> u64 {
        self.total.calls
    }

    /// Clears the daily figures once the date has moved on. Every counter goes through
    /// here, so a board left running over midnight starts the new day on its next event.
    fn begin_day(&mut self) {
        let today = Local::now().date_naive().to_string();
        if self.today_date != today {
            self.today = UsageStatistics::default();
            self.today_date = today;
        }
    }

    pub fn add_caller(&mut self, user_name: String) {
        self.begin_day();
        self.total.calls += 1;
        self.today.calls += 1;
        self.last_callers.push(LastCaller {
            user_name,
            time: Utc::now().to_rfc3339(),
        });
        if self.last_callers.len() > 10 {
            self.last_callers.remove(0);
        }
    }

    pub fn add_message(&mut self) {
        self.begin_day();
        self.total.messages += 1;
        self.today.messages += 1;
    }

    pub fn add_download(&mut self, state: &icy_net::protocol::TransferState) {
        self.begin_day();
        let files = state.send_state.finished_files.len() as u64;
        // `file_size` is the file being sent and is cleared as each one finishes, so the
        // running total is the only thing left to count once a batch is done.
        let kb = state.send_state.total_bytes_transfered / 1024;
        self.total.downloads += files;
        self.total.downloads_kb += kb;

        self.today.downloads += files;
        self.today.downloads_kb += kb;
    }

    pub fn add_upload(&mut self, state: &icy_net::protocol::TransferState) {
        self.begin_day();
        let files = state.recieve_state.finished_files.len() as u64;
        let kb = state.recieve_state.total_bytes_transfered / 1024;
        self.total.uploads += files;
        self.total.uploads_kb += kb;

        self.today.uploads += files;
        self.today.uploads_kb += kb;
    }
}

impl IcyBoardSerializer for Statistics {
    const FILE_TYPE: &'static str = "statistics";
}

impl PCBoardBinImporter for Statistics {
    const SIZE: usize = 100;

    fn import_data(data: &[u8]) -> Res<Self> {
        const LAST_CALLER_LEN: usize = 54;
        const TIME_LEN: usize = 6;

        let last_caller = import_cp437_string(&data[0..LAST_CALLER_LEN], true);

        // let time = String::from_utf8_lossy(&data[LAST_CALLER_LEN..LAST_CALLER_LEN+TIME_LEN]).to_string();

        let i = LAST_CALLER_LEN + TIME_LEN;
        let new_msgs = i32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        let new_calls = i32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]);
        let total_up = i32::from_le_bytes([data[i + 8], data[i + 9], data[i + 10], data[i + 11]]);
        let total_dn = i32::from_le_bytes([data[i + 12], data[i + 13], data[i + 14], data[i + 15]]);
        let mut res = Statistics::default();
        res.last_callers.push(LastCaller {
            user_name: last_caller,
            time: Utc::now().to_rfc3339(),
        });
        res.total.calls = new_calls as u64;
        res.total.messages = new_msgs as u64;
        res.total.uploads_kb = total_up as u64;
        res.total.downloads_kb = total_dn as u64;
        Ok(res)
    }
}

impl PCBoardImport for Statistics {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        PCBoardBinImporter::import_pcboard(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{Statistics, UsageStatistics};
    use icy_net::protocol::TransferState;
    use std::path::PathBuf;

    /// What a protocol leaves behind after a batch: every file moved to `finished_files`,
    /// with the running total in `total_bytes_transfered` and `file_size` cleared.
    fn finished_batch(files: &[(&str, u64)]) -> TransferState {
        let mut state = TransferState::new("Test".to_string());
        for (name, size) in files {
            state.send_state.file_name = name.to_string();
            state.send_state.file_size = *size;
            state.send_state.total_bytes_transfered += *size;
            state.send_state.finish_file(PathBuf::from(name));

            state.recieve_state.file_name = name.to_string();
            state.recieve_state.file_size = *size;
            state.recieve_state.total_bytes_transfered += *size;
            state.recieve_state.finish_file(PathBuf::from(name));
        }
        state.is_finished = true;
        state
    }

    #[test]
    fn a_finished_download_counts_files_and_kilobytes() {
        let mut stats = Statistics::default();
        stats.add_download(&finished_batch(&[("A.ZIP", 2048), ("B.ZIP", 1024)]));
        assert_eq!(stats.total.downloads, 2);
        assert_eq!(stats.total.downloads_kb, 3);
        assert_eq!(stats.today.downloads_kb, 3);
    }

    #[test]
    fn a_finished_upload_counts_files_and_kilobytes() {
        let mut stats = Statistics::default();
        stats.add_upload(&finished_batch(&[("A.ZIP", 4096)]));
        assert_eq!(stats.total.uploads, 1);
        assert_eq!(stats.total.uploads_kb, 4);
        assert_eq!(stats.today.uploads_kb, 4);
    }

    /// The caller number is the lifetime one, which is what ends up in the caller log.
    #[test]
    fn the_caller_number_counts_every_call_ever_taken() {
        let mut stats = Statistics::default();
        stats.total.calls = 1_061_431;
        stats.add_caller("RAY COOK".to_string());
        assert_eq!(stats.cur_caller_number(), 1_061_432);
    }

    #[test]
    fn yesterdays_figures_are_cleared_on_the_next_day() {
        let mut stats = Statistics {
            today_date: "1993-09-06".to_string(),
            today: UsageStatistics {
                calls: 42,
                downloads: 7,
                ..Default::default()
            },
            ..Default::default()
        };
        stats.add_caller("RAY COOK".to_string());
        assert_eq!(stats.today.calls, 1);
        assert_eq!(stats.today.downloads, 0);
    }

    /// A second call on the same day adds to it rather than starting over.
    #[test]
    fn todays_figures_survive_within_the_day() {
        let mut stats = Statistics::default();
        stats.add_caller("RAY COOK".to_string());
        stats.add_caller("JOHN DOE".to_string());
        assert_eq!(stats.today.calls, 2);
    }

    #[test]
    fn a_posted_message_is_counted() {
        let mut stats = Statistics::default();
        stats.add_message();
        assert_eq!(stats.total.messages, 1);
        assert_eq!(stats.today.messages, 1);
    }
}

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
    /// Every call the board has ever taken - the number `PCBoard` writes to the caller log
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

    pub fn add_download_totals(&mut self, files: u64, bytes: u64) {
        self.begin_day();
        let kb = bytes / 1024;
        self.total.downloads += files;
        self.total.downloads_kb += kb;

        self.today.downloads += files;
        self.today.downloads_kb += kb;
    }

    pub fn add_upload_totals(&mut self, files: u64, bytes: u64) {
        self.begin_day();
        let kb = bytes / 1024;
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

    #[test]
    fn a_finished_download_counts_files_and_kilobytes() {
        let mut stats = Statistics::default();
        stats.add_download_totals(2, 2048 + 1024);
        assert_eq!(stats.total.downloads, 2);
        assert_eq!(stats.total.downloads_kb, 3);
        assert_eq!(stats.today.downloads_kb, 3);
    }

    #[test]
    fn a_finished_upload_counts_files_and_kilobytes() {
        let mut stats = Statistics::default();
        stats.add_upload_totals(1, 4096);
        assert_eq!(stats.total.uploads, 1);
        assert_eq!(stats.total.uploads_kb, 4);
        assert_eq!(stats.today.uploads_kb, 4);
    }

    #[test]
    fn transfer_totals_roll_over_and_add_with_per_batch_kilobyte_rounding() {
        let mut stats = Statistics {
            today_date: "1993-09-06".into(),
            today: UsageStatistics {
                downloads: 99,
                uploads: 99,
                ..Default::default()
            },
            ..Default::default()
        };
        stats.add_download_totals(2, 2047);
        stats.add_download_totals(1, 1023);
        stats.add_upload_totals(2, 3071);
        stats.add_upload_totals(1, 1023);
        for counts in [&stats.today, &stats.total] {
            assert_eq!(counts.downloads, 3);
            assert_eq!(counts.downloads_kb, 1);
            assert_eq!(counts.uploads, 3);
            assert_eq!(counts.uploads_kb, 2);
        }
    }

    #[tokio::test]
    async fn writer_preserves_concurrent_deltas_and_applies_reset_in_order() {
        use crate::icy_board::{IcyBoard, IcyBoardSerializer};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("statistics.toml");
        let mut board = IcyBoard::new();
        board.config.paths.statistics_file = path.clone();
        let board = Arc::new(Mutex::new(board));
        let mut jobs = Vec::new();
        for _ in 0..16 {
            let board = board.clone();
            jobs.push(tokio::spawn(async move {
                IcyBoard::write_statistics(&board, move |statistics| {
                    statistics.add_message();
                    statistics.add_download_totals(2, 3072);
                    statistics.add_upload_totals(1, 2048);
                })
                .await
                .unwrap();
            }));
        }
        for job in jobs {
            job.await.unwrap();
        }
        let saved = Statistics::load(&path).unwrap();
        for counts in [&saved.today, &saved.total] {
            assert_eq!(counts.messages, 16);
            assert_eq!(counts.downloads, 32);
            assert_eq!(counts.downloads_kb, 48);
            assert_eq!(counts.uploads, 16);
            assert_eq!(counts.uploads_kb, 32);
        }
        assert_eq!(board.lock().await.statistics.total.messages, 16);
        IcyBoard::write_statistics(&board, move |statistics| *statistics = Default::default())
            .await
            .unwrap();
        IcyBoard::write_statistics(&board, move |statistics| statistics.add_message()).await.unwrap();
        let saved = Statistics::load(&path).unwrap();
        assert_eq!(saved.total.messages, 1);
        assert_eq!(saved.total.downloads, 0);
        assert_eq!(saved.total.uploads, 0);

        // A directory as the destination forces the historical log-only save failure.
        board.lock().await.config.paths.statistics_file = directory.path().to_path_buf();
        IcyBoard::write_statistics(&board, move |statistics| statistics.add_message()).await.unwrap();
        assert_eq!(board.lock().await.statistics.total.messages, 2);
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

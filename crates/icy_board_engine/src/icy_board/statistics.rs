use std::path::Path;

use chrono::Utc;
use icy_ppe::{tables::import_cp437_string, Res};
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
    pub today: UsageStatistics,
    pub total: UsageStatistics,
}

impl Statistics {
    pub fn add_caller(&mut self, user_name: String) {
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

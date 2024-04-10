use super::IcyBoardSerializer;
use super::{is_false, is_null_32, is_null_64, is_null_8, PCBoardImport, PCBoardTextImport};
use icy_ppe::Res;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct SecurityLevel {
    #[serde(default)]
    #[serde(rename = "enabled")]
    #[serde(skip_serializing_if = "is_false")]
    pub is_enabled: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub security: u8,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub time_per_day: u32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub calls_per_day: u32,

    #[serde(default)]
    #[serde(rename = "demo_account")]
    #[serde(skip_serializing_if = "is_false")]
    pub is_demo_account: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_alias: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub enforce_time_limit: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub enforce_read_mail: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub uldl_ratio: u32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub uldl_kb_ratio: u32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub daily_file_limit: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub daily_file_kb_limit: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub file_limit: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub file_kb_limit: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub file_credit: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub file_kb_credit: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SecurityLevelDefinitions {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "level")]
    pub levels: Vec<SecurityLevel>,
}

impl PCBoardTextImport for SecurityLevelDefinitions {
    fn import_data(data: String) -> Res<Self> {
        let mut res = Self::default();
        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            let splitted_line = line.split(',').collect::<Vec<&str>>();
            if splitted_line.len() != 18 {
                continue;
            }

            let password = splitted_line[0].to_string();
            let security = splitted_line[1].parse::<u8>().unwrap_or(0);
            let time_per_day = splitted_line[2].parse::<u32>().unwrap_or(0);
            let daily_file_kb_limit = splitted_line[3].parse::<u64>().unwrap_or(0);
            // 4 Base Baud - not needed
            // 5 Batch Limit - not needed

            let uldl_ratio = splitted_line[6].parse::<u32>().unwrap_or(0);
            let uldl_kb_ratio = splitted_line[7].parse::<u32>().unwrap_or(0);

            let file_limit = splitted_line[8].parse::<u64>().unwrap_or(0);
            let file_kb_limit = splitted_line[9].parse::<u64>().unwrap_or(0);

            let enforce_time_limit = splitted_line[10] == "Y";
            let allow_alias = splitted_line[11] == "Y";
            let enforce_read_mail = splitted_line[12] == "Y";
            let is_demo_account = splitted_line[13] == "Y";
            // skip 14? - seems to be unused bool flag
            let file_credit = splitted_line[15].parse::<u64>().unwrap_or(0);
            let file_kb_credit = splitted_line[16].parse::<u64>().unwrap_or(0);

            let is_enabled = splitted_line[17] == "Y";

            // new settings
            let calls_per_day = 0;
            let daily_file_limit = 0;

            res.levels.push(SecurityLevel {
                is_enabled,
                password,
                security,
                time_per_day,
                calls_per_day,
                is_demo_account,
                allow_alias,
                enforce_time_limit,
                enforce_read_mail,
                uldl_ratio,
                uldl_kb_ratio,
                daily_file_limit,
                daily_file_kb_limit,
                file_limit,
                file_kb_limit,
                file_credit,
                file_kb_credit,
            });
        }
        Ok(res)
    }
}

impl PCBoardImport for SecurityLevelDefinitions {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        PCBoardTextImport::import_pcboard(path)
    }
}

impl IcyBoardSerializer for SecurityLevelDefinitions {
    const FILE_TYPE: &'static str = "securiy level";
}

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Res;

use super::{IcyBoardSerializer, PCBoardBinImporter, PCBoardImport};

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AccountingConfig {
    pub new_user_balance: f64,
    pub warn_level: f64,

    pub charge_per_logon: f64,
    pub charge_per_time: f64,
    pub charge_per_peak_time: f64,
    pub charge_per_group_chat_time: f64,

    pub charge_per_msg_read: f64,
    pub charge_per_msg_read_captured: f64,
    pub charge_per_msg_written: f64,
    pub charge_per_msg_write_echoed: f64,
    pub charge_per_msg_write_private: f64,

    pub charge_per_download_file: f64,
    pub charge_per_download_bytes: f64,

    pub pay_back_for_upload_file: f64,
    pub pay_back_for_upload_bytes: f64,
}

impl AccountingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn export_pcboard(&self) -> Vec<u8> {
        let mut res = Vec::new();

        res.extend_from_slice(&self.new_user_balance.to_le_bytes());
        res.extend_from_slice(&self.charge_per_logon.to_le_bytes());
        res.extend_from_slice(&self.charge_per_time.to_le_bytes());
        res.extend_from_slice(&self.charge_per_peak_time.to_le_bytes());
        res.extend_from_slice(&self.charge_per_group_chat_time.to_le_bytes());
        res.extend_from_slice(&self.charge_per_msg_read.to_le_bytes());
        res.extend_from_slice(&self.charge_per_msg_read_captured.to_le_bytes());
        res.extend_from_slice(&self.charge_per_msg_written.to_le_bytes());
        res.extend_from_slice(&self.charge_per_msg_write_echoed.to_le_bytes());
        res.extend_from_slice(&self.charge_per_msg_write_private.to_le_bytes());
        res.extend_from_slice(&self.charge_per_download_file.to_le_bytes());
        res.extend_from_slice(&self.charge_per_download_bytes.to_le_bytes());
        res.extend_from_slice(&self.pay_back_for_upload_file.to_le_bytes());
        res.extend_from_slice(&self.pay_back_for_upload_bytes.to_le_bytes());
        res.extend_from_slice(&self.warn_level.to_le_bytes());
        res
    }
}

impl IcyBoardSerializer for AccountingConfig {
    const FILE_TYPE: &'static str = "accounting";
}

impl PCBoardBinImporter for AccountingConfig {
    const SIZE: usize = 15 * 8;

    fn import_data(mut data: &[u8]) -> Res<Self> {
        let new_user_balance = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_logon = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_time = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_peak_time = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_group_chat = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_msg_read = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_msg_read_capture = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_msg_write = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_msg_write_echoed = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_msg_write_private = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_download_file = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let charge_per_download_bytes = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let pay_back_for_upload_file = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let pay_back_for_upload_bytes = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        data = &data[8..];
        let warn_level = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        Ok(AccountingConfig {
            new_user_balance,
            charge_per_logon,
            charge_per_time,
            charge_per_peak_time,
            charge_per_group_chat_time: charge_per_group_chat,
            charge_per_msg_read,
            charge_per_msg_read_captured: charge_per_msg_read_capture,
            charge_per_msg_written: charge_per_msg_write,
            charge_per_msg_write_echoed,
            charge_per_msg_write_private,
            charge_per_download_file,
            charge_per_download_bytes,
            pay_back_for_upload_file,
            pay_back_for_upload_bytes,
            warn_level,
        })
    }
}

impl PCBoardImport for AccountingConfig {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        PCBoardBinImporter::import_pcboard(path)
    }
}

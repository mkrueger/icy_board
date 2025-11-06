use std::{mem, path::PathBuf, time::Instant};

#[derive(Debug, Clone)]
pub enum OutputLogMessage {
    Info(String),
    Warning(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct TransferInformation {
    pub file_name: String,
    pub file_size: u64,
    pub total_bytes_transfered: u64,
    pub cur_bytes_transfered: u64,

    pub errors: usize,
    pub warnings: usize,
    pub check_size: String,

    pub bytes_transferred_timed: u64,

    pub output_log: Vec<OutputLogMessage>,

    pub finished_files: Vec<(String, PathBuf)>,
}

impl TransferInformation {
    pub fn finish_file(&mut self, file: PathBuf) {
        let mut file_name = String::new();
        mem::swap(&mut self.file_name, &mut file_name);
        self.finished_files.push((file_name, file));
        self.file_size = 0;
        self.cur_bytes_transfered = 0;
    }

    pub fn has_log_entries(&self) -> bool {
        !self.output_log.is_empty()
    }

    pub fn errors(&self) -> usize {
        self.errors
    }

    pub fn warnings(&self) -> usize {
        self.warnings
    }

    pub fn log_count(&self) -> usize {
        self.output_log.len()
    }

    /// Get's a log message where
    /// `category` 0 = all, 1 = warnings, 2 = errors
    /// `index` is the index of the message
    pub fn get_log_message(&self, category: usize, index: usize) -> Option<&OutputLogMessage> {
        match category {
            0 => self.output_log.get(index),
            1 => self.output_log.iter().filter(|p| matches!(p, OutputLogMessage::Warning(_))).nth(index),
            2 => self.output_log.iter().filter(|p| matches!(p, OutputLogMessage::Error(_))).nth(index),
            _ => None,
        }
    }

    pub fn log_info(&mut self, txt: impl Into<String>) {
        self.output_log.push(OutputLogMessage::Info(txt.into()));
    }

    pub fn log_warning(&mut self, txt: impl Into<String>) {
        self.warnings += 1;
        self.output_log.push(OutputLogMessage::Warning(txt.into()));
    }

    pub fn log_error(&mut self, txt: impl Into<String>) {
        self.errors += 1;
        self.output_log.push(OutputLogMessage::Error(txt.into()));
    }
}

impl Default for TransferInformation {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            file_size: 0,
            total_bytes_transfered: 0,
            cur_bytes_transfered: 0,
            errors: 0,
            warnings: 0,
            finished_files: Vec::new(),
            check_size: String::new(),
            output_log: Vec::new(),
            bytes_transferred_timed: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransferState {
    pub current_state: &'static str,
    pub is_finished: bool,

    pub protocol_name: String,

    pub start_time: Instant,

    pub send_state: TransferInformation,

    pub recieve_state: TransferInformation,

    pub request_cancel: bool,
}

impl TransferState {
    pub fn new(protocol_name: String) -> Self {
        Self {
            current_state: "",
            protocol_name,
            is_finished: false,
            start_time: Instant::now(),
            send_state: TransferInformation::default(),
            recieve_state: TransferInformation::default(),
            request_cancel: false,
        }
    }

    pub fn get_bps(&self, download: bool) -> usize {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0; // Avoid division by zero
        }

        let bytes_transferred = if download {
            self.recieve_state.cur_bytes_transfered
        } else {
            self.send_state.cur_bytes_transfered
        };

        (bytes_transferred as f64 / elapsed) as usize
    }
}

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

    pub start_time: Instant,
    pub bps: f32,
    pub bps_counter: u64,

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
        self.reset_cur_transfer();
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

    pub fn reset_cur_transfer(&mut self) {
        self.start_time = Instant::now();
        self.cur_bytes_transfered = 0;
        self.bps_counter = 0;
        self.bytes_transferred_timed = 0;
    }
}

impl Default for TransferInformation {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            start_time: Instant::now(),
            file_size: 0,
            total_bytes_transfered: 0,
            cur_bytes_transfered: 0,
            bps: 0.0,
            bps_counter: 0,
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
            send_state: TransferInformation::default(),
            recieve_state: TransferInformation::default(),
            request_cancel: false,
        }
    }

    pub fn get_current_bps(&self, download: bool) -> usize {
        let (elapsed, bytes_transferred) = if download {
            let elapsed = self.recieve_state.start_time.elapsed().as_secs_f64();
            if elapsed < 0.001 {
                return 0; // Avoid division by zero
            }
            (elapsed, self.recieve_state.cur_bytes_transfered)
        } else {
            let elapsed = self.send_state.start_time.elapsed().as_secs_f64();
            if elapsed < 0.001 {
                return 0; // Avoid division by zero
            }
            (elapsed, self.send_state.cur_bytes_transfered)
        };
        let bps = (bytes_transferred as f64 / elapsed) as usize;

        if bps == 0 {
            return 0;
        }

        let magnitude = (bps as f64).log10().floor() as i32;
        if magnitude < 2 {
            return bps;
        }

        // Round to 2 significant figures
        let divisor = 10_usize.pow((magnitude - 1) as u32);
        let rounded = ((bps + divisor / 2) / divisor) * divisor;

        rounded
    }
}

use std::path::Path;

use icy_board_engine::Res;
use std::fmt::Write as _;

#[derive(Default)]
pub struct ImportLog {
    pub output: String,
}

impl ImportLog {
    pub(crate) fn created_directory(&mut self, dir: std::path::PathBuf) {
        let _ = writeln!(self.output, "Directory {} created.", dir.display());
    }

    pub fn log_error(&mut self, res: Option<std::io::Error>) -> Res<()> {
        match res {
            None => Ok(()),
            Some(e) => {
                let _ = writeln!(self.output, "Error {}", e);
                Err(e.into())
            }
        }
    }
    pub fn log_boxed_error(&mut self, e: &dyn std::error::Error) {
        let _ = writeln!(self.output, "Error {}", e);
    }

    pub(crate) fn converted_file(&mut self, src: &Path, destination: &Path, converted_to_utf8: bool) {
        if converted_to_utf8 {
            self.output
                .push_str(&format!("Converted {} to {} using utf-8 output.\n", src.display(), destination.display()));
        } else {
            let _ = writeln!(self.output, "Converted {} to {}", src.display(), destination.display());
        }
    }

    pub(crate) fn translated_file(&mut self, src: &Path, destination: &Path) {
        let _ = writeln!(self.output, "Translated {} to {}", src.display(), destination.display());
    }

    pub(crate) fn copy_file(&mut self, src: &Path, destination: &Path) {
        let _ = writeln!(self.output, "Copied {} to {}", src.display(), destination.display());
    }

    pub(crate) fn create_new_file(&mut self, new_name: impl Into<String>) {
        let _ = writeln!(self.output, "Created {}.", new_name.into());
    }

    pub(crate) fn log(&mut self, arg: &str) {
        self.output.push_str(arg);
        self.output.push('\n');
    }
}

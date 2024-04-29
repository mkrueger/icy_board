use std::path::Path;

use icy_board_engine::Res;

#[derive(Default)]
pub struct ImportLog {
    pub output: String,
}

impl ImportLog {
    pub(crate) fn created_directory(&mut self, dir: std::path::PathBuf) {
        self.output.push_str(&format!("Directory {} created.\n", dir.display()));
    }

    pub fn log_error(&mut self, res: Option<std::io::Error>) -> Res<()> {
        match res {
            None => Ok(()),
            Some(e) => {
                self.output.push_str(&format!("Error {}\n", e));
                Err(e.into())
            }
        }
    }
    pub fn log_boxed_error(&mut self, e: &dyn std::error::Error) {
        self.output.push_str(&format!("Error {}\n", e));
    }

    pub(crate) fn converted_file(&mut self, src: &Path, destination: &Path, converted_to_utf8: bool) {
        if converted_to_utf8 {
            self.output
                .push_str(&format!("Converted {} to {} using utf-8 output.\n", src.display(), destination.display()));
        } else {
            self.output.push_str(&format!("Converted {} to {}\n", src.display(), destination.display()));
        }
    }

    pub(crate) fn translated_file(&mut self, src: &Path, destination: &Path) {
        self.output.push_str(&format!("Translated {} to {}\n", src.display(), destination.display()));
    }

    pub(crate) fn copy_file(&mut self, src: &Path, destination: &Path) {
        self.output.push_str(&format!("Copied {} to {}\n", src.display(), destination.display()));
    }

    pub(crate) fn create_new_file(&mut self, new_name: impl Into<String>) {
        self.output.push_str(&format!("Created {}.\n", new_name.into()));
    }

    pub(crate) fn log(&mut self, arg: &str) {
        self.output.push_str(arg);
        self.output.push('\n');
    }
}

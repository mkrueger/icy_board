use std::{
    fs,
    path::{Path, PathBuf},
};

use codepages::tables::CP437_TO_UNICODE;

pub struct ErrorContainer {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    pub span: core::ops::Range<usize>,
    pub file_name: PathBuf,
}

#[derive(Default)]
pub struct ErrorReporter {
    cur_file: PathBuf,
    pub errors: Vec<ErrorContainer>,
    pub warnings: Vec<ErrorContainer>,
}

impl ErrorReporter {
    pub fn file_name(&self) -> &Path {
        &self.cur_file
    }

    pub fn set_file_name(&mut self, file_name: &Path) {
        self.cur_file = file_name.to_path_buf();
    }

    pub fn report_error<T: std::error::Error + 'static + Send + Sync>(&mut self, span: core::ops::Range<usize>, error: T) {
        self.errors.push(ErrorContainer {
            error: Box::new(error),
            span,
            file_name: self.cur_file.clone(),
        });
    }

    pub fn report_error_file<T: std::error::Error + 'static + Send + Sync>(&mut self, file_name: PathBuf, span: core::ops::Range<usize>, error: T) {
        self.errors.push(ErrorContainer {
            error: Box::new(error),
            span,
            file_name,
        });
    }

    pub fn report_warning<T: std::error::Error + 'static + Send + Sync>(&mut self, span: core::ops::Range<usize>, warning: T) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
            file_name: self.cur_file.clone(),
        });
    }

    pub fn report_warning_file<T: std::error::Error + 'static + Send + Sync>(&mut self, file_name: PathBuf, span: core::ops::Range<usize>, warning: T) {
        self.warnings.push(ErrorContainer {
            error: Box::new(warning),
            span,
            file_name,
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn report(&self) {}
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Detect,
    CP437,
    Utf8,
}

/// .
///
/// # Errors
///
/// This function will return an error if .
pub fn load_with_encoding<P: AsRef<Path>>(file_name: &P, encoding: Encoding) -> std::io::Result<String> {
    if encoding == Encoding::Detect {
        let src_data = fs::read(file_name)?;
        let src = codepages::tables::get_utf8(&src_data);
        return Ok(src);
    }
    let src_data = fs::read(file_name)?;
    let src = if encoding == Encoding::CP437 {
        let mut res = String::new();
        for b in src_data {
            res.push(CP437_TO_UNICODE[b as usize]);
        }
        res
    } else {
        codepages::tables::get_utf8(&src_data)
    };
    Ok(src)
}

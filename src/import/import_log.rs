pub struct MissingFile {
    pub file: String,
    pub context: String,
}

impl MissingFile {
    pub fn new(file: impl Into<String>, context: impl Into<String>) -> Self {
        MissingFile {
            file: file.into(),
            context: context.into(),
        }
    }
}

pub struct ImportLog {
    pub missing_files: Vec<MissingFile>,
}

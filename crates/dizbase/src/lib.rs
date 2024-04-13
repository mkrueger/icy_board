pub mod file_base;
pub mod file_base_scanner;
mod macros;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod extensions {
    /// filename.IDX - File index
    pub const FILE_INDEX: &str = "idx";

    /// filename.FMD - Lastread information
    pub const FILE_METADATA: &str = "fmd";
}

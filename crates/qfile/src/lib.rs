//! # Qfile
//!
//! The Qfile crate provides functionality for accessing a file by path, case-insensitive, including automatic detection, creation of a path with a new file or opening an existing file. It includes several submodules to handle different aspects of file handling, such as initialization, reading, and writing. The crate also defines some custom errors related to file operations.
//!
/*
Module Items

- find: a module that defines functions for finding file paths that match a given set of names, with options to exclude certain directories or follow symbolic links.
- init: a module that defines functions for initializing a file path, including creating the necessary directories if they don't already exist.
paths: a module that defines functions for manipulating file paths, such as joining and normalizing them.
- qerror: a module that defines custom error types for the crate.
- read: a module that defines functions for reading file contents.
- write: a module that defines functions for writing to files.
- prelude_async: a module that defines traits and utility functions for asynchronous file operations.
- prelude_sync: a module that defines traits and utility functions for synchronous file operations.

Exported Items

- QPackError: the custom error type for the crate, which includes variants for common file-related errors.
- QTraitAsync: a trait for asynchronous file operations that defines methods for reading, writing, and manipulating file paths.
- QTraitSync: a trait for synchronous file operations that defines methods for reading, writing, and manipulating file paths.
 */

mod find;
mod init;
mod paths;
mod qerror;
mod read;
mod write;
pub use qerror::QPackError;
use std::path::PathBuf;
mod prelude_async;
mod prelude_sync;
pub use async_mutex::Mutex as AsyncMutex;
pub use prelude_async::QTraitAsync;
pub use prelude_sync::QTraitSync;
//Flag: an enum that defines flags for specifying whether to use an existing file
// or create a new one, or let the crate automatically determine which to use.
#[derive(Debug, Clone)]
enum Flag {
    Old,
    Auto,
    New,
}
/// Directory: an enumeration defining options for specifying which directories should be searched in the file system files
#[derive(Debug, Clone)]
pub enum Directory<T: AsRef<str> + Send + Sync> {
    ThisPlace(Vec<T>),
    Everywhere,
}
// CodeStatus: an enum that defines options
// for specifying whether the current code is being executed synchronously or asynchronously
#[derive(Debug, Clone, PartialEq)]
enum CodeStatus {
    SyncStatus,
    AsyncStatus,
}

/// QFilePath: a struct that represents a file path and includes methods for manipulating it,
/// such as checking the current code execution status and updating the path if necessary.
/// To use the methods, you need to import `QTraitSync` or `QTraitASync` or both.
#[derive(Debug, Clone)]
pub struct QFilePath {
    request_items: Vec<String>, // Vector of items requested by user
    user_path: PathBuf,         // User-specified path
    file_name: PathBuf,         // User-specified file name
    correct_path: PathBuf,      // The final path to the file
    flag: Flag,                 // Flag to indicate if the file already exists or not
    update_path: bool,          // Flag to indicate if the file path needs to be updated
    status: CodeStatus,         // Status of the code, whether it is running synchronously or asynchronously
}
impl QFilePath {
    /*
    check_status_code is a private method that checks
    if the status of the QFilePath instance matches the expected CodeStatus and returns an error if they do not match.
     */
    fn check_status_code(&self, status: CodeStatus) -> Result<(), QPackError> {
        let check = |err_st: QPackError| -> Result<(), QPackError> {
            if self.status == status {
                return Ok(());
            }
            return Err(err_st);
        };
        match self.status {
            CodeStatus::SyncStatus => check(QPackError::AsyncCallFromSync),
            CodeStatus::AsyncStatus => check(QPackError::SyncCallFromAsync),
        }
    }
}
impl Drop for QFilePath {
    fn drop(&mut self) {}
}

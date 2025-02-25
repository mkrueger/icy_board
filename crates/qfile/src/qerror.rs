use std::error::Error;

use thiserror::Error;
#[derive(Error, Debug)]
/// This Rust code defines an error type QPackError for handling QFilePath cases, using the Error trait from the Rust standard library and the Error macro from the thiserror crate.
///
/// The QPackError type has several variants, each representing a different error condition that can arise while using the QFilePath library:
pub enum QPackError {
    /// - `UnixPathIsIncorrect`: Returns an error if you use a non-unix format for the path.
    #[error(
        "You are using the windows path format for Unix. Use `unix` format for the path:\n> ./folder1/folder2/file.txt\n> ../folder2/file.txt\n> ./file.txt"
    )]
    UnixPathIsIncorrect,
    /// - `WindowsPathIsIncorrect`: Returns an error if you use a non-windows format for the path.
    #[error(
        "You are using the unix path format for Windows. Use `windows` format for the path:\n> .\\folder1\\folder2\\file.txt\n> ..\\folder2\\file.txt\n> .\\file.txt"
    )]
    WindowsPathIsIncorrect,
    /// - `SystemNotDefined`: Returns an error if the library is not prepared for this operating system.
    #[error("SystemNotDefined")]
    SystemNotDefined,
    /// - `PathIsEmpty`: Returns an error if you specify an empty path.
    #[error("The path is empty")]
    PathIsEmpty,
    /// - `PathIsIncorrect`: Returns an error if the path is incorrect.
    #[error("The path is incorrect")]
    PathIsIncorrect,
    /// - `NotQPackError`: Returns an error if you try to get QPackError from Box<dyn Error> that contains error != QPackError.
    #[error("Not covered error")]
    NotQPackError,
    /// - `AsyncCallFromSync`: Returns an error if an asynchronous call is made from SyncPack (use a similar function from AsyncPack).
    #[error("Asynchronous call from SyncPack (use a similar function from SyncPack)")]
    AsyncCallFromSync,
    /// - `SyncCallFromAsync`: Returns an error if a synchronous call is made from AsyncPack (use a similar function from SyncPack).
    #[error("Synchronous call from AsyncPack (use a similar function from SyncPack)")]
    SyncCallFromAsync,
    /// - `IoError`: Returns an error from IO, which is wrapped using the from attribute.
    #[error("Error from IO")]
    IoError(#[from] std::io::Error),
}

/// Converting from Box<QPackError> to QPackError
impl From<Box<QPackError>> for QPackError {
    fn from(value: Box<QPackError>) -> Self {
        *value
    }
}
/// Converting from Box<dyn Error + Send + Sync> to QPackError
impl From<Box<dyn Error + Send + Sync>> for QPackError {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        if let Ok(unpacked_value) = value.downcast::<QPackError>() {
            return unpacked_value.into();
        }
        QPackError::NotQPackError
    }
}
/// Converting from Box<dyn Error + Send + Sync> to Box<QPackError>
impl From<Box<dyn Error + Send + Sync>> for Box<QPackError> {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        if let Ok(unpacked_value) = value.downcast::<QPackError>() {
            return unpacked_value.into();
        }
        Box::new(QPackError::NotQPackError)
    }
}

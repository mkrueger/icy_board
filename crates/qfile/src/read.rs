use super::{QFilePath, QPackError};
use crate::init::work_with_elements::*;
use crate::paths::get_path::*;
use futures_lite::AsyncReadExt;
use std::io::Read;
pub mod read {
    use super::*;
    // Synchronously read the contents of a file specified by `QFilePath`
    //
    // # Arguments
    // * `slf` - mutable reference to the `QFilePath` instance
    //
    // # Returns
    // * `Result<String, QPackError>` - returns the contents of the file as a `String` if successful, otherwise returns a `QPackError`
    pub fn read(slf: &mut QFilePath) -> Result<String, QPackError> {
        // Get the file path as a `String` and then read the contents of the file into the `text` variable
        let mut text = String::new();
        return_file(&get_path_string(slf)?)?.read_to_string(&mut text)?;
        Ok(text)
    }
    // Asynchronously read the contents of a file specified by `QFilePath`
    //
    // # Arguments
    // * `slf` - mutable reference to the `QFilePath` instance
    //
    // # Returns
    // * `Result<String, QPackError>` - returns the contents of the file as a `String` if successful, otherwise returns a `QPackError`
    pub async fn async_read(slf: &mut QFilePath) -> Result<String, QPackError> {
        let mut text = String::new();
        // Get the file path as a `String` asynchronously and then read the contents of the file into the `text` variable
        async_return_file(&async_get_path_string(slf).await?).await?.read_to_string(&mut text).await?;
        Ok(text)
    }
}

use super::{Flag, PathBuf, QFilePath, QPackError};
use crate::init::path_for_write::*;
use crate::init::work_with_elements::*;
use crate::paths::get_path::*;
use async_fs;
use async_recursion::async_recursion;
use futures_lite::AsyncWriteExt;
use std::error::Error;
use std::fs;
use std::io::Write;
/*
This Rust code defines a write module containing an auto_write function that can be used to write text to a file. The function takes a modifiable reference to a QFilePath object and a string of text to write.

The function first checks if the user has updated the path, and if so, updates the correct path. Then it checks the current status of the flag, which indicates whether the file is old, new or should be defined automatically.

If the file is old, the function opens the file in add mode and writes text at the end of the file. If the file does not exist, the function creates a new file and writes the text. If the file is auto, the function first checks if the file exists. If it exists, the function sets the old flag and recursively calls itself to write text. If the file does not exist, the function creates a path and recursively calls itself to write text.
 */
pub mod write {
    use super::*;
    pub fn auto_write<T: AsRef<str>>(slf: &mut QFilePath, text: T) -> Result<(), QPackError> {
        if slf.update_path {
            slf.correct_path = PathBuf::from(format!("{}{}", slf.user_path.to_str().unwrap(), slf.file_name.to_str().unwrap()));
        }
        match slf.flag {
            Flag::Old => {
                let temp = get_path_buf(slf)?;
                slf.flag = Flag::Auto;
                fs::OpenOptions::new().append(true).open(temp)?.write_all(text.as_ref().as_bytes())?;
            }
            Flag::New => {
                let path = get_path_buf(slf)?;
                let file = fs::File::create(path);
                match file {
                    Ok(_) => {
                        slf.update_path = false;
                        let temp = get_path_buf(slf)?;
                        slf.flag = Flag::Auto;
                        fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(temp)?
                            .write_all(text.as_ref().as_bytes())?;
                    }
                    Err(err) => {
                        return Err(QPackError::IoError(err));
                    }
                };
            }
            Flag::Auto => {
                let path = get_path_buf(slf)?;
                let file: Result<fs::File, QPackError> = return_file(&path.to_str().unwrap());
                match file {
                    Ok(_) => {
                        slf.flag = Flag::Old;
                        auto_write(slf, text)?;
                    }
                    Err(err) => {
                        if let QPackError::IoError(err) = err {
                            match err.kind() {
                                _ => {
                                    path_create(slf, err.kind())?;
                                    auto_write(slf, text)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[async_recursion]
    pub async fn async_auto_write<T: AsRef<str> + Send + Sync>(slf: &mut QFilePath, text: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        if slf.update_path {
            slf.correct_path = PathBuf::from(format!("{}{}", slf.user_path.to_str().unwrap(), slf.file_name.to_str().unwrap()));
        }
        match slf.flag {
            Flag::Old => {
                let temp = async_get_path_buf(slf).await?;
                slf.flag = Flag::Auto;
                async_fs::OpenOptions::new()
                    .append(true)
                    .open(temp)
                    .await?
                    .write_all(text.as_ref().as_bytes())
                    .await?;
            }
            Flag::New => {
                let path = async_get_path_buf(slf).await?;
                let file = async_fs::File::create(path).await;
                match file {
                    Ok(_) => {
                        slf.update_path = false;
                        let temp = async_get_path_buf(slf).await?;
                        slf.flag = Flag::Auto;
                        async_fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(temp)
                            .await?
                            .write_all(text.as_ref().as_bytes())
                            .await?;
                    }
                    Err(err) => {
                        return Err(QPackError::from(QPackError::IoError(err)).into());
                    }
                };
            }
            Flag::Auto => {
                let path = async_get_path_buf(slf).await?;
                let file: Result<async_fs::File, QPackError> = async_return_file(&path.to_str().unwrap()).await;
                match file {
                    Ok(_) => {
                        slf.flag = Flag::Old;
                        async_auto_write(slf, text).await?;
                    }
                    Err(err) => {
                        if let QPackError::IoError(err) = err {
                            match err.kind() {
                                _ => {
                                    async_path_create(slf, err.kind()).await?;
                                    async_auto_write(slf, text).await?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn write_only_new<T: AsRef<str>>(slf: &mut QFilePath, text: T) -> Result<(), QPackError> {
        slf.flag = Flag::New;
        if let Err(err) = auto_write(slf, &text) {
            if let QPackError::IoError(err) = err {
                match err.kind() {
                    _ => {
                        path_create(slf, err.kind())?;
                        auto_write(slf, &text)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub async fn async_write_only_new<T: AsRef<str> + Send + Sync>(slf: &mut QFilePath, text: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        slf.flag = Flag::New;
        if let Err(err) = async_auto_write(slf, &text).await {
            if let Ok(err) = err.downcast::<QPackError>() {
                if let QPackError::IoError(err) = *err {
                    match err.kind() {
                        _ => {
                            async_path_create(slf, err.kind()).await?;
                            async_auto_write(slf, &text).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

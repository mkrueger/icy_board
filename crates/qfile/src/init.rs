use super::{CodeStatus, Flag, PathBuf, QFilePath, QPackError};
use async_fs;
use async_mutex::Mutex;
use futures_lite::stream::StreamExt;
use lazy_static::lazy_static;
use regex::Regex;
use std::{fs, path::Path};
/*
This is a module called constructor that contains functions for creating QFilePath objects.
The core function is a helper function that takes a path and a status, checks if the path is valid, and constructs and returns a new QFilePath object with the given path and status parameters.
 */
pub mod constructor {
    use super::*;
    fn core<T: AsRef<str>>(path_file: T) -> Result<(), QPackError> {
        // Check if the path_file is an empty string.
        if path_file.as_ref().to_string().is_empty() {
            return Err(QPackError::PathIsEmpty);
        }
        // Convert the path_file to a PathBuf object.
        let path_file = PathBuf::from(path_file.as_ref());
        // Check if the path is valid for the current operating system.
        if cfg!(unix) {
            if path_file.to_str().unwrap().contains("\\") {
                return Err(QPackError::UnixPathIsIncorrect);
            }
        } else if cfg!(windows) {
            if path_file.to_str().unwrap().contains("/") {
                return Err(QPackError::WindowsPathIsIncorrect);
            }
        } else {
            return Err(QPackError::SystemNotDefined);
        }
        Ok(())
        // Create and return a new QFilePath object with the given path_file and status parameters.
    }
    fn core_run_sync<T: AsRef<str>>(path_file: T, status: CodeStatus) -> Result<QFilePath, QPackError> {
        core(&path_file)?;
        let path_file = PathBuf::from(path_file.as_ref());
        Ok(QFilePath {
            request_items: Default::default(),
            user_path: path_file,
            file_name: Default::default(),
            correct_path: Default::default(),
            flag: Flag::Auto,
            update_path: false,
            status,
        })
    }
    async fn core_run_async<T: AsRef<str>>(path_file: T, status: CodeStatus) -> Result<Mutex<QFilePath>, QPackError> {
        core(&path_file)?;
        let path_file = PathBuf::from(path_file.as_ref());
        Ok(Mutex::new(QFilePath {
            request_items: Default::default(),
            user_path: path_file,
            file_name: Default::default(),
            correct_path: Default::default(),
            flag: Flag::Auto,
            update_path: false,
            status,
        }))
    }
    // The add_path function is a public synchronous function that calls the core function with a SyncStatus parameter.
    pub fn add_path<T: AsRef<str>>(path_file: T) -> Result<QFilePath, QPackError> {
        core_run_sync(path_file, CodeStatus::SyncStatus)
    }
    // The async_add_path function is a public asynchronous function that calls the core function with an AsyncStatus parameter.
    pub async fn async_add_path<T: AsRef<str> + Send + Sync>(path_file: T) -> Result<Mutex<QFilePath>, QPackError> {
        core_run_async(path_file, CodeStatus::AsyncStatus).await
    }
}
/*
This code defines a module called path_separation. The module contains functions to separate a file path into its constituent parts. The core function is a private helper function that performs the actual work of separating the path. It takes a mutable reference to a QFilePath object and modifies it in place.

The core function calls the first_slash helper function, which checks if the path has a leading slash and adds one if not. It uses a regular expression to check for the leading slash depending on the operating system.

The core function then extracts each element of the path into a vector, starting from the root directory, and stores it in the request_items field of the QFilePath object. It removes the last element if it is empty (which can happen if the path ends with a slash) and replaces "." and ".." with "./" and "../", respectively, if necessary. Finally, it reverses the vector so that it goes from the root directory
 */
pub mod path_separation {
    use super::*;
    fn core(slf: &mut QFilePath) {
        fn first_slash(sl: &mut QFilePath) {
            // Get the string representation of the path and check if it starts with a slash.
            let temp = sl.user_path.display().to_string();
            lazy_static! {
                static ref SL: Regex = {
                    #[cfg(unix)]
                    {
                        // Use regex to check for a Unix path that starts with a slash, "../", or "./".
                        Regex::new(r"^/|^\.\./|^\./").unwrap()
                    }
                    #[cfg(windows)]
                    {
                        // Use regex to check for a Windows path that starts with a drive letter followed by a backslash, "..\","disk:\\", or ".\"
                        Regex::new(r"^.:\\|^\.\.\\|^\.\\").unwrap()
                    }
                };
            }
            // If the path does not start with a slash, add "./" or ".\\" as appropriate.
            if !SL.is_match(&temp) {
                sl.user_path = PathBuf::from(format!(
                    "{}{}",
                    {
                        #[cfg(unix)]
                        {
                            "./"
                        }
                        #[cfg(windows)]
                        {
                            ".\\"
                        }
                    },
                    sl.user_path.display()
                ));
            }
        }
        // Call the first_slash function to ensure that the path starts with "./" or ".\\" if necessary
        first_slash(slf);
        // Set the request_items field of the QFilePath struct to a Vec of strings containing each element of the path.
        slf.request_items = slf.user_path.ancestors().map(|element| element.display().to_string()).collect();
        // If the last element of the path is an empty string, remove it.
        if slf.request_items.last().unwrap().eq("") {
            slf.request_items.pop();
            // If the new last element of the path is "." or "..", add "./" or ".\\" as appropriate.
            if let Some(value) = slf.request_items.last_mut() {
                #[cfg(unix)]
                {
                    if value.eq(&mut ".") {
                        *value = String::from("./")
                    }
                    if value.eq(&mut "..") {
                        *value = String::from("../")
                    }
                }
                #[cfg(windows)]
                {
                    if value.eq(&mut ".") {
                        *value = String::from(".\\")
                    }
                    if value.eq(&mut "..") {
                        *value = String::from("..\\")
                    }
                }
            }
        }
        // Reverse the order of the elements in the request_items field.
        slf.request_items.reverse();
    }
    pub fn way_step_by_step(slf: &mut QFilePath) {
        core(slf)
    }
    pub async fn async_way_step_by_step(slf: &mut QFilePath) {
        core(slf)
    }
}
/*
The work_with_elements module is used in this code to provide functions for working with files and directories in the filesystem. It includes functions for getting the contents of a directory, opening files, creating directories. These functions are used by other modules in the codebase to manipulate files and directories. The module provides synchronous and asynchronous versions of these functions, allowing for efficient file operations in both blocking and non-blocking contexts.
 */
pub mod work_with_elements {
    use super::*;
    use crate::paths::get_path::{async_get_path_buf, async_get_path_string, get_path_buf, get_path_string};
    // Returns a vector of string paths to files in the directory
    pub fn directory_contents(path: &str) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        if let Ok(mut paths) = std::fs::read_dir(path) {
            loop {
                if let Some(item) = paths.next() {
                    if let Ok(items) = item {
                        files.push(items.path().display().to_string());
                    };
                } else {
                    break;
                }
            }
        }
        return files;
    }
    // Asynchronously returns a vector of string paths to files in the directory
    pub async fn async_directory_contents(path: &str) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        if let Ok(mut paths) = async_fs::read_dir(path).await {
            loop {
                if let Some(item) = paths.next().await {
                    if let Ok(items) = item {
                        files.push(items.path().display().to_string());
                    };
                } else {
                    break;
                }
            }
        }
        return files;
    }
    // Returns a file object based on the file path
    pub fn return_file(path: &str) -> Result<fs::File, QPackError> {
        match fs::File::open(path) {
            Ok(file) => Ok(file),
            Err(err) => Err(QPackError::IoError(err)),
        }
    }
    // Asynchronously returns a file object based on the file path
    pub async fn async_return_file(path: &str) -> Result<async_fs::File, QPackError> {
        match async_fs::File::open(path).await {
            Ok(file) => Ok(file),
            Err(err) => Err(QPackError::IoError(err)),
        }
    }
    // Returns a file object based on the QFilePath struct
    pub fn file(slf: &mut QFilePath) -> Result<fs::File, QPackError> {
        // let path = ;
        match return_file(&get_path_string(slf)?) {
            Ok(file) => Ok(file),
            Err(err) => Err(err),
        }
    }
    // Asynchronously returns a file object based on the QFilePath struct
    pub async fn async_file(slf: &mut QFilePath) -> Result<async_fs::File, QPackError> {
        match async_return_file(&async_get_path_string(slf).await?).await {
            Ok(file) => Ok(file),
            Err(err) => Err(err),
        }
    }
    // Creates a folder and all parent folders in the path
    pub fn folder_create(slf: &mut QFilePath) -> Result<(), QPackError> {
        Ok(fs::DirBuilder::new().recursive(true).create(get_path_buf(slf)?)?)
    }
    // Asynchronously creates a folder and all parent folders in the path
    pub async fn async_folder_create(slf: &mut QFilePath) -> Result<(), QPackError> {
        Ok(async_fs::DirBuilder::new().recursive(true).create(async_get_path_buf(slf).await?).await?)
    }
}
/*
The correct_path module in this code is responsible for correcting the user-provided file path. It uses the functions defined in path_separation and work_with_elements modules to identify and correct any invalid elements in the path.

The core function is the main function that performs the correction of the path. It takes a QFilePath struct reference, a vector of possible directories, the index of the current user item, a counter to keep track of the number of corrected elements, and the total number of items in the path. It iterates over each item in the path and tries to correct it by comparing it with the possible directories. If a match is found, the item is replaced, and the counter is incremented.

The correct_path function uses the core function to correct the path synchronously, while the async_correct_path function does the same but asynchronously.

Overall, the correct_path module ensures that the user-provided file path is correct and can be used to locate the required file.
 */
pub mod correct_path {
    use super::path_separation::*;
    use super::work_with_elements::*;
    use super::*;
    fn core(
        slf: &mut QFilePath,        // Mutable reference to a QFilePath struct
        directory_cnt: Vec<String>, // Vector of directories
        user_i: usize,              // Index of the current user
        counter: &mut usize,        // Mutable reference to a counter variable
        len: usize,                 // Length of request_items
    ) {
        // let mut counter = 0;
        // for user_i in 0..slf.request_items.len() {
        let mut possible_directories = directory_cnt; // Copying directory_cnt vector
        // Looping through possible_directories vector

        for pos_j in 0..possible_directories.len() {
            if slf
                .request_items
                .get(user_i + 1)
                .unwrap_or(&slf.request_items.get(user_i).unwrap().to_lowercase())
                .to_lowercase()
                == possible_directories[pos_j].to_lowercase()
            {
                slf.request_items[user_i + 1] = possible_directories.remove(pos_j);
                *counter += 1;
                break;
            }
        }
        // }
        if user_i < len - 1 {
            if Path::new(slf.request_items.last().unwrap()).exists() {
                // Set correct_path to the path
                slf.correct_path = PathBuf::from(slf.request_items.last().unwrap());
            } else if cfg!(unix) {
                if Path::new(&slf.request_items[*counter]).exists() && *counter != (0 as usize) {
                    slf.correct_path = PathBuf::from(format!(
                        "{}{}",
                        slf.request_items[*counter],
                        slf.request_items.last().unwrap().split_at(slf.request_items[*counter].len()).1
                    ));
                }
            }
        }
    }
    pub fn correct_path(slf: &mut QFilePath) {
        let mut counter = 0;
        if slf.request_items.is_empty() {
            way_step_by_step(slf);
        };
        let len = slf.request_items.len(); // Storing the length of request_items
        // Looping through request_items
        for user_i in 0..len {
            core(slf, directory_contents(slf.request_items[user_i].as_str()), user_i, &mut counter, len);
        }
    }
    pub async fn async_correct_path(slf: &mut QFilePath) {
        let mut counter = 0;
        if slf.request_items.is_empty() {
            async_way_step_by_step(slf).await;
        };
        let len = slf.request_items.len();
        for user_i in 0..len {
            core(
                slf,
                // Calling directory_contents function on the user's request item
                async_directory_contents(slf.request_items[user_i].as_str()).await,
                user_i,
                &mut counter,
                len,
            );
        }
    }
}
/*
The path_for_write module in this code is used to create a new file or directory at a specified path.

The module has a core function which takes a QFilePath struct and a PathBuf representing the full path, and returns a string containing the path without the filename. This function is used to update the QFilePath struct with the new file or directory's path and name, and set a flag indicating that the file or directory is new.

The path_create function is a public function that takes a QFilePath struct and an std::io::ErrorKind indicating the reason the file or directory could not be created. If the error is NotFound, meaning the path does not exist, it calls the get_path_buf function from the get_path module to get the full path and then creates the file or directory using the core function. If the error is anything else, an QPackError is returned with the IoError variant.

Similarly, the async_path_create function is an asynchronous version of path_create that calls the async_get_path_buf function from the get_path module and uses async_fs::DirBuilder to create the file or directory asynchronously. It also returns an QPackError if any error occurs while creating the file or directory.
 */
pub mod path_for_write {
    use super::*;
    use crate::paths::get_path::*;
    // This function creates a path for writing a file by splitting the full path into filename and the path without filename
    // It then updates the user_path, file_name, flag and update_path of the QFilePath struct and returns the path without the filename as a string
    fn core(slf: &mut QFilePath, fullpath: PathBuf) -> String {
        let filename = fullpath.file_name().unwrap().to_str().unwrap();
        let path_without_file = fullpath.to_str().unwrap().rsplit_once(filename).unwrap().0;
        {
            slf.user_path = PathBuf::from(path_without_file);
            slf.update_path = true;
            slf.file_name = PathBuf::from(filename);
            slf.flag = Flag::New;
        };
        path_without_file.to_string()
    }
    // This function creates a path for writing a file
    // If the error is NotFound, it gets the full path using get_path_buf() and creates the directories for the path using core()
    // Otherwise it returns an IoError wrapped in a QPackError
    pub fn path_create(slf: &mut QFilePath, err: std::io::ErrorKind) -> Result<(), QPackError> {
        match err {
            std::io::ErrorKind::NotFound => {
                let fullpath = get_path_buf(slf)?;
                std::fs::DirBuilder::new().recursive(true).create(core(slf, fullpath))?;
                Ok(())
            }
            _ => Err(QPackError::IoError(err.into())),
        }
    }
    pub async fn async_path_create(slf: &mut QFilePath, err: std::io::ErrorKind) -> Result<(), QPackError> {
        match err {
            std::io::ErrorKind::NotFound => {
                let fullpath = async_get_path_buf(slf).await?;
                async_fs::DirBuilder::new().recursive(true).create(core(slf, fullpath)).await?;
                Ok(())
            }
            _ => Err(QPackError::IoError(err.into())),
        }
    }
}
#[test]
fn check_correct_path() {
    let mut qfile = self::constructor::add_path("SRC").unwrap();
    self::correct_path::correct_path(&mut qfile);
    dbg!(qfile);
}

use crate::find::pathfinder::find_paths;
use crate::init::{
    constructor::add_path,
    work_with_elements::{file, folder_create},
};
use crate::paths::get_path::{get_path_buf, get_path_string};
use crate::read::read::*;
use crate::write::write::{auto_write, write_only_new};
use crate::CodeStatus;
use crate::Directory;
use crate::{QFilePath, QPackError};
use std::sync::mpsc::{SendError, Sender};
use std::{fs, path::PathBuf};

/// The prelude_sync module is a collection of frequently used items that are imported automatically when the QPack library is used. This module saves the user from having to import each item manually.

pub trait QTraitSync {
    /// The add_path constructor from the qfile library allows you to create an object of type QFilePack that represents a file in a given path. (**Not case-sensitive**)
    /// To create the object you must pass the path to the file in string format.
    /// The path can be absolute or relative, and can also contain ... symbols to jump to a higher level in the folder hierarchy.
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// let mut file = QFilePath::add_path("my_folder/my_file.txt")?;
    /// ```
    fn add_path<T: AsRef<str>>(path_file: T) -> Result<QFilePath, QPackError>;
    /// This method returns a file specified by the QFilePath object
    /// if you want to use the full `std::fs` feature set,
    /// using this method, you can get the file
    /// if it exists, case insensitive (use auto_write to create the file)
    ///  # Example
    /// ```
    /// use std::fs::File;
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// let mut file = QFilePath::add_path("file.txt")?;
    /// let file = file.file()?;
    /// let mut perms = file.metadata()?.permissions();
    /// perms.set_readonly(true);
    /// file.set_permissions(perms)?;
    /// ```
    fn file(&mut self) -> Result<fs::File, QPackError>;
    /// This method creates a folder and all parent folders in the path, case insensitive
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///  
    /// let mut file = QFilePath::add_path("NewFolder/Folder/Folder")?;
    /// file.folder_create()?; // ↓ ↓ ↓
    ///
    /// ```
    ///
    /// ---
    /// # Result
    ///
    /// | Path                  | Unix format                    | Windows  format                     |
    /// | --------------------- | ------------------------------ | ------------------------------ |
    /// | The path we specified | `.polYGon/myfolder/new_Folder` | `.polYGon\myfolder\new_Folder` |
    /// | Real path             | `.Polygon/MyFOLDER`            | `.Polygon\MyFOLDER`            |
    /// | Result                | `.Polygon/MyFOLDER/new_Folder` | `.Polygon\MyFOLDER\new_Folder` |

    fn folder_create(&mut self) -> Result<(), QPackError>;
    //================================================================
    /// A method to get the correct path (`PathBuf`), if the file exists, is not case-sensitive.
    /// After the first use, the correct path is saved in QFilePath for reuse as a cache.
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// //Real path : `.Polygon/MyFOLDER/FIle.txt`
    /// let mut file = QFilePath::add_path(".polYGon/myfolder/FILE.txt")?;
    /// println!("{:#?}", file.get_path_buf()?); // ↓ ↓ ↓
    /// ```
    ///
    /// ---
    /// # Output
    ///
    /// |Path|Unix format|Windows format|
    /// |---|---|---|
    /// |The path we specified|`.polYGon/myfolder/FILE.txt`|`.polYGon\myfolder\FILE.txxt`|
    /// |Real path|`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    /// |Result |`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    ///
    fn get_path_buf(&mut self) -> Result<PathBuf, QPackError>;
    /// A method to get the correct path (`String`), if the file exists, is not case-sensitive.
    /// After the first use, the correct path is saved in QFilePath for reuse as a cache.
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// //Real path : `.Polygon/MyFOLDER/FIle.txt`
    /// let mut file = QFilePath::add_path(".polYGon/myfolder/FILE.txt")?;
    /// println!("{}", file.get_path_string()?); // ↓ ↓ ↓
    /// ```
    ///
    /// ---
    /// # Output
    ///
    /// |Path|Unix format|Windows format|
    /// |---|---|---|
    /// |The path we specified|`.polYGon/myfolder/FILE.txt`|`.polYGon\myfolder\FILE.txxt`|
    /// |Real path|`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    /// |Result |`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    ///
    fn get_path_string(&mut self) -> Result<String, QPackError>;
    //================================================================
    /// Method for reading the contents of a file (`String`), case insensitive
    ///
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///     
    /// // real path : myFolder/file.txt
    /// let mut file = QFilePath::add_path("MyFolder/file.TXT")?;
    /// let text = file.read()?;
    /// println!("content: {}", text);
    /// ```
    fn read(&mut self) -> Result<String, QPackError>;
    /// The method for writing to a file depends on the current context, case insensitive
    /// * If the file exists - adds new content to the file
    /// * If file does not exist - creates files and, if necessary, all parent folders specified in the path. After that writes the new content
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// // real path : myFolder/file.txt
    /// let mut file = QFilePath::add_path("MyFolder/file.TXT")?;
    /// file.auto_write("text1 text1 text1")?;
    /// file.auto_write("text2 text2 text2")?;
    /// ```
    fn auto_write<T: AsRef<str>>(&mut self, text: T) -> Result<(), QPackError>;
    /// The method for writing to a file depends on the current context, case insensitive
    /// * If the file exists - overwrites all the content with the new content
    /// * If file does not exist - creates files and, if necessary, all parent folders specified in the path. After that writes the new content
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitSync};
    ///
    /// // real path : myFolder/file.txt
    /// let mut file = QFilePath::add_path("MyFolder/file.TXT")?;
    /// file.write_only_new("text1 text1 text1")?;
    /// file.write_only_new("text2 text2 text2")?;
    /// ```
    fn write_only_new<T: AsRef<str>>(&mut self, text: T) -> Result<(), QPackError>;
}
impl QTraitSync for QFilePath {
    //================================================================
    fn add_path<T: AsRef<str>>(path_file: T) -> Result<QFilePath, QPackError> {
        add_path(path_file)
    }
    fn file(&mut self) -> Result<fs::File, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        file(self)
    }
    fn folder_create(&mut self) -> Result<(), QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        folder_create(self)
    }
    //================================================================
    fn get_path_buf(&mut self) -> Result<PathBuf, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        get_path_buf(self)
    }
    fn get_path_string(&mut self) -> Result<String, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        get_path_string(self)
    }
    //================================================================
    fn read(&mut self) -> Result<String, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        read(self)
    }
    fn auto_write<T: AsRef<str>>(&mut self, text: T) -> Result<(), QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        auto_write(self, text)
    }
    fn write_only_new<T: AsRef<str>>(&mut self, text: T) -> Result<(), QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::SyncStatus)?;
        write_only_new(self, text)
    }
}
impl QFilePath {
    // Perhaps there will be an asynchronous version
    /// This is the method that kicks off the directory search.
    /// It takes in the search location, names of files to search for, excluded directories, whether or not to follow symbolic links, and a channel to send the results back on.
    ///
    /// **It uses the rayon crate to parallelize the search over multiple threads for better performance.**
    ///
    /// The algorithm first filters out the excluded directories, and then iterates through the remaining directories to find all the files that match the specified search criteria. If a match is found, the path of the file is sent to the Sender object.
    ///
    /// # Example
    ///```
    /// use qfile::{Directory, QFilePath};
    /// use std::sync::mpsc;
    ///
    /// QFilePath::find_paths(
    ///     // specifies the directories to search from, where the search should start.
    ///     Directory::ThisPlace(vec!["src", "another_folder", "/home/user/my_project"]),
    ///     // names of items to search in the file system
    ///     vec!["main.rs", "lib.rs", "photo-1-2.jpg"],
    ///     // folders to exclude search
    ///     Some(vec!["src/tests", "another_folder/tmp"]),
    ///     // follow links
    ///     false,
    ///     // Sender channel
    ///     tx,
    /// )?;
    /// for path in rx {
    ///     println!("{}", path.display());
    /// }
    ///```
    pub fn find_paths<T: AsRef<str> + Send + Sync>(
        place: Directory<T>,
        names: Vec<T>,
        excluded_dirs: Option<Vec<T>>,
        follow_link: bool,
        sender: Sender<PathBuf>,
    ) -> Result<(), SendError<PathBuf>> {
        find_paths(place, names, excluded_dirs, follow_link, sender)
    }
}

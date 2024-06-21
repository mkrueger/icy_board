// use crate::find::pathfinder::find_paths;
use crate::init::{
    constructor::async_add_path,
    work_with_elements::{async_file, async_folder_create},
};
use crate::paths::get_path::{async_get_path_buf, async_get_path_string};
use crate::read::read::async_read;
use crate::write::write::{async_auto_write, async_write_only_new};
use crate::CodeStatus;
use crate::{QFilePath, QPackError};
use async_fs;
use async_mutex::Mutex;
use async_trait::async_trait;
use std::error::Error;
use std::path::PathBuf;

/// The prelude_async module is a collection of frequently used items that are imported automatically when the QPack library is used. This module saves the user from having to import each item manually.
#[async_trait]
pub trait QTraitAsync {
    /// The add_path constructor from the qfile library allows to create an object of type Mutex<QFilePack>, asynchronous mutex is used.
    /// To create an object, you must pass the path to the file in string format.
    /// The path can be absolute or relative, and can also contain characters ... to jump to a higher level in the folder hierarchy. (**Not case-sensitive**)
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// let file = QFilePath::async_add_path("my_folder/my_file.txt").await?;
    /// ```
    async fn async_add_path<T: AsRef<str> + Send + Sync>(path_file: T) -> Result<Mutex<QFilePath>, QPackError>;
    /// This method returns a file specified by the QFilePath object
    /// if you want to use the full `async_fs` feature set,
    /// using this method, you can get the file
    /// if it exists, case insensitive (use auto_write to create the file)
    ///  # Example
    /// ```
    /// use async_fs::File;
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// let file = QFilePath::async_add_path("file.txt").await?;
    /// let file = file.lock().await.async_file().await?;
    /// let mut perms = file.metadata().await?.permissions();
    /// perms.set_readonly(true);
    /// file.set_permissions(perms).await?;
    /// ```
    async fn async_file(&mut self) -> Result<async_fs::File, QPackError>;
    /// This method creates a folder and all parent folders in the path, case insensitive
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// let file = QFilePath::async_add_path(".polYGon/myfolder/new_Folder").await?;
    /// file.lock().await.async_folder_create().await?; // ↓ ↓ ↓
    ///
    /// ```
    ///
    /// ---
    /// # Result
    ///
    /// | Path                  | Unix format                    | Windows format                   |
    /// | --------------------- | ------------------------------ | ------------------------------ |
    /// | The path we specified | `.polYGon/myfolder/new_Folder` | `.polYGon\myfolder\new_Folder` |
    /// | Real path             | `.Polygon/MyFOLDER`            | `.Polygon\MyFOLDER`            |
    /// | Result                | `.Polygon/MyFOLDER/new_Folder` | `.Polygon\MyFOLDER\new_Folder` |
    async fn async_folder_create(&mut self) -> Result<(), QPackError>;
    //================================================================
    /// A method to get the correct path (`PathBuf`), if the file exists, is not case-sensitive.
    /// After the first use, the correct path is saved in QFilePath for reuse as a cache.
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// //Real path : `.Polygon/MyFOLDER/FIle.txt`
    /// let file = QFilePath::async_add_path(".polYGon/myfolder/FILE.txt")?;
    /// println!("{:#?}", file.lock().await.get_path_buf().await?); // ↓ ↓ ↓
    /// ```
    ///
    /// ---
    /// # Output
    ///
    /// |Path|Unix format|Windows  format|
    /// |---|---|---|
    /// |The path we specified|`.polYGon/myfolder/FILE.txt`|`.polYGon\myfolder\FILE.txxt`|
    /// |Real path|`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    /// |Result |`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    ///
    async fn async_get_path_buf(&mut self) -> Result<PathBuf, QPackError>;
    /// A method to get the correct path (`String`), if the file exists, is not case-sensitive.
    /// After the first use, the correct path is saved in QFilePath for reuse as a cache.
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    ///  //Real path : `.Polygon/MyFOLDER/FIle.txt`
    /// let file = QFilePath::async_add_path(".polYGon/myfolder/FILE.txt")?;
    /// println!("{}", file.lock().await.get_path_string().await?); // ↓ ↓ ↓
    /// ```
    ///
    /// ---
    /// # Output
    ///
    /// |Path|Unix format|Windows  format|
    /// |---|---|---|
    /// |The path we specified|`.polYGon/myfolder/FILE.txt`|`.polYGon\myfolder\FILE.txxt`|
    /// |Real path|`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    /// |Result |`.Polygon/MyFOLDER/FIle.txt`|`.Polygon\MyFOLDER\FIle.txt`|
    ///
    async fn async_get_path_string(&mut self) -> Result<String, QPackError>;
    //================================================================
    /// Method for reading the contents of a file (`String`), case insensitive
    ///
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///     
    /// // real path : myFolder/file.txt
    /// let file = QFilePath::async_add_path("MyFolder/file.TXT").await?;
    /// let text = file.lock().await.async_read().await?;
    /// println!("content: {}", text);
    /// ```
    async fn async_read(&mut self) -> Result<String, QPackError>;
    /// The method for writing to a file depends on the current context, case insensitive
    /// * If the file exists - adds new content to the file
    /// * If file does not exist - creates files and, if necessary, all parent folders specified in the path. After that writes the new content
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// // real path : myFolder/file.txt
    /// let file = QFilePath::async_add_path("MyFolder/file.TXT").await?;
    /// file.lock().await.auto_write("text1 text1 text1").await?;
    /// file.lock().await.auto_write("text2 text2 text2").await?;
    /// ```
    async fn async_auto_write<T: AsRef<str> + Send + Sync>(&mut self, text: T) -> Result<(), Box<dyn Error + Send + Sync>>;
    /// The method for writing to a file depends on the current context, case insensitive
    /// * If the file exists - overwrites all the content with the new content
    /// * If file does not exist - creates files and, if necessary, all parent folders specified in the path. After that writes the new content
    /// # Example
    /// ```
    /// use qfile::{QFilePath, QTraitAsync};
    ///
    /// // real path : myFolder/file.txt
    /// let file = QFilePath::async_add_path("MyFolder/file.TXT").await?;
    /// file.lock().await.write_only_new("text1 text1 text1")?;
    /// file.lock().await.write_only_new("text2 text2 text2")?;
    /// ```
    async fn async_write_only_new<T: AsRef<str> + Send + Sync>(&mut self, text: T) -> Result<(), Box<dyn Error + Send + Sync>>;
}
#[async_trait]
impl QTraitAsync for QFilePath {
    //================================================================
    async fn async_add_path<T: AsRef<str> + Send + Sync>(path_file: T) -> Result<Mutex<QFilePath>, QPackError> {
        async_add_path(path_file).await
    }
    async fn async_file(&mut self) -> Result<async_fs::File, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_file(self).await
    }
    async fn async_folder_create(&mut self) -> Result<(), QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_folder_create(self).await
    }
    //================================================================
    async fn async_get_path_buf(&mut self) -> Result<PathBuf, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_get_path_buf(self).await
    }
    async fn async_get_path_string(&mut self) -> Result<String, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_get_path_string(self).await
    }
    //================================================================
    async fn async_read(&mut self) -> Result<String, QPackError> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_read(self).await
    }
    async fn async_auto_write<T: AsRef<str> + Send + Sync>(&mut self, text: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_auto_write(self, text).await
    }
    async fn async_write_only_new<T: AsRef<str> + Send + Sync>(&mut self, text: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        QFilePath::check_status_code(&self, CodeStatus::AsyncStatus)?;
        async_write_only_new(self, text).await
    }
}

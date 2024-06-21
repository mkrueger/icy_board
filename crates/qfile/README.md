
[![Crate](https://img.shields.io/crates/v/qfile?color=green)](https://crates.io/crates/qfile)
[![Docs](https://img.shields.io/docsrs/qfile)](https://docs.rs/qfile/latest/qfile/)
[![Changelog](https://img.shields.io/badge/changelog-qfile-blue)](https://github.com/m62624/qfile/blob/main/CHANGELOG.md)

# Qfile

The Qfile crate provides functionality for accessing a file by path, case-insensitive, including automatic detection, creation of a path with a new file or opening an existing file. It includes several submodules to handle different aspects of file handling, such as initialization, reading, and writing. The crate also defines some custom errors related to file operations.

> qfile is not the rust version of qfile from the QT framework

# Paths syntax

## Unix format
  
```rust
let path1 = "File.txt";
let path2 = "./File.txt";
let path3 = "../../File.txt";
let path4 = String::from("Folder/Folder/File.txt");
```

## Windows format 
  
```rust
let path1 = "File.txt";
let path2 = ".\\File.txt";
let path3 = "..\\..\\File.txt";
let path4 = "D:\\Folder\\file.txt";
let path5 = r"D:\Folder\file.txt";
let path6 = String::from("D:\\Folder\\file.txt");
```

---
# Paths finder
This is the method that kicks off the directory search.
It takes in the search location, names of files to search for, excluded directories, whether or not to follow symbolic links, and a channel to send the results back on.

**It uses the [rayon](https://crates.io/crates/rayon) crate to parallelize the search over multiple threads for better performance.**

The algorithm first filters out the excluded directories, and then iterates through the remaining directories to find all the files that match the specified search criteria. If a match is found, the path of the file is sent to the Sender object.

```rust
use qfile::{QFilePath,Directory};
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();
QFilePath::find_paths(
    // specifies the directories to search from, where the search should start.
    Directory::ThisPlace(vec!["src", "another_folder", "/home/user/my_project"]),
    // names of items to search in the file system
    vec!["main.rs", "lib.rs", "photo-1-2.jpg"],
    // folders to exclude search
    Some(vec!["src/tests", "another_folder/tmp"]),
    // follow links
    false,
    // Sender channel
    tx,
)?;
for path in rx {
    println!("{}", path.display().to_string());
}
```

# Writing to a file
The method for writing to a file depends on the current context, case insensitive
* If the file exists - adds new content to the file
* If file does not exist - creates files and, if necessary, all parent folders specified in the path. After that writes the new content

### Example (Sync Code)
```rust
 use qfile::{QFilePath, QTraitSync};

 // real path : myFolder/file.txt
 let mut file = QFilePath::add_path("MyFolder/file.TXT")?;
 file.auto_write("text1 text1 text1")?;
 file.auto_write("text2 text2 text2")?;
```

### Example (Async code)
```rust 
use qfile::{QFilePath, QTraitAsync};
// real path : myFolder/file.txt
let mut file = QFilePath::async_add_path("MyFolder/file.TXT").await?;
file.lock().await.auto_write("text1 text1 text1").await?;
file.lock().await.auto_write("text2 text2 text2").await?;
```
 - If the path exists, we work with the file

 |                            | Unix format                       | Windows format                      |
 | -------------------------- | ---------------------------- | ---------------------------- |
 | **The path we specified**: | `folder1/FolDER2/file.TXT`   | `folder1\FolDER2\file.TXT`   |
 | **Real path** :            | `./Folder1/Folder2/file.txt` | `.\Folder1\Folder2\file.txt` |
 | **Result** :               | `./Folder1/Folder2/file.txt` | `.\Folder1\Folder2\file.txt` |

 - If the file/path is not found, creates a new path with the file

 |                            | Unix format                               | Windows format                            |
 | -------------------------- | ----------------------------------- | ----------------------------------- |
 | **The path we specified**: | `./main_folder/folder_new/file.txt` | `.\main_folder\folder_new\file.txt` |
 | **Real path** :            | `./Main_Folder`                     | `.\Main_Folder`                     |
 | **Result** :               | `./Main_Folder/folder_new/file.txt` | `.\Main_Folder\folder_new\file.txt` |
 
 > * The Windows file system treats file and directory names as **case insensitive**. `file.txt` and `FILE.txt` will be treated as equivalent files (Although the path is **case insensitive** in windows (`..\FOLDER\file.txt`), you can return a **case-sensitive** path with : `get_path_string()` or `get_path_buf()`).

---
# Reading a file

 Method for reading the contents of a file (`String`), case insensitive

### Example (Sync code)
```rust
use qfile::{QFilePath, QTraitSync};
    
// real path : myFolder/file.txt
let mut file = QFilePath::add_path("MyFolder/file.TXT")?;
let text = file.read()?;
println!("content: {}", text);
```
### Example (Aync code)
```rust
use qfile::{QFilePath, QTraitAsync};

// real path : myFolder/file.txt
let mut file = QFilePath::async_add_path("MyFolder/file.TXT").await?;
let text = file.lock().await.async_read().await?;
println!("content: {}", text);
```

---

 # Changelog
 [List](https://github.com/m62624/qfile/blob/main/CHANGELOG.md)
 # License
 [MIT](https://choosealicense.com/licenses/mit/)

# Changelog
## [3.0.1] - 2023.02.20
### Changed
* changed the lifetime from `static` to `'a` in `find_paths`
* `convert_sync_send` replaced by `fn from` from trait From
## [3.0.0] - 2023.02.19
### Changed
* Changed the entire internal structure of the project.
* Changed documentation (removed unnecessary examples), added more information about how the code works in the source code
### Added
* Added asynchronous operations 
* Added a file search release using the crate [rayon](https://crates.io/crates/rayon)
* Synchronous operations and asynchronous operations are separated by traits for easy importing 
    - `QTraitSync` 
    - `QTraitAsync`
## [2.2.3] - 2023.02.02
### Fixed
Return `io::Error` instead of `panic` to catch the error (changes on [crates.io](https://crates.io/crates/qfile)): 
- `auto_write`
- `write_only_new`
## [2.2.2] - 2023.01.21
### Changed
- The operating system definition was changed
- Changed conditions for `linux` & `macos` to `unix`
### Fixed
- The name `slash` was restored
- Fixed the visibility area for methods
## [2.2.1] - 2023.01.17
### Added
`PathIsEmpty` - new error type, if the path is empty specified in the constructor
### Fixed
Removed the `last` variable warning
## [2.2.0] - 2023.01.17
### Changed
Now on linux and windows the same behavior when creating folders/files
#### New behavior
Linux :
 |                            |                                |
 | -------------------------- | ------------------------------ |
 | **The path we specified**: | `./FOLDER/Folder_new/file.txt` |
 | **Real path** :            | `./folder`                     |
 | **Result** :               | `./folder/Folder_new/file.txt` |

Windows :
 |                            |                                |
 | -------------------------- | ------------------------------ |
 | **The path we specified**: | `.\FOLDER\Folder_new\file.txt` |
 | **Real path** :            | `.\folder`                     |
 | **Result** :               | `.\folder\Folder_new\file.txt` |

#### Old behavior
Linux :
 |                            |                                                         |
 | -------------------------- | ------------------------------------------------------- |
 | **The path we specified**: | `./FOLDER/Folder_new/file.txt`                          |
 | **Real path** :            | `./folder`                                              |
 | **Result** :               | `./FOLDER/Folder_new/file.txt` - (**new created path**) |
 |                            | `./folder` - (**original path**)                        |

Windows :
 |                            |                                                  |
 | -------------------------- | ------------------------------------------------ |
 | **The path we specified**: | `.\FOLDER\Folder_new\file.txt`                   |
 | **Real path** :            | `.\folder`                                       |
 | **Result** :               | `.\folder\Folder_new\file.txt` - (**real path**) |

## [2.1.0] - 2023.01.11
### Changed
- `add_path()` returns `Result<Self, OsPathError>`
### Added 
- Custom errors:
  - `UnixPathIncorrect`
  - `WindowsPathIncorrect`

If you catch these errors, you can get a message:

---
#### Windows
```rust
use qfile::*;
fn main() {
    let path = QFilePath::add_path("./folder/file.txt");
    if let Err(err) = path {
        println!("{err}");
    }
}
```
Output:
> You are using the unix path format for Windows. Use `windows` format for the path:\
> \> .\folder1\folder2\file.txt\
> \> ..\folder2\file.txt\
> \> .\file.txt

---
#### Linux
```rust
use qfile::*;
fn main() {
    let path = QFilePath::add_path(".\\folder\\file.txt");
    if let Err(err) = path {
        println!("{err}");
    }
}
```
Output:
> You are using the windows path format for Unix. Use `unix` format for the path\
> \> ./folder1/folder2/file.txt \
> \> ../folder2/file.txt\
> \> ./file.txt

---

## [2.0.0] - 2023.01.11
### Added
- `get_path_str` - returns [`PathBuf`](https://doc.rust-lang.org/stable/std/path/struct.PathBuf.html) in `&str` format
- New examples and descriptions of how naming files work
### Changed
- `cache_path` renamed to `get_path_buf`. Now `get_path_buf` returns [`PathBuf`](https://doc.rust-lang.org/stable/std/path/struct.PathBuf.html).
- API - Changed the name of the methods, for a better understanding of their work
- updated documentation

## [1.1.4] - 2023.01.08
### Fixed
Files and folders were created in different paths - fixed 
## [1.1.3] - 2023.01.08
### Changed 
Now it is not necessary to specify `./` at the beginning of the path (but you can still write it)

Example:

| Before                      | After                    |
| --------------------------- | ------------------------ |
| ./Folder/folder/file.txt    | Folder/folder/file.txt   |
| ./file.txt                  | file.txt                 |
| .\\Folder\\Folder\\file.txt | Folder\\Folder\\file.txt |
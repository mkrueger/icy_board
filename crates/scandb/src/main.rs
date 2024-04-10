use std::{path::PathBuf, time::SystemTime};

use clap::{Parser, Subcommand};
use dizbase::{
    file_base::{metadata::MetadaType, FileBase},
    file_base_scanner::scan_file_directory,
};
use thiserror::Error;

// pub mod filebase_viewer;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Error, Debug)]
pub enum MyError {
    #[error("IO error")]
    Utf8Error,
}

#[derive(clap::Parser)]
#[command(version="", about="IcyBoard file base maintainance utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    ScanFileDirectory {
        /// input directory to scan
        input: PathBuf,
        /// file database
        file: PathBuf,
    },
    List {
        /// file database
        file: PathBuf,
    },
    Search {
        /// file database
        file: PathBuf,
        pattern: String,
    },
}

pub fn main() {
    let arguments = Cli::parse();
    match &arguments.command {
        Some(Commands::ScanFileDirectory { input, file }) => {
            scan_file_directory(input, file);
        }
        Some(Commands::List { file }) => {
            list_file_base(file);
        }
        Some(Commands::Search { file, pattern }) => {
            search_file_base(file, pattern);
        }
        None => {}
    }
}

fn search_file_base(file: &PathBuf, pattern: &str) {
    let mut base = FileBase::open(file).unwrap();
    let time = SystemTime::now();
    base.load_headers().unwrap();
    println!("Loading headers took {:?}", time.elapsed().unwrap());
    let time = SystemTime::now();

    let headers = base.find_files(pattern).unwrap();
    println!(
        "Searching multi thread took {:?} found {}",
        time.elapsed().unwrap(),
        headers.len()
    );

    for header in headers {
        println!(
            "{:20} {:-15}kb {}",
            header.name(),
            header.size() / 1024,
            header.file_date().unwrap().format("%d/%m/%Y")
        );
        match base.read_metadata(header) {
            Ok(metadata) => {
                println!("Metadata {}", metadata.len());
                for m in &metadata {
                    match m.get_type() {
                        MetadaType::Unknown(t) => {
                            println!("  Unknown type {}", t);
                            continue;
                        }
                        MetadaType::UploaderName => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                        MetadaType::Password => {
                            println!("  {:10?} {:?}", m.get_type(), m.data);
                            continue;
                        }
                        MetadaType::Tags => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                        MetadaType::FileID => {
                            println!(
                                "  {:10?}\n{}\n--------------------\n",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                        MetadaType::Sauce => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                    }
                }
            }
            Err(err) => eprintln!("{}", err),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    ASCII,
    CP437,
    Unicode,
}

enum State {
    Default,
    TwoByteContinuation,
    ThreeByteLow,
}

const UTF8_ACCEPT: usize = 0;
const UTF8_REJECT: usize = 1;

const utf8d: [usize; 400] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 00..1f
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 20..3f
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 40..5f
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 60..7f
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, // 80..9f
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, // a0..bf
    8, 8, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // c0..df
    0xa, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x4, 0x3, 0x3, // e0..ef
    0xb, 0x6, 0x6, 0x6, 0x5, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, // f0..ff
    0x0, 0x1, 0x2, 0x3, 0x5, 0x8, 0x7, 0x1, 0x1, 0x1, 0x4, 0x6, 0x1, 0x1, 0x1, 0x1, // s0..s0
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1,
    1, // s1..s2
    1, 2, 1, 1, 1, 1, 1, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1,
    1, // s3..s4
    1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 1, 3, 1, 1, 1, 1, 1,
    1, // s5..s6
    1, 3, 1, 1, 1, 1, 1, 3, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // s7..s8
];

fn decode(state: &mut usize, codep: &mut u8, byte: u8) -> usize {
    let t = utf8d[byte as usize];

    *codep = if (*state != UTF8_ACCEPT) {
        (byte & 0x3f) | (*codep << 6)
    } else {
        ((0xff as u8).wrapping_shr(t as u32)) & (byte)
    };

    *state = utf8d[256 + *state * 16 + t];
    return *state;
}

fn guess_file_type(data: &[u8]) -> FileType {
    let mut file_type = FileType::ASCII;
    let mut data = &data[..];
    let mut state = 0;
    let mut codep = 0;
    for b in data {
        if b > &0x7F {
            file_type = FileType::Unicode;
        }
        if decode(&mut state, &mut codep, *b) == UTF8_REJECT {
            return FileType::CP437;
        }
    }
    /*
    while !data.is_empty() {
        let byte1 = data[0];
        if byte1 <= 0x7F {
            // 00..7F
            data = &data[1..];
            continue;
        }
        file_type = FileType::Unicode;
        break;
    }

    let mut bytes;
    while !data.is_empty() {
        let byte1 = data[0];
        if byte1 <= 0x7F {
            // 00..7F
            bytes = 1;
        } else if data.len() >= 2 && byte1 >= 0xC2 && byte1 <= 0xDF && data[1] <= 0xBF {
            // C2..DF, 80..BF
            bytes = 2;
        } else if data.len() >= 3 {
            let byte2 = data[1];

            // Is byte2, byte3 between 0x80 ~ 0xBF
            let byte2_ok = (0x80..0xBF).contains(&byte2);
            let byte3_ok = (0x80..0xBF).contains(&data[2]);

            if byte2_ok && byte3_ok &&
                    ((byte1 == 0xE0 && byte2 >= 0xA0) ||
                     // E1..EC, 80..BF, 80..BF
                     (byte1 >= 0xE1 && byte1 <= 0xEC) ||
                     // ED, 80..9F, 80..BF
                     (byte1 == 0xED && byte2 <= 0x9F) ||
                     // EE..EF, 80..BF, 80..BF
                     (byte1 >= 0xEE && byte1 <= 0xEF))

                    {
                bytes = 3;
            } else if data.len() >= 4 {
                // Is byte4 between 0x80 ~ 0xBF
                let byte4_ok = (0x80..0xBF).contains(&data[3]);

                if byte2_ok && byte3_ok && byte4_ok &&
                         // F0, 90..BF, 80..BF, 80..BF
                        ((byte1 == 0xF0 && byte2 >= 0x90) ||
                         // F1..F3, 80..BF, 80..BF, 80..BF
                         (byte1 >= 0xF1 && byte1 <= 0xF3) ||
                         // F4, 80..8F, 80..BF, 80..BF
                         (byte1 == 0xF4 && byte2 <= 0x8F)) {
                    bytes = 4;
                } else {
                    return FileType::CP437;
                }
            } else {
                return FileType::CP437;
            }
        } else {
            return FileType::CP437;
        }
        data = &data[bytes..];
    }*/
    file_type
}

fn list_file_base(file: &PathBuf) {
    let mut base = FileBase::open(file).unwrap();
    let time = SystemTime::now();
    base.load_headers().unwrap();
    println!("Loading headers took {:?}", time.elapsed().unwrap());

    let mut ascii = 0;
    let mut utf8 = 0;
    let mut cp437 = 0;

    for header in base.get_headers() {
        match base.read_metadata(header) {
            Ok(metadata) => {
                for m in &metadata {
                    match m.get_type() {
                        MetadaType::Unknown(t) => {
                            println!("  Unknown type {}", t);
                            continue;
                        }
                        MetadaType::UploaderName => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                        MetadaType::Password => {
                            println!("  {:10?} {:?}", m.get_type(), m.data);
                            continue;
                        }
                        MetadaType::Tags => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                        MetadaType::FileID => {
                            let t = guess_file_type(&m.data);
                            match t {
                                FileType::ASCII => ascii += 1,
                                FileType::CP437 => cp437 += 1,
                                FileType::Unicode => utf8 += 1,
                            }

                            if t == FileType::Unicode {
                                println!(
                                    "{:20} {:-15}kb {}",
                                    header.name(),
                                    header.size() / 1024,
                                    header.file_date().unwrap().format("%d/%m/%Y")
                                );
                                if String::from_utf8(m.data.clone()).is_err() {
                                    for x in 0..m.data.len() {
                                        if !String::from_utf8(m.data[x..].to_vec()).is_err() {
                                            println!("------------{}", x);
                                            for b in 1206..m.data.len() {
                                                print!("{:02X} ", m.data[b])
                                            }
                                            println!("------------");

                                            println!();
                                            break;
                                        }
                                    }
                                }
                                println!(
                                    "  UNicode: {:10?}\n{}\n--------------------\n",
                                    m.get_type(),
                                    String::from_utf8(m.data.clone()).unwrap()
                                );
                            } /*
                              println!(
                                  "  {:10?}\n{}\n--------------------\n",
                                  m.get_type(),
                                  String::from_utf8_lossy(&m.data)
                              );*/
                            continue;
                        }
                        MetadaType::Sauce => {
                            println!(
                                "  {:10?} {}",
                                m.get_type(),
                                String::from_utf8_lossy(&m.data)
                            );
                            continue;
                        }
                    }
                }
            }
            Err(err) => eprintln!("{}", err),
        }
    }
    println!("ASCII: {} CP437: {} UTF8: {}", ascii, cp437, utf8);
}

#[cfg(test)]
mod test {
    use crate::guess_file_type;

    #[test]
    fn test_utf8() {
        let t = guess_file_type(b"Hello World");
        assert_eq!(t, super::FileType::ASCII);

        let t = guess_file_type(b"Hello World\x80");
        assert_eq!(t, super::FileType::CP437);

        let t = guess_file_type(b"Hello World\xE2\x82\xAC");
        assert_eq!(t, super::FileType::Unicode);
    }
}

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
            scan_file_directory(input, file).unwrap();
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
    println!("Searching multi thread took {:?} found {}", time.elapsed().unwrap(), headers.len());

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
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                        MetadaType::Password => {
                            println!("  {:10?} {:?}", m.get_type(), m.data);
                            continue;
                        }
                        MetadaType::Tags => {
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                        MetadaType::FileID => {
                            println!("  {:10?}\n{}\n--------------------\n", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                        MetadaType::Sauce => {
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                    }
                }
            }
            Err(err) => eprintln!("{}", err),
        }
    }
}

fn list_file_base(file: &PathBuf) {
    let mut base = FileBase::open(file).unwrap();
    let time = SystemTime::now();
    base.load_headers().unwrap();
    println!("Loading headers took {:?}", time.elapsed().unwrap());
    /*
    for (i, ch) in CP437_TO_UNICODE_NO_CTRL_CODES.iter().enumerate() {

        print!("vec![");
        let mut first = true;
        for b in ch.to_string().bytes() {
            if !first {
                print!(", ");
            } else {
                first = false;
            }
            print!("0x{:02x}", b);
        }
        println!("], // '{}' ({})", ch, i);
    }
    println!();*/

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
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                        MetadaType::Password => {
                            println!("  {:10?} {:?}", m.get_type(), m.data);
                            continue;
                        }
                        MetadaType::Tags => {
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                        MetadaType::FileID => {
                            let t = convert_to_utf8(&m.data);
                            println!("  {:10?} {}", m.get_type(), t);
                        }
                        MetadaType::Sauce => {
                            println!("  {:10?} {}", m.get_type(), String::from_utf8_lossy(&m.data));
                            continue;
                        }
                    }
                }
            }
            Err(err) => eprintln!("{}", err),
        }
    }
}

fn convert_to_utf8(data: &[u8]) -> String {
    if data.is_ascii() {
        return unsafe { String::from_utf8_unchecked(data.to_vec()) };
    }
    if let Ok(utf) = String::from_utf8(data.to_vec()) {
        return utf;
    }
    let mut res = String::new();
    for c in data {
        res.push(CP437_TO_UNICODE[*c as usize]);
    }
    res
}

pub const CP437_TO_UNICODE: [char; 256] = [
    '\u{0000}', '\u{263a}', '\u{263b}', '\u{2665}', '\u{2666}', '\u{2663}', '\u{2660}', '\u{2022}', '\u{25d8}', '\u{25cb}', '\u{25d9}', '\u{2642}', '\u{2640}',
    '\u{266a}', '\u{266b}', '\u{263c}', '\u{25ba}', '\u{25c4}', '\u{2195}', '\u{203c}', '\u{00b6}', '\u{00a7}', '\u{25ac}', '\u{21a8}', '\u{2191}', '\u{2193}',
    '\u{2192}', '\u{2190}', '\u{221f}', '\u{2194}', '\u{25b2}', '\u{25bc}', '\u{0020}', '\u{0021}', '\u{0022}', '\u{0023}', '\u{0024}', '\u{0025}', '\u{0026}',
    '\u{0027}', '\u{0028}', '\u{0029}', '\u{002a}', '\u{002b}', '\u{002c}', '\u{002d}', '\u{002e}', '\u{002f}', '\u{0030}', '\u{0031}', '\u{0032}', '\u{0033}',
    '\u{0034}', '\u{0035}', '\u{0036}', '\u{0037}', '\u{0038}', '\u{0039}', '\u{003a}', '\u{003b}', '\u{003c}', '\u{003d}', '\u{003e}', '\u{003f}', '\u{0040}',
    '\u{0041}', '\u{0042}', '\u{0043}', '\u{0044}', '\u{0045}', '\u{0046}', '\u{0047}', '\u{0048}', '\u{0049}', '\u{004a}', '\u{004b}', '\u{004c}', '\u{004d}',
    '\u{004e}', '\u{004f}', '\u{0050}', '\u{0051}', '\u{0052}', '\u{0053}', '\u{0054}', '\u{0055}', '\u{0056}', '\u{0057}', '\u{0058}', '\u{0059}', '\u{005a}',
    '\u{005b}', '\u{005c}', '\u{005d}', '\u{005e}', '\u{005f}', '\u{0060}', '\u{0061}', '\u{0062}', '\u{0063}', '\u{0064}', '\u{0065}', '\u{0066}', '\u{0067}',
    '\u{0068}', '\u{0069}', '\u{006a}', '\u{006b}', '\u{006c}', '\u{006d}', '\u{006e}', '\u{006f}', '\u{0070}', '\u{0071}', '\u{0072}', '\u{0073}', '\u{0074}',
    '\u{0075}', '\u{0076}', '\u{0077}', '\u{0078}', '\u{0079}', '\u{007a}', '\u{007b}', '\u{007c}', '\u{007d}', '\u{007e}', '\u{007f}', '\u{00c7}', '\u{00fc}',
    '\u{00e9}', '\u{00e2}', '\u{00e4}', '\u{00e0}', '\u{00e5}', '\u{00e7}', '\u{00ea}', '\u{00eb}', '\u{00e8}', '\u{00ef}', '\u{00ee}', '\u{00ec}', '\u{00c4}',
    '\u{00c5}', '\u{00c9}', '\u{00e6}', '\u{00c6}', '\u{00f4}', '\u{00f6}', '\u{00f2}', '\u{00fb}', '\u{00f9}', '\u{00ff}', '\u{00d6}', '\u{00dc}', '\u{00a2}',
    '\u{00a3}', '\u{00a5}', '\u{20a7}', '\u{0192}', '\u{00e1}', '\u{00ed}', '\u{00f3}', '\u{00fa}', '\u{00f1}', '\u{00d1}', '\u{00aa}', '\u{00ba}', '\u{00bf}',
    '\u{2310}', '\u{00ac}', '\u{00bd}', '\u{00bc}', '\u{00a1}', '\u{00ab}', '\u{00bb}', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{2561}',
    '\u{2562}', '\u{2556}', '\u{2555}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255d}', '\u{255c}', '\u{255b}', '\u{2510}', '\u{2514}', '\u{2534}', '\u{252c}',
    '\u{251c}', '\u{2500}', '\u{253c}', '\u{255e}', '\u{255f}', '\u{255a}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256c}', '\u{2567}',
    '\u{2568}', '\u{2564}', '\u{2565}', '\u{2559}', '\u{2558}', '\u{2552}', '\u{2553}', '\u{256b}', '\u{256a}', '\u{2518}', '\u{250c}', '\u{2588}', '\u{2584}',
    '\u{258c}', '\u{2590}', '\u{2580}', '\u{03b1}', '\u{00df}', '\u{0393}', '\u{03c0}', '\u{03a3}', '\u{03c3}', '\u{00b5}', '\u{03c4}', '\u{03a6}', '\u{0398}',
    '\u{03a9}', '\u{03b4}', '\u{221e}', '\u{03c6}', '\u{03b5}', '\u{2229}', '\u{2261}', '\u{00b1}', '\u{2265}', '\u{2264}', '\u{2320}', '\u{2321}', '\u{00f7}',
    '\u{2248}', '\u{00b0}', '\u{2219}', '\u{00b7}', '\u{221a}', '\u{207f}', '\u{00b2}', '\u{25a0}', '\u{00a0}',
];

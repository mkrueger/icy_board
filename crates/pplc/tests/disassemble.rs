//! Runtime 400 has no legacy word encoding, so the disassembler must not try to
//! serialize its instructions into one.
use std::{fs, process::Command};

fn disassemble(source: &str, runtime: &str) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!("pplc_disasm_{}_{:?}", std::process::id(), std::thread::current().id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("disasm.pps");
    fs::write(&file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pplc"))
        .env_remove("PPL_LANG_VERSION")
        .env("NO_COLOR", "1")
        .arg("--disassemble")
        .arg("--runtime")
        .arg(runtime)
        .arg(&file)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr);
    let ppe_written = file.with_extension("ppe").exists();
    fs::remove_dir_all(&dir).unwrap();
    assert!(!ppe_written, "--disassemble wrote an executable");
    (output.status.code().unwrap_or(-1), strip_ansi(&text))
}

/// The dump styles single values, so escapes land in the middle of a phrase.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\u{1b}' {
            result.push(character);
            continue;
        }
        if characters.next() == Some('[') {
            for character in characters.by_ref() {
                if character.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    result
}

#[test]
fn a_short_circuit_program_disassembles_at_runtime_400() {
    let (code, text) = disassemble("BOOLEAN flag\nflag = TRUE && FALSE\nPRINTLN flag\n", "400");

    assert_eq!(code, 0, "{text}");
    assert!(text.contains("&&"), "{text}");
    assert!(text.contains("LET") && text.contains("PrintLn"), "{text}");
    assert!(text.contains("Instructions: 3"), "{text}");
    assert!(!text.contains("script buffer size"), "{text}");
}

#[test]
fn a_legacy_target_still_shows_its_word_encoding() {
    let (code, text) = disassemble("BOOLEAN flag\nflag = TRUE\nPRINTLN flag\n", "340");

    assert_eq!(code, 0, "{text}");
    assert!(text.contains("Real uncompressed script buffer size: 24 bytes"), "{text}");
    assert!(text.contains("0008 0001 0000 0002"), "{text}");
}

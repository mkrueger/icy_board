//! Reaches the PPL 400 container decoders.
//!
//! Random bytes practically never reproduce the eight magic bytes a 400 file starts with, so a
//! plain byte fuzzer only ever measures the legacy loader. Both targets here therefore build on
//! real 400 files: `fuzz_container` mutates the wire image to cover the header, the directory and
//! the compressed payloads, while `fuzz_sections` keeps the container intact and corrupts one
//! section so the payload decoders are reached behind it.

use std::sync::LazyLock;

use icy_board_engine::{
    executable::{
        Executable,
        container::{Compression, Container, HEADER_SIZE, LoadLimits, MAGIC},
    },
    parser::UserTypeRegistry,
};

const MAX_INPUT: usize = 256 * 1024;
const LIMITS: LoadLimits = LoadLimits {
    file_bytes: 1024 * 1024,
    section_bytes: 1024 * 1024,
    decoded_bytes: 4 * 1024 * 1024,
    constant_bytes: 4 * 1024 * 1024,
    sections: 64,
    zstd_window_log: 20,
};
const SEEDS: [&[u8]; 3] = [
    include_bytes!("../../crates/icy_board_engine/tests/stored_ppe/host_objects.ppe"),
    include_bytes!("../../crates/icy_board_engine/tests/stored_ppe/optional_arguments.ppe"),
    include_bytes!("../../crates/icy_board_engine/tests/stored_ppe/language_core.ppe"),
];
const PAYLOADS: [[u8; 4]; 7] = [*b"TYPE", *b"CONS", *b"VARS", *b"ROUT", *b"IMPT", *b"CODE", *b"DBUG"];
static CONTAINERS: LazyLock<Vec<Container>> = LazyLock::new(|| SEEDS.iter().map(|bytes| Container::decode(bytes, &LIMITS).unwrap()).collect());
static WIRE_SEEDS: LazyLock<Vec<Vec<u8>>> = LazyLock::new(|| {
    CONTAINERS
        .iter()
        .flat_map(|container| [Compression::None, Compression::Zstd].map(|compression| container.encode(compression, &LIMITS).unwrap()))
        .collect()
});
// Rebuilding the board catalog per input would dominate the run time.
static REGISTRY: LazyLock<UserTypeRegistry> = LazyLock::new(UserTypeRegistry::icy_board_registry);

fn mutate(bytes: &mut Vec<u8>, edits: &[u8]) {
    for edit in edits.chunks(6).take(1024).filter(|edit| edit.len() >= 2) {
        let operands = edit.len() - 2;
        let mut address = [0; 4];
        address[..operands].copy_from_slice(&edit[..operands]);
        let offset = u32::from_le_bytes(address) as usize;
        let value = edit[operands + 1];
        let position = offset % bytes.len().max(1);
        match edit[operands] % 6 {
            0 if !bytes.is_empty() => bytes[position] = value,
            1 if !bytes.is_empty() => bytes[position] ^= value,
            2 if bytes.len() < MAX_INPUT => bytes.insert(offset % (bytes.len() + 1), value),
            3 if !bytes.is_empty() => {
                bytes.remove(position);
            }
            4 => bytes.truncate(offset % (bytes.len() + 1)),
            5 if !bytes.is_empty() => {
                let end = position.saturating_add(4).min(bytes.len());
                bytes[position..end].fill(value);
            }
            _ => {}
        }
    }
}

fn check_executable(bytes: &[u8], compression: Compression) {
    let Ok(executable) = Executable::from_container_with_registry(bytes, &LIMITS, &REGISTRY) else {
        return;
    };
    let Ok(mut written) = executable.to_buffer_with_compression(compression) else {
        return;
    };
    let normalized = Executable::from_buffer(&mut written, false).expect("written PPE 400 must load");
    let mut rewritten = normalized.to_buffer_with_compression(compression).expect("normalized PPE 400 must write");
    Executable::from_buffer(&mut rewritten, false).expect("rewritten PPE 400 must load");
    assert_eq!(written, rewritten, "normalized PPE 400 encoding changed");
}

pub fn fuzz_container(data: &[u8]) {
    if data.len() > MAX_INPUT {
        return;
    }
    let Some((&mode, rest)) = data.split_first() else { return };
    let bytes = if data.starts_with(MAGIC) {
        data.to_vec()
    } else {
        match mode % 3 {
            0 => rest.to_vec(),
            _ => {
                let Some((&seed, edits)) = rest.split_first() else { return };
                let mut bytes = WIRE_SEEDS[seed as usize % WIRE_SEEDS.len()].clone();
                mutate(&mut bytes, edits);
                if mode % 3 == 2 && bytes.len() >= HEADER_SIZE {
                    let length = bytes.len() as u64;
                    bytes[48..56].copy_from_slice(&length.to_le_bytes());
                }
                bytes
            }
        }
    };
    let Ok(container) = Container::decode(&bytes, &LIMITS) else { return };
    for compression in [Compression::None, Compression::Zstd] {
        let Ok(encoded) = container.encode(compression, &LIMITS) else { continue };
        assert_eq!(container, Container::decode(&encoded, &LIMITS).expect("encoded container must load"));
    }
}

fn section_input(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 3 || data.len() > MAX_INPUT {
        return None;
    }
    let mut container = CONTAINERS[data[0] as usize % CONTAINERS.len()].clone();
    // Keep identity verification from masking mutations of the program sections.
    container.sections.retain(|section| section.kind != *b"IDEN");
    let section = container
        .sections
        .iter_mut()
        .find(|section| section.kind == PAYLOADS[data[1] as usize % PAYLOADS.len()])?;
    let mut edits = &data[3..];
    if data[2] & 2 != 0 {
        section.entries = u32::from_le_bytes(edits.get(..4)?.try_into().unwrap());
        edits = &edits[4..];
    }
    if data[2] & 4 != 0 {
        section.data = edits.to_vec();
    } else {
        mutate(&mut section.data, edits);
    }
    let compression = if data[2] & 1 == 0 { Compression::None } else { Compression::Zstd };
    container.encode(compression, &LIMITS).ok()
}

pub fn fuzz_sections(data: &[u8]) {
    if let Some(bytes) = section_input(data) {
        check_executable(&bytes, if data[2] & 1 == 0 { Compression::None } else { Compression::Zstd });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_mutations_do_not_require_a_full_address() {
        let mut bytes = vec![1, 2, 3];
        mutate(&mut bytes, &[0, 42]);
        assert_eq!(bytes, [42, 2, 3]);
        mutate(&mut bytes, &[2, 0, 43]);
        assert_eq!(bytes, [42, 2, 43]);
        mutate(&mut bytes, &[0, 4, 0]);
        assert!(bytes.is_empty());
        mutate(&mut bytes, &[2, 44]);
        assert_eq!(bytes, [44]);
    }

    #[test]
    fn seeds_reach_executable_roundtrips_in_both_compression_modes() {
        let registry = UserTypeRegistry::icy_board_registry();
        for seed in 0..SEEDS.len() {
            for section in 0..PAYLOADS.len() {
                for compression in 0..2 {
                    let input = [seed as u8, section as u8, compression];
                    let bytes = section_input(&input).expect("seed section must exist");
                    Executable::from_container_with_registry(&bytes, &LIMITS, &registry).unwrap();
                    fuzz_sections(&input);
                }
            }
        }
        for seed in 0..WIRE_SEEDS.len() {
            fuzz_container(&WIRE_SEEDS[seed]);
            fuzz_container(&[1, seed as u8]);
            fuzz_container(&[2, seed as u8]);
            let mut raw = vec![0];
            raw.extend_from_slice(&WIRE_SEEDS[seed]);
            fuzz_container(&raw);
        }
    }

    #[test]
    fn payload_mutations_pass_container_checks_and_reach_section_decoders() {
        let registry = UserTypeRegistry::icy_board_registry();
        for section in 0..PAYLOADS.len() {
            for compression in 0..2 {
                let input = [2, section as u8, 4 | compression, 255];
                let bytes = section_input(&input).unwrap();
                let container = Container::decode(&bytes, &LIMITS).unwrap();
                assert!(container.sections.iter().all(|section| section.kind != *b"IDEN"));
                assert_eq!(container.sections.iter().find(|item| item.kind == PAYLOADS[section]).unwrap().data, [255]);
                let error = Executable::from_container_with_registry(&bytes, &LIMITS, &registry)
                    .err()
                    .expect("truncated section must fail");
                assert!(!error.to_string().contains("identity"), "{error}");
            }
        }
    }

    #[test]
    fn section_counts_are_mutated_without_breaking_the_container() {
        let registry = UserTypeRegistry::icy_board_registry();
        for section in 0..PAYLOADS.len() {
            let mut input = vec![2, section as u8, 2];
            input.extend_from_slice(&u32::MAX.to_le_bytes());
            let bytes = section_input(&input).unwrap();
            let container = Container::decode(&bytes, &LIMITS).unwrap();
            assert_eq!(container.sections.iter().find(|item| item.kind == PAYLOADS[section]).unwrap().entries, u32::MAX);
            let error = Executable::from_container_with_registry(&bytes, &LIMITS, &registry)
                .err()
                .expect("entry limit must fail");
            assert!(error.to_string().contains("section entries"), "{error}");
        }
    }

    #[test]
    fn wire_seeds_exercise_zstd_corruption_and_wide_mutation_offsets() {
        use icy_board_engine::executable::container::DIRECTORY_ENTRY_SIZE;
        let mut checked = 0;
        for seed in (1..WIRE_SEEDS.len()).step_by(2) {
            let bytes = &WIRE_SEEDS[seed];
            let count = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
            for entry in bytes[HEADER_SIZE..HEADER_SIZE + count * DIRECTORY_ENTRY_SIZE].chunks_exact(DIRECTORY_ENTRY_SIZE) {
                if u32::from_le_bytes(entry[8..12].try_into().unwrap()) != 1 {
                    continue;
                }
                let offset = u64::from_le_bytes(entry[16..24].try_into().unwrap()) as usize;
                let length = u64::from_le_bytes(entry[24..32].try_into().unwrap()) as usize;
                let mut edit = ((offset + length - 1) as u32).to_le_bytes().to_vec();
                edit.extend_from_slice(&[1, 1]);
                let mut damaged = bytes.clone();
                mutate(&mut damaged, &edit);
                assert!(Container::decode(&damaged, &LIMITS).is_err());
                let mut input = vec![1, seed as u8];
                input.extend_from_slice(&edit);
                fuzz_container(&input);
                checked += 1;
            }
        }
        assert!(checked >= SEEDS.len());
        let mut bytes = vec![0; 100_000];
        let mut edit = 90_000u32.to_le_bytes().to_vec();
        edit.extend_from_slice(&[0, 42]);
        mutate(&mut bytes, &edit);
        assert_eq!(bytes[90_000], 42);
        assert_eq!(bytes.iter().filter(|&&byte| byte != 0).count(), 1);
    }
}

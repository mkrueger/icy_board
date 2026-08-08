//! The MASK_* functions, whose character sets PCBoard's callers rely on.

use super::run_ppl;

#[test]
fn test_mask_alnum_is_letters_and_digits_only() {
    assert_eq!(run_ppl("PRINT MASK_ALNUM()"), "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");
}

#[test]
fn test_mask_alpha_is_letters_only() {
    assert_eq!(run_ppl("PRINT MASK_ALPHA()"), "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");
}

#[test]
fn test_mask_num_is_digits_only() {
    assert_eq!(run_ppl("PRINT MASK_NUM()"), "0123456789");
}

#[test]
fn test_mask_ascii_is_every_printable_character() {
    assert_eq!(run_ppl("PRINT LEN(MASK_ASCII())"), "95");
}

#[test]
fn test_mask_file_lets_a_dos_name_through() {
    assert_eq!(run_ppl("PRINT INSTR(MASK_FILE(), \"@\") > 0"), "1");
    assert_eq!(run_ppl("PRINT INSTR(MASK_FILE(), \" \") > 0"), "0");
}

#[test]
fn test_mask_path_adds_the_separators() {
    assert_eq!(run_ppl("PRINT INSTR(MASK_PATH(), \"\\\") > 0"), "1");
    assert_eq!(run_ppl("PRINT INSTR(MASK_PATH(), \":\") > 0"), "1");
}

pub mod tables;

/// Converts \r\n -> \n and stops to read at EOF char (0x1A)
pub fn normalize_file(input: &[u8]) -> Vec<u8> {
    let mut res = Vec::new();
    let mut was_cr = false;
    for &c in input {
        if c == b'\r' {
            continue;
        }
        if c == b'\n' {
            res.push(b'\n');
            continue;
        }
        if c == 0x1A {
            break;
        }
        was_cr = c == b'\r';
        res.push(c);
    }
    res
}

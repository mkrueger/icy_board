use codepages::tables::{CP437_TO_UNICODE, UNICODE_TO_CP437};

pub fn import_cp437_string(chunk: &[u8], trim_end: bool) -> String {
    let mut res = String::new();
    let mut got_cr = false;
    for &c in chunk {
        if c == 0 || c == 0x1A {
            break;
        }
        if c == b'\r' {
            got_cr = true;
            continue;
        }
        if got_cr {
            got_cr = false;
            if c != b'\n' {
                res.push(CP437_TO_UNICODE[b'\r' as usize]);
            }
        }
        res.push(CP437_TO_UNICODE[c as usize]);
    }
    if trim_end {
        while res.ends_with(' ') {
            res.pop();
        }
    }
    res
}

pub fn export_cp437_string(txt: &str, len: usize, filler: u8) -> Vec<u8> {
    let mut res = Vec::with_capacity(len);
    for c in txt.chars() {
        if let Some(&cp437) = UNICODE_TO_CP437.get(&c) {
            res.push(cp437);
        } else {
            res.push(b' ');
        }
        if res.len() >= len {
            break;
        }
    }
    res.extend(std::iter::repeat(filler).take(len - res.len()));
    res
}

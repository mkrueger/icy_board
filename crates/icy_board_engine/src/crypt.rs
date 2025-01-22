const CRYPT_DATA: [u8; 17] = [
    0x8C, 0x53, 0xB8, 0xA7, 0x9E, 0x0F, 0x0A, 0xCB, 0x28, 0x62, 0x2D, 0x50, 0x7E, 0x05, 0x3D, 0x4E, 0x35,
];

fn crypt3(block: &mut [u8]) {
    let mut len = block.len();
    let mut i = 0;
    let mut j = 0;
    while len > 0 {
        block[i] ^= CRYPT_DATA[j].wrapping_add(len as u8);
        j = (j + 1) % CRYPT_DATA.len();
        len -= 1;
        i += 1;
    }
}

fn decrypt2(block: &mut [u8]) {
    if block.len() > CHUNK_SIZE {
        panic!("Block size is too large.");
    }
    let size = block.len() as i32;
    let mut i = 0;
    let mut seed = 0xDB24;
    let mut rotate_count = 0;
    let mut dx = size >> 1;
    while dx > 0 {
        let cur_word = (block[i + 1] as u16) << 8 | block[i] as u16;
        let dl = dx as u8;
        rotate_count = ((seed & 0xFF) + (dl as u16)) & 0xFF;
        let outx = u16::rotate_right(cur_word, rotate_count as u32) ^ seed;
        block[i] = (outx as u8) ^ dl;
        i += 1;
        block[i] = (outx >> 8) as u8 ^ dl;
        i += 1;
        seed = cur_word;
        dx -= 1;
    }

    if size % 2 == 1 {
        block[i] = u8::rotate_right(block[i] ^ (seed as u8), (rotate_count & 0xFF) as u32);
    }
}

fn encrypt2(block: &mut [u8]) {
    if block.len() > CHUNK_SIZE {
        panic!("Block size is too large.");
    }
    let size = block.len() as i32;
    let mut seed = 0xDB24;
    let mut rotate_count = 0;
    let mut dx = size >> 1;
    let mut i = 0;
    while dx > 0 {
        let cur_word = (block[i + 1] as u16) << 8 | block[i] as u16;
        let dl = dx as u16 & 0xFF;
        rotate_count = (seed & 0xFF) + (dl & 0xFF);
        let tmp = dl | dl << 8;
        let outx = u16::rotate_left(cur_word ^ seed ^ tmp, rotate_count as u32);
        block[i] = outx as u8;
        i += 1;
        block[i] = (outx >> 8) as u8;
        i += 1;
        seed = outx;
        dx -= 1;
    }

    if size % 2 == 1 {
        block[i] = u8::rotate_left(block[i], (rotate_count & 0xFF) as u32) ^ (seed as u8);
    }
}

#[allow(clippy::pedantic)]
fn decrypt(block: &mut [u8], version: u16) {
    if version < 300 || version >= 400 {
        return;
    }
    if version >= 330 {
        crypt3(block);
    }

    decrypt2(block);

    if version >= 340 && !block.is_empty() {
        block[0] ^= b'T';
    }
}

#[allow(clippy::pedantic)]
fn encrypt(block: &mut [u8], version: u16) {
    if version < 300 || version >= 400 {
        return;
    }
    if version >= 340 {
        block[0] ^= b'T';
    }
    encrypt2(block);
    if version >= 330 {
        crypt3(block);
    }
}

pub fn encode_rle(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < src.len() {
        let cur = src[i];
        i += 1;
        if cur == 0 {
            let mut count = 1;
            while i < src.len() && count < 255 && src[i] == 0 {
                i += 1;
                count += 1;
            }
            result.push(0);
            result.push(count as u8);
        } else {
            result.push(cur);
        }
    }
    result
}

pub fn decode_rle(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < src.len() {
        let cur = src[i];
        i += 1;

        result.push(cur);
        if cur == 0 {
            if i >= src.len() {
                break;
            }
            let count = src[i].saturating_sub(1);
            i += 1;
            result.resize(result.len() + count as usize, 0);
        }
    }
    result
}

const CHUNK_SIZE: usize = 2027;

pub fn encrypt_chunks(buffer: &mut [u8], version: u16, use_rle: bool) {
    let mut offset: usize = 0;
    while offset < buffer.len() {
        let add_byte = use_rle && offset + CHUNK_SIZE - 1 < buffer.len() && buffer[offset + CHUNK_SIZE - 1] == 0;
        let end = (offset + CHUNK_SIZE).min(buffer.len());
        let chunk = &mut buffer[offset..end];
        encrypt(chunk, version);
        offset += CHUNK_SIZE;
        if add_byte {
            offset += 1;
        }
    }
}

pub fn decrypt_chunks(buffer: &mut [u8], version: u16, use_rle: bool) {
    let code_data_len = buffer.len();
    let mut offset = 0;

    while offset < code_data_len {
        let end = (offset + CHUNK_SIZE).min(code_data_len);
        let chunk = &mut buffer[offset..end];
        decrypt(chunk, version);
        if use_rle && offset + CHUNK_SIZE < code_data_len && chunk[CHUNK_SIZE - 1] == 0 {
            offset += 1;
        }
        offset += CHUNK_SIZE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_cypt3_simple() {
        let o_buffer = b"Hello World here I'am.";
        let mut buffer = o_buffer.clone();
        crypt3(&mut buffer);
        assert_ne!(buffer, *o_buffer);
        crypt3(&mut buffer);
        assert_eq!(buffer, *o_buffer);
    }

    #[test]
    fn test_decrypt_simple() {
        let mut buffer: [u8; 11] = [188, 113, 184, 117, 181, 219, 236, 219, 189, 187, 189];
        decrypt(&mut buffer, 300);
        let expected: [u8; 11] = [25, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0];
        assert_eq!(buffer, expected);
    }

    fn test_rle(data: &[u8]) {
        let encoded = encode_rle(data);
        let res = decode_rle(&encoded);
        assert_ne!(encoded.len(), res.len(), "array length should differ."); // assert there are some 0 in the input data.
        assert_eq!(res.len(), data.len(), "array size mismatch.");
        for i in 0..data.len() {
            assert_eq!(res[i], data[i], "data mismatch.");
        }
    }

    #[test]
    fn test_encode_rle() {
        let encoded = encode_rle(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&encoded, &[0, 9]);
    }

    #[test]
    fn test_revert_rle() {
        test_rle(&[0, 1, 2, 3, 4, 5]);
        test_rle(&[0, 1, 2, 0, 3, 4, 5, 0, 0]);
        test_rle(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
        test_rle(&[1, 2, 3, 4, 0, 6, 7, 0, 9]);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_encrypt2_simple() {
        let o_buffer = b"Hello World here I'am.";
        let mut buffer = o_buffer.clone();
        encrypt2(&mut buffer);
        assert_ne!(buffer, *o_buffer);
        decrypt2(&mut buffer);
        assert_eq!(buffer, *o_buffer);
    }

    #[test]
    fn test_large_crypt3() {
        let mut data = vec![0u8; CHUNK_SIZE];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }

        crypt3(&mut data);
        crypt3(&mut data);

        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    #[test]
    fn test_large_crypt2() {
        let mut data = vec![0u8; CHUNK_SIZE];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }

        encrypt2(&mut data);
        decrypt2(&mut data);

        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    #[test]
    fn test_chunks_with_rle_330() {
        let mut data = vec![0u8; CHUNK_SIZE * 16];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }
        encrypt_chunks(&mut data, 330, true);
        decrypt_chunks(&mut data, 330, true);
        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    #[test]
    fn test_chunks_with_rle_300() {
        let mut data = vec![0u8; CHUNK_SIZE * 16];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }
        encrypt_chunks(&mut data, 300, true);
        decrypt_chunks(&mut data, 300, true);
        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    #[test]
    fn test_chunks_with_rle_340() {
        let mut data = vec![0u8; CHUNK_SIZE * 16];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }
        encrypt_chunks(&mut data, 340, true);
        decrypt_chunks(&mut data, 340, true);
        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    #[test]
    fn test_chunks_without_rle() {
        let mut data = vec![0u8; CHUNK_SIZE * 16];
        for i in 0..data.len() {
            if is_prime(i) {
                data[i] = i as u8;
            }
        }
        encrypt_chunks(&mut data, 330, false);
        decrypt_chunks(&mut data, 330, false);
        for i in 0..data.len() {
            if is_prime(i) {
                assert_eq!(data[i], i as u8);
            } else {
                assert_eq!(data[i], 0);
            }
        }
    }

    fn is_prime(num: usize) -> bool {
        if num <= 1 {
            return false;
        }

        for i in 2..=(num as f64).sqrt() as usize {
            if num % i == 0 {
                return false;
            }
        }

        true
    }
}

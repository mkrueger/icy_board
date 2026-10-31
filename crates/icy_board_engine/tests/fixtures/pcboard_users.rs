fn field(text: &str, size: usize, padding: u8) -> Vec<u8> {
    let mut data = text.as_bytes().to_vec();
    assert!(data.len() <= size);
    data.resize(size, padding);
    data
}

fn descriptor(name: &str, keyword: &str, version: u16, size: u16, conference_size: u16, offset: u32) -> Vec<u8> {
    let mut data = field(name, 15, 0);
    data.extend(version.to_le_bytes());
    data.extend(size.to_le_bytes());
    data.extend(conference_size.to_le_bytes());
    data.extend(field(keyword, 9, 0));
    data.extend(offset.to_le_bytes());
    assert_eq!(data.len(), 34);
    data
}

fn base_record(name: &str) -> Vec<u8> {
    let mut data = field(name, 25, b' ');
    data.extend(4321u32.to_le_bytes());
    data.extend(123u32.to_le_bytes());
    data
}

pub fn users_record(name: &str, inf_number: u32) -> Vec<u8> {
    let mut data = vec![0; 400];
    data[..25].copy_from_slice(&field(name, 25, b' '));
    data[49..61].copy_from_slice(&field("SECRET", 12, b' '));
    data[98] = b'N';
    data[99] = b'Z';
    data[100] = 1 << 4;
    data[107] = 42;
    data[110] = 23;
    data[193] = 1;
    data[198] = 2;
    data[203] = 4;
    data[224] = b'N';
    data[385..389].copy_from_slice(&inf_number.to_le_bytes());
    data[398..400].copy_from_slice(&49u16.to_le_bytes());
    data
}

pub fn basic_inf(name: &str, alias: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(6u16.to_le_bytes());
    data.extend(0u16.to_le_bytes());
    data.extend(33u16.to_le_bytes());
    data.extend(0u32.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(58u32.to_le_bytes());
    data.extend(descriptor("PCBALIAS", "", 1, 25, 0, 33));
    data.extend(base_record(name));
    data.extend(field(alias, 25, b' '));
    data
}

pub fn extended_inf() -> Vec<u8> {
    let mut record = base_record("EXTENDED");
    // Independent USERSYS.DOC layout for 50 conferences: 7/7/2/2/2 byte bitmaps.
    let mut conferences = vec![0; 20];
    conferences[0] = 1 << 3;
    conferences[6] = 1 << 1;
    conferences[7] = 1;
    conferences[12] = 1;
    conferences[13] = 1;
    conferences[14] = 1;
    conferences[15] = 2;
    conferences[16] = 2;
    conferences[17] = 2;
    conferences[18] = 4;
    conferences[19] = 2;
    record.extend(conferences);
    for conference in 40..50 {
        let pointer: u32 = match conference {
            40 => 11,
            49 => u32::MAX - 1,
            _ => 0,
        };
        record.extend(pointer.to_le_bytes());
    }
    assert_eq!(record.len(), 93);
    let tpa_offset = record.len() as u32;
    record.extend(0..=255u8);
    for conference in 0..50u8 {
        record.extend([conference, 0x80]);
    }
    let alias_offset = record.len() as u32;
    record.extend(field("EXTENDEDALIAS", 25, b' '));
    let qwk_offset = record.len() as u32;
    for value in [800u16, 65, 40, 70, 12] {
        record.extend(value.to_le_bytes());
    }
    record.push(1);
    record.extend([0; 19]);
    record.extend(0..50u8);

    let mut data = Vec::new();
    data.extend(6u16.to_le_bytes());
    data.extend(10u16.to_le_bytes());
    data.extend(33u16.to_le_bytes());
    data.extend(60u32.to_le_bytes());
    data.extend(3u16.to_le_bytes());
    data.extend((record.len() as u32).to_le_bytes());
    data.extend(descriptor("AUDITAPP", "AUDIT", 42, 256, 2, tpa_offset));
    data.extend(descriptor("PCBALIAS", "", 1, 25, 0, alias_offset));
    data.extend(descriptor("PCBQWKNET", "QWKCFG", 7, 30, 1, qwk_offset));
    data.extend(record);
    data
}

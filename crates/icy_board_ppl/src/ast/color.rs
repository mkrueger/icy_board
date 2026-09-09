pub fn rgba_value(red: i32, green: i32, blue: i32, alpha: i32) -> u32 {
    ((red.clamp(0, 255) as u32) << 24) | ((green.clamp(0, 255) as u32) << 16) | ((blue.clamp(0, 255) as u32) << 8) | alpha.clamp(0, 255) as u32
}

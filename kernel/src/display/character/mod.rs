mod font8x8;
/// Gets the 8x8 font character. If the character is not found, it returns the exclamation mark.
pub fn get_char(c: char) -> [u8; 8] {
    let c = c as u32;
    match c {
        0x00..=0x7F => font8x8::BASIC[c as usize],
        0xa0..=0xff => font8x8::LATIN[(c - 0xa0) as usize],
        0x390..=0x3C9 => font8x8::GREEK[(c - 0x390) as usize],
        0x2500..=0x257F => font8x8::BOX_DRAWING[(c - 0x2500) as usize],
        0x2580..=0x259F => font8x8::BLOCK[(c - 0x2580) as usize],
        0x3041..=0x309F => font8x8::HIRAGANA[(c - 0x3041) as usize],
        _ => font8x8::BASIC[22], // !
    }
}

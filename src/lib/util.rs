pub fn encode_u32_from_four_u8s(
    bytes: &[u8],
    first_idx: u8,
    second_idx: u8,
    third_idx: u8,
    fourth_idx: u8,
) -> u32 {
    u32::from_be_bytes([
        bytes[first_idx as usize],
        bytes[second_idx as usize],
        bytes[third_idx as usize],
        bytes[fourth_idx as usize],
    ])
}

pub fn encode_u16_from_two_u8s(bytes: &[u8], first_idx: u8, second_idx: u8) -> u16 {
    u16::from_be_bytes([bytes[first_idx as usize], bytes[second_idx as usize]])
}

pub fn encode_lookup_to_dns(bytes: &Vec<u8>) -> Vec<u8> {
    let dns = String::from_utf8(bytes.to_owned()).expect("Could not parse UTF-8 from bytes");

    let split = dns.split('.');
    let mut encoded = Vec::new();
    for part in split {
        encoded.push(part.len() as u8);
        encoded.extend(part.as_bytes());
    }
    encoded.push(b'\0');
    encoded
}

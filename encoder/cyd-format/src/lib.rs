#![cfg_attr(not(test), no_std)]

pub const HEADER_SIZE: usize = 5;

pub fn parse_header(header: &[u8; HEADER_SIZE]) -> (u16, u16, u8) {
    let width = u16::from_le_bytes([header[0], header[1]]);
    let height = u16::from_le_bytes([header[2], header[3]]);
    let fps = header[4];

    (width, height, fps)
}

pub fn encode_header(header: &mut [u8; HEADER_SIZE], width: u16, height: u16, fps: u8) {
    header[..2].copy_from_slice(&width.to_le_bytes());
    header[2..4].copy_from_slice(&height.to_le_bytes());
    header[4] = fps;
}

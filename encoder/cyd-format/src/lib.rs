#![cfg_attr(not(test), no_std)]

pub const HEADER_SIZE: usize = 1;

pub fn parse_header(header: &[u8; HEADER_SIZE]) -> u8 {
    header[0]
}

pub fn encode_header(header: &mut [u8; HEADER_SIZE], fps: u8) {
    header[0] = fps;
}

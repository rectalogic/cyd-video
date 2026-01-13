pub trait FormatHeader<const HEADER_SIZE: usize> {
    const MAX_WIDTH: usize;
    const MAX_HEIGHT: usize;

    fn parse(header: &[u8; HEADER_SIZE]) -> Self;
    fn encode(&self, header: &mut [u8; HEADER_SIZE]);
    fn header_size() -> usize {
        HEADER_SIZE
    }
    fn fps(&self) -> u8;
}

pub mod mjpeg;
pub mod yuv;

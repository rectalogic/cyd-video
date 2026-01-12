pub trait FormatHeader {
    const SIZE: usize;
    const MAX_WIDTH: usize;
    const MAX_HEIGHT: usize;

    fn parse(header: &[u8]) -> Self;
    fn encode(&self, header: &mut [u8]);
    fn fps(&self) -> u8;
}

pub mod mjpeg;

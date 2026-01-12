use super::FormatHeader;

pub struct MjpegHeader {
    fps: u8,
}

impl MjpegHeader {
    pub fn new(fps: u8) -> Self {
        Self { fps }
    }
}

impl FormatHeader<1> for MjpegHeader {
    const MAX_WIDTH: usize = 192;
    const MAX_HEIGHT: usize = 144;

    fn parse(header: &[u8; 1]) -> Self {
        Self::new(header[0])
    }

    fn encode(&self, header: &mut [u8; 1]) {
        header[0] = self.fps;
    }

    fn fps(&self) -> u8 {
        self.fps
    }
}

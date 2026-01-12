use super::FormatHeader;

pub struct MjpegHeader {
    fps: u8,
}

impl MjpegHeader {
    pub fn new(fps: u8) -> Self {
        Self { fps }
    }
}

impl FormatHeader for MjpegHeader {
    const SIZE: usize = 1;
    const MAX_WIDTH: usize = 192;
    const MAX_HEIGHT: usize = 144;

    fn parse(header: &[u8]) -> Self {
        assert_eq!(header.len(), Self::SIZE);
        Self::new(header[0])
    }

    fn encode(&self, header: &mut [u8]) {
        assert_eq!(header.len(), Self::SIZE);
        header[0] = self.fps;
    }

    fn fps(&self) -> u8 {
        self.fps
    }
}

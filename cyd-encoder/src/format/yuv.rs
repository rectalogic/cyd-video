use super::FormatHeader;

pub struct YuvHeader {
    width: u16,
    height: u16,
    fps: u8,
}

impl YuvHeader {
    pub fn new(width: u16, height: u16, fps: u8) -> Self {
        Self { width, height, fps }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}

impl FormatHeader<5> for YuvHeader {
    const MAX_WIDTH: usize = 320;
    const MAX_HEIGHT: usize = 240;

    fn parse(header: &[u8; 5]) -> Self {
        let width = u16::from_le_bytes([header[0], header[1]]);
        let height = u16::from_le_bytes([header[2], header[3]]);
        let fps = header[4];

        Self::new(width, height, fps)
    }

    fn encode(&self, header: &mut [u8; 5]) {
        header[..2].copy_from_slice(&self.width.to_le_bytes());
        header[2..4].copy_from_slice(&self.height.to_le_bytes());
        header[4] = self.fps;
    }

    fn fps(&self) -> u8 {
        self.fps
    }
}

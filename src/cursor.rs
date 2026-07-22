use embedded_io::{Error, ErrorKind, ErrorType, Read, Seek, SeekFrom};

#[derive(thiserror::Error, Debug)]
pub enum CursorError {
    #[error("seek position out of bounds")]
    InvalidSeek,
}

impl Error for CursorError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::InvalidInput
    }
}

pub struct Cursor<'a> {
    original: &'a [u8],
    current: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self {
            original: slice,
            current: slice,
        }
    }

    fn position(&self) -> u64 {
        (self.original.len() - self.current.len()) as u64
    }
}

impl ErrorType for Cursor<'_> {
    type Error = CursorError;
}

impl Read for Cursor<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match self.current.read(buf) {
            Ok(n) => Ok(n),
            Err(infallible) => match infallible {},
        }
    }
}

impl Seek for Cursor<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_pos: i64 = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => self.original.len() as i64 + n,
            SeekFrom::Current(n) => self.position() as i64 + n,
        };

        if new_pos < 0 || new_pos as usize > self.original.len() {
            return Err(CursorError::InvalidSeek);
        }

        let new_pos = new_pos as usize;
        self.current = &self.original[new_pos..];
        Ok(new_pos as u64)
    }
}

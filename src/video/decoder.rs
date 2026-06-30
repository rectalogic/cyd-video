use crate::video::esp_new_jpeg::{
    jpeg_calloc_align, jpeg_dec_close, jpeg_dec_config_t, jpeg_dec_get_outbuf_len,
    jpeg_dec_get_process_count, jpeg_dec_handle_t, jpeg_dec_header_info_t, jpeg_dec_io_t,
    jpeg_dec_open, jpeg_dec_parse_header, jpeg_dec_process, jpeg_error_t, jpeg_free_align,
    jpeg_pixel_format_t,
};
use core::{
    ffi::{self, c_int},
    iter::Peekable,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MjpegError {
    #[error("MJPEG stream exhausted")]
    StreamExhausted,
    #[error("Unknown JPEG error")]
    Unknown(ffi::c_int),
    #[error("Device error or wrong termination of input stream")]
    JpegErrFail,
    #[error("Insufficient memory for the image")]
    JpegErrNoMem,
    #[error("Input data is not enough")]
    JpegErrNoMoreData,
    #[error("Parameter error")]
    JpegErrInvalidParam,
    #[error("Data format error (may be damaged data)")]
    JpegErrBadData,
    #[error("Right format but not supported")]
    JpegErrUnsupportFmt,
    #[error("Not supported JPEG standard")]
    JpegErrUnsupportStd,
}

impl From<ffi::c_int> for MjpegError {
    fn from(value: ffi::c_int) -> Self {
        match value {
            jpeg_error_t::JPEG_ERR_FAIL => MjpegError::JpegErrFail,
            jpeg_error_t::JPEG_ERR_NO_MEM => MjpegError::JpegErrNoMem,
            jpeg_error_t::JPEG_ERR_NO_MORE_DATA => MjpegError::JpegErrNoMoreData,
            jpeg_error_t::JPEG_ERR_INVALID_PARAM => MjpegError::JpegErrInvalidParam,
            jpeg_error_t::JPEG_ERR_BAD_DATA => MjpegError::JpegErrBadData,
            jpeg_error_t::JPEG_ERR_UNSUPPORT_FMT => MjpegError::JpegErrUnsupportFmt,
            jpeg_error_t::JPEG_ERR_UNSUPPORT_STD => MjpegError::JpegErrUnsupportStd,
            _ => MjpegError::Unknown(value),
        }
    }
}

pub struct MjpegDecoder {
    handle: jpeg_dec_handle_t,
    header_info: jpeg_dec_header_info_t,
    mcu_buffer: McuBuffer,
    mcu_count: c_int,
}

struct McuBuffer {
    ptr: *mut u8,
    size: usize,
}

impl McuBuffer {
    fn new(size: usize) -> Result<Self, MjpegError> {
        let ptr = unsafe { jpeg_calloc_align(size, 16) };
        if ptr.is_null() {
            return Err(MjpegError::JpegErrNoMem);
        }
        Ok(Self {
            ptr: ptr as *mut u8,
            size,
        })
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr.cast_const(), self.size) }
    }
}

impl Drop for McuBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                jpeg_free_align(self.ptr as *mut _);
            }
        }
    }
}

pub struct McuBlock<'a> {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub data: &'a [u8],
}

impl MjpegDecoder {
    pub fn new(jpeg_data: &[u8]) -> Result<Self, MjpegError> {
        let config = jpeg_dec_config_t {
            output_type: jpeg_pixel_format_t::JPEG_PIXEL_FORMAT_RGB565_LE,
            block_enable: true,
            ..Default::default()
        };

        let mut handle: jpeg_dec_handle_t = core::ptr::null_mut();
        let ret = unsafe {
            jpeg_dec_open(
                &config as *const jpeg_dec_config_t as *mut jpeg_dec_config_t,
                &mut handle,
            )
        };
        if ret != jpeg_error_t::JPEG_ERR_OK {
            return Err(ret.into());
        }

        let mut jpeg_io = jpeg_dec_io_t {
            inbuf: jpeg_data.as_ptr() as *mut u8,
            inbuf_len: jpeg_data.len() as i32,
            ..Default::default()
        };

        let mut header_info = jpeg_dec_header_info_t::default();
        let ret = unsafe {
            jpeg_dec_parse_header(
                handle,
                &mut jpeg_io as *mut jpeg_dec_io_t,
                &mut header_info as *mut jpeg_dec_header_info_t,
            )
        };
        if ret != jpeg_error_t::JPEG_ERR_OK {
            return Err(ret.into());
        }

        let mut mcu_len: c_int = 0;
        let ret = unsafe { jpeg_dec_get_outbuf_len(handle, &mut mcu_len as *mut c_int) };
        if ret != jpeg_error_t::JPEG_ERR_OK || mcu_len == 0 {
            return Err(ret.into());
        }
        let mcu_buffer = McuBuffer::new(mcu_len as usize)?;

        let mut mcu_count: c_int = 0;
        let ret = unsafe { jpeg_dec_get_process_count(handle, &mut mcu_count as *mut c_int) };
        if ret != jpeg_error_t::JPEG_ERR_OK || mcu_count == 0 {
            return Err(ret.into());
        }

        Ok(Self {
            handle,
            header_info,
            mcu_count,
            mcu_buffer,
        })
    }

    pub fn decode<F>(&mut self, jpeg_data: &[u8], mut on_block: F) -> Result<(), MjpegError>
    where
        F: FnMut(McuBlock),
    {
        let mut jpeg_io = jpeg_dec_io_t {
            inbuf: jpeg_data.as_ptr() as *mut u8,
            inbuf_len: jpeg_data.len() as i32,
            outbuf: self.mcu_buffer.ptr,
            ..Default::default()
        };

        // XXX need to reset jpeg_io.inbuf_remain?
        // XXX need to parse header for each frame, so inbuf_remain is set properly?
        for i in 0..self.mcu_count as u16 {
            jpeg_io.out_size = 0;
            let ret = unsafe { jpeg_dec_process(self.handle, &mut jpeg_io as *mut jpeg_dec_io_t) };
            if ret != jpeg_error_t::JPEG_ERR_OK {
                return Err(ret.into());
            }
            let block_data = &self.mcu_buffer.as_slice()[0..jpeg_io.out_size as usize];
            // Calculate block height: out_size / (width * 2 bytes per pixel)
            let block_width = self.header_info.width;
            let block_height = if block_width > 0 {
                (jpeg_io.out_size as u16) / (block_width * 2)
            } else {
                0
            };
            let block = McuBlock {
                x: 0,
                y: i * block_height,
                width: block_width,
                height: block_height,
                data: block_data,
            };
            on_block(block);
        }
        Ok(())
    }
}

impl Drop for MjpegDecoder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                jpeg_dec_close(self.handle);
            }
        }
    }
}

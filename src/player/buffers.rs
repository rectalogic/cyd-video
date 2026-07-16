extern crate alloc;
use alloc::vec::Vec;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

pub struct Buffers<const SIZE: usize> {
    buffers: Channel<CriticalSectionRawMutex, Buffer<SIZE>, SIZE>,
    recycle: Channel<CriticalSectionRawMutex, Buffer<SIZE>, SIZE>,
}

pub struct Buffer<const SIZE: usize> {
    pub data: Vec<u8>,
    recycle: &'static Channel<CriticalSectionRawMutex, Buffer<SIZE>, SIZE>,
}

impl<const SIZE: usize> Drop for Buffer<SIZE> {
    fn drop(&mut self) {
        let _ = self.recycle.try_send(Buffer {
            data: core::mem::take(&mut self.data),
            recycle: self.recycle,
        });
    }
}

impl<const SIZE: usize> Buffers<SIZE> {
    pub const fn new() -> Self {
        Self {
            buffers: Channel::new(),
            recycle: Channel::new(),
        }
    }

    pub fn init(&'static self) {
        for _ in 0..SIZE {
            let _ = self.recycle.try_send(Buffer {
                data: Vec::new(),
                recycle: &self.recycle,
            });
        }
    }

    pub async fn get_recycled(&self) -> Buffer<SIZE> {
        self.recycle.receive().await
    }

    pub async fn send(&self, buffer: Buffer<SIZE>) {
        self.buffers.send(buffer).await;
    }

    pub async fn receive(&self) -> Buffer<SIZE> {
        self.buffers.receive().await
    }
}

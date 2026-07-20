extern crate alloc;
use alloc::vec::Vec;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, TimeoutError, with_timeout};

pub struct Buffers<const SIZE: usize, B> {
    buffers: Channel<CriticalSectionRawMutex, B, SIZE>,
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

impl<const SIZE: usize, B: Send> Buffers<SIZE, B> {
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

    pub async fn send(&self, buffer: B) {
        self.buffers.send(buffer).await;
    }

    pub fn can_send(&self) -> bool {
        !self.buffers.is_full()
    }

    pub async fn receive(&self) -> B {
        self.buffers.receive().await
    }

    pub async fn receive_timeout(&self, timeout: Duration) -> Result<(), TimeoutError> {
        with_timeout(timeout, self.buffers.ready_to_receive()).await
    }
}

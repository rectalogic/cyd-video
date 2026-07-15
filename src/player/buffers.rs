extern crate alloc;
use alloc::vec::Vec;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

pub struct Buffers<const SIZE: usize> {
    buffers: Channel<CriticalSectionRawMutex, Vec<u8>, SIZE>,
    recycle: Channel<CriticalSectionRawMutex, Vec<u8>, SIZE>,
}

impl<const SIZE: usize> Buffers<SIZE> {
    pub const fn new() -> Self {
        Self {
            buffers: Channel::new(),
            recycle: Channel::new(),
        }
    }

    pub fn init(&self) {
        for _ in 0..SIZE {
            let _ = self.recycle.try_send(Vec::new());
        }
    }

    pub async fn get_recycled(&self) -> Vec<u8> {
        self.recycle.receive().await
    }

    pub async fn recycle(&self, buffer: Vec<u8>) {
        self.recycle.send(buffer).await;
    }

    pub async fn send(&self, buffer: Vec<u8>) {
        self.buffers.send(buffer).await;
    }

    pub async fn receive(&self) -> Vec<u8> {
        self.buffers.receive().await
    }
}

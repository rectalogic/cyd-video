#![cfg_attr(not(test), no_std)]
#[cfg(feature = "embed-video")]
mod cursor;
pub mod display;
pub mod error;
pub mod player;
pub mod sdcard;
pub mod touch;

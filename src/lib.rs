#![cfg_attr(not(test), no_std)]
mod display;
pub use display::{Display, DisplayError, DisplayPeripherals};

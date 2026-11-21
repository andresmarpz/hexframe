#![cfg(windows)]

pub mod aac;
pub mod audio;
mod client_area;
pub mod clock;
pub mod com;
pub mod crash;
pub mod d3d;
pub mod h264;
pub mod install;
pub mod instance;
mod pacer;
pub mod probe;
mod session;
pub mod vproc;
mod watchdog;
pub mod wgc;

pub use session::WindowsMediaSource;

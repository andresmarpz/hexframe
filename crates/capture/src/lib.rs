#![cfg(windows)]

pub mod aac;
pub mod adapters;
pub mod audio;
mod client_area;
pub mod clipboard;
pub mod clock;
pub mod com;
pub mod crash;
pub mod d3d;
pub mod displays;
mod frame_copy;
mod gpu_usage;
pub mod h264;
pub mod hotkey;
pub mod install;
pub mod instance;
mod pacer;
pub mod probe;
mod procstats;
pub mod recycle;
pub mod selftest;
mod session;
pub mod transcode;
pub mod video_log;
pub mod vproc;
mod watchdog;
pub mod wgc;
pub mod window;

pub use session::WindowsMediaSource;

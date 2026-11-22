#![cfg(windows)]

#[cfg(any(test, feature = "fault-injection"))]
macro_rules! fault {
    ($point:ident) => {
        $crate::fault::hit($crate::fault::Point::$point)
    };
}

#[cfg(not(any(test, feature = "fault-injection")))]
macro_rules! fault {
    ($point:ident) => {
        Ok::<(), ::windows::core::Error>(())
    };
}

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
#[cfg(any(test, feature = "fault-injection"))]
pub mod fault;
mod frame_copy;
mod gpu_usage;
pub mod h264;
pub mod hotkey;
pub mod install;
pub mod instance;
mod pacer;
pub mod probe;
pub mod procstats;
pub mod recycle;
pub mod selftest;
mod session;
#[cfg(any(test, feature = "fault-injection"))]
pub mod testing;
pub mod transcode;
pub mod video_log;
pub mod vproc;
pub mod wgc;
pub mod window;

pub use session::WindowsMediaSource;

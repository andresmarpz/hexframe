#[cfg(not(windows))]
mod stub;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub(crate) use greyscreen_capture::install;
#[cfg(not(windows))]
pub(crate) use stub::*;
#[cfg(windows)]
pub(crate) use win::*;

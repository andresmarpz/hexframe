#![forbid(unsafe_code)]

pub mod aac;
pub mod avc;
pub mod bytes;
mod clip;
pub mod finalize;
pub mod fragmented;
pub mod moov;
pub mod reader;
mod recover;

#[derive(Debug, thiserror::Error)]
pub enum Mp4Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Mp4Error>;

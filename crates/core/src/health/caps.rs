#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUnit {
    Bytes,
    Bits,
}

pub fn buffer_size(peak_bitrate: u32, unit: BufferUnit) -> u32 {
    match unit {
        BufferUnit::Bytes => peak_bitrate / 8,
        BufferUnit::Bits => peak_bitrate,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapMismatch {
    pub property: &'static str,
    pub set: u32,
    pub read_back: Option<u32>,
}

pub fn cap_mismatch(
    property: &'static str,
    set: u32,
    read_back: Option<u32>,
) -> Option<CapMismatch> {
    (read_back != Some(set)).then_some(CapMismatch {
        property,
        set,
        read_back,
    })
}

pub fn cap_warning(encoder: &str, mismatch: &CapMismatch) -> String {
    let read = mismatch
        .read_back
        .map_or_else(|| "nothing readable".to_owned(), |value| value.to_string());
    format!(
        "{encoder} did not keep {}: set {}, read back {read}. The bitrate cap may not apply.",
        mismatch.property, mismatch.set
    )
}

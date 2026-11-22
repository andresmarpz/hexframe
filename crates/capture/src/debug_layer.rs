use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::Win32::Graphics::Direct3D11::{
    D3D11_MESSAGE, D3D11_RLDO_DETAIL, D3D11_RLDO_IGNORE_INTERNAL, ID3D11Debug, ID3D11Device,
    ID3D11InfoQueue,
};
use windows::core::Interface;

static ENABLED: AtomicBool = AtomicBool::new(false);
static DEVICE_REMOVED: AtomicU32 = AtomicU32::new(0);
static REPORTS: Mutex<Vec<LiveObjectsReport>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveObjectsReport {
    pub count: u32,
    pub lines: Vec<String>,
}

pub fn enable() {
    ENABLED.store(true, Ordering::Release);
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

pub(crate) fn record_device_removed() {
    DEVICE_REMOVED.fetch_add(1, Ordering::AcqRel);
}

pub fn device_removed_count() -> u32 {
    DEVICE_REMOVED.load(Ordering::Acquire)
}

pub fn take_live_reports() -> Vec<LiveObjectsReport> {
    std::mem::take(&mut *REPORTS.lock().unwrap_or_else(|e| e.into_inner()))
}

pub(crate) fn attach(device: &ID3D11Device) -> Option<ID3D11Debug> {
    if !enabled() {
        return None;
    }
    device.cast::<ID3D11Debug>().ok()
}

pub(crate) fn report_live(debug: &ID3D11Debug) {
    let Ok(queue) = debug.cast::<ID3D11InfoQueue>() else {
        tracing::warn!("the debug layer has no info queue, live objects not reported");
        return;
    };
    // SAFETY: the queue is a live interface on the debug device and the call takes no arguments.
    unsafe { queue.ClearStoredMessages() };
    // SAFETY: the debug interface is live and the call only takes flags.
    if let Err(error) =
        unsafe { debug.ReportLiveDeviceObjects(D3D11_RLDO_DETAIL | D3D11_RLDO_IGNORE_INTERNAL) }
    {
        tracing::warn!(%error, "live object report failed");
        return;
    }
    let report = live_report(read_messages(&queue));
    tracing::info!(count = report.count, "d3d live objects at session end");
    REPORTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(report);
}

fn live_report(messages: Vec<String>) -> LiveObjectsReport {
    let lines: Vec<String> = messages
        .into_iter()
        .filter(|text| text.trim_start().starts_with("Live "))
        .collect();
    LiveObjectsReport {
        count: lines.len() as u32,
        lines,
    }
}

fn read_messages(queue: &ID3D11InfoQueue) -> Vec<String> {
    // SAFETY: the queue is live and the call only reads a counter.
    let stored = unsafe { queue.GetNumStoredMessages() };
    (0..stored)
        .filter_map(|index| read_message(queue, index))
        .collect()
}

fn read_message(queue: &ID3D11InfoQueue, index: u64) -> Option<String> {
    let mut bytes = 0usize;
    // SAFETY: a null message pointer asks only for the byte length, written to the local.
    unsafe { queue.GetMessage(index, None, &mut bytes) }.ok()?;
    let mut buffer = vec![0u64; bytes.div_ceil(std::mem::size_of::<u64>())];
    let message = buffer.as_mut_ptr().cast::<D3D11_MESSAGE>();
    // SAFETY: the buffer holds at least `bytes` bytes and is aligned for D3D11_MESSAGE.
    unsafe { queue.GetMessage(index, Some(message), &mut bytes) }.ok()?;
    // SAFETY: GetMessage filled the struct, and its description lives in the same buffer.
    let (text, len) = unsafe { ((*message).pDescription, (*message).DescriptionByteLength) };
    if text.is_null() {
        return None;
    }
    // SAFETY: the description points into the buffer for `len` bytes, including the final null.
    let raw = unsafe { std::slice::from_raw_parts(text, len) };
    Some(
        String::from_utf8_lossy(raw)
            .trim_end_matches('\0')
            .to_owned(),
    )
}

#[cfg(test)]
mod tests;

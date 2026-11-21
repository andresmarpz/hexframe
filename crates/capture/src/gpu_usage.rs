use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Sender, bounded};
use greyscreen_core::health::gpu::{GpuUsage, summarize};
use windows::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{PCWSTR, w};

const PERIOD: Duration = Duration::from_secs(1);
const PDH_OK: u32 = 0;

#[derive(Debug)]
pub(crate) struct GpuUsageSampler {
    latest: Arc<Mutex<Option<GpuUsage>>>,
    _stop: Sender<()>,
}

impl GpuUsageSampler {
    pub(crate) fn start(luid: Option<u64>) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let (stop, stopped) = bounded::<()>(0);
        let shared = Arc::clone(&latest);
        let spawned = std::thread::Builder::new()
            .name("gpu-usage".into())
            .spawn(move || run(luid, &shared, &stopped));
        if let Err(error) = spawned {
            tracing::warn!(%error, "GPU usage sampling is off: thread did not start");
        }
        Self {
            latest,
            _stop: stop,
        }
    }

    pub(crate) fn latest(&self) -> Option<GpuUsage> {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner())
    }
}

struct Query {
    handle: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl Query {
    fn open() -> Result<Self, u32> {
        let mut handle = PDH_HQUERY::default();
        // SAFETY: a null data source selects live data and `handle` is a live out pointer.
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut handle) };
        if status != PDH_OK {
            return Err(status);
        }
        let mut counter = PDH_HCOUNTER::default();
        // SAFETY: `handle` is an open query, the path is a static wide string and `counter` is a live out pointer.
        let status = unsafe {
            PdhAddEnglishCounterW(
                handle,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut counter,
            )
        };
        if status != PDH_OK {
            // SAFETY: `handle` was opened above and is closed once.
            unsafe { PdhCloseQuery(handle) };
            return Err(status);
        }
        Ok(Self { handle, counter })
    }

    fn collect(&self) -> bool {
        // SAFETY: `handle` is an open query.
        unsafe { PdhCollectQueryData(self.handle) == PDH_OK }
    }

    fn read(&self) -> Option<Vec<(String, f64)>> {
        let mut bytes = 0u32;
        let mut count = 0u32;
        // SAFETY: a null buffer with a zero size only asks for the needed size.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
        };
        if status != PDH_MORE_DATA || bytes == 0 {
            return None;
        }
        let mut buffer = vec![0u64; (bytes as usize).div_ceil(8)];
        let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        // SAFETY: the buffer is at least `bytes` long and 8-byte aligned, as the item array needs.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(items),
            )
        };
        if status != PDH_OK {
            return None;
        }
        let mut out = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            // SAFETY: PDH filled `count` items at the start of the buffer, and the name pointers refer into it.
            let item = unsafe { &*items.add(index) };
            let status = item.FmtValue.CStatus;
            if status != PDH_CSTATUS_VALID_DATA && status != PDH_CSTATUS_NEW_DATA {
                continue;
            }
            // SAFETY: the name is a nul-terminated wide string inside the buffer.
            let Ok(name) = (unsafe { item.szName.to_string() }) else {
                continue;
            };
            // SAFETY: PDH_FMT_DOUBLE was requested, so the double member is the one set.
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            out.push((name, value));
        }
        Some(out)
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        // SAFETY: `handle` is an open query and is closed once.
        unsafe { PdhCloseQuery(self.handle) };
    }
}

fn run(
    luid: Option<u64>,
    latest: &Mutex<Option<GpuUsage>>,
    stopped: &crossbeam_channel::Receiver<()>,
) {
    let query = match Query::open() {
        Ok(query) => query,
        Err(status) => {
            tracing::warn!(
                status = format_args!("{status:#x}"),
                "GPU usage sampling is off: the counter is not available"
            );
            return;
        }
    };
    if !query.collect() {
        tracing::warn!("GPU usage sampling is off: the first collection failed");
        return;
    }
    let own_pid = std::process::id();
    loop {
        if !matches!(
            stopped.recv_timeout(PERIOD),
            Err(crossbeam_channel::RecvTimeoutError::Timeout)
        ) {
            return;
        }
        if !query.collect() {
            continue;
        }
        let Some(samples) = query.read() else {
            continue;
        };
        let usage = summarize(
            samples.iter().map(|(name, value)| (name.as_str(), *value)),
            luid,
            own_pid,
        );
        *latest.lock().unwrap_or_else(|e| e.into_inner()) = Some(usage);
    }
}

#[cfg(test)]
mod tests;

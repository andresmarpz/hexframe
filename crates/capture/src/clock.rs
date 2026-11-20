use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub fn qpc_now_hns() -> Hns {
    let mut counter = 0i64;
    let mut frequency = 0i64;
    // SAFETY: both pointers refer to live local i64 values.
    let ok = unsafe {
        QueryPerformanceCounter(&mut counter).is_ok()
            && QueryPerformanceFrequency(&mut frequency).is_ok()
    };
    if !ok || frequency <= 0 {
        return 0;
    }
    (i128::from(counter) * i128::from(HNS_PER_SEC) / i128::from(frequency)) as Hns
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QpcMap {
    pub offset: Hns,
}

impl QpcMap {
    pub fn now() -> Self {
        Self {
            offset: qpc_now_hns() - mono_now(),
        }
    }

    pub fn to_mono(&self, qpc_hns: Hns) -> Hns {
        qpc_hns - self.offset
    }
}

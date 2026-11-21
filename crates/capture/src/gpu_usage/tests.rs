use std::time::Duration;

use super::GpuUsageSampler;
use crate::d3d::{GpuDevice, luid_u64};

#[test]
#[ignore = "needs a Windows PC with a GPU"]
fn live_sampler_publishes_usage_for_the_primary_adapter() {
    let gpu = GpuDevice::for_primary_monitor().expect("primary adapter");
    let luid = luid_u64(gpu.adapter_luid);
    let sampler = GpuUsageSampler::start(Some(luid));
    std::thread::sleep(Duration::from_secs(3));
    let usage = sampler.latest().expect("a published value");
    println!("adapter {} luid {luid:#x}: {usage:?}", gpu.adapter_name);
    assert!((0.0..=100.0).contains(&usage.busy_3d_pct));
    assert!((0.0..=100.0).contains(&usage.self_3d_pct));
    assert!((0.0..=100.0).contains(&usage.encode_pct));
    assert!((0.0..=100.0).contains(&usage.copy_pct));
}

use std::sync::OnceLock;
use std::time::Instant;

pub type Hns = i64;

pub const HNS_PER_SEC: i64 = 10_000_000;

pub fn mono_now() -> Hns {
    static BASE: OnceLock<Instant> = OnceLock::new();
    let base = BASE.get_or_init(Instant::now);
    (base.elapsed().as_nanos() / 100) as Hns
}

pub fn hns_to_secs(h: Hns) -> f64 {
    h as f64 / HNS_PER_SEC as f64
}

pub fn secs_to_hns(s: f64) -> Hns {
    (s * HNS_PER_SEC as f64).round() as Hns
}

#[cfg(test)]
mod tests;

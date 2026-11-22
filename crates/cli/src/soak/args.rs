use std::path::PathBuf;

use anyhow::{Result, bail};

use super::check::{AvLimits, Limits};
use super::profiles::{WindowPlan, WindowTarget};

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct Args {
    #[arg(
        long,
        conflicts_with_all = ["cycles", "seconds", "gap_secs", "game_minutes", "warmup_secs", "warmup_cycles", "max_mem_mb_per_min", "max_gpu_mb_per_min", "max_handle_growth", "max_thread_growth"]
    )]
    pub(crate) profile: Option<String>,
    #[arg(long)]
    pub(crate) list_profiles: bool,
    #[arg(long, value_enum, default_value = "league")]
    pub(crate) window: WindowTarget,
    #[arg(long, default_value_t = 8)]
    pub(crate) cycles: u32,
    #[arg(long, default_value_t = 30)]
    pub(crate) seconds: u32,
    #[arg(long, default_value_t = 10)]
    pub(crate) gap_secs: u32,
    #[arg(long, value_name = "MINUTES")]
    pub(crate) game_minutes: Option<u32>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) dir: Option<PathBuf>,
    #[arg(long)]
    pub(crate) keep: bool,
    #[arg(long)]
    pub(crate) d3d_debug: bool,
    #[arg(long, default_value_t = 60.0)]
    pub(crate) warmup_secs: f64,
    #[arg(long, default_value_t = 2)]
    pub(crate) warmup_cycles: u32,
    #[arg(long, default_value_t = 2.0, value_name = "MB")]
    pub(crate) max_mem_mb_per_min: f64,
    #[arg(long, default_value_t = 2.0, value_name = "MB")]
    pub(crate) max_gpu_mb_per_min: f64,
    #[arg(long, default_value_t = 50)]
    pub(crate) max_handle_growth: u32,
    #[arg(long, default_value_t = 4)]
    pub(crate) max_thread_growth: u32,
    #[arg(skip)]
    pub(crate) max_dropped_pct: Option<f64>,
    #[arg(skip)]
    pub(crate) encoder_contains: Option<&'static str>,
    #[arg(skip)]
    pub(crate) min_video_secs: Option<f64>,
    #[arg(skip)]
    pub(crate) until_saved: bool,
    #[arg(skip)]
    pub(crate) av_limits: Option<AvLimits>,
    #[arg(skip)]
    pub(crate) random_secs: Option<(u32, u32)>,
    #[arg(skip)]
    pub(crate) failed_clip_saves: bool,
    #[arg(skip)]
    pub(crate) window_plan: Option<WindowPlan>,
}

impl Args {
    pub(super) fn out_path(&self) -> PathBuf {
        self.out
            .clone()
            .unwrap_or_else(|| PathBuf::from("greyscreen-soak.json"))
    }

    pub(super) fn limits(&self) -> Limits {
        Limits {
            mem_mb_per_min: self.max_mem_mb_per_min,
            gpu_mb_per_min: self.max_gpu_mb_per_min,
            handle_growth: self.max_handle_growth,
            thread_growth: self.max_thread_growth,
            warmup_secs: self.warmup_secs,
            warmup_cycles: self.warmup_cycles,
            max_dropped_pct: self.max_dropped_pct,
            encoder_contains: self.encoder_contains,
            av: self.av_limits,
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.game_minutes.is_none() && (self.cycles == 0 || self.seconds == 0) {
            bail!("--cycles and --seconds must be at least 1");
        }
        if self.game_minutes == Some(0) {
            bail!("--game-minutes must be at least 1");
        }
        if self.game_minutes.is_some() && self.window == WindowTarget::Cloaked {
            bail!("the game runs record League; use --window league");
        }
        Ok(())
    }
}

use anyhow::{Result, bail};

use super::check::AvLimits;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Shape {
    Cycles {
        cycles: u32,
        seconds: u32,
        gap_secs: u32,
    },
    RandomCycles {
        cycles: u32,
        min_secs: u32,
        max_secs: u32,
    },
    Games {
        minutes: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum WindowTarget {
    League,
    Cloaked,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProfileLimits {
    pub(crate) warmup_secs: f64,
    pub(crate) warmup_cycles: u32,
    pub(crate) max_mem_mb_per_min: f64,
    pub(crate) max_gpu_mb_per_min: f64,
    pub(crate) max_handle_growth: u32,
    pub(crate) max_thread_growth: u32,
    pub(crate) max_dropped_pct: Option<f64>,
    pub(crate) encoder_contains: Option<&'static str>,
    pub(crate) av: Option<AvLimits>,
}

const AV_LIMITS: AvLimits = AvLimits {
    max_skew_ms: 40.0,
    max_drift_ms_per_hour: 1.0,
};

const DEFAULT_LIMITS: ProfileLimits = ProfileLimits {
    warmup_secs: 60.0,
    warmup_cycles: 2,
    max_mem_mb_per_min: 2.0,
    max_gpu_mb_per_min: 2.0,
    max_handle_growth: 50,
    max_thread_growth: 4,
    max_dropped_pct: None,
    encoder_contains: None,
    av: None,
};

const SOFTWARE_ENCODER: &str = "Microsoft H.264 Encoder MFT";

pub(crate) struct Profile {
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) setup: &'static str,
    pub(crate) shape: Shape,
    pub(crate) limits: ProfileLimits,
    pub(crate) uses_window: bool,
    pub(crate) min_video_secs: Option<f64>,
    pub(crate) until_saved: bool,
}

const fn cycles(cycles: u32, seconds: u32, gap_secs: u32) -> Shape {
    Shape::Cycles {
        cycles,
        seconds,
        gap_secs,
    }
}

pub(crate) const PROFILES: &[Profile] = &[
    Profile {
        name: "smoke",
        summary: "3 test recordings of 10 s through the engine",
        setup: "none",
        shape: cycles(3, 10, 3),
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "capture-cycles-200",
        summary: "200 start and stop cycles while test frames arrive",
        setup: "none",
        shape: cycles(200, 5, 2),
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "encoder-sessions-50",
        summary: "50 encoder sessions of 15 s: memory, GPU memory and handles stay flat",
        setup: "run once with the D3D debug layer on and read the live objects after",
        shape: cycles(50, 15, 2),
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "encoder-opens-1000",
        summary: "1000 encoder opens of 2 s: handles and memory stay flat",
        setup: "run once with the D3D debug layer on and read the live objects after",
        shape: cycles(1000, 2, 0),
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "cycles-random-1000",
        summary: "1000 start and stop cycles with random 1 to 8 s runs and random gaps",
        setup: "none; the report notes the random seed",
        shape: Shape::RandomCycles {
            cycles: 1000,
            min_secs: 1,
            max_secs: 8,
        },
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "game-30",
        summary: "30 minutes of League recording, back to back games",
        setup: "a League session Andres has freed",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "game-minimised-30",
        summary: "30 minutes with the game minimised",
        setup: "a League session Andres has freed; minimise the game after the recording starts",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "game-hags-30",
        summary: "30 minutes with hardware-accelerated GPU scheduling on",
        setup: "a League session Andres has freed; turn HAGS on in Windows graphics settings",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "game-software-30",
        summary: "30 minutes of software encoding with flat allocations",
        setup: "a League session Andres has freed, on a PC where the hardware encoder is missing or fails to open; the run fails when another encoder ran",
        shape: Shape::Games { minutes: 30 },
        limits: ProfileLimits {
            encoder_contains: Some(SOFTWARE_ENCODER),
            ..DEFAULT_LIMITS
        },
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "game-hdd-40",
        summary: "40 minutes to a slow drive with no drops",
        setup: "a League session Andres has freed; pass --dir on an HDD or USB stick; the run fails when dropped_pct is above 0",
        shape: Shape::Games { minutes: 40 },
        limits: ProfileLimits {
            max_dropped_pct: Some(0.0),
            ..DEFAULT_LIMITS
        },
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "av-60",
        summary: "60 minutes of game process loopback",
        setup: "a League session Andres has freed; A/V skew is in the per-second diagnostics and drift, discontinuities and gaps are in cycles[].health.audio",
        shape: Shape::Games { minutes: 60 },
        limits: ProfileLimits {
            av: Some(AV_LIMITS),
            ..DEFAULT_LIMITS
        },
        uses_window: true,
        min_video_secs: None,
        until_saved: false,
    },
    Profile {
        name: "av-180",
        summary: "3 hours of loopback, one recording up to the engine's 3-hour limit",
        setup: "a League session Andres has freed for three hours, game started in the first 20 minutes; the run ends at the first saved recording and fails when it is shorter than 10700 s",
        shape: Shape::Games { minutes: 200 },
        limits: ProfileLimits {
            av: Some(AV_LIMITS),
            ..DEFAULT_LIMITS
        },
        uses_window: true,
        min_video_secs: Some(10_700.0),
        until_saved: true,
    },
    Profile {
        name: "long-8h",
        summary: "back-to-back test recordings for 8 hours on the primary monitor",
        setup: "only when Andres frees the PC overnight; no one needs to play; the leak check has few checkpoints, so read the report",
        shape: cycles(3, 9_600, 10),
        limits: DEFAULT_LIMITS,
        uses_window: false,
        min_video_secs: None,
        until_saved: false,
    },
];

impl super::Args {
    pub(super) fn resolve(&self) -> Result<Self> {
        let Some(name) = &self.profile else {
            return Ok(self.clone());
        };
        let profile = find(name)?;
        if profile.uses_window && self.window == WindowTarget::Cloaked {
            bail!("the cloaked test window is not built yet; use --window league");
        }
        let mut args = self.clone();
        match profile.shape {
            Shape::Cycles {
                cycles,
                seconds,
                gap_secs,
            } => {
                args.cycles = cycles;
                args.seconds = seconds;
                args.gap_secs = gap_secs;
                args.game_minutes = None;
            }
            Shape::RandomCycles {
                cycles,
                min_secs,
                max_secs,
            } => {
                args.cycles = cycles;
                args.seconds = max_secs;
                args.gap_secs = 0;
                args.random_secs = Some((min_secs, max_secs));
                args.game_minutes = None;
            }
            Shape::Games { minutes } => args.game_minutes = Some(minutes),
        }
        let limits = profile.limits;
        args.warmup_secs = limits.warmup_secs;
        args.warmup_cycles = limits.warmup_cycles;
        args.max_mem_mb_per_min = limits.max_mem_mb_per_min;
        args.max_gpu_mb_per_min = limits.max_gpu_mb_per_min;
        args.max_handle_growth = limits.max_handle_growth;
        args.max_thread_growth = limits.max_thread_growth;
        args.max_dropped_pct = limits.max_dropped_pct;
        args.encoder_contains = limits.encoder_contains;
        args.av_limits = limits.av;
        args.min_video_secs = profile.min_video_secs;
        args.until_saved = profile.until_saved;
        if args.out.is_none() {
            args.out = Some(format!("greyscreen-soak-{name}.json").into());
        }
        Ok(args)
    }
}

pub(crate) fn find(name: &str) -> Result<&'static Profile> {
    match PROFILES.iter().find(|p| p.name == name) {
        Some(profile) => Ok(profile),
        None => bail!(
            "unknown profile {name:?}; known: {}",
            PROFILES
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(crate) fn describe() -> String {
    PROFILES
        .iter()
        .map(|p| {
            let shape = match p.shape {
                Shape::Cycles {
                    cycles, seconds, ..
                } => format!("{cycles} cycles of {seconds} s"),
                Shape::RandomCycles {
                    cycles,
                    min_secs,
                    max_secs,
                } => format!("{cycles} random cycles of {min_secs} to {max_secs} s"),
                Shape::Games { minutes } => format!("games for {minutes} min"),
            };
            format!(
                "{:<22} {shape}: {}\n  setup: {}",
                p.name, p.summary, p.setup
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

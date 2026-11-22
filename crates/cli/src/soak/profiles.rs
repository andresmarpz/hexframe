use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Shape {
    Cycles {
        cycles: u32,
        seconds: u32,
        gap_secs: u32,
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
}

const DEFAULT_LIMITS: ProfileLimits = ProfileLimits {
    warmup_secs: 60.0,
    warmup_cycles: 2,
    max_mem_mb_per_min: 2.0,
    max_gpu_mb_per_min: 2.0,
    max_handle_growth: 50,
    max_thread_growth: 4,
};

const LONG_LIMITS: ProfileLimits = ProfileLimits {
    warmup_secs: 300.0,
    ..DEFAULT_LIMITS
};

pub(crate) struct Profile {
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) setup: &'static str,
    pub(crate) shape: Shape,
    pub(crate) limits: ProfileLimits,
    pub(crate) uses_window: bool,
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
    },
    Profile {
        name: "capture-cycles-200",
        summary: "200 start and stop cycles while test frames arrive",
        setup: "none",
        shape: cycles(200, 5, 2),
        limits: DEFAULT_LIMITS,
        uses_window: false,
    },
    Profile {
        name: "encoder-sessions-50",
        summary: "50 encoder sessions of 15 s: memory, GPU memory and handles stay flat",
        setup: "run once with the D3D debug layer on and read the live objects after",
        shape: cycles(50, 15, 2),
        limits: DEFAULT_LIMITS,
        uses_window: false,
    },
    Profile {
        name: "game-30",
        summary: "30 minutes of League recording, back to back games",
        setup: "a League session Andres has freed",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "game-minimised-30",
        summary: "30 minutes with the game minimised",
        setup: "a League session Andres has freed; minimise the game after the recording starts",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "game-hags-30",
        summary: "30 minutes with hardware-accelerated GPU scheduling on",
        setup: "a League session Andres has freed; turn HAGS on in Windows graphics settings",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "game-software-30",
        summary: "30 minutes of software encoding with flat allocations",
        setup: "a League session Andres has freed, on a PC or iGPU where only the software encoder runs",
        shape: Shape::Games { minutes: 30 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "game-hdd-40",
        summary: "40 minutes to a slow drive with no drops",
        setup: "a League session Andres has freed; pass --dir on an HDD or USB stick; dropped_pct in the report must stay 0",
        shape: Shape::Games { minutes: 40 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "av-60",
        summary: "60 minutes of game process loopback",
        setup: "a League session Andres has freed; A/V skew is read from the per-second diagnostics in the report",
        shape: Shape::Games { minutes: 60 },
        limits: DEFAULT_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "av-180",
        summary: "3 hours of loopback, one recording up to the engine's 3-hour limit",
        setup: "a League session Andres has freed for three hours; the recording stops and finalises at the limit",
        shape: Shape::Games { minutes: 190 },
        limits: LONG_LIMITS,
        uses_window: true,
    },
    Profile {
        name: "long-8h",
        summary: "back-to-back recordings for 8 hours",
        setup: "only when Andres frees the PC overnight",
        shape: Shape::Games { minutes: 480 },
        limits: LONG_LIMITS,
        uses_window: true,
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
            Shape::Games { minutes } => args.game_minutes = Some(minutes),
        }
        let limits = profile.limits;
        args.warmup_secs = limits.warmup_secs;
        args.warmup_cycles = limits.warmup_cycles;
        args.max_mem_mb_per_min = limits.max_mem_mb_per_min;
        args.max_gpu_mb_per_min = limits.max_gpu_mb_per_min;
        args.max_handle_growth = limits.max_handle_growth;
        args.max_thread_growth = limits.max_thread_growth;
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

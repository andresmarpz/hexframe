use greyscreen_core::problem::Problem;
use greyscreen_engine::EngineStatus;
use rune::Color;
use rune_kit::Kit;

use crate::ui::{Ui, format_clock};

#[derive(Clone, Copy)]
pub(super) enum Dot {
    Solid(Color),
    Hollow(Color),
}

pub(super) struct Face {
    pub dot: Dot,
    pub glow: Option<Color>,
    pub title: String,
    pub title_color: Color,
    pub subtitle: String,
    pub tabular: bool,
}

impl Face {
    pub(super) fn of(s: &Ui, k: &Kit) -> Face {
        let face = |dot, glow, title: &str, title_color, subtitle: &str| Face {
            dot,
            glow,
            title: title.to_owned(),
            title_color,
            subtitle: subtitle.to_owned(),
            tabular: false,
        };
        let armed = s.settings.auto_record;
        match &s.status {
            EngineStatus::Idle if armed => face(
                Dot::Solid(k.c.success.fg),
                Some(k.c.success.fg),
                "Ready",
                k.c.text_base,
                "Next game will be recorded",
            ),
            EngineStatus::Idle => paused(k),
            EngineStatus::GameRunning => face(
                Dot::Hollow(k.c.text_faint),
                None,
                "Game in progress",
                k.c.text_faint,
                "Not recording",
            ),
            EngineStatus::Waiting => face(
                Dot::Solid(k.c.warning.fg),
                None,
                "League is running",
                k.c.text_base,
                "Waiting for the game to load",
            ),
            EngineStatus::Finalizing => face(
                Dot::Solid(k.c.warning.fg),
                None,
                "Saving...",
                k.c.text_base,
                "Finishing the video file",
            ),
            EngineStatus::Recording { .. } => Face {
                tabular: true,
                ..face(
                    Dot::Solid(k.c.danger.fg),
                    Some(k.c.danger.fg),
                    "Recording",
                    k.c.text_base,
                    &format_clock(s.elapsed_secs),
                )
            },
            EngineStatus::Error(message) => {
                let problem = Problem::from_error(message);
                face(
                    Dot::Solid(k.c.danger.fg),
                    None,
                    problem.title(),
                    k.c.danger.fg,
                    problem.action(),
                )
            }
        }
    }
}

fn paused(k: &Kit) -> Face {
    Face {
        dot: Dot::Hollow(k.c.text_faint),
        glow: None,
        title: "Paused".to_owned(),
        title_color: k.c.text_faint,
        subtitle: "Next game won't be recorded".to_owned(),
        tabular: false,
    }
}

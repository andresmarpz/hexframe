use greyscreen_core::settings::Settings;

use super::{FeedKind, FeedRow, PreviewStatus, RecRow, ResultKind, Ui};

pub(super) fn ui() -> Ui {
    let mut ui = Ui::blank(Settings::default());
    ui.status = PreviewStatus::Recording {
        elapsed_secs: 12 * 60 + 34,
        champion: "Ahri".into(),
        mode: "CLASSIC".into(),
    };
    ui.feed = feed();
    ui.recordings = recordings();
    ui
}

fn feed() -> Vec<FeedRow> {
    use FeedKind::*;
    let rows = [
        (95, FirstBlood, "You took First Blood on Zed"),
        (214, Kill, "You killed Zed"),
        (251, Turret, "Your team destroyed the outer mid turret"),
        (366, Dragon, "Your team slew the Infernal Drake"),
        (402, Death, "You were slain by Lee Sin"),
        (455, Assist, "You assisted on Jinx killing Caitlyn"),
        (518, Herald, "Your team took the Rift Herald"),
        (587, Kill, "You killed Lux"),
        (601, Multikill, "Double Kill: Lux and Leona"),
        (655, Inhibitor, "Your team destroyed the top inhibitor"),
        (702, Baron, "Your team slew Baron Nashor"),
        (754, Ace, "Your team scored an Ace"),
    ];
    rows.into_iter()
        .zip(1u32..)
        .map(|((game_secs, kind, text), id)| FeedRow {
            id,
            game_secs,
            kind,
            text: text.into(),
        })
        .collect()
}

fn recordings() -> Vec<RecRow> {
    use ResultKind::*;
    let rows = [
        (
            "2026-11-20 19:55",
            "Ahri",
            "CLASSIC",
            Win,
            33 * 60 + 12,
            1_820,
        ),
        (
            "2026-11-20 18:41",
            "Ahri",
            "CLASSIC",
            Loss,
            28 * 60 + 47,
            1_540,
        ),
        ("2026-11-19 22:10", "Yasuo", "ARAM", Win, 19 * 60 + 5, 1_010),
        (
            "2026-11-19 20:32",
            "Lux",
            "CLASSIC",
            Win,
            36 * 60 + 40,
            1_960,
        ),
        (
            "2026-11-18 21:15",
            "Jinx",
            "CLASSIC",
            Loss,
            41 * 60 + 3,
            2_240,
        ),
        ("2026-11-17 23:02", "Ahri", "ARAM", Win, 17 * 60 + 52, 940),
        (
            "2026-11-16 19:20",
            "Viego",
            "CLASSIC",
            Unknown,
            22 * 60 + 18,
            1_190,
        ),
        (
            "2026-11-15 20:48",
            "Thresh",
            "CLASSIC",
            Loss,
            31 * 60 + 29,
            1_680,
        ),
    ];
    rows.into_iter()
        .map(
            |(started, champion, mode, result, duration_secs, size_mb)| RecRow {
                id: format!("{started}_{champion}_{mode}"),
                started: started.into(),
                champion: champion.into(),
                mode: mode.into(),
                result,
                duration_secs,
                size_bytes: size_mb * 1_000_000,
            },
        )
        .collect()
}

use greyscreen_core::game::GameResult;
use greyscreen_core::highlight::Tone;
use rune::{Cursor, El, Side, col, el, row};
use rune_kit::{Kit, SidebarRow, SidebarSection};

use crate::ui::library::format::{ago, kda, recent_games};
use crate::ui::{RECENT_LIMIT, Screen, Ui};
use crate::view::champion::champion_icon;
use crate::view::widgets::tone_color;

const AVATAR_SIZE: f32 = 16.0;
const ROW_HEIGHT: f32 = 44.0;
const TICK_WIDTH: f32 = 3.0;
const TICK_HEIGHT: f32 = 20.0;
const CELL_HEIGHT: f32 = 42.0;
const CELL_TARGET: f32 = 40.0;
const CELL_ICON: f32 = 28.0;

struct Game {
    id: String,
    champion: String,
    result: GameResult,
    score: String,
    ago: String,
    active: bool,
}

impl Game {
    fn result_word(&self) -> &'static str {
        match self.result {
            GameResult::Win => "Victory",
            GameResult::Loss => "Defeat",
            GameResult::Unknown => "Result unknown",
        }
    }

    fn tick(&self, k: &Kit) -> El {
        let tone = match self.result {
            GameResult::Win => Tone::Team,
            GameResult::Loss => Tone::Death,
            GameResult::Unknown => Tone::Neutral,
        };
        el().w(TICK_WIDTH)
            .h(TICK_HEIGHT)
            .radius(TICK_WIDTH / 2.0)
            .fill(tone_color(k, tone))
            .a11y_hidden()
    }
}

fn games(s: &Ui) -> Vec<Game> {
    let now = crate::qa::now();
    let open = s.selected_id().filter(|_| s.screen == Screen::Games);
    recent_games(&s.recordings, RECENT_LIMIT)
        .into_iter()
        .map(|rec| {
            let me = &rec.meta.game.info.me;
            let figures = kda(&rec.meta.events, &rec.meta.game.info);
            Game {
                id: rec.meta.id.clone(),
                champion: me.champion.clone(),
                result: rec.meta.game.result,
                score: format!(
                    "{} / {} / {}",
                    figures.kills, figures.deaths, figures.assists
                ),
                ago: ago(&rec.meta.started_at, &now),
                active: open == Some(rec.meta.id.as_str()),
            }
        })
        .collect()
}

fn list_row(game: &Game, k: &Kit) -> El {
    let id = game.id.clone();
    SidebarRow::new(game.champion.clone())
        .avatar(champion_icon(k, &game.champion, AVATAR_SIZE))
        .subtitle(format!("{} · {}", game.result_word(), game.score))
        .subtitle_color(k.c.text_muted)
        .detail(game.ago.clone())
        .trailing(game.tick(k))
        .height(ROW_HEIGHT)
        .selected(game.active)
        .on_select(move |ui: &mut Ui, _| ui.open_review(id.clone()))
        .build(k)
        .key(format!("nav-game-{}", game.id))
}

pub(super) fn section(s: &Ui, k: &Kit) -> Option<El> {
    let games = games(s);
    if games.is_empty() {
        return None;
    }
    Some(
        SidebarSection::new("Recent")
            .key("nav-recent")
            .rows(games.iter().map(|game| list_row(game, k)))
            .build(k),
    )
}

fn rail_cell(game: &Game, k: &Kit) -> El {
    let id = game.id.clone();
    let tip = format!(
        "{} · {} {} · {} ago",
        game.champion,
        game.result_word(),
        game.score,
        game.ago
    );
    let mut target = row()
        .key(format!("nav-game-{}", game.id))
        .role("button")
        .label(format!("Review {} game, {tip}", game.champion))
        .help(tip)
        .help_side(Side::Right)
        .focusable(true)
        .size(CELL_TARGET, CELL_TARGET)
        .center()
        .radius(k.radius.md)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_review(id.clone()))
        .child(champion_icon(k, &game.champion, CELL_ICON));
    if game.active {
        target = target.fill(k.c.pressed);
    }
    row().w_full().h(CELL_HEIGHT).center().child(target).child(
        game.tick(k)
            .absolute(0.0, (CELL_HEIGHT - TICK_HEIGHT) / 2.0),
    )
}

pub(super) fn rail(s: &Ui, k: &Kit) -> El {
    col()
        .key("nav-recent")
        .w_full()
        .gap(2.0)
        .children(games(s).iter().map(|game| rail_cell(game, k)))
}

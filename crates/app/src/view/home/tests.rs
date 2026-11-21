use greyscreen_core::highlight::{Highlight, Objective, Tone};
use greyscreen_engine::FeedItem;

use super::feed::badge_title;

fn item(highlights: Vec<Highlight>) -> FeedItem {
    FeedItem {
        id: 1,
        rec_secs: 1.0,
        game_secs: 1.0,
        text: String::new(),
        highlights,
    }
}

#[test]
fn badge_title_uses_the_display_title_of_the_label() {
    let double = item(vec![Highlight::MyMultikill { streak: 2 }]);
    assert_eq!(badge_title(&double), ("DOUBLE KILL".to_owned(), Tone::Big));
    let death = item(vec![Highlight::MyDeath]);
    assert_eq!(badge_title(&death), ("DEATH".to_owned(), Tone::Death));
}

#[test]
fn badge_title_names_enemy_structures_as_lost() {
    let lost = item(vec![Highlight::EnemyStructure(
        greyscreen_core::highlight::Structure::Turret,
    )]);
    assert_eq!(badge_title(&lost), ("TURRET LOST".to_owned(), Tone::Enemy));
}

#[test]
fn badge_title_picks_the_dominant_label() {
    let both = item(vec![
        Highlight::MyKill,
        Highlight::TeamObjective(Objective::Baron),
    ]);
    assert_eq!(badge_title(&both).0, "KILL");
}

#[test]
fn events_without_a_highlight_are_called_game() {
    assert_eq!(
        badge_title(&item(Vec::new())),
        ("GAME".to_owned(), Tone::Neutral)
    );
}

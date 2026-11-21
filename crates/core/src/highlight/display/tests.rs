use super::*;
use crate::highlight::{Highlight, Objective, Structure};

#[test]
fn every_highlight_label_has_a_title() {
    let all = [
        Highlight::MyKill,
        Highlight::MyDeath,
        Highlight::MyAssist,
        Highlight::MyMultikill { streak: 2 },
        Highlight::MyMultikill { streak: 5 },
        Highlight::MyFirstBlood,
        Highlight::AllyKill,
        Highlight::EnemyKill,
        Highlight::TeamObjective(Objective::Baron),
        Highlight::EnemyObjective(Objective::Dragon("Fire".into())),
        Highlight::TeamStructure(Structure::Turret),
        Highlight::EnemyStructure(Structure::Inhibitor),
        Highlight::MyAce,
        Highlight::EnemyAce,
        Highlight::GameWon,
        Highlight::GameLost,
    ];
    for highlight in all {
        assert_ne!(
            describe_label(highlight.label()).title,
            "Event",
            "{highlight:?}"
        );
    }
}

#[test]
fn titles_read_as_plain_words() {
    assert_eq!(describe_label("Multikill2").title, "Double kill");
    assert_eq!(describe_label("MyDeath").title, "Death");
    assert_eq!(describe_label("EnemyTurret").title, "Turret lost");
    assert_eq!(describe_label("Turret").title, "Turret taken");
}

#[test]
fn deaths_and_kills_have_different_tones() {
    assert_eq!(describe_label("MyDeath").tone, Tone::Death);
    assert_eq!(describe_label("MyKill").tone, Tone::Kill);
}

#[test]
fn the_biggest_moment_wins() {
    assert_eq!(
        dominant_label(["MyKill", "FirstBlood", "Multikill2"]),
        Some("Multikill2")
    );
    assert_eq!(dominant_label(["MyAssist", "MyDeath"]), Some("MyDeath"));
    assert_eq!(dominant_label([]), None);
}

#[test]
fn unknown_labels_are_neutral_events() {
    let shown = describe_label("Something");
    assert_eq!((shown.title, shown.tone), ("Event", Tone::Neutral));
}

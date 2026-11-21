use super::{runtime, tap, visible_texts};

#[test]
fn the_record_switch_is_called_record_games_and_explained() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let texts = visible_texts(&rt);
    assert!(texts.iter().any(|t| t == "Record games"));
    assert!(
        texts
            .iter()
            .any(|t| t == "Start recording when a League game starts.")
    );
    assert!(!texts.iter().any(|t| t == "Record automatically"));
}

#[test]
fn the_audio_switch_says_what_is_recorded() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert!(
        visible_texts(&rt).iter().any(
            |t| t == "Records League's own audio. Discord and your microphone aren't recorded."
        )
    );
}

#[test]
fn the_size_estimate_ends_with_a_period() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let line = visible_texts(&rt)
        .into_iter()
        .find(|t| t.contains("MB per minute"))
        .expect("size estimate");
    assert!(line.ends_with('.'), "{line}");
}

#[test]
fn the_about_card_carries_the_riot_notice() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "settings-nav-about");
    assert!(rt.find("riot-notice").is_some());
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.starts_with("Greyscreen isn't endorsed by Riot Games")
                && t.ends_with("registered trademarks of Riot Games, Inc."))
    );
}

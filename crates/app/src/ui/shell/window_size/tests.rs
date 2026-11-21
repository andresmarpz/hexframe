use super::*;

fn poll_at(ui: &mut Ui, clock: f64) {
    ui.save_settled_window_size(clock, &mut Tx::default());
}

#[test]
fn a_resize_is_saved_once_it_settles() {
    let mut ui = Ui::preview();
    ui.note_window_size(1500.0, 900.0);
    poll_at(&mut ui, 10.0);
    assert_eq!(ui.settings.window_size, None);
    poll_at(&mut ui, 10.5);
    assert_eq!(ui.settings.window_size, None);
    poll_at(&mut ui, 11.0);
    assert_eq!(ui.settings.window_size, Some((1500, 900)));
}

#[test]
fn a_new_size_restarts_the_wait() {
    let mut ui = Ui::preview();
    ui.note_window_size(1500.0, 900.0);
    poll_at(&mut ui, 10.0);
    ui.note_window_size(1600.0, 900.0);
    poll_at(&mut ui, 10.7);
    poll_at(&mut ui, 11.0);
    assert_eq!(ui.settings.window_size, None);
    poll_at(&mut ui, 11.6);
    assert_eq!(ui.settings.window_size, Some((1600, 900)));
}

#[test]
fn sizes_below_the_minimum_and_full_screen_are_ignored() {
    let mut ui = Ui::preview();
    ui.note_window_size(0.0, 0.0);
    ui.note_window_size(f32::NAN, 700.0);
    ui.window_full_screen = true;
    ui.note_window_size(2560.0, 1440.0);
    poll_at(&mut ui, 10.0);
    poll_at(&mut ui, 12.0);
    assert_eq!(ui.settings.window_size, None);
}

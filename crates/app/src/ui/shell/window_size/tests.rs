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

fn placed() -> WindowPlacement {
    WindowPlacement {
        content_size: (2000, 1200),
        scale: 2.0,
        ..WindowPlacement::default()
    }
}

#[test]
fn a_normal_window_is_saved_in_points() {
    assert_eq!(
        resolve(Some(placed()), (1, 1), None),
        Some(((1000, 600), false))
    );
}

#[test]
fn a_maximized_window_keeps_the_size_it_restores_to() {
    let maximized = WindowPlacement {
        maximized: true,
        content_size: (2560, 1369),
        scale: 1.0,
        restore: Some(((10, 10), (1400, 800))),
        ..WindowPlacement::default()
    };
    assert_eq!(
        resolve(Some(maximized), (2560, 1369), None),
        Some(((1400, 800), true))
    );
}

#[test]
fn a_maximized_window_with_no_restore_record_keeps_the_saved_size() {
    let maximized = WindowPlacement {
        maximized: true,
        scale: 1.0,
        ..WindowPlacement::default()
    };
    assert_eq!(
        resolve(Some(maximized), (2560, 1369), Some((1300, 700))),
        Some(((1300, 700), true))
    );
}

#[test]
fn a_minimized_or_full_screen_window_saves_nothing() {
    let minimized = WindowPlacement {
        minimized: true,
        ..placed()
    };
    let full_screen = WindowPlacement {
        full_screen: true,
        ..placed()
    };
    assert_eq!(resolve(Some(minimized), (1, 1), None), None);
    assert_eq!(resolve(Some(full_screen), (1, 1), None), None);
}

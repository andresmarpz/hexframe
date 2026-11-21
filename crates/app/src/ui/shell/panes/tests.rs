use super::*;

#[test]
fn a_cancelled_drag_puts_the_width_back() {
    let mut ui = Ui::preview();
    ui.press_pane_edge(PaneEdge::List);
    ui.drag_pane_edge(PaneEdge::List, 80.0);
    assert_eq!(ui.settings.list_width, 380);
    ui.release_pane_edge(true);
    assert_eq!(ui.settings.list_width, 300);
}

#[test]
fn moves_without_a_press_or_for_another_edge_change_nothing() {
    let mut ui = Ui::preview();
    ui.drag_pane_edge(PaneEdge::Nav, 50.0);
    assert_eq!(ui.settings.nav_width, 220);
    ui.press_pane_edge(PaneEdge::Nav);
    ui.drag_pane_edge(PaneEdge::List, 50.0);
    assert_eq!(ui.settings.list_width, 300);
}

#[test]
fn assistive_steps_move_eight_points_inside_the_range() {
    let mut ui = Ui::preview();
    ui.step_pane_edge(PaneEdge::List, 2);
    assert_eq!(ui.settings.list_width, 316);
    ui.step_pane_edge(PaneEdge::List, -100);
    assert_eq!(ui.settings.list_width, 240);
}

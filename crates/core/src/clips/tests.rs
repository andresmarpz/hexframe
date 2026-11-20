use super::*;

const S: Hns = HNS_PER_SEC;

fn planner() -> ClipPlanner {
    ClipPlanner::new(15 * S, 5 * S)
}

#[test]
fn single_highlight_gets_pre_and_post_padding() {
    let mut p = planner();
    p.on_highlight(100 * S, "Kill", 1);
    let plans = p.flush();
    assert_eq!(
        plans,
        vec![ClipPlan {
            start: 85 * S,
            end: 105 * S,
            label: "Kill".into(),
            events: vec![1],
        }]
    );
}

#[test]
fn nearby_highlights_merge_and_join_labels() {
    let mut p = planner();
    p.on_highlight(100 * S, "Kill", 1);
    p.on_highlight(110 * S, "Double", 2);
    let plans = p.flush();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].start, 85 * S);
    assert_eq!(plans[0].end, 115 * S);
    assert_eq!(plans[0].label, "Kill+Double");
    assert_eq!(plans[0].events, vec![1, 2]);
}

#[test]
fn distant_highlights_stay_separate() {
    let mut p = planner();
    p.on_highlight(100 * S, "Kill", 1);
    p.on_highlight(130 * S, "Death", 2);
    let plans = p.flush();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].end, 105 * S);
    assert_eq!(plans[1].start, 115 * S);
}

#[test]
fn pre_window_touching_previous_end_does_not_merge() {
    let mut p = planner();
    p.on_highlight(100 * S, "A", 1);
    p.on_highlight(120 * S, "B", 2);
    assert_eq!(p.flush().len(), 2);
}

#[test]
fn more_than_three_labels_collapse_to_more() {
    let mut p = planner();
    for (i, label) in ["A", "B", "C", "D", "E"].iter().enumerate() {
        p.on_highlight(100 * S + i as Hns * S, label, i as u32);
    }
    let plans = p.flush();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].label, "A+B+C+more");
    assert_eq!(plans[0].events, vec![0, 1, 2, 3, 4]);
}

#[test]
fn plans_never_exceed_ninety_seconds() {
    let mut p = planner();
    for i in 0..20u32 {
        p.on_highlight(100 * S + Hns::from(i) * 10 * S, "K", i);
    }
    let plans = p.flush();
    assert!(plans.len() > 1);
    for plan in &plans {
        assert!(plan.end - plan.start <= 90 * S, "{plan:?}");
    }
    let ids: Vec<u32> = plans.iter().flat_map(|p| p.events.clone()).collect();
    assert_eq!(ids, (0..20).collect::<Vec<_>>());
}

#[test]
fn oversized_padding_is_capped_for_a_single_highlight() {
    let mut p = ClipPlanner::new(100 * S, 100 * S);
    p.on_highlight(500 * S, "K", 1);
    let plans = p.flush();
    assert_eq!(plans[0].end - plans[0].start, 90 * S);
}

#[test]
fn plan_is_due_one_second_after_its_end() {
    let mut p = planner();
    p.on_highlight(100 * S, "Kill", 1);
    assert!(p.due(105 * S).is_empty());
    assert!(p.due(106 * S - 1).is_empty());
    let plans = p.due(106 * S);
    assert_eq!(plans.len(), 1);
    assert!(p.due(200 * S).is_empty());
    assert!(p.flush().is_empty());
}

#[test]
fn extending_an_open_plan_postpones_it() {
    let mut p = planner();
    p.on_highlight(100 * S, "A", 1);
    p.on_highlight(104 * S, "B", 2);
    assert!(p.due(106 * S).is_empty());
    assert_eq!(p.due(110 * S).len(), 1);
}

#[test]
fn closed_plan_is_due_while_the_next_one_is_still_open() {
    let mut p = planner();
    p.on_highlight(100 * S, "A", 1);
    p.on_highlight(200 * S, "B", 2);
    let plans = p.due(206 * S - 1);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].label, "A");
    assert_eq!(p.due(206 * S)[0].label, "B");
}

#[test]
fn flush_returns_pending_plans_in_order_and_empties_the_planner() {
    let mut p = planner();
    p.on_highlight(100 * S, "A", 1);
    p.on_highlight(200 * S, "B", 2);
    let plans = p.flush();
    let labels: Vec<_> = plans.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["A", "B"]);
    assert!(p.flush().is_empty());
}

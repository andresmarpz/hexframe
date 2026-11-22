use super::LogOnce;

#[test]
fn the_first_failure_logs_and_repeats_stay_quiet() {
    let mut streak = LogOnce::new();
    assert!(streak.failure());
    assert!((0..1000).all(|_| !streak.failure()));
    assert!(streak.is_failing());
}

#[test]
fn a_failure_after_a_success_logs_again() {
    let mut streak = LogOnce::new();
    assert!(streak.failure());
    streak.success();
    assert!(!streak.is_failing());
    assert!(streak.failure());
    assert!(!streak.failure());
}

#[test]
fn a_success_without_a_failure_changes_nothing() {
    let mut streak = LogOnce::new();
    streak.success();
    assert!(streak.failure());
}

use std::collections::VecDeque;

use greyscreen_core::time::Hns;

pub(super) const MAX_OUTPUTS_WITHOUT_PARAMETERS: u32 = 120;

pub(super) fn match_tick(
    in_flight: &mut VecDeque<(u64, Hns)>,
    output_pts: Hns,
    first_output: bool,
) -> Option<u64> {
    if output_pts == 0 && !first_output {
        return in_flight.pop_front().map(|(tick, _)| tick);
    }
    while in_flight.front().is_some_and(|&(_, pts)| pts < output_pts) {
        in_flight.pop_front();
    }
    match in_flight.front() {
        Some(&(tick, pts)) if pts == output_pts => {
            in_flight.pop_front();
            Some(tick)
        }
        _ => None,
    }
}

pub(super) fn parameter_fault(
    started: Option<(&[u8], &[u8])>,
    sps: Option<&[u8]>,
    pps: Option<&[u8]>,
    outputs_without: u32,
) -> Option<&'static str> {
    match started {
        Some((known_sps, known_pps)) => {
            let changed = sps.is_some_and(|seen| seen != known_sps)
                || pps.is_some_and(|seen| seen != known_pps);
            changed.then_some("encoder parameters changed")
        }
        None => (outputs_without >= MAX_OUTPUTS_WITHOUT_PARAMETERS)
            .then_some("encoder sent no parameter sets"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(entries: &[(u64, Hns)]) -> VecDeque<(u64, Hns)> {
        entries.iter().copied().collect()
    }

    #[test]
    fn exact_pts_pops_its_tick() {
        let mut q = queue(&[(0, 0), (1, 100), (2, 200)]);
        assert_eq!(match_tick(&mut q, 0, true), Some(0));
        assert_eq!(match_tick(&mut q, 100, false), Some(1));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn lost_outputs_are_skipped_by_pts() {
        let mut q = queue(&[(0, 0), (1, 100), (2, 200), (3, 300)]);
        assert_eq!(match_tick(&mut q, 200, false), Some(2));
        assert_eq!(match_tick(&mut q, 300, false), Some(3));
        assert!(q.is_empty());
    }

    #[test]
    fn missing_pts_after_the_first_falls_back_to_fifo() {
        let mut q = queue(&[(4, 400), (5, 500)]);
        assert_eq!(match_tick(&mut q, 0, false), Some(4));
        assert_eq!(match_tick(&mut q, 0, false), Some(5));
        assert_eq!(match_tick(&mut q, 0, false), None);
    }

    #[test]
    fn zero_pts_on_the_first_output_matches_tick_zero() {
        let mut q = queue(&[(0, 0), (1, 100)]);
        assert_eq!(match_tick(&mut q, 0, true), Some(0));
    }

    #[test]
    fn unknown_pts_matches_nothing() {
        let mut q = queue(&[(1, 100), (2, 200)]);
        assert_eq!(match_tick(&mut q, 150, false), None);
        assert_eq!(q.len(), 1);
        assert_eq!(match_tick(&mut q, 500, false), None);
        assert!(q.is_empty());
    }

    #[test]
    fn parameter_sets_never_arriving_fail_after_the_limit() {
        assert_eq!(parameter_fault(None, None, None, 119), None);
        assert_eq!(
            parameter_fault(None, None, None, 120),
            Some("encoder sent no parameter sets")
        );
    }

    #[test]
    fn changed_in_band_parameters_fail() {
        let known = Some((&[1u8, 2][..], &[3u8][..]));
        assert_eq!(parameter_fault(known, Some(&[1, 2]), Some(&[3]), 0), None);
        assert_eq!(parameter_fault(known, None, None, 0), None);
        assert_eq!(
            parameter_fault(known, Some(&[9]), None, 0),
            Some("encoder parameters changed")
        );
        assert_eq!(
            parameter_fault(known, None, Some(&[9]), 500),
            Some("encoder parameters changed")
        );
    }
}

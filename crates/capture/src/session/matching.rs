pub(super) const MAX_OUTPUTS_WITHOUT_PARAMETERS: u32 = 120;

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

use proptest::prelude::*;

use super::*;

#[derive(Debug, Clone, Copy)]
enum Step {
    Arrive { torn: bool },
    Promote,
    StartReading,
    DoneReading,
    Thumbnail,
    SizeChange,
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        any::<bool>().prop_map(|torn| Step::Arrive { torn }),
        Just(Step::Promote),
        Just(Step::StartReading),
        Just(Step::DoneReading),
        Just(Step::Thumbnail),
        Just(Step::SizeChange),
    ]
}

fn tagged(tag: u8, len: usize) -> Vec<u8> {
    vec![tag; len]
}

fn check_write_slot(guard: &HoldBack) -> Result<usize, TestCaseError> {
    let slot = guard.write_slot();
    prop_assert!(slot < RING_SLOTS);
    for taken in [guard.published(), guard.pending(), guard.reading()] {
        prop_assert_ne!(taken, Some(slot));
    }
    Ok(slot)
}

fn run(steps: &[Step]) -> Result<(), TestCaseError> {
    let mut guard = HoldBack::default();
    let mut width = WIDTH;
    let mut tag = 0u8;
    for step in steps {
        let slot = check_write_slot(&guard)?;
        let reading_before = guard.reading();
        let read_before = reading_before.map(|r| guard.thumbs[r].clone());
        match *step {
            Step::Arrive { torn } => {
                tag = tag.wrapping_add(1);
                let base = tagged(tag, width * 27);
                let mut thumb = if torn {
                    torn_from(&base, width * 10)
                } else {
                    base
                };
                guard.store_thumb(slot, &mut thumb, width);
                guard.arrive(slot);
            }
            Step::Promote => {
                if let Some(promoted) = guard.promote() {
                    prop_assert_ne!(Some(promoted), guard.reading());
                    prop_assert_eq!(guard.pending(), None);
                }
            }
            Step::StartReading => {
                guard.start_reading();
            }
            Step::DoneReading => guard.done_reading(),
            Step::Thumbnail => {
                tag = tag.wrapping_add(1);
                let mut thumb = tagged(tag, width * 27);
                guard.store_thumb(slot, &mut thumb, width);
            }
            Step::SizeChange => {
                width = if width == WIDTH { WIDTH / 2 } else { WIDTH };
                guard.reset();
                prop_assert_eq!(guard.published(), None);
                prop_assert_eq!(guard.pending(), None);
                prop_assert_eq!(guard.reading(), None);
                let fresh = check_write_slot(&guard)?;
                prop_assert!(fresh < RING_SLOTS);
            }
        }
        if let (Some(before), Some(reading)) = (
            read_before,
            guard.reading().filter(|r| Some(*r) == reading_before),
        ) {
            prop_assert_eq!(&guard.thumbs[reading], &before);
        }
        check_write_slot(&guard)?;
    }
    Ok(())
}

proptest! {
    #[test]
    fn the_write_slot_is_never_published_pending_or_being_read(
        steps in proptest::collection::vec(step(), 1..400),
    ) {
        run(&steps)?;
    }
}

#[test]
fn ten_thousand_arrivals_stay_inside_the_ring() {
    let mut guard = HoldBack::default();
    for i in 0..10_000u32 {
        let slot = guard.write_slot();
        assert!(slot < RING_SLOTS);
        let mut thumb = tagged((i % 251) as u8, CELLS);
        guard.store_thumb(slot, &mut thumb, WIDTH);
        guard.arrive(slot);
        if i % 3 == 0 {
            guard.start_reading();
        }
        if i % 5 == 0 {
            guard.done_reading();
        }
        assert!(guard.thumbs.len() == RING_SLOTS);
    }
}

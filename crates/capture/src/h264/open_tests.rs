use std::collections::HashSet;
use std::sync::atomic::Ordering;

use greyscreen_core::encoder::EncoderKind;
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};
use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncCommonBufferSize, CODECAPI_AVEncCommonMaxBitRate,
    CODECAPI_AVEncCommonMeanBitRate, CODECAPI_AVEncCommonQuality,
    CODECAPI_AVEncCommonRateControlMode, CODECAPI_AVEncMPVDefaultBPictureCount,
    CODECAPI_AVEncMPVGOPSize, CODECAPI_AVEncVideoForceKeyFrame, CODECAPI_AVLowLatencyMode,
    MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, eAVEncCommonRateControlMode_Quality,
};
use windows::core::GUID;

use super::setup::{Opened, Spec, open_candidates};
use super::transform::{Candidate, Mft};
use super::{H264Encoder, H264Error, take_property_log};
use crate::com::MfGuard;
use crate::fault::Armed;
use crate::testing::fake_transform::{
    Call, CallKind, Calls, FakeCandidate, FakeTransform, Step, device_manager,
};

const SIZE: u32 = 64;
const QUALITY: u32 = 70;
const NVIDIA: u32 = 0x10de;

fn spec(quality: Option<u32>) -> Spec {
    Spec {
        width: SIZE,
        height: SIZE,
        fps: 60,
        mean_bitrate: 8_000_000,
        peak_bitrate: 12_000_000,
        quality,
        quality_vs_speed: None,
        gop_frames: 120,
    }
}

fn open(candidates: Vec<FakeCandidate>, quality: Option<u32>) -> super::Result<Opened> {
    open_candidates(
        candidates
            .into_iter()
            .map(|candidate| Box::new(candidate) as Box<dyn Candidate>)
            .collect(),
        device_manager().unwrap(),
        &spec(quality),
        NVIDIA,
    )
}

fn open_one(fake: FakeTransform, quality: Option<u32>) -> super::Result<Opened> {
    open(vec![FakeCandidate::new(fake)], quality)
}

fn encoder(opened: Opened) -> super::Result<H264Encoder> {
    H264Encoder::from_opened(opened, EncoderKind::Hardware, "fake".into(), SIZE, SIZE)
}

fn recorded(calls: &Calls) -> Vec<Call> {
    calls.lock().unwrap().clone()
}

fn kinds(calls: &Calls) -> Vec<CallKind> {
    recorded(calls).iter().map(|call| call.kind).collect()
}

fn set_properties(calls: &[Call]) -> Vec<GUID> {
    calls
        .iter()
        .filter(|call| call.kind == CallKind::SetValue)
        .filter_map(|call| call.property)
        .collect()
}

fn position(calls: &[Call], wanted: CallKind) -> usize {
    calls.iter().position(|call| call.kind == wanted).unwrap()
}

fn begin_streaming() -> CallKind {
    CallKind::Message(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING.0)
}

fn clean_calls() -> Vec<CallKind> {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let fake = FakeTransform::new();
    let calls = fake.calls();
    open_one(fake, None).unwrap();
    kinds(&calls)
}

fn failing_first_candidates() -> Vec<(&'static str, FakeCandidate)> {
    let at = |step, nth| FakeCandidate::new(FakeTransform::new().failing_at(step, nth, E_FAIL));
    vec![
        (
            "activate",
            FakeCandidate::new(FakeTransform::new()).failing_activation(E_FAIL),
        ),
        ("set the D3D manager", at(Step::SetD3dManager, 0)),
        ("set the output type", at(Step::SetOutputType, 0)),
        ("set the input type", at(Step::SetInputType, 0)),
        ("flush before streaming", at(Step::ProcessMessage, 0)),
        ("begin streaming", at(Step::ProcessMessage, 1)),
        ("start of stream", at(Step::ProcessMessage, 2)),
    ]
}

#[test]
fn every_failing_open_step_rejects_a_lone_candidate_with_one_shutdown() {
    for (step, candidate) in failing_first_candidates() {
        let _armed = Armed::new();
        let _mf = MfGuard::start().unwrap();
        let shutdowns = candidate.shutdown_objects();
        let error = open(vec![candidate], None).unwrap_err();
        assert!(matches!(error, H264Error::NoHardwareEncoder(_)), "{step}");
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1, "{step}");
    }
}

#[test]
fn a_rejected_candidate_leaves_nothing_behind_for_the_next_one() {
    let clean = clean_calls();
    for (step, first) in failing_first_candidates() {
        let _armed = Armed::new();
        let _mf = MfGuard::start().unwrap();
        let second_fake = FakeTransform::new();
        let second_calls = second_fake.calls();
        let first_shutdowns = first.shutdown_objects();
        let second = FakeCandidate::new(second_fake).named("second");
        let second_shutdowns = second.shutdown_objects();
        let opened = open(vec![first.named("first"), second], None).unwrap();
        assert!(opened.name.starts_with("second"), "{step}: {}", opened.name);
        assert_eq!(first_shutdowns.load(Ordering::SeqCst), 1, "{step}");
        assert_eq!(second_shutdowns.load(Ordering::SeqCst), 0, "{step}");
        assert_eq!(kinds(&second_calls), clean, "{step}");
    }
}

#[test]
fn a_failing_output_stream_refresh_shuts_the_encoder_down_once_and_the_next_open_works() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let fake = FakeTransform::new().failing_at(Step::OutputStreamInfo, 0, E_FAIL);
    let calls = fake.calls();
    let opened = open_one(fake, None).unwrap();
    assert!(encoder(opened).is_err());
    let seen = kinds(&calls);
    let shutdowns = seen
        .iter()
        .filter(|kind| **kind == CallKind::Shutdown)
        .count();
    assert_eq!(shutdowns, 1);
    assert_eq!(seen.last(), Some(&CallKind::Shutdown));
    let again = open_one(FakeTransform::new(), None).unwrap();
    assert!(encoder(again).is_ok());
}

#[test]
fn refusing_an_optional_property_still_opens() {
    let optional = [
        CODECAPI_AVEncMPVDefaultBPictureCount,
        CODECAPI_AVLowLatencyMode,
        CODECAPI_AVEncCommonMaxBitRate,
        CODECAPI_AVEncCommonBufferSize,
    ];
    for property in optional {
        let _armed = Armed::new();
        let _mf = MfGuard::start().unwrap();
        let fake = FakeTransform::new().refusing(property, E_FAIL);
        assert!(open_one(fake, None).is_ok(), "{property:?}");
    }
}

#[test]
fn refusing_a_required_property_rejects_the_candidate() {
    let required = [
        CODECAPI_AVEncMPVGOPSize,
        CODECAPI_AVEncCommonRateControlMode,
        CODECAPI_AVEncCommonMeanBitRate,
    ];
    for property in required {
        let _armed = Armed::new();
        let _mf = MfGuard::start().unwrap();
        let candidate = FakeCandidate::new(FakeTransform::new().refusing(property, E_FAIL));
        let shutdowns = candidate.shutdown_objects();
        let error = open(vec![candidate], None).unwrap_err();
        assert!(
            matches!(error, H264Error::NoHardwareEncoder(_)),
            "{property:?}"
        );
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1, "{property:?}");
    }
}

fn captured_logs(run: impl FnOnce()) -> String {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buffer = Buffer(Arc::default());
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    let bytes = buffer.0.lock().unwrap().clone();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn quality_mode() -> u32 {
    eAVEncCommonRateControlMode_Quality.0 as u32
}

#[test]
fn accepted_quality_mode_is_chosen_and_never_changed_after_the_output_type() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let fake = FakeTransform::new().echoing_values();
    let calls = fake.calls();
    let opened = open_one(fake, Some(QUALITY)).unwrap();
    assert!(opened.name.contains("quality 70"), "{}", opened.name);
    let calls = recorded(&calls);
    let output_type = position(&calls, CallKind::SetOutputType);
    let mode_sets: Vec<usize> = calls
        .iter()
        .enumerate()
        .filter(|(_, call)| {
            call.kind == CallKind::SetValue
                && call.property == Some(CODECAPI_AVEncCommonRateControlMode)
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(mode_sets.len(), 1);
    assert!(mode_sets[0] < output_type);
    assert_eq!(calls[mode_sets[0]].value, Some(quality_mode()));
    assert!(
        calls[output_type..]
            .iter()
            .all(|call| call.kind != CallKind::SetValue)
    );
}

fn falls_back_to_vbr(fake: FakeTransform, reason: &str) {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let mut name = String::new();
    let logs = captured_logs(|| {
        name = open_one(fake, Some(QUALITY)).unwrap().name;
    });
    assert!(name.contains("vbr"), "{name}");
    assert!(logs.contains("peak-constrained VBR"), "{logs}");
    assert!(logs.contains(reason), "{logs}");
}

#[test]
fn refused_quality_mode_falls_back_to_vbr_with_a_logged_reason() {
    let fake = FakeTransform::new().echoing_values().refusing_value(
        CODECAPI_AVEncCommonRateControlMode,
        quality_mode(),
        E_FAIL,
    );
    falls_back_to_vbr(fake, "rejected Quality rate control mode");
}

#[test]
fn quality_mode_read_back_wrong_falls_back_to_vbr_with_a_logged_reason() {
    let fake = FakeTransform::new()
        .echoing_values()
        .reading_back(CODECAPI_AVEncCommonRateControlMode, quality_mode() + 1);
    falls_back_to_vbr(fake, "rate control mode read back as");
}

#[test]
fn quality_value_read_back_wrong_falls_back_to_vbr_with_a_logged_reason() {
    let fake = FakeTransform::new()
        .echoing_values()
        .reading_back(CODECAPI_AVEncCommonQuality, QUALITY + 1);
    falls_back_to_vbr(fake, "quality read back as");
}

#[test]
fn the_joined_error_keeps_the_first_hardware_error_with_its_label_and_text() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let first = FakeCandidate::new(FakeTransform::new().failing_at(Step::SetOutputType, 0, E_FAIL))
        .named("first gpu (vendor 1)");
    let second =
        FakeCandidate::new(FakeTransform::new().failing_at(Step::SetInputType, 0, E_INVALIDARG))
            .named("second gpu (vendor 2)");
    let H264Error::NoHardwareEncoder(text) = open(vec![first, second], None).unwrap_err() else {
        panic!("open failures are NoHardwareEncoder");
    };
    let first_at = text.find("first gpu (vendor 1): ").expect(&text);
    let second_at = text.find("second gpu (vendor 2): ").expect(&text);
    assert!(first_at < second_at, "{text}");
    let first_part = &text[first_at..second_at];
    assert!(first_part.contains("SetOutputType failed"), "{text}");
    assert!(first_part.contains("0x80004005"), "{text}");
    assert!(
        text[second_at..].contains("SetInputType(NV12) failed"),
        "{text}"
    );
}

#[test]
fn every_codec_property_is_set_once_and_only_force_keyframe_follows_streaming() {
    for quality in [None, Some(QUALITY)] {
        let _armed = Armed::new();
        let _mf = MfGuard::start().unwrap();
        let fake = FakeTransform::new().echoing_values();
        let calls = fake.calls();
        take_property_log();
        let opened = open_one(fake, quality).unwrap();
        let logged = take_property_log();
        let mut encoder = encoder(opened).unwrap();
        encoder.force_keyframe().unwrap();
        let calls = recorded(&calls);
        let streaming = position(&calls, begin_streaming());
        let before = set_properties(&calls[..streaming]);
        let names: HashSet<&str> = logged
            .iter()
            .map(|line| line.split_once(':').unwrap().0)
            .collect();
        assert_eq!(
            names.len(),
            logged.len(),
            "{quality:?}: duplicate names {logged:?}"
        );
        assert_eq!(logged.len(), before.len(), "{quality:?}: {logged:?}");
        let unique: HashSet<GUID> = before.iter().copied().collect();
        assert_eq!(unique.len(), before.len(), "{quality:?}: set twice");
        assert!(!unique.contains(&CODECAPI_AVEncVideoForceKeyFrame));
        assert_eq!(
            set_properties(&calls[streaming..]),
            [CODECAPI_AVEncVideoForceKeyFrame],
            "{quality:?}"
        );
    }
}

#[test]
fn open_and_encode_calls_all_run_on_one_thread() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let fake = FakeTransform::new();
    let calls = fake.calls();
    let mut encoder = encoder(open_one(fake, None).unwrap()).unwrap();
    encoder.force_keyframe().unwrap();
    drop(encoder);
    let here = std::thread::current().id();
    assert!(recorded(&calls).iter().all(|call| call.thread == here));
}

#[test]
fn a_second_shutdown_sends_nothing_and_no_call_follows_the_first() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let fake = FakeTransform::new();
    let calls = fake.calls();
    let mut encoder = encoder(open_one(fake, None).unwrap()).unwrap();
    encoder.shutdown();
    let after_first = recorded(&calls).len();
    encoder.shutdown();
    drop(encoder);
    let calls = recorded(&calls);
    assert_eq!(calls.len(), after_first);
    assert_eq!(calls.last().map(|call| call.kind), Some(CallKind::Shutdown));
    let shutdowns = calls
        .iter()
        .filter(|call| call.kind == CallKind::Shutdown)
        .count();
    assert_eq!(shutdowns, 1);
}

trait AmbiguousIfSend<A> {
    fn some_item() {}
}

impl<T: ?Sized> AmbiguousIfSend<()> for T {}
impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}

const _: fn() = || {
    let _ = <H264Encoder as AmbiguousIfSend<_>>::some_item;
    let _ = <Box<dyn Mft> as AmbiguousIfSend<_>>::some_item;
};

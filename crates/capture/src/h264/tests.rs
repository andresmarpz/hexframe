use super::pump::to_annexb;

const IDR: [u8; 3] = [0x65, 0x88, 0x84];
const SLICE: [u8; 3] = [0x41, 0x9a, 0x01];
const START: [u8; 4] = [0, 0, 0, 1];

fn expected(nals: &[&[u8]]) -> Vec<u8> {
    nals.iter().flat_map(|n| [&START[..], n].concat()).collect()
}

#[test]
fn annexb_input_is_normalised_to_four_byte_start_codes() {
    let raw = [&[0, 0, 1][..], &IDR, &START, &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&IDR, &SLICE]));
}

#[test]
fn length_prefixed_input_gets_start_codes() {
    let raw = [&[0, 0, 0, 3][..], &IDR, &[0, 0, 0, 3], &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&IDR, &SLICE]));
}

#[test]
fn prefix_nal_units_are_dropped() {
    let raw = [&START[..], &[0x6e, 0x01], &START, &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&SLICE]));
}

#[test]
fn garbage_is_rejected() {
    assert!(to_annexb(&[9, 9, 9, 9, 9]).is_err());
    assert!(to_annexb(&[]).is_err());
}

#[test]
fn open_steps_map_to_their_failure() {
    use super::open_error::classify;
    use greyscreen_core::encoder::{OpenFailure, OpenStep};

    let nvidia = 0x10de;
    let other = 0x8000_4005_u32 as i32;
    assert_eq!(
        classify(OpenStep::Start, nvidia, other),
        OpenFailure::StartFailed
    );
    assert_eq!(
        classify(OpenStep::D3dManager, nvidia, other),
        OpenFailure::Unsupported
    );
    let removed = 0x887A_0005_u32 as i32;
    assert_eq!(
        classify(OpenStep::D3dManager, nvidia, removed),
        OpenFailure::DeviceRemoved
    );
}

mod on_fake {
    use std::time::{Duration, Instant};

    use greyscreen_core::encoder::EncoderKind;
    use windows::Win32::Foundation::E_FAIL;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Texture2D,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};

    use super::super::setup::Opened;
    use super::super::{DRAIN_TIMEOUT, H264Encoder, H264Error, Mode};
    use crate::com::MfGuard;
    use crate::testing::fake_transform::{
        CallKind, Calls, FakeTransform, SimulatedClock, Step, device_manager,
    };
    use crate::testing::warp;
    use crate::vproc::Nv12Frame;

    const SIZE: u32 = 64;
    const FRAME_HNS: i64 = 166_667;

    struct Rig {
        _mf: MfGuard,
        encoder: H264Encoder,
        calls: Calls,
        texture: ID3D11Texture2D,
    }

    fn rig(fake: FakeTransform) -> Rig {
        let mf = MfGuard::start().unwrap();
        let gpu = warp::device().unwrap();
        let desc = D3D11_TEXTURE2D_DESC {
            Width: SIZE,
            Height: SIZE,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_NV12,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut texture = None;
        // SAFETY: the device is live; `desc` and `texture` outlive the call.
        unsafe {
            gpu.device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .unwrap()
        };
        let calls = fake.calls();
        let opened = Opened {
            mode: Mode::Async {
                _manager: device_manager().unwrap(),
            },
            mft: Box::new(fake),
            input_id: 0,
            output_id: 0,
            name: "fake".to_string(),
            sequence_header: Vec::new(),
        };
        let encoder =
            H264Encoder::from_opened(opened, EncoderKind::Hardware, "fake".into(), SIZE, SIZE)
                .unwrap();
        Rig {
            _mf: mf,
            encoder,
            calls,
            texture: texture.unwrap(),
        }
    }

    fn frame(rig: &Rig, index: usize) -> Nv12Frame {
        Nv12Frame {
            texture: rig.texture.clone(),
            slot: index,
        }
    }

    #[test]
    fn clean_script_gives_one_packet_per_frame_on_one_thread() {
        let mut rig = rig(FakeTransform::new());
        let mut packets = Vec::new();
        for index in 0..4 {
            let frame = frame(&rig, index);
            rig.encoder
                .submit(&frame, index as i64 * FRAME_HNS, FRAME_HNS)
                .unwrap();
            rig.encoder.drain_output(&mut packets).unwrap();
        }
        let clock = SimulatedClock::default();
        let calls = rig.calls.clone();
        rig.encoder.finish_with(&mut packets, &clock).unwrap();
        assert_eq!(packets.len(), 4);
        assert!(packets[0].key);
        assert!(packets[1..].iter().all(|packet| !packet.key));
        let pts: Vec<i64> = packets.iter().map(|packet| packet.pts_hns).collect();
        assert_eq!(pts, [0, FRAME_HNS, 2 * FRAME_HNS, 3 * FRAME_HNS]);
        let calls = calls.lock().unwrap();
        let here = std::thread::current().id();
        assert!(calls.iter().all(|call| call.thread == here));
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.kind == CallKind::ProcessInput)
                .count(),
            4
        );
    }

    #[test]
    fn drain_that_never_completes_returns_at_the_simulated_bound() {
        let mut rig = rig(FakeTransform::new().never_draining());
        let frame = frame(&rig, 0);
        rig.encoder.submit(&frame, 0, FRAME_HNS).unwrap();
        let clock = SimulatedClock::default();
        let started = Instant::now();
        let mut packets = Vec::new();
        rig.encoder.finish_with(&mut packets, &clock).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(clock.elapsed() >= DRAIN_TIMEOUT);
        assert!(clock.elapsed() < DRAIN_TIMEOUT + Duration::from_millis(10));
        assert_eq!(packets.len(), 1);
    }

    #[test]
    fn scripted_input_failure_surfaces_and_shutdown_still_runs() {
        let mut rig = rig(FakeTransform::new().failing_at(Step::ProcessInput, 1, E_FAIL));
        let first = frame(&rig, 0);
        rig.encoder.submit(&first, 0, FRAME_HNS).unwrap();
        let second = frame(&rig, 1);
        let error = rig
            .encoder
            .submit(&second, FRAME_HNS, FRAME_HNS)
            .unwrap_err();
        assert!(matches!(error, H264Error::Windows(_)));
        let calls = rig.calls.clone();
        drop(rig);
        assert!(
            calls
                .lock()
                .unwrap()
                .iter()
                .any(|call| matches!(call.kind, CallKind::Message(_)))
        );
    }

    #[test]
    fn starved_encoder_reports_not_accepting() {
        let mut rig = rig(FakeTransform::new().never_needing_input());
        let frame = frame(&rig, 0);
        let error = rig.encoder.submit(&frame, 0, FRAME_HNS).unwrap_err();
        assert!(matches!(error, H264Error::NotAccepting));
        assert!(!rig.encoder.need_input().unwrap());
    }
}

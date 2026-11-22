use super::*;

const RATE: u32 = 48_000;
const FRAME: i64 = 1500;

fn video(index: i64, key: bool) -> Arc<Packet> {
    Arc::new(Packet {
        track: Track::Video,
        dts: index * FRAME,
        pts: index * FRAME,
        duration: FRAME as u32,
        key,
        data: Arc::from(vec![0u8; 4]),
    })
}

fn audio(index: i64) -> Arc<Packet> {
    Arc::new(Packet {
        track: Track::Audio,
        dts: index * 1024,
        pts: index * 1024,
        duration: 1024,
        key: true,
        data: Arc::from(vec![0u8; 4]),
    })
}

fn filled(seconds: i64) -> ClipRing {
    let mut ring = ClipRing::new(1_000, 4 * HNS_PER_SEC, RATE);
    let mut next_audio = 0;
    for n in 0..seconds * 60 {
        ring.push(video(n, n % 60 == 0));
        let video_end = (n + 1) * FRAME * i64::from(RATE) / 90_000;
        while next_audio * 1024 < video_end {
            ring.push(audio(next_audio));
            next_audio += 1;
        }
    }
    ring
}

fn at(ring: &ClipRing, secs: f64) -> Hns {
    ring.t0 + (secs * HNS_PER_SEC as f64) as Hns
}

#[test]
fn prune_keeps_from_idr_older_than_keep() {
    let mut ring = filled(10);
    ring.prune();
    let first = ring
        .packets
        .iter()
        .find(|p| p.track == Track::Video)
        .map(|p| (p.dts, p.key));
    assert_eq!(first, Some((5 * 90_000, true)));
    let videos = ring
        .packets
        .iter()
        .filter(|p| p.track == Track::Video)
        .count();
    assert_eq!(videos, 5 * 60);
}

#[test]
fn prune_does_nothing_before_keep_elapsed() {
    let mut ring = filled(3);
    let before = ring.packets.len();
    ring.prune();
    assert_eq!(ring.packets.len(), before);
}

#[test]
fn range_starts_at_idr_at_or_before_start() {
    let ring = filled(10);
    let packets = ring.range(at(&ring, 3.4), at(&ring, 6.0));
    let first_video = packets.iter().find(|p| p.track == Track::Video);
    assert_eq!(
        first_video.map(|p| (p.dts, p.key)),
        Some((3 * 90_000, true))
    );
    let last_video = packets.iter().rfind(|p| p.track == Track::Video);
    assert_eq!(last_video.map(|p| p.dts), Some(6 * 90_000 - FRAME));
}

#[test]
fn range_includes_overlapping_audio_only() {
    let ring = filled(10);
    let packets = ring.range(at(&ring, 3.0), at(&ring, 4.0));
    let audio: Vec<_> = packets.iter().filter(|p| p.track == Track::Audio).collect();
    assert!(!audio.is_empty());
    assert!(audio.iter().all(|p| ring.start_of(p) < at(&ring, 4.0)));
    assert!(audio.iter().all(|p| ring.end_of(p) > at(&ring, 3.0)));
}

#[test]
fn range_before_buffer_starts_at_oldest_idr() {
    let mut ring = filled(10);
    ring.prune();
    let packets = ring.range(at(&ring, 0.0), at(&ring, 7.0));
    let first_video = packets.iter().find(|p| p.track == Track::Video);
    assert_eq!(first_video.map(|p| p.dts), Some(5 * 90_000));
}

#[test]
fn range_is_empty_without_idr() {
    let mut ring = ClipRing::new(0, HNS_PER_SEC, RATE);
    ring.push(video(1, false));
    assert!(ring.range(0, HNS_PER_SEC).is_empty());
}

#[test]
fn ten_thousand_packets_leave_the_ring_at_a_steady_size() {
    let mut ring = ClipRing::new(1_000, 4 * HNS_PER_SEC, RATE);
    let mut sizes = Vec::new();
    for n in 0..10_000 {
        let key = n % 60 == 0;
        ring.push(video(n, key));
        if key {
            ring.prune();
            sizes.push(ring.packets.len());
        }
    }
    let settled = &sizes[sizes.len() / 2..];
    let largest = settled.iter().max().copied().unwrap_or(0);
    assert!(largest <= 6 * 60, "{largest}");
    assert_eq!(settled.last(), settled.first());
}

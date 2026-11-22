use super::pcm_queue;
use crate::audio::PcmBlock;

#[test]
fn queue_disconnects_when_the_worker_receiver_drops() {
    let (tx, rx) = pcm_queue();
    drop(rx);
    let block = PcmBlock {
        first_frame: 0,
        samples: vec![0; 4],
    };
    assert!(tx.send(block).is_err());
}

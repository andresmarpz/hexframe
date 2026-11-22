use greyscreen_core::tear::{Arrival, HoldBack};
use greyscreen_core::time::{Hns, mono_now};
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub(super) struct Latest {
    pub(super) texture: ID3D11Texture2D,
    pub(super) size: (u32, u32),
    pub(super) at: Hns,
    pub(super) seq: u64,
    pub(super) content_seq: u64,
}

#[derive(Default)]
pub(super) struct Published {
    pub(super) current: Option<Latest>,
    pub(super) pending: Option<Latest>,
    pub(super) holdback: HoldBack,
    seq: u64,
    content_seq: u64,
}

pub(super) enum Outcome {
    Kept,
    Torn,
    Rewound,
}

impl Published {
    pub(super) fn has_frame(&self) -> bool {
        self.current.is_some() || self.pending.is_some()
    }

    pub(super) fn latest(
        &mut self,
        promote_after: Hns,
    ) -> Option<(ID3D11Texture2D, (u32, u32), u64, u64)> {
        let stale = self
            .pending
            .as_ref()
            .is_some_and(|pending| mono_now() - pending.at >= promote_after);
        if stale {
            self.holdback.promote();
            self.current = self.pending.take();
        }
        let latest = self.current.as_ref().map(|latest| {
            (
                latest.texture.clone(),
                latest.size,
                latest.seq,
                latest.content_seq,
            )
        })?;
        self.holdback.start_reading();
        Some(latest)
    }

    pub(super) fn arrive(
        &mut self,
        slot: usize,
        texture: ID3D11Texture2D,
        size: (u32, u32),
        at: Hns,
        checked: bool,
    ) -> Outcome {
        if !checked {
            self.seq += 1;
            self.holdback.publish_now(slot);
            self.content_seq += 1;
            self.pending = None;
            self.current = Some(Latest {
                texture,
                size,
                at,
                seq: self.seq,
                content_seq: self.content_seq,
            });
            return Outcome::Kept;
        }
        let arrival = self.holdback.arrive(slot);
        if arrival == Arrival::Rewound {
            return Outcome::Rewound;
        }
        self.seq += 1;
        let seq = self.seq;
        let make = |content_seq| Latest {
            texture,
            size,
            at,
            seq,
            content_seq,
        };
        match arrival {
            Arrival::Held => {
                self.content_seq += 1;
                self.pending = Some(make(self.content_seq));
                Outcome::Kept
            }
            Arrival::Published(_) => {
                if let Some(pending) = self.pending.take() {
                    self.current = Some(pending);
                }
                self.content_seq += 1;
                self.pending = Some(make(self.content_seq));
                Outcome::Kept
            }
            Arrival::Merged { content_changed } => {
                let content_seq = if content_changed {
                    self.pending
                        .take()
                        .map(|pending| pending.content_seq)
                        .unwrap_or_else(|| {
                            self.content_seq += 1;
                            self.content_seq
                        })
                } else {
                    self.pending = None;
                    self.current
                        .as_ref()
                        .map_or(self.content_seq, |current| current.content_seq)
                };
                self.current = Some(make(content_seq));
                Outcome::Kept
            }
            Arrival::Dropped(_) => {
                self.content_seq += 1;
                self.current = Some(make(self.content_seq));
                self.pending = None;
                Outcome::Torn
            }
            Arrival::Rewound => Outcome::Rewound,
        }
    }
}

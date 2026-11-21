use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use greyscreen_core::encoder::{self, CheckOutcome, EncoderCheck, SOFTWARE_NOTICE};
use greyscreen_core::media::{CaptureConfig, MediaSource};
use greyscreen_core::problem::Problem;

use crate::api::{NoticeAction, NoticeLevel, UiMsg};

pub struct EncoderCheckSpec {
    pub media: Arc<dyn MediaSource>,
    pub cfg: CaptureConfig,
    pub cache: PathBuf,
    pub version: String,
    pub delay: Duration,
}

impl std::fmt::Debug for EncoderCheckSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncoderCheckSpec")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

pub fn spawn_encoder_check(
    spec: EncoderCheckSpec,
    sink: Box<dyn Fn(UiMsg) + Send>,
) -> Option<JoinHandle<()>> {
    let spawned = std::thread::Builder::new()
        .name("encoder-check".into())
        .spawn(move || {
            std::thread::sleep(spec.delay);
            run_encoder_check(&spec, &*sink);
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!(%err, "cannot start the encoder check");
            None
        }
    }
}

pub fn run_encoder_check(spec: &EncoderCheckSpec, sink: &dyn Fn(UiMsg)) -> Option<EncoderCheck> {
    let fingerprint = spec.media.encoder_fingerprint()?;
    let key = encoder::check_key(&fingerprint, &spec.cfg, &spec.version);
    let mut check = match load(&spec.cache) {
        Some(cached) if cached.key == key && cached.outcome != CheckOutcome::Failed => {
            tracing::info!(encoder = %cached.encoder, "encoder check cached");
            cached
        }
        _ => fresh(spec, key),
    };
    sink(UiMsg::Encoder(check.clone()));
    if check.wants_notice() {
        sink(UiMsg::NoticeWithAction {
            level: NoticeLevel::Warn,
            text: SOFTWARE_NOTICE.to_owned(),
            action: NoticeAction::Link(Problem::NoEncoder.help_url()),
        });
        check.noticed = true;
        save(&spec.cache, &check);
    }
    Some(check)
}

fn fresh(spec: &EncoderCheckSpec, key: String) -> EncoderCheck {
    let attempts = spec.media.check_encoders(&spec.cfg, false);
    for attempt in &attempts {
        match &attempt.result {
            Ok(detail) => {
                tracing::info!(encoder = %attempt.label(), adapter = %attempt.adapter, %detail, "encoder check passed")
            }
            Err(error) => {
                tracing::warn!(encoder = %attempt.label(), adapter = %attempt.adapter, %error, "encoder check failed")
            }
        }
    }
    let check = EncoderCheck::from_attempts(key, &attempts);
    match check.outcome {
        CheckOutcome::Failed => tracing::error!(detail = %check.detail, "no video encoder works"),
        _ => tracing::info!(encoder = %check.encoder, "encoder check done"),
    }
    save(&spec.cache, &check);
    check
}

fn load(path: &Path) -> Option<EncoderCheck> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(check) => Some(check),
        Err(err) => {
            tracing::warn!(%err, path = %path.display(), "encoder check cache unreadable");
            None
        }
    }
}

fn save(path: &Path, check: &EncoderCheck) {
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            let json = serde_json::to_string_pretty(check).map_err(std::io::Error::other)?;
            std::fs::write(path, json)
        });
    if let Err(err) = written {
        tracing::warn!(%err, path = %path.display(), "cannot save the encoder check");
    }
}

#[cfg(test)]
mod tests;

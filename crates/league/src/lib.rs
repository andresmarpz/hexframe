pub(crate) mod client;
pub mod ddragon;
pub(crate) mod dto;
pub(crate) mod feedback;
pub mod locator;
pub(crate) mod parse;
pub(crate) mod poller;
pub(crate) mod updates;

pub use client::{ClientError, LiveClient};
pub use ddragon::{ChampionIcons, ChampionIndex, DdragonError, SyncReport};
pub use feedback::{FeedbackClient, FeedbackError};
pub use locator::{AlwaysPresent, GameLocator};
pub use poller::{LeagueMsg, Poller, PollerHandle};
pub use updates::{GithubReleases, UpdateError};

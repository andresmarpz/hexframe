use std::cmp::Ordering;

pub const REPO: &str = "andresmarpz/greyscreen";
pub const EXE_ASSET: &str = "greyscreen.exe";

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreId {
    Number(u64),
    Text(String),
}

impl Ord for PreId {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (PreId::Number(a), PreId::Number(b)) => a.cmp(b),
            (PreId::Number(_), PreId::Text(_)) => Ordering::Less,
            (PreId::Text(_), PreId::Number(_)) => Ordering::Greater,
            (PreId::Text(a), PreId::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for PreId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<PreId>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split_once('+').map_or(text, |(core, _)| core);
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let major = number(parts.next()?)?;
        let minor = number(parts.next()?)?;
        let patch = number(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            Some(pre) => pre.split('.').map(pre_id).collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(Version {
            major,
            minor,
            patch,
            pre,
        })
    }

    pub(crate) fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

fn number(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

fn pre_id(text: &str) -> Option<PreId> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return None;
    }
    Some(match number(text) {
        Some(n) => PreId::Number(n),
        None => PreId::Text(text.to_owned()),
    })
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let core =
            (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        core.then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.pre.cmp(&other.pre),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

pub fn parse_digest(digest: &str) -> Option<String> {
    let hex = digest.strip_prefix("sha256:")?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hex.to_lowercase())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub prerelease: bool,
    pub published: Option<String>,
    pub exe: Option<Asset>,
}

impl Release {
    pub fn version(&self) -> Option<Version> {
        Version::parse(&self.tag)
    }

    fn is_prerelease(&self, version: &Version) -> bool {
        self.prerelease || version.is_prerelease()
    }
}

pub fn newest_update<'a>(running: &Version, releases: &'a [Release]) -> Option<&'a Release> {
    let include_previews = running.is_prerelease();
    releases
        .iter()
        .filter(|release| release.exe.is_some())
        .filter_map(|release| Some((release.version()?, release)))
        .filter(|(version, release)| include_previews || !release.is_prerelease(version))
        .filter(|(version, _)| version > running)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, release)| release)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub tag: String,
    pub prerelease: bool,
    pub size: u64,
    pub notes_url: String,
}

impl UpdateInfo {
    pub fn from_release(release: &Release) -> Self {
        let version = release.version();
        Self {
            tag: release.tag.clone(),
            prerelease: version
                .as_ref()
                .is_some_and(|version| release.is_prerelease(version)),
            size: release.exe.as_ref().map_or(0, |asset| asset.size),
            notes_url: notes_url(&release.tag),
        }
    }

    pub fn version(&self) -> &str {
        self.tag.strip_prefix('v').unwrap_or(&self.tag)
    }
}

pub fn notes_url(tag: &str) -> String {
    format!("https://github.com/{REPO}/releases/tag/{tag}")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpdateState {
    #[default]
    Unknown,
    Checking,
    UpToDate,
    AvailableManual(UpdateInfo),
    Available(UpdateInfo),
    Downloading(UpdateInfo),
    Ready(UpdateInfo),
    Installing(UpdateInfo),
    Failed(String),
}

impl UpdateState {
    pub fn info(&self) -> Option<&UpdateInfo> {
        match self {
            UpdateState::AvailableManual(info)
            | UpdateState::Available(info)
            | UpdateState::Downloading(info)
            | UpdateState::Ready(info)
            | UpdateState::Installing(info) => Some(info),
            _ => None,
        }
    }
}

const DOWNLOAD_PREFIX: &str = "greyscreen-";
const LEGACY_DOWNLOAD_PREFIX: &str = "greyscreen-";

pub fn download_file_name(tag: &str) -> String {
    format!("{DOWNLOAD_PREFIX}{tag}.exe")
}

pub fn is_stale_download(file_name: &str, running: &Version) -> bool {
    let Some(tag) = file_name
        .strip_prefix(DOWNLOAD_PREFIX)
        .or_else(|| file_name.strip_prefix(LEGACY_DOWNLOAD_PREFIX))
        .and_then(|rest| rest.strip_suffix(".exe"))
    else {
        return false;
    };
    Version::parse(tag).is_some_and(|version| version <= *running)
}

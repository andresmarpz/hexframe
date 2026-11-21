#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    Recording,
    Review,
    General,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Recording, Group::Review, Group::General];

    pub const fn label(self) -> &'static str {
        match self {
            Group::Recording => "Recording",
            Group::Review => "Review",
            Group::General => "General",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    App,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub scope: Scope,
    pub defaults: &'static [&'static str],
    pub fixed: bool,
}

impl Spec {
    const fn new(
        id: &'static str,
        label: &'static str,
        group: Group,
        scope: Scope,
        defaults: &'static [&'static str],
    ) -> Spec {
        Spec {
            id,
            label,
            group,
            scope,
            defaults,
            fixed: false,
        }
    }

    const fn fixed(mut self) -> Spec {
        self.fixed = true;
        self
    }
}

macro_rules! actions {
    ($($variant:ident => $spec:expr,)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Action {
            $($variant),+
        }

        impl Action {
            pub const ALL: &'static [Action] = &[$(Action::$variant),+];

            pub fn spec(self) -> &'static Spec {
                match self {
                    $(Action::$variant => {
                        const SPEC: Spec = $spec;
                        &SPEC
                    })+
                }
            }
        }
    };
}

actions! {
    SaveClip => Spec::new("save_clip", "Save clip", Group::Recording, Scope::Global, &["F9"]),
    PlayPause => Spec::new("play_pause", "Play or pause", Group::Review, Scope::App, &["Space"]),
    SeekBack => Spec::new("seek_back", "Seek back 5 seconds", Group::Review, Scope::App, &["Left", "J"]),
    SeekForward => Spec::new("seek_forward", "Seek forward 5 seconds", Group::Review, Scope::App, &["Right", "L"]),
    PreviousEvent => Spec::new("previous_event", "Previous event", Group::Review, Scope::App, &["Shift+Left"]),
    NextEvent => Spec::new("next_event", "Next event", Group::Review, Scope::App, &["Shift+Right"]),
    FrameBack => Spec::new("frame_back", "Frame back", Group::Review, Scope::App, &[","]),
    FrameForward => Spec::new("frame_forward", "Frame forward", Group::Review, Scope::App, &["."]),
    SpeedDown => Spec::new("speed_down", "Slower", Group::Review, Scope::App, &["["]),
    SpeedUp => Spec::new("speed_up", "Faster", Group::Review, Scope::App, &["]"]),
    Mute => Spec::new("mute", "Mute", Group::Review, Scope::App, &["M"]),
    ToggleFullscreen => Spec::new("toggle_fullscreen", "Toggle fullscreen", Group::Review, Scope::App, &["F"]),
    ExitFullscreen => Spec::new("exit_fullscreen", "Exit fullscreen or close the game", Group::Review, Scope::App, &["Escape"]).fixed(),
    OpenSettings => Spec::new("open_settings", "Open settings", Group::General, Scope::App, &["Mod+,"]),
    PreviousGame => Spec::new("previous_game", "Previous game", Group::General, Scope::App, &["Up"]),
    NextGame => Spec::new("next_game", "Next game", Group::General, Scope::App, &["Down"]),
    FocusPlayer => Spec::new("focus_player", "Focus the player", Group::General, Scope::App, &["Enter"]),
    ToggleSidebar => Spec::new("toggle_sidebar", "Show or hide the games list", Group::General, Scope::App, &["Mod+Shift+B"]),
    FocusSearch => Spec::new("focus_search", "Search games", Group::General, Scope::App, &["Mod+F"]),
}

impl Action {
    pub fn id(self) -> &'static str {
        self.spec().id
    }

    pub fn label(self) -> &'static str {
        self.spec().label
    }

    pub fn group(self) -> Group {
        self.spec().group
    }

    pub fn scope(self) -> Scope {
        self.spec().scope
    }

    pub fn from_id(id: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.id() == id)
    }
}

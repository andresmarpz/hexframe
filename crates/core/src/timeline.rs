use crate::game::{GameEvent, GameInfo};
use crate::highlight::{Highlight, Objective, Structure, classify};
use crate::library::EventEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Multikill {
    Double,
    Triple,
    Quadra,
    Penta,
}

impl Multikill {
    pub fn from_streak(streak: u8) -> Option<Self> {
        match streak {
            0 | 1 => None,
            2 => Some(Self::Double),
            3 => Some(Self::Triple),
            4 => Some(Self::Quadra),
            _ => Some(Self::Penta),
        }
    }

    pub fn streak(self) -> u8 {
        match self {
            Self::Double => 2,
            Self::Triple => 3,
            Self::Quadra => 4,
            Self::Penta => 5,
        }
    }

    pub fn caption(self) -> &'static str {
        match self {
            Self::Double => "Double Kill",
            Self::Triple => "Triple Kill",
            Self::Quadra => "Quadra Kill",
            Self::Penta => "Penta Kill",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveKind {
    Dragon,
    Elder,
    Baron,
    Herald,
    Grubs,
    Tower,
    Inhibitor,
}

impl ObjectiveKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dragon => "Dragon",
            Self::Elder => "Elder Dragon",
            Self::Baron => "Baron",
            Self::Herald => "Herald",
            Self::Grubs => "Grubs",
            Self::Tower => "Tower",
            Self::Inhibitor => "Inhibitor",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Baron => 0,
            Self::Elder => 1,
            Self::Dragon => 2,
            Self::Herald => 3,
            Self::Grubs => 4,
            Self::Inhibitor => 5,
            Self::Tower => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Mine,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxKind {
    Empty,
    Objective {
        kind: ObjectiveKind,
        side: Side,
    },
    Kill {
        kills: u32,
        multikill: Option<Multikill>,
    },
    Death,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinuteBox {
    pub minute: u32,
    pub kind: BoxKind,
    pub events: Vec<String>,
}

impl MinuteBox {
    pub fn caption(&self) -> Option<&'static str> {
        match self.kind {
            BoxKind::Kill {
                multikill: Some(multikill),
                ..
            } => Some(multikill.caption()),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Minute {
    kills: u32,
    deaths: u32,
    multikill: Option<Multikill>,
    objectives: Vec<(ObjectiveKind, Side)>,
    names: Vec<String>,
}

impl Minute {
    fn add(&mut self, event: &EventEntry, info: &GameInfo) {
        for highlight in classify(&event.kind, info) {
            match highlight {
                Highlight::MyKill => {
                    self.kills += 1;
                    self.names.push("Kill".to_owned());
                }
                Highlight::MyDeath => {
                    self.deaths += 1;
                    self.names.push("Death".to_owned());
                }
                Highlight::MyMultikill { streak } => {
                    if let Some(multikill) = Multikill::from_streak(streak) {
                        self.multikill = self.multikill.max(Some(multikill));
                        self.names.push(multikill.caption().to_owned());
                    }
                }
                Highlight::MyFirstBlood => self.names.push("First Blood".to_owned()),
                Highlight::TeamObjective(objective) => {
                    self.objective(objective_kind(&objective), Side::Mine);
                }
                Highlight::EnemyObjective(objective) => {
                    self.objective(objective_kind(&objective), Side::Enemy);
                }
                Highlight::TeamStructure(structure) => {
                    self.objective(structure_kind(structure), Side::Mine);
                }
                Highlight::EnemyStructure(structure) => {
                    self.objective(structure_kind(structure), Side::Enemy);
                }
                _ => {}
            }
        }
    }

    fn objective(&mut self, kind: ObjectiveKind, side: Side) {
        self.objectives.push((kind, side));
        self.names.push(match side {
            Side::Mine => kind.name().to_owned(),
            Side::Enemy => format!("Enemy {}", kind.name()),
        });
    }

    fn finish(mut self, minute: u32) -> MinuteBox {
        let kind = if self.deaths > 0 {
            BoxKind::Death
        } else if self.kills > 0 || self.multikill.is_some() {
            BoxKind::Kill {
                kills: self.kills.max(1),
                multikill: self.multikill,
            }
        } else {
            self.objectives
                .iter()
                .min_by_key(|(kind, side)| (kind.rank(), *side == Side::Enemy))
                .map_or(BoxKind::Empty, |&(kind, side)| BoxKind::Objective {
                    kind,
                    side,
                })
        };
        if self.multikill.is_some() {
            self.names.retain(|name| name != "Kill");
        }
        MinuteBox {
            minute,
            kind,
            events: collapse(self.names),
        }
    }
}

fn collapse(names: Vec<String>) -> Vec<String> {
    let mut counted: Vec<(String, u32)> = Vec::new();
    for name in names {
        match counted.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, count)) => *count += 1,
            None => counted.push((name, 1)),
        }
    }
    counted
        .into_iter()
        .map(|(name, count)| {
            if count > 1 {
                format!("{name} ×{count}")
            } else {
                name
            }
        })
        .collect()
}

fn objective_kind(objective: &Objective) -> ObjectiveKind {
    match objective {
        Objective::Dragon(kind) if kind.eq_ignore_ascii_case("elder") => ObjectiveKind::Elder,
        Objective::Dragon(_) => ObjectiveKind::Dragon,
        Objective::Herald => ObjectiveKind::Herald,
        Objective::Baron => ObjectiveKind::Baron,
        Objective::VoidGrub => ObjectiveKind::Grubs,
    }
}

fn structure_kind(structure: Structure) -> ObjectiveKind {
    match structure {
        Structure::Turret => ObjectiveKind::Tower,
        Structure::Inhibitor => ObjectiveKind::Inhibitor,
    }
}

fn minute_of(game_secs: f64) -> usize {
    (game_secs.max(0.0) / 60.0).floor() as usize
}

pub fn minute_boxes(events: &[EventEntry], info: &GameInfo, duration_secs: f64) -> Vec<MinuteBox> {
    let offset = events
        .first()
        .map_or(0.0, |event| event.rec_secs - event.game_secs);
    let last_event = events
        .iter()
        .map(|event| event.game_secs)
        .fold(0.0, f64::max);
    let length = (duration_secs - offset).max(last_event);
    let count = minute_of(length) + 1;
    let mut minutes: Vec<Minute> = (0..count).map(|_| Minute::default()).collect();
    for event in events {
        minutes[minute_of(event.game_secs).min(count - 1)].add(event, info);
    }
    minutes
        .into_iter()
        .zip(0u32..)
        .map(|(minute, index)| minute.finish(index))
        .collect()
}

pub fn kill_participation(events: &[EventEntry], info: &GameInfo) -> Option<u32> {
    let mut team_kills = 0u32;
    let mut mine = 0u32;
    for event in events {
        let GameEvent::ChampionKill {
            killer, assisters, ..
        } = &event.kind
        else {
            continue;
        };
        let my_kill = info.is_me(killer);
        let team_kill = my_kill
            || info
                .player_named(killer)
                .is_some_and(|player| player.team == info.me.team);
        team_kills += u32::from(team_kill);
        mine += u32::from(my_kill || assisters.iter().any(|name| info.is_me(name)));
    }
    (team_kills > 0)
        .then(|| (f64::from(mine.min(team_kills)) * 100.0 / f64::from(team_kills)).round() as u32)
}

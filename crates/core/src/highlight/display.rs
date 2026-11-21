#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    Big,
    Death,
    Kill,
    Team,
    Enemy,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelDisplay {
    pub title: &'static str,
    pub tone: Tone,
    rank: u8,
}

const fn shown(title: &'static str, tone: Tone, rank: u8) -> LabelDisplay {
    LabelDisplay { title, tone, rank }
}

pub fn describe_label(label: &str) -> LabelDisplay {
    match label {
        "Multikill5" => shown("Pentakill", Tone::Big, 1),
        "Multikill4" => shown("Quadra kill", Tone::Big, 2),
        "Multikill3" => shown("Triple kill", Tone::Big, 3),
        "Multikill2" => shown("Double kill", Tone::Big, 4),
        "Multikill" => shown("Multikill", Tone::Big, 5),
        "Ace" => shown("Ace", Tone::Big, 6),
        "FirstBlood" => shown("First blood", Tone::Big, 7),
        "Victory" => shown("Victory", Tone::Big, 8),
        "Defeat" => shown("Defeat", Tone::Enemy, 8),
        "MyDeath" => shown("Death", Tone::Death, 10),
        "MyKill" => shown("Kill", Tone::Kill, 11),
        "MyAssist" => shown("Assist", Tone::Kill, 12),
        "Baron" => shown("Baron", Tone::Team, 20),
        "Dragon" => shown("Dragon", Tone::Team, 21),
        "Herald" => shown("Herald", Tone::Team, 22),
        "VoidGrub" => shown("Void grubs", Tone::Team, 23),
        "Inhibitor" => shown("Inhibitor taken", Tone::Team, 24),
        "Turret" => shown("Turret taken", Tone::Team, 25),
        "EnemyBaron" => shown("Enemy baron", Tone::Enemy, 30),
        "EnemyDragon" => shown("Enemy dragon", Tone::Enemy, 31),
        "EnemyHerald" => shown("Enemy herald", Tone::Enemy, 32),
        "EnemyVoidGrub" => shown("Enemy void grubs", Tone::Enemy, 33),
        "EnemyInhibitor" => shown("Inhibitor lost", Tone::Enemy, 34),
        "EnemyTurret" => shown("Turret lost", Tone::Enemy, 35),
        "EnemyAce" => shown("Enemy ace", Tone::Enemy, 36),
        "AllyKill" => shown("Ally kill", Tone::Neutral, 40),
        "EnemyKill" => shown("Enemy kill", Tone::Neutral, 41),
        _ => shown("Event", Tone::Neutral, 99),
    }
}

pub fn dominant_label<'a>(labels: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    labels
        .into_iter()
        .min_by_key(|label| describe_label(label).rank)
}

#[cfg(test)]
mod tests;

use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameUsage {
    pub id: String,
    pub started_at: Timestamp,
    pub video_bytes: u64,
    pub other_bytes: u64,
}

impl GameUsage {
    pub fn total_bytes(&self) -> u64 {
        self.video_bytes + self.other_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deletion {
    Video { id: String },
    Game { id: String },
}

impl Deletion {
    pub fn id(&self) -> &str {
        match self {
            Deletion::Video { id } | Deletion::Game { id } => id,
        }
    }
}

pub fn plan(games: &[GameUsage], limit_bytes: u64, protected: Option<&str>) -> Vec<Deletion> {
    let mut total: u64 = games.iter().map(GameUsage::total_bytes).sum();
    if total <= limit_bytes {
        return Vec::new();
    }
    let mut candidates: Vec<&GameUsage> = games
        .iter()
        .filter(|game| Some(game.id.as_str()) != protected)
        .collect();
    candidates.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut plan = Vec::new();
    let mut freed_video = vec![false; candidates.len()];
    for (i, game) in candidates.iter().enumerate() {
        if total <= limit_bytes {
            return plan;
        }
        if game.video_bytes > 0 {
            plan.push(Deletion::Video {
                id: game.id.clone(),
            });
            total -= game.video_bytes;
            freed_video[i] = true;
        }
    }
    for (i, game) in candidates.iter().enumerate() {
        if total <= limit_bytes {
            break;
        }
        let remaining = if freed_video[i] {
            game.other_bytes
        } else {
            game.total_bytes()
        };
        plan.retain(|deletion| deletion.id() != game.id);
        plan.push(Deletion::Game {
            id: game.id.clone(),
        });
        total -= remaining;
    }
    plan
}

#[cfg(test)]
mod tests;

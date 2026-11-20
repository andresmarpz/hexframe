use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameStats {
    pub game_mode: String,
    pub game_time: Option<f64>,
    pub map_name: String,
    pub map_number: Option<u32>,
    pub map_terrain: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EventData {
    #[serde(rename = "Events")]
    pub events: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlayerEntry {
    pub champion_name: String,
    pub raw_champion_name: String,
    pub is_bot: bool,
    pub summoner_name: String,
    #[serde(rename = "riotId")]
    pub riot_id: String,
    #[serde(rename = "riotIdGameName")]
    pub riot_id_game_name: String,
    #[serde(rename = "riotIdTagLine")]
    pub riot_id_tag_line: String,
    pub team: String,
}

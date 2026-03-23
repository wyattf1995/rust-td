use bevy::prelude::*;
use std::collections::HashMap;

const STORAGE_KEY: &str = "neon_command_highscores";
const SETTINGS_KEY: &str = "neon_command_settings";

// ── Game Settings ──

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticleDensity {
    Low,
    Medium,
    High,
}

impl ParticleDensity {
    pub fn name(&self) -> &str {
        match self {
            ParticleDensity::Low => "LOW",
            ParticleDensity::Medium => "MED",
            ParticleDensity::High => "HIGH",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ParticleDensity::Low => ParticleDensity::Medium,
            ParticleDensity::Medium => ParticleDensity::High,
            ParticleDensity::High => ParticleDensity::Low,
        }
    }

    pub fn death_particle_count(&self) -> usize {
        match self {
            ParticleDensity::Low => 3,
            ParticleDensity::Medium => 5,
            ParticleDensity::High => 10,
        }
    }

    pub fn trail_skip(&self) -> u32 {
        match self {
            ParticleDensity::Low => 3,
            ParticleDensity::Medium => 1,
            ParticleDensity::High => 1,
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "LOW" => ParticleDensity::Low,
            "MED" => ParticleDensity::Medium,
            "HIGH" => ParticleDensity::High,
            _ => ParticleDensity::Medium,
        }
    }
}

#[derive(Resource, Clone)]
pub struct GameSettings {
    pub screen_shake: bool,
    pub particle_density: ParticleDensity,
    pub show_damage_numbers: bool,
    pub show_range_on_hover: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            screen_shake: true,
            particle_density: ParticleDensity::Medium,
            show_damage_numbers: true,
            show_range_on_hover: true,
        }
    }
}

impl GameSettings {
    fn to_json(&self) -> String {
        format!(
            r#"{{"shake":{},"particles":"{}","damage_numbers":{},"range_hover":{}}}"#,
            self.screen_shake,
            self.particle_density.name(),
            self.show_damage_numbers,
            self.show_range_on_hover
        )
    }

    fn from_json(json: &str) -> Self {
        let mut settings = Self::default();
        let trimmed = json.trim().trim_start_matches('{').trim_end_matches('}');
        for part in trimmed.split(',') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("\"shake\":") {
                settings.screen_shake = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"particles\":") {
                let name = val.trim().trim_matches('"');
                settings.particle_density = ParticleDensity::from_str(name);
            } else if let Some(val) = part.strip_prefix("\"damage_numbers\":") {
                settings.show_damage_numbers = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"range_hover\":") {
                settings.show_range_on_hover = val.trim() == "true";
            }
        }
        settings
    }
}

/// Resource to toggle settings overlay visibility
#[derive(Resource, Default)]
pub struct SettingsOpen(pub bool);

#[cfg(target_arch = "wasm32")]
pub fn load_settings() -> GameSettings {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if let Ok(Some(json)) = storage.get_item(SETTINGS_KEY) {
            return GameSettings::from_json(&json);
        }
    }
    GameSettings::default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_settings(settings: &GameSettings) {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if storage.set_item(SETTINGS_KEY, &settings.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save settings (storage full or disabled)".into());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_settings() -> GameSettings {
    GameSettings::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_settings(_settings: &GameSettings) {}

#[derive(Clone, Debug)]
pub struct HighScoreEntry {
    pub wave: usize,
    pub score: u32,
}

#[derive(Resource, Default, Debug)]
pub struct HighScores {
    pub scores: HashMap<String, HighScoreEntry>,
}

impl HighScores {
    pub fn get(&self, map_name: &str) -> Option<&HighScoreEntry> {
        self.scores.get(map_name)
    }

    /// Returns true if this is a new record.
    pub fn update_if_better(&mut self, map_name: &str, wave: usize, score: u32) -> bool {
        if let Some(existing) = self.scores.get(map_name) {
            if wave > existing.wave || (wave == existing.wave && score > existing.score) {
                self.scores.insert(
                    map_name.to_string(),
                    HighScoreEntry { wave, score },
                );
                return true;
            }
            false
        } else {
            self.scores.insert(
                map_name.to_string(),
                HighScoreEntry { wave, score },
            );
            true
        }
    }

    /// Manual JSON serialization (no serde dependency).
    fn to_json(&self) -> String {
        let mut json = String::from("{");
        for (i, (key, entry)) in self.scores.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "\"{}\":{{\"wave\":{},\"score\":{}}}",
                key, entry.wave, entry.score
            ));
        }
        json.push('}');
        json
    }

    /// Manual JSON deserialization (no serde dependency).
    fn from_json(json: &str) -> Self {
        let mut scores = HashMap::new();
        let trimmed = json.trim().trim_start_matches('{').trim_end_matches('}');
        if trimmed.is_empty() {
            return Self { scores };
        }

        // Split by entries: "MapName":{"wave":N,"score":N}
        // We parse by finding each top-level key-value pair
        let mut chars = trimmed.chars().peekable();
        loop {
            // Skip whitespace and commas
            while chars.peek().is_some_and(|c| *c == ' ' || *c == ',' || *c == '\n') {
                chars.next();
            }
            if chars.peek().is_none() {
                break;
            }

            // Parse key
            if chars.next() != Some('"') {
                break;
            }
            let key: String = chars.by_ref().take_while(|c| *c != '"').collect();

            // Skip colon
            while chars.peek().is_some_and(|c| *c == ':' || *c == ' ') {
                chars.next();
            }

            // Parse value object {...}
            if chars.next() != Some('{') {
                break;
            }
            let obj_str: String = chars.by_ref().take_while(|c| *c != '}').collect();

            // Parse wave and score from obj_str like "wave":15,"score":4280
            let mut wave: usize = 0;
            let mut score: u32 = 0;
            for part in obj_str.split(',') {
                let part = part.trim();
                if let Some(val) = part.strip_prefix("\"wave\":") {
                    wave = val.trim().parse().unwrap_or(0);
                } else if let Some(val) = part.strip_prefix("\"score\":") {
                    score = val.trim().parse().unwrap_or(0);
                }
            }

            scores.insert(key, HighScoreEntry { wave, score });
        }

        Self { scores }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_highscores() -> HighScores {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();

    if let Some(storage) = storage {
        if let Ok(Some(json)) = storage.get_item(STORAGE_KEY) {
            return HighScores::from_json(&json);
        }
    }
    HighScores::default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_highscores(high_scores: &HighScores) {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();

    if let Some(storage) = storage {
        if storage.set_item(STORAGE_KEY, &high_scores.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save high scores (storage full or disabled)".into());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_highscores() -> HighScores {
    HighScores::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_highscores(_high_scores: &HighScores) {
    // No-op on native
}

fn init_persistence(mut commands: Commands) {
    commands.insert_resource(load_highscores());
    commands.insert_resource(load_settings());
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .init_resource::<GameSettings>()
            .init_resource::<SettingsOpen>()
            .add_systems(Startup, init_persistence);
    }
}

use bevy::prelude::*;
use std::collections::HashMap;

const STORAGE_KEY: &str = "neon_command_highscores";

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
            while chars.peek().map_or(false, |c| *c == ' ' || *c == ',' || *c == '\n') {
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
            while chars.peek().map_or(false, |c| *c == ':' || *c == ' ') {
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
        let _ = storage.set_item(STORAGE_KEY, &high_scores.to_json());
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

fn init_highscores(mut commands: Commands) {
    let scores = load_highscores();
    commands.insert_resource(scores);
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .add_systems(Startup, init_highscores);
    }
}

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
    pub reduce_motion: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            screen_shake: true,
            particle_density: ParticleDensity::Medium,
            show_damage_numbers: true,
            show_range_on_hover: true,
            reduce_motion: false,
        }
    }
}

impl GameSettings {
    fn to_json(&self) -> String {
        format!(
            r#"{{"shake":{},"particles":"{}","damage_numbers":{},"range_hover":{},"reduce_motion":{}}}"#,
            self.screen_shake,
            self.particle_density.name(),
            self.show_damage_numbers,
            self.show_range_on_hover,
            self.reduce_motion
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
            } else if let Some(val) = part.strip_prefix("\"reduce_motion\":") {
                settings.reduce_motion = val.trim() == "true";
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
pub fn save_settings(settings: &GameSettings) -> bool {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if storage.set_item(SETTINGS_KEY, &settings.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save settings (storage full or disabled)".into());
            return false;
        }
        true
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_settings() -> GameSettings {
    GameSettings::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_settings(_settings: &GameSettings) -> bool { true }

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
pub fn save_highscores(high_scores: &HighScores) -> bool {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();

    if let Some(storage) = storage {
        if storage.set_item(STORAGE_KEY, &high_scores.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save high scores (storage full or disabled)".into());
            return false;
        }
        true
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_highscores() -> HighScores {
    HighScores::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_highscores(_high_scores: &HighScores) -> bool {
    true
}

// ── Lifetime Stats ──

const LIFETIME_KEY: &str = "neon_command_lifetime";

#[derive(Resource, Clone, Debug)]
pub struct LifetimeStats {
    pub total_games: u32,
    pub total_kills: u32,
    pub total_waves: u32,
    pub total_gold_earned: u32,
    pub best_wave_ever: usize,
}

impl Default for LifetimeStats {
    fn default() -> Self {
        Self {
            total_games: 0,
            total_kills: 0,
            total_waves: 0,
            total_gold_earned: 0,
            best_wave_ever: 0,
        }
    }
}

impl LifetimeStats {
    fn to_json(&self) -> String {
        format!(
            r#"{{"games":{},"kills":{},"waves":{},"gold":{},"best_wave":{}}}"#,
            self.total_games, self.total_kills, self.total_waves,
            self.total_gold_earned, self.best_wave_ever
        )
    }

    fn from_json(json: &str) -> Self {
        let mut stats = Self::default();
        let trimmed = json.trim().trim_start_matches('{').trim_end_matches('}');
        for part in trimmed.split(',') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("\"games\":") {
                stats.total_games = val.trim().parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("\"kills\":") {
                stats.total_kills = val.trim().parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("\"waves\":") {
                stats.total_waves = val.trim().parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("\"gold\":") {
                stats.total_gold_earned = val.trim().parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("\"best_wave\":") {
                stats.best_wave_ever = val.trim().parse().unwrap_or(0);
            }
        }
        stats
    }

    pub fn record_game(&mut self, kills: u32, waves: usize, gold_earned: u32) {
        self.total_games += 1;
        self.total_kills += kills;
        self.total_waves += waves as u32;
        self.total_gold_earned += gold_earned;
        self.best_wave_ever = self.best_wave_ever.max(waves);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_lifetime_stats() -> LifetimeStats {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if let Ok(Some(json)) = storage.get_item(LIFETIME_KEY) {
            return LifetimeStats::from_json(&json);
        }
    }
    LifetimeStats::default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_lifetime_stats(stats: &LifetimeStats) -> bool {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if storage.set_item(LIFETIME_KEY, &stats.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save lifetime stats (storage full or disabled)".into());
            return false;
        }
        true
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_lifetime_stats() -> LifetimeStats {
    LifetimeStats::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_lifetime_stats(_stats: &LifetimeStats) -> bool { true }

// ── Tips Tracking ──

const TIPS_KEY: &str = "neon_command_tips";

/// Tracks which contextual tips have been shown (persisted across sessions).
#[derive(Resource, Clone, Debug)]
pub struct TipsShown {
    pub specialization: bool,
    pub synergy: bool,
    pub early_send: bool,
    pub targeting: bool,
    pub welcome: bool,
    pub economy: bool,
}

impl Default for TipsShown {
    fn default() -> Self {
        Self {
            specialization: false,
            synergy: false,
            early_send: false,
            targeting: false,
            welcome: false,
            economy: false,
        }
    }
}

impl TipsShown {
    fn to_json(&self) -> String {
        format!(
            r#"{{"spec":{},"syn":{},"early":{},"targ":{},"welcome":{},"economy":{}}}"#,
            self.specialization, self.synergy, self.early_send, self.targeting,
            self.welcome, self.economy
        )
    }

    fn from_json(json: &str) -> Self {
        let mut tips = Self::default();
        let trimmed = json.trim().trim_start_matches('{').trim_end_matches('}');
        for part in trimmed.split(',') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("\"spec\":") {
                tips.specialization = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"syn\":") {
                tips.synergy = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"early\":") {
                tips.early_send = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"targ\":") {
                tips.targeting = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"welcome\":") {
                tips.welcome = val.trim() == "true";
            } else if let Some(val) = part.strip_prefix("\"economy\":") {
                tips.economy = val.trim() == "true";
            }
        }
        tips
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_tips() -> TipsShown {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if let Ok(Some(json)) = storage.get_item(TIPS_KEY) {
            return TipsShown::from_json(&json);
        }
    }
    TipsShown::default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_tips(tips: &TipsShown) -> bool {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten();
    if let Some(storage) = storage {
        if storage.set_item(TIPS_KEY, &tips.to_json()).is_err() {
            web_sys::console::warn_1(&"Failed to save tips to localStorage".into());
            return false;
        }
        true
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_tips() -> TipsShown {
    TipsShown::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_tips(_tips: &TipsShown) -> bool { true }

/// Warning toast when localStorage save fails.
#[derive(Resource, Default)]
pub struct SaveWarning {
    pub timer: Option<Timer>,
}

impl SaveWarning {
    pub fn trigger(&mut self) {
        self.timer = Some(Timer::from_seconds(8.0, TimerMode::Once));
    }
}

fn init_persistence(mut commands: Commands) {
    commands.insert_resource(load_highscores());
    commands.insert_resource(load_settings());
    commands.insert_resource(load_lifetime_stats());
    commands.insert_resource(load_tips());
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .init_resource::<GameSettings>()
            .init_resource::<SettingsOpen>()
            .init_resource::<LifetimeStats>()
            .init_resource::<TipsShown>()
            .init_resource::<SaveWarning>()
            .add_systems(Startup, init_persistence);
    }
}

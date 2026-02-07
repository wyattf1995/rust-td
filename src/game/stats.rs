use bevy::prelude::*;
use std::collections::HashMap;

use super::enemy::EnemyType;
use super::tower::TowerType;

use crate::GameState;

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameStats>()
            .add_systems(OnEnter(GameState::Playing), reset_stats);
    }
}

fn reset_stats(mut stats: ResMut<GameStats>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    *stats = GameStats::default();
}

/// Per-tower tracking
#[derive(Debug, Default, Clone)]
pub struct TowerInstanceStats {
    pub tower_type: TowerType,
    pub max_level: u32,
    pub total_damage: f32,
    pub total_kills: u32,
    pub gold_invested: u32,
    pub gold_recovered: u32,
    pub was_sold: bool,
}

/// Per-wave gold breakdown
#[derive(Debug, Default, Clone)]
pub struct WaveGoldRecord {
    pub gold_from_kills: u32,
    pub gold_from_bonus: u32,
    pub gold_from_interest: u32,
    pub gold_from_early_send: u32,
    pub gold_spent: u32,
}

/// Game-wide statistics resource
#[derive(Resource, Debug, Default)]
pub struct GameStats {
    // Summary
    pub waves_survived: usize,
    pub total_score: u32,
    pub total_gold_earned: u32,
    pub total_gold_spent: u32,
    pub total_enemies_killed: u32,
    pub total_enemies_escaped: u32,
    pub max_combo: u32,

    // Per-tower (keyed by sequential TowerId)
    pub next_tower_id: u32,
    pub entity_to_tower_id: HashMap<Entity, u32>,
    pub tower_stats: HashMap<u32, TowerInstanceStats>,

    // Per-wave gold timeline
    pub wave_gold: Vec<WaveGoldRecord>,
    pub current_wave_gold: WaveGoldRecord,

    // Kill counts by enemy type
    pub kills_by_type: HashMap<u8, u32>,

    // Tower placement counts
    pub placements_by_type: HashMap<u8, u32>,
}

impl GameStats {
    pub fn register_tower(&mut self, entity: Entity, tower_type: TowerType, cost: u32) {
        let id = self.next_tower_id;
        self.next_tower_id += 1;
        self.entity_to_tower_id.insert(entity, id);
        self.tower_stats.insert(id, TowerInstanceStats {
            tower_type,
            max_level: 1,
            total_damage: 0.0,
            total_kills: 0,
            gold_invested: cost,
            gold_recovered: 0,
            was_sold: false,
        });
        self.total_gold_spent += cost;
        self.current_wave_gold.gold_spent += cost;

        let key = tower_type as u8;
        *self.placements_by_type.entry(key).or_insert(0) += 1;
    }

    pub fn record_upgrade(&mut self, entity: Entity, cost: u32, new_level: u32) {
        self.total_gold_spent += cost;
        self.current_wave_gold.gold_spent += cost;
        if let Some(&id) = self.entity_to_tower_id.get(&entity) {
            if let Some(ts) = self.tower_stats.get_mut(&id) {
                ts.gold_invested += cost;
                ts.max_level = ts.max_level.max(new_level);
            }
        }
    }

    pub fn record_sell(&mut self, entity: Entity, sell_value: u32) {
        self.total_gold_earned += sell_value;
        if let Some(&id) = self.entity_to_tower_id.get(&entity) {
            if let Some(ts) = self.tower_stats.get_mut(&id) {
                ts.gold_recovered = sell_value;
                ts.was_sold = true;
            }
        }
    }

    pub fn record_damage(&mut self, source_entity: Entity, damage: f32) {
        if let Some(&id) = self.entity_to_tower_id.get(&source_entity) {
            if let Some(ts) = self.tower_stats.get_mut(&id) {
                ts.total_damage += damage;
            }
        }
    }

    pub fn record_kill(&mut self, enemy_type: EnemyType, reward: u32) {
        self.total_enemies_killed += 1;
        self.total_gold_earned += reward;
        self.current_wave_gold.gold_from_kills += reward;

        let key = enemy_type as u8;
        *self.kills_by_type.entry(key).or_insert(0) += 1;
    }

    pub fn record_kill_tower(&mut self, source_entity: Entity) {
        if let Some(&id) = self.entity_to_tower_id.get(&source_entity) {
            if let Some(ts) = self.tower_stats.get_mut(&id) {
                ts.total_kills += 1;
            }
        }
    }

    pub fn record_escape(&mut self) {
        self.total_enemies_escaped += 1;
    }

    pub fn record_wave_complete(&mut self, bonus: u32, interest: u32) {
        self.waves_survived += 1;
        self.total_gold_earned += bonus + interest;
        self.current_wave_gold.gold_from_bonus += bonus;
        self.current_wave_gold.gold_from_interest += interest;

        // Finalize this wave and start a new record
        self.wave_gold.push(self.current_wave_gold.clone());
        self.current_wave_gold = WaveGoldRecord::default();
    }

    pub fn record_early_send(&mut self, amount: u32) {
        self.total_gold_earned += amount;
        self.current_wave_gold.gold_from_early_send += amount;
    }

    pub fn update_max_combo(&mut self, combo: u32) {
        self.max_combo = self.max_combo.max(combo);
    }

    pub fn finalize(&mut self, score: u32, waves: usize) {
        self.total_score = score;
        self.waves_survived = waves;
    }

    /// Find the tower with highest total damage
    pub fn best_tower(&self) -> Option<&TowerInstanceStats> {
        self.tower_stats.values().max_by(|a, b| {
            a.total_damage.partial_cmp(&b.total_damage).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn enemy_type_from_key(key: u8) -> Option<EnemyType> {
        match key {
            0 => Some(EnemyType::Basic),
            1 => Some(EnemyType::Fast),
            2 => Some(EnemyType::Tank),
            3 => Some(EnemyType::Armored),
            4 => Some(EnemyType::Flying),
            5 => Some(EnemyType::Boss),
            6 => Some(EnemyType::Splitter),
            7 => Some(EnemyType::MiniSplitter),
            _ => None,
        }
    }
}

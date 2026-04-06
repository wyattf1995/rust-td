use bevy::prelude::*;
use std::collections::HashMap;

use crate::analytics::{Analytics, track_with_context};
use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes, ZDepth};

use crate::loading::GameAssets;

use crate::persistence::GameSettings;

use super::{
    abilities::PlayerAbilities,
    ease_out,
    economy::{PlayerEconomy, KillStreak},
    map::GameMap,
    rand_simple,
    rules,
    stats::GameStats,
    GameEntity,
    ScreenShake,
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveManager>()
            .add_event::<EnemyKilledEvent>()
            .add_event::<EnemyEscapedEvent>()
            .add_event::<SpawnMiniSplittersEvent>()
            .add_systems(OnEnter(GameState::Playing), reset_wave_manager)
            .add_systems(
                Update,
                (
                    wave_spawner,
                    enemy_movement,
                    poison_tick,
                    enemy_health_check,
                    update_health_bars,
                    handle_enemy_killed,
                    spawn_mini_splitters,
                    handle_enemy_escaped,
                    check_wave_complete,
                    update_death_effects,
                    update_gold_numbers,
                    update_enemy_status_visuals,
                    update_flying_shadows,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Enemy types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EnemyType {
    Basic,
    Fast,
    Tank,
    Armored,      // Resistant to damage
    Flying,       // Ignores some path (takes shortcuts)
    Boss,         // Big and tough
    Splitter,     // Splits into mini-splitters on death
    MiniSplitter, // Small fast enemy spawned from Splitter
}

impl EnemyType {
    pub fn health(&self) -> f32 {
        match self {
            EnemyType::Basic => 115.0,
            EnemyType::Fast => 78.0,
            EnemyType::Tank => 385.0,
            EnemyType::Armored => 285.0,
            EnemyType::Flying => 98.0,
            EnemyType::Boss => 2300.0,
            EnemyType::Splitter => 180.0,
            EnemyType::MiniSplitter => 45.0,
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            EnemyType::Basic => 60.0,
            EnemyType::Fast => 110.0,
            EnemyType::Tank => 35.0,
            EnemyType::Armored => 42.0,
            EnemyType::Flying => 95.0,
            EnemyType::Boss => 30.0,
            EnemyType::Splitter => 45.0,
            EnemyType::MiniSplitter => 75.0,
        }
    }

    pub fn reward(&self) -> u32 {
        match self {
            EnemyType::Basic => 5,
            EnemyType::Fast => 8,
            EnemyType::Tank => 20,
            EnemyType::Armored => 16,
            EnemyType::Flying => 10,
            EnemyType::Boss => 120,
            EnemyType::Splitter => 12,
            EnemyType::MiniSplitter => 2,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            EnemyType::Basic => GameColors::ENEMY_BASIC,
            EnemyType::Fast => GameColors::ENEMY_FAST,
            EnemyType::Tank => GameColors::ENEMY_TANK,
            EnemyType::Armored => GameColors::ENEMY_ARMORED,
            EnemyType::Flying => GameColors::ENEMY_FLYING,
            EnemyType::Boss => GameColors::ENEMY_BOSS,
            EnemyType::Splitter => GameColors::ENEMY_SPLITTER,
            EnemyType::MiniSplitter => GameColors::ENEMY_MINI_SPLITTER,
        }
    }

    pub fn size(&self) -> f32 {
        match self {
            EnemyType::Basic => ShapeSizes::ENEMY_BASIC,
            EnemyType::Fast => ShapeSizes::ENEMY_FAST,
            EnemyType::Tank => ShapeSizes::ENEMY_TANK,
            EnemyType::Armored => ShapeSizes::ENEMY_ARMORED,
            EnemyType::Flying => ShapeSizes::ENEMY_FLYING,
            EnemyType::Boss => ShapeSizes::ENEMY_BOSS,
            EnemyType::Splitter => ShapeSizes::ENEMY_SPLITTER,
            EnemyType::MiniSplitter => ShapeSizes::ENEMY_MINI_SPLITTER,
        }
    }

    /// Armor reduces damage taken (0.0 = no armor, 0.4 = 40% damage reduction)
    pub fn armor(&self) -> f32 {
        match self {
            EnemyType::Armored => 0.4,
            EnemyType::Boss => 0.2,
            _ => 0.0,
        }
    }

    /// Whether this enemy can fly (skip path sections)
    pub fn is_flying(&self) -> bool {
        matches!(self, EnemyType::Flying)
    }

    /// Whether this enemy splits into smaller enemies on death
    pub fn is_splitter(&self) -> bool {
        matches!(self, EnemyType::Splitter)
    }
}

/// Enemy component
#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub health: f32,
    pub max_health: f32,
    pub speed: f32,
    pub base_speed: f32,
    pub reward: u32,
    pub path_index: usize,
    pub slow_timer: Option<Timer>,
    pub marked_dead: bool,
    pub poison_damage: f32,         // Damage per second from poison
    pub poison_timer: Option<Timer>, // Duration of poison effect
    pub bonus_armor: f32,           // Extra armor from wave modifiers
    pub last_hit_by: Option<Entity>, // Tower entity that last dealt damage (for stats)
}

impl Enemy {
    pub fn new(enemy_type: EnemyType) -> Self {
        Self {
            enemy_type,
            health: enemy_type.health(),
            max_health: enemy_type.health(),
            speed: enemy_type.speed(),
            base_speed: enemy_type.speed(),
            reward: enemy_type.reward(),
            path_index: 0,
            slow_timer: None,
            marked_dead: false,
            poison_damage: 0.0,
            poison_timer: None,
            bonus_armor: 0.0,
            last_hit_by: None,
        }
    }

    /// Get total armor (base + bonus)
    pub fn total_armor(&self) -> f32 {
        self.enemy_type.armor() + self.bonus_armor
    }

    pub fn apply_slow(&mut self, duration: f32, slow_factor: f32) {
        self.speed = self.base_speed * slow_factor;
        self.slow_timer = Some(Timer::from_seconds(duration, TimerMode::Once));
    }

    /// Apply damage after armor reduction. Returns actual damage dealt.
    /// Returns 0.0 if enemy is already dead.
    pub fn apply_armor_damage(&mut self, raw_damage: f32) -> f32 {
        if self.marked_dead || self.health <= 0.0 {
            return 0.0;
        }
        let actual = (raw_damage * (1.0 - self.total_armor())).max(0.0);
        self.health = (self.health - actual).max(0.0);
        actual
    }

    pub fn apply_poison(&mut self, dps: f32, duration: f32) {
        // Stack poison damage
        self.poison_damage += dps;
        // Take max of remaining duration and new duration (don't lose existing progress)
        let remaining = self.poison_timer.as_ref().map(|t| t.remaining_secs()).unwrap_or(0.0);
        let new_duration = duration.max(remaining);
        self.poison_timer = Some(Timer::from_seconds(new_duration, TimerMode::Once));
    }
}

/// Health bar component
#[derive(Component)]
pub struct HealthBar {
    pub enemy: Entity,
}

/// Health bar fill component
#[derive(Component)]
pub struct HealthBarFill {
    pub enemy: Entity,
}

/// Death effect that fades out
#[derive(Component)]
pub struct DeathEffect {
    pub lifetime: Timer,
    pub velocity: Vec2,
}

/// Floating gold number when enemy killed
#[derive(Component)]
pub struct GoldNumber {
    pub lifetime: Timer,
    pub velocity: Vec2,
}

/// Wave modifiers for variety
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaveModifier {
    None,
    SpeedBoost,   // Enemies 50% faster
    ArmoredWave,  // All enemies +25% armor
    Swarm,        // 2x enemies, 50% HP each
    Regen,        // Enemies heal 2% HP/sec
    GoldRush,     // 2x gold, 1.5x enemy count
}

impl WaveModifier {
    pub fn name(&self) -> &'static str {
        match self {
            WaveModifier::None => "",
            WaveModifier::SpeedBoost => "SPEED BOOST",
            WaveModifier::ArmoredWave => "ARMORED WAVE",
            WaveModifier::Swarm => "SWARM",
            WaveModifier::Regen => "REGENERATION",
            WaveModifier::GoldRush => "GOLD RUSH",
        }
    }
}

/// Preview data for the next wave
pub struct WavePreview {
    pub counts: Vec<(EnemyType, usize)>,
    pub modifier: WaveModifier,
}

/// Wave manager - infinite scaling survival mode
#[derive(Resource)]
pub struct WaveManager {
    pub current_wave: usize,
    pub spawn_timer: Timer,
    pub enemies_to_spawn: Vec<(EnemyType, f32)>, // (type, health_multiplier)
    pub wave_active: bool,
    pub enemies_alive: u32,
    pub perfect_wave: bool,  // No enemies escaped this wave
    pub health_multiplier: f32, // Scales enemy HP each wave
    pub last_interest: u32,  // Interest earned last wave (for UI)
    pub last_bonus: u32,     // Bonus earned last wave (for UI)
    pub current_modifier: WaveModifier, // Active wave modifier
    pub wave_ended_at: Option<f64>, // elapsed_seconds when wave completed
}

impl Default for WaveManager {
    fn default() -> Self {
        Self {
            current_wave: 0,
            spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            enemies_to_spawn: Vec::new(),
            wave_active: false,
            enemies_alive: 0,
            perfect_wave: true,
            health_multiplier: 1.0,
            last_interest: 0,
            last_bonus: 0,
            current_modifier: WaveModifier::None,
            wave_ended_at: None,
        }
    }
}

impl WaveManager {
    /// Generate and start the next wave - infinite scaling (balanced curve)
    pub fn start_wave(&mut self) {
        let wave_num = self.current_wave + 1;

        let (health_mult, modifier) = Self::compute_wave_params(wave_num);
        self.health_multiplier = health_mult;
        self.current_modifier = modifier;

        // Calculate spawn delay: starts at 1.0s, decreases to minimum 0.3s
        let spawn_delay = (1.0 - (wave_num as f32 * 0.03)).max(0.3);
        self.spawn_timer = Timer::from_seconds(spawn_delay, TimerMode::Repeating);

        // Generate enemies for this wave
        self.enemies_to_spawn = Self::generate_wave_enemies(wave_num, self.health_multiplier, self.current_modifier);

        self.wave_active = true;
        self.perfect_wave = true;
    }

    /// Compute health_multiplier and modifier for a given wave number (without mutating)
    fn compute_wave_params(wave_num: usize) -> (f32, WaveModifier) {
        let base_mult = 1.0 + (wave_num as f32).powf(1.25) * 0.05;
        let late_game_mult = if wave_num > 12 {
            1.0 + (wave_num - 12) as f32 * 0.08
        } else {
            1.0
        };
        let mut health_multiplier = base_mult * late_game_mult;

        let modifier = if wave_num >= 6 && wave_num.is_multiple_of(3) {
            match (wave_num / 3) % 5 {
                0 => WaveModifier::SpeedBoost,
                1 => WaveModifier::Swarm,
                2 => WaveModifier::ArmoredWave,
                3 => WaveModifier::GoldRush,
                _ => WaveModifier::Regen,
            }
        } else {
            WaveModifier::None
        };

        if modifier == WaveModifier::Swarm {
            health_multiplier *= 0.5;
        }

        (health_multiplier, modifier)
    }

    /// Preview the next wave composition (without starting it)
    pub fn preview_next_wave(&self) -> WavePreview {
        let wave_num = self.current_wave + 1;
        let (health_mult, modifier) = Self::compute_wave_params(wave_num);
        let enemies = Self::generate_wave_enemies(wave_num, health_mult, modifier);

        // Aggregate counts per enemy type using HashMap for O(n)
        let mut counts: HashMap<EnemyType, usize> = HashMap::with_capacity(8);
        for (etype, _) in &enemies {
            *counts.entry(*etype).or_insert(0) += 1;
        }

        WavePreview {
            counts: counts.into_iter().collect(),
            modifier,
        }
    }

    /// Procedurally generate enemies for a wave
    fn generate_wave_enemies(wave_num: usize, health_multiplier: f32, modifier: WaveModifier) -> Vec<(EnemyType, f32)> {
        let multiplier = health_multiplier;

        // Calculate modifier factor for enemy count
        let modifier_factor = match modifier {
            WaveModifier::Swarm => 2.0,     // Double enemies (but half HP applied in start_wave)
            WaveModifier::GoldRush => 1.5,  // 50% more enemies
            _ => 1.0,
        };

        let base_count = rules::wave_base_count(wave_num, modifier_factor);

        let mut enemies = Vec::with_capacity((base_count * 1.5) as usize + 5);

        // Determine wave type based on wave number
        let wave_type = wave_num % 5;
        let is_boss_wave = wave_num.is_multiple_of(10) && wave_num > 0; // Boss every 10 waves now

        if is_boss_wave {
            // Boss wave every 10 waves
            let boss_count = (wave_num / 10).min(3); // Cap at 3 bosses
            for _ in 0..boss_count {
                enemies.push((EnemyType::Boss, multiplier));
            }
            // Fewer escort enemies
            let escort_count = (base_count * 0.4) as usize;
            for _ in 0..escort_count / 2 {
                enemies.push((EnemyType::Fast, multiplier));
                enemies.push((EnemyType::Basic, multiplier));
            }
        } else {
            match wave_type {
                1 => {
                    // Swarm wave - lots of weak enemies
                    let count = (base_count * 1.2) as usize;
                    for _ in 0..count {
                        enemies.push((EnemyType::Basic, multiplier));
                    }
                    if wave_num >= 4 {
                        for _ in 0..(count / 4) {
                            enemies.push((EnemyType::Fast, multiplier));
                        }
                    }
                }
                2 => {
                    // Speed wave - fast enemies
                    let count = (base_count * 0.7) as usize;
                    for _ in 0..count {
                        enemies.push((EnemyType::Fast, multiplier));
                    }
                    // Some basics as filler
                    for _ in 0..(count / 2) {
                        enemies.push((EnemyType::Basic, multiplier));
                    }
                    if wave_num >= 12 {
                        for _ in 0..(count / 3) {
                            enemies.push((EnemyType::Flying, multiplier));
                        }
                    }
                }
                3 => {
                    // Tank wave - heavy enemies (delayed introduction)
                    let count = (base_count * 0.6) as usize;
                    // Mostly basics early on
                    for _ in 0..count {
                        enemies.push((EnemyType::Basic, multiplier));
                    }
                    if wave_num >= 8 {
                        for _ in 0..(count / 3) {
                            enemies.push((EnemyType::Tank, multiplier));
                        }
                        // Add splitters to tank waves
                        let splitter_count = ((wave_num - 7) / 4).clamp(1, 3);
                        for _ in 0..splitter_count {
                            enemies.push((EnemyType::Splitter, multiplier));
                        }
                    }
                    if wave_num >= 13 {
                        for _ in 0..(count / 4) {
                            enemies.push((EnemyType::Armored, multiplier));
                        }
                    }
                }
                4 => {
                    // Mixed wave - variety
                    let count = (base_count * 0.8) as usize;
                    for _ in 0..count {
                        enemies.push((EnemyType::Basic, multiplier));
                    }
                    for _ in 0..(count / 3) {
                        enemies.push((EnemyType::Fast, multiplier));
                    }
                    if wave_num >= 9 {
                        for _ in 0..(count / 4) {
                            enemies.push((EnemyType::Tank, multiplier));
                        }
                    }
                    // Add splitters starting wave 7 (forces AOE strategy)
                    if wave_num >= 7 {
                        let splitter_count = ((wave_num - 6) / 3).clamp(1, 5);
                        for _ in 0..splitter_count {
                            enemies.push((EnemyType::Splitter, multiplier));
                        }
                    }
                }
                _ => {
                    // Mini-boss wave every 5 (but not 10, 20, etc.)
                    let count = (base_count * 0.5) as usize;
                    for _ in 0..count {
                        enemies.push((EnemyType::Basic, multiplier));
                    }
                    for _ in 0..(count / 2) {
                        enemies.push((EnemyType::Fast, multiplier));
                    }
                    if wave_num >= 5 {
                        // Add some tanks as mini-boss
                        let tank_count = (wave_num / 5).min(4);
                        for _ in 0..tank_count {
                            enemies.push((EnemyType::Tank, multiplier));
                        }
                    }
                }
            }
        }

        // Shuffle enemies for variety (simple deterministic shuffle based on wave)
        let len = enemies.len();
        for i in 0..len {
            let swap_idx = (i + wave_num * 7) % len;
            enemies.swap(i, swap_idx);
        }

        enemies
    }

    /// Calculate early-send bonus based on how quickly the player starts the next wave
    pub fn early_send_bonus(&self, current_time: f64) -> u32 {
        let Some(ended_at) = self.wave_ended_at else { return 0 };
        let wait_seconds = (current_time - ended_at) as f32;
        // Grace period: 0-5s = full bonus, 5-20s = decaying, >20s = nothing
        // Bonus scales with wave number (higher waves = more reward for aggression)
        let base = 10 + self.current_wave as u32 * 3;
        if wait_seconds <= 5.0 {
            base
        } else if wait_seconds < 20.0 {
            let decay = (wait_seconds - 5.0) / 15.0;
            (base as f32 * (1.0 - decay)) as u32
        } else {
            0
        }
    }

    /// Calculate wave completion bonus
    pub fn wave_bonus(&self) -> u32 {
        let wave = self.current_wave as u32;
        // Base bonus scales with wave progression (slightly reduced for balance)
        let base_bonus = 12 + (wave * 3) + ((wave as f32).sqrt() * 4.0) as u32;
        if self.perfect_wave {
            (base_bonus as f32 * 1.4) as u32  // 40% bonus for no leaks
        } else {
            base_bonus
        }
    }
}

fn reset_wave_manager(mut wave_manager: ResMut<WaveManager>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    *wave_manager = WaveManager::default();
}

/// Events
#[derive(Event)]
pub struct EnemyKilledEvent {
    pub enemy: Entity,
    pub reward: u32,
    pub position: Vec3,
    pub enemy_type: EnemyType,
    pub path_index: usize,
    pub last_hit_by: Option<Entity>,
}

#[derive(Event)]
pub struct EnemyEscapedEvent {
    pub enemy: Entity,
}

/// Event to spawn mini-splitters when a Splitter dies
#[derive(Event)]
pub struct SpawnMiniSplittersEvent {
    pub position: Vec3,
    pub path_index: usize,
    pub health_multiplier: f32,
}

fn wave_spawner(
    mut commands: Commands,
    mut wave_manager: ResMut<WaveManager>,
    map: Res<GameMap>,
    time: Res<Time>,
) {
    if !wave_manager.wave_active || wave_manager.enemies_to_spawn.is_empty() {
        return;
    }

    wave_manager.spawn_timer.tick(time.delta());

    if wave_manager.spawn_timer.just_finished() {
        if let Some((enemy_type, health_mult)) = wave_manager.enemies_to_spawn.pop() {
            // Spawn at path start
            if let Some(&(x, y)) = map.path.first() {
                let pos = GameMap::grid_to_world(x, y);
                let mut enemy = Enemy::new(enemy_type);
                // Apply health multiplier from wave scaling
                enemy.health *= health_mult;
                enemy.max_health *= health_mult;

                // Apply ArmoredWave modifier (+25% armor)
                if wave_manager.current_modifier == WaveModifier::ArmoredWave {
                    enemy.bonus_armor = 0.25;
                }

                let size = enemy_type.size();
                let color = enemy_type.color();

                let enemy_entity = commands
                    .spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::splat(size)),
                                ..default()
                            },
                            transform: Transform::from_translation(pos.extend(ZDepth::ENEMY)),
                            ..default()
                        },
                        enemy,
                        GameEntity,
                    ))
                    .id();

                // Spawn flying shadow for flying enemies
                if enemy_type.is_flying() {
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::srgba(0.0, 0.0, 0.0, 0.3),
                                custom_size: Some(Vec2::splat(size)),
                                ..default()
                            },
                            transform: Transform::from_translation(
                                Vec3::new(pos.x + 4.0, pos.y - 4.0, ZDepth::FLYING_SHADOW)
                            ),
                            ..default()
                        },
                        FlyingShadow { enemy: enemy_entity },
                        GameEntity,
                    ));
                }

                // Spawn health bar background
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: GameColors::HEALTH_BAR_BG,
                            custom_size: Some(Vec2::new(size + 4.0, ShapeSizes::HEALTH_BAR_BG_HEIGHT)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(pos.x, pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET, ZDepth::HEALTH_BAR_BG)),
                        ..default()
                    },
                    HealthBar { enemy: enemy_entity },
                    GameEntity,
                ));

                // Spawn health bar fill
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: GameColors::HEALTH_HIGH,
                            custom_size: Some(Vec2::new(size, ShapeSizes::HEALTH_BAR_HEIGHT)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(pos.x, pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET, ZDepth::HEALTH_BAR_FILL)),
                        ..default()
                    },
                    HealthBarFill { enemy: enemy_entity },
                    GameEntity,
                ));

                wave_manager.enemies_alive += 1;
            }
        }
    }
}

fn enemy_movement(
    mut enemies: Query<(Entity, &mut Enemy, &mut Transform)>,
    map: Res<GameMap>,
    wave_manager: Res<WaveManager>,
    time: Res<Time>,
    mut escaped_events: EventWriter<EnemyEscapedEvent>,
) {
    let speed_modifier = if wave_manager.current_modifier == WaveModifier::SpeedBoost {
        1.5 // 50% faster
    } else {
        1.0
    };

    let regen_active = wave_manager.current_modifier == WaveModifier::Regen;

    for (entity, mut enemy, mut transform) in &mut enemies {
        if enemy.health <= 0.0 || enemy.marked_dead {
            continue;
        }

        // Apply regen modifier (2% HP/sec)
        if regen_active {
            enemy.health = (enemy.health + enemy.max_health * 0.02 * time.delta_seconds())
                .min(enemy.max_health);
        }

        // Update slow timer
        if let Some(ref mut timer) = enemy.slow_timer {
            timer.tick(time.delta());
            if timer.finished() {
                enemy.speed = enemy.base_speed;
                enemy.slow_timer = None;
            }
        }

        // Get current and next path positions
        // Guard against empty path (shouldn't happen, but prevents panic)
        if map.path.len() < 2 || enemy.path_index >= map.path.len().saturating_sub(1) {
            // Enemy reached the end or invalid path
            escaped_events.send(EnemyEscapedEvent { enemy: entity });
            enemy.marked_dead = true;
            continue;
        }

        let current_pos = transform.translation.truncate();

        // Flying enemies take shortcuts - skip to a waypoint further ahead
        let target_index = if enemy.enemy_type.is_flying() {
            // Skip up to 5 waypoints ahead (creates diagonal movement)
            let skip = 5.min(map.path.len().saturating_sub(1).saturating_sub(enemy.path_index));
            enemy.path_index + skip
        } else {
            enemy.path_index + 1
        };

        // Bounds check before accessing path
        let target_index = target_index.min(map.path.len().saturating_sub(1));
        let (next_x, next_y) = map.path[target_index];
        let target_pos = GameMap::grid_to_world(next_x, next_y);

        let direction = (target_pos - current_pos).normalize_or_zero();
        // Apply speed modifier from wave
        let movement = direction * enemy.speed * speed_modifier * time.delta_seconds();

        transform.translation.x += movement.x;
        transform.translation.y += movement.y;

        // Check if reached next waypoint
        let reach_distance = if enemy.enemy_type.is_flying() { 15.0 } else { 5.0 };
        if current_pos.distance(target_pos) < reach_distance {
            enemy.path_index = target_index;
        }
    }
}

/// Apply poison damage over time.
/// Poison is "true damage" — it bypasses armor by design, unlike napalm which
/// goes through apply_armor_damage(). This makes poison towers the counter to
/// heavily armored enemies. DPS stacks additively from multiple poison sources.
fn poison_tick(
    mut enemies: Query<&mut Enemy>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds();

    for mut enemy in &mut enemies {
        if enemy.marked_dead {
            continue;
        }

        // Get poison info before borrowing timer mutably
        let poison_dps = enemy.poison_damage;

        // Tick timer and check if finished
        let should_clear = if let Some(ref mut timer) = enemy.poison_timer {
            timer.tick(time.delta());
            timer.finished()
        } else {
            continue; // No poison, skip
        };

        // Apply poison damage
        if poison_dps > 0.0 {
            enemy.health = (enemy.health - poison_dps * delta).max(0.0);
        }

        // Clear poison when timer expires
        if should_clear {
            enemy.poison_damage = 0.0;
            enemy.poison_timer = None;
        }
    }
}

fn enemy_health_check(
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
    mut killed_events: EventWriter<EnemyKilledEvent>,
) {
    for (entity, mut enemy, transform) in &mut enemies {
        if enemy.health <= 0.0 && !enemy.marked_dead {
            enemy.marked_dead = true;
            killed_events.send(EnemyKilledEvent {
                enemy: entity,
                reward: enemy.reward,
                position: transform.translation,
                enemy_type: enemy.enemy_type,
                path_index: enemy.path_index,
                last_hit_by: enemy.last_hit_by,
            });
        }
    }
}

struct HealthBarData {
    health_pct: f32,
    size: f32,
    bar_x: f32,
    bar_y: f32,
    visible: Visibility,
}

fn get_health_bar_data(enemy: &Enemy, enemy_transform: &Transform) -> HealthBarData {
    let size = enemy.enemy_type.size();
    let health_pct = if enemy.max_health > 0.0 {
        (enemy.health / enemy.max_health).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let visible = if health_pct >= 1.0 {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    HealthBarData {
        health_pct,
        size,
        bar_x: enemy_transform.translation.x,
        bar_y: enemy_transform.translation.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET,
        visible,
    }
}

fn update_health_bars(
    enemies: Query<(&Enemy, &Transform)>,
    mut health_bars: Query<(&HealthBar, &mut Transform, &mut Visibility), Without<Enemy>>,
    mut health_fills: Query<(&HealthBarFill, &mut Transform, &mut Sprite, &mut Visibility), (Without<Enemy>, Without<HealthBar>)>,
) {
    for (health_bar, mut bar_transform, mut visibility) in &mut health_bars {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_bar.enemy) {
            let data = get_health_bar_data(enemy, enemy_transform);
            *visibility = data.visible;
            bar_transform.translation.x = data.bar_x;
            bar_transform.translation.y = data.bar_y;
        }
    }

    for (health_fill, mut fill_transform, mut sprite, mut visibility) in &mut health_fills {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_fill.enemy) {
            let data = get_health_bar_data(enemy, enemy_transform);
            *visibility = data.visible;
            fill_transform.translation.x = data.bar_x;
            fill_transform.translation.y = data.bar_y;

            sprite.custom_size = Some(Vec2::new(data.size * data.health_pct, ShapeSizes::HEALTH_BAR_HEIGHT));

            // Color based on health
            sprite.color = if data.health_pct > 0.6 {
                GameColors::HEALTH_HIGH
            } else if data.health_pct > 0.3 {
                GameColors::HEALTH_MID
            } else {
                GameColors::HEALTH_LOW
            };
        }
    }
}

/// Spawn floating "+Xg" gold text at the enemy's death position.
fn spawn_gold_toast(
    commands: &mut Commands,
    assets: &GameAssets,
    pos: Vec2,
    reward: u32,
    is_boosted: bool,
) {
    let offset_x = (rand_simple(pos.x) - 0.5) * 16.0;
    let velocity = Vec2::new(offset_x, 35.0);
    let toast_size = if is_boosted { 18.0 } else { 14.0 };
    let toast_color = if is_boosted { GameColors::ABILITY_GOLD_RUSH } else { GameColors::GOLD_TEXT };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                format!("+{}g", reward),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: toast_size,
                    color: toast_color,
                },
            ),
            transform: Transform::from_translation(Vec3::new(pos.x, pos.y + 20.0, ZDepth::FLOATING_TEXT)),
            ..default()
        },
        GoldNumber {
            lifetime: Timer::from_seconds(0.7, TimerMode::Once),
            velocity,
        },
        GameEntity,
    ));
}

/// Spawn death particle burst: scatter particles around the death position,
/// plus brighter core particles at medium/high particle density.
fn spawn_death_particles(
    commands: &mut Commands,
    settings: &GameSettings,
    death_pos: Vec2,
    particle_color: Color,
) {
    let particle_count = settings.particle_density.death_particle_count();

    // Regular scatter particles
    for i in 0..particle_count {
        let seed = death_pos.x + i as f32;
        let angle = rand_simple(seed) * std::f32::consts::TAU;
        let speed = 80.0 + rand_simple(seed + 100.0) * 40.0;
        let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: particle_color,
                    custom_size: Some(Vec2::splat(ShapeSizes::DEATH_PARTICLE_SIZE)),
                    ..default()
                },
                transform: Transform::from_translation(death_pos.extend(ZDepth::DEATH_EFFECT)),
                ..default()
            },
            DeathEffect {
                lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                velocity,
            },
            GameEntity,
        ));
    }

    // Core particles: larger, brighter, slower (Medium/High density only)
    if particle_count >= 8 {
        let core_count = if particle_count >= 12 { 3 } else { 2 };
        let srgba = particle_color.to_srgba();
        let bright_color = Color::srgb(
            (srgba.red * 1.4).min(1.0),
            (srgba.green * 1.4).min(1.0),
            (srgba.blue * 1.4).min(1.0),
        );
        for i in 0..core_count {
            let seed = death_pos.y + i as f32 * 3.7;
            let angle = rand_simple(seed) * std::f32::consts::TAU;
            let speed = 40.0 + rand_simple(seed + 200.0) * 30.0;
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: bright_color,
                        custom_size: Some(Vec2::splat(9.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(death_pos.extend(ZDepth::DEATH_EFFECT + 0.05)),
                    ..default()
                },
                DeathEffect {
                    lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                    velocity,
                },
                GameEntity,
            ));
        }
    }
}

/// Despawn health bars, health fills, and flying shadows associated with an enemy.
fn despawn_enemy_visuals(
    commands: &mut Commands,
    enemy: Entity,
    health_bars: &Query<(Entity, &HealthBar)>,
    health_fills: &Query<(Entity, &HealthBarFill)>,
    shadows: &Query<(Entity, &FlyingShadow)>,
) {
    for (entity, bar) in health_bars {
        if bar.enemy == enemy {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }
    for (entity, fill) in health_fills {
        if fill.enemy == enemy {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }
    for (entity, shadow) in shadows {
        if shadow.enemy == enemy {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }
}

fn handle_enemy_killed(
    mut commands: Commands,
    mut events: EventReader<EnemyKilledEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut wave_manager: ResMut<WaveManager>,
    mut kill_streak: ResMut<KillStreak>,
    mut splitter_events: EventWriter<SpawnMiniSplittersEvent>,
    abilities: Res<PlayerAbilities>,
    assets: Res<GameAssets>,
    mut screen_shake: ResMut<ScreenShake>,
    settings: Res<GameSettings>,
    mut stats: ResMut<GameStats>,
    health_bars: Query<(Entity, &HealthBar)>,
    health_fills: Query<(Entity, &HealthBarFill)>,
    shadows: Query<(Entity, &FlyingShadow)>,
) {
    // Gold Rush modifier: scales 2x base → 3x max at wave 20+
    let wave_gold_rush = wave_manager.current_modifier == WaveModifier::GoldRush;
    let ability_gold_rush = abilities.gold_rush_active.is_some();
    let gold_rush_multiplier = if wave_gold_rush || ability_gold_rush {
        let wave = wave_manager.current_wave as f32;
        2.0 + (wave * 0.05).min(1.0)
    } else {
        1.0
    };

    for event in events.read() {
        // Register kill for combo system
        kill_streak.register_kill();

        // Screen shake on high combo
        if kill_streak.count >= 10 {
            screen_shake.trauma = (screen_shake.trauma + 0.15).min(1.0);
        }

        // Apply combo multiplier and gold rush to gold reward
        let multiplied_reward = (event.reward as f32 * kill_streak.multiplier() * gold_rush_multiplier) as u32;
        economy.gold = economy.gold.saturating_add(multiplied_reward);
        economy.score = economy.score.saturating_add(multiplied_reward);
        wave_manager.enemies_alive = wave_manager.enemies_alive.saturating_sub(1);

        // Track stats
        stats.record_kill(event.enemy_type, multiplied_reward);
        stats.update_max_combo(kill_streak.count);
        if let Some(tower_entity) = event.last_hit_by {
            stats.record_kill_tower(tower_entity);
        }

        // Spawn gold earned toast (bigger and brighter during Gold Rush)
        spawn_gold_toast(
            &mut commands,
            &assets,
            event.position.truncate(),
            multiplied_reward,
            gold_rush_multiplier > 1.0,
        );

        // If this was a Splitter, spawn mini-splitters
        if event.enemy_type.is_splitter() {
            splitter_events.send(SpawnMiniSplittersEvent {
                position: event.position,
                path_index: event.path_index,
                health_multiplier: wave_manager.health_multiplier,
            });
        }

        // Spawn death particle burst (type-colored scatter + core)
        spawn_death_particles(
            &mut commands,
            &settings,
            event.position.truncate(),
            event.enemy_type.color(),
        );

        // Boss kills get extra screen shake
        if event.enemy_type == EnemyType::Boss {
            screen_shake.trauma = (screen_shake.trauma + 0.3).min(1.0);
        }

        // Despawn enemy
        if let Some(entity_commands) = commands.get_entity(event.enemy) {
            entity_commands.despawn_recursive();
        }

        despawn_enemy_visuals(&mut commands, event.enemy, &health_bars, &health_fills, &shadows);
    }
}

/// Spawn mini-splitters when a Splitter dies
fn spawn_mini_splitters(
    mut commands: Commands,
    mut events: EventReader<SpawnMiniSplittersEvent>,
    mut wave_manager: ResMut<WaveManager>,
) {
    for event in events.read() {
        let enemy_type = EnemyType::MiniSplitter;
        let size = enemy_type.size();
        let color = enemy_type.color();

        // Spawn 3 mini-splitters in a triangle pattern around the death position
        let offsets = [
            Vec2::new(0.0, 15.0),      // Above
            Vec2::new(-13.0, -8.0),    // Bottom left
            Vec2::new(13.0, -8.0),     // Bottom right
        ];

        for offset in offsets {
            let pos = event.position.truncate() + offset;
            let mut enemy = Enemy::new(enemy_type);

            // Apply health multiplier from wave scaling
            enemy.health *= event.health_multiplier;
            enemy.max_health *= event.health_multiplier;

            // Inherit path progress from parent Splitter
            enemy.path_index = event.path_index;

            let enemy_entity = commands
                .spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color,
                            custom_size: Some(Vec2::splat(size)),
                            ..default()
                        },
                        transform: Transform::from_translation(pos.extend(ZDepth::ENEMY)),
                        ..default()
                    },
                    enemy,
                    GameEntity,
                ))
                .id();

            // Spawn health bar background
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::HEALTH_BAR_BG,
                        custom_size: Some(Vec2::new(size + 4.0, ShapeSizes::HEALTH_BAR_BG_HEIGHT)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        pos.x,
                        pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET,
                        ZDepth::HEALTH_BAR_BG,
                    )),
                    ..default()
                },
                HealthBar { enemy: enemy_entity },
                GameEntity,
            ));

            // Spawn health bar fill
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::HEALTH_HIGH,
                        custom_size: Some(Vec2::new(size, ShapeSizes::HEALTH_BAR_HEIGHT)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        pos.x,
                        pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET,
                        ZDepth::HEALTH_BAR_FILL,
                    )),
                    ..default()
                },
                HealthBarFill { enemy: enemy_entity },
                GameEntity,
            ));

            wave_manager.enemies_alive += 1;
        }
    }
}

fn update_death_effects(
    mut commands: Commands,
    mut effects: Query<(Entity, &mut DeathEffect, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut effect, mut sprite, mut transform) in &mut effects {
        effect.lifetime.tick(time.delta());

        let progress = ease_out(effect.lifetime.fraction());
        let alpha = 1.0 - progress;

        // Move particle and decelerate
        transform.translation.x += effect.velocity.x * time.delta_seconds();
        transform.translation.y += effect.velocity.y * time.delta_seconds();
        effect.velocity *= 0.92_f32.powf(time.delta_seconds() * 60.0);

        // Shrink and fade
        let scale = 1.0 - progress * 0.6;
        transform.scale = Vec3::splat(scale);
        sprite.color = sprite.color.with_alpha(alpha * 0.9);

        if effect.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn handle_enemy_escaped(
    mut commands: Commands,
    mut events: EventReader<EnemyEscapedEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut wave_manager: ResMut<WaveManager>,
    mut next_state: ResMut<NextState<GameState>>,
    mut stats: ResMut<GameStats>,
    health_bars: Query<(Entity, &HealthBar)>,
    health_fills: Query<(Entity, &HealthBarFill)>,
    shadows: Query<(Entity, &FlyingShadow)>,
) {
    for event in events.read() {
        economy.lives = economy.lives.saturating_sub(1);
        wave_manager.enemies_alive = wave_manager.enemies_alive.saturating_sub(1);
        wave_manager.perfect_wave = false;
        stats.record_escape();

        // Despawn enemy
        if let Some(entity_commands) = commands.get_entity(event.enemy) {
            entity_commands.despawn_recursive();
        }

        despawn_enemy_visuals(&mut commands, event.enemy, &health_bars, &health_fills, &shadows);

        if economy.lives == 0 {
            next_state.set(GameState::GameOver);
        }
    }
}

fn check_wave_complete(
    mut wave_manager: ResMut<WaveManager>,
    mut economy: ResMut<PlayerEconomy>,
    analytics: Res<Analytics>,
    time: Res<Time>,
    mut stats: ResMut<GameStats>,
) {
    if wave_manager.wave_active
        && wave_manager.enemies_to_spawn.is_empty()
        && wave_manager.enemies_alive == 0
    {
        // Award interest on current gold (before bonus)
        let interest = economy.calculate_interest();
        economy.gold = economy.gold.saturating_add(interest);

        // Award wave completion bonus
        let bonus = wave_manager.wave_bonus();
        economy.gold = economy.gold.saturating_add(bonus);
        economy.score = economy.score.saturating_add(bonus);

        // Store interest and bonus for UI display
        wave_manager.last_interest = interest;
        wave_manager.last_bonus = bonus;

        stats.record_wave_complete(bonus, interest);

        let completed_wave = wave_manager.current_wave;

        wave_manager.wave_active = false;
        wave_manager.wave_ended_at = Some(time.elapsed_seconds_f64());
        wave_manager.current_wave += 1;

        // Track milestone waves (5, 10, 15, 20...) to measure progression
        if completed_wave.is_multiple_of(5) || completed_wave == 1 {
            let wave_str = completed_wave.to_string();
            let score_str = economy.score.to_string();
            let gold_str = economy.gold.to_string();
            track_with_context(
                &analytics,
                "wave_completed",
                &[
                    ("wave", &wave_str),
                    ("score", &score_str),
                    ("gold", &gold_str),
                ],
            );
        }

        // No victory condition - infinite survival mode!
        // Game continues until player loses all lives
    }
}

fn update_gold_numbers(
    mut commands: Commands,
    mut numbers: Query<(Entity, &mut GoldNumber, &mut Transform, &mut Text)>,
    time: Res<Time>,
) {
    for (entity, mut number, mut transform, mut text) in &mut numbers {
        number.lifetime.tick(time.delta());

        // Move upward
        transform.translation.x += number.velocity.x * time.delta_seconds();
        transform.translation.y += number.velocity.y * time.delta_seconds();

        // Slow down velocity
        number.velocity *= 0.95;

        // Fade out
        let alpha = 1.0 - number.lifetime.fraction();
        if let Some(section) = text.sections.get_mut(0) {
            section.style.color = GameColors::GOLD_TEXT.with_alpha(alpha);
        }

        if number.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Tint enemies based on active status effects (poison/slow)
fn update_enemy_status_visuals(
    mut enemies: Query<(&Enemy, &mut Sprite)>,
) {
    for (enemy, mut sprite) in &mut enemies {
        if enemy.marked_dead {
            continue;
        }

        let base_color = enemy.enemy_type.color();

        if enemy.poison_timer.is_some() {
            // Blend 30% toward poison green
            let poison = GameColors::PROJECTILE_POISON;
            sprite.color = Color::srgb(
                base_color.to_srgba().red * 0.7 + poison.to_srgba().red * 0.3,
                base_color.to_srgba().green * 0.7 + poison.to_srgba().green * 0.3,
                base_color.to_srgba().blue * 0.7 + poison.to_srgba().blue * 0.3,
            );
        } else if enemy.slow_timer.is_some() {
            // Blend 30% toward ice cyan
            let slow = GameColors::TOWER_SLOW;
            sprite.color = Color::srgb(
                base_color.to_srgba().red * 0.7 + slow.to_srgba().red * 0.3,
                base_color.to_srgba().green * 0.7 + slow.to_srgba().green * 0.3,
                base_color.to_srgba().blue * 0.7 + slow.to_srgba().blue * 0.3,
            );
        } else {
            sprite.color = base_color;
        }
    }
}

/// Flying enemy shadow component
#[derive(Component)]
pub struct FlyingShadow {
    pub enemy: Entity,
}

fn update_flying_shadows(
    mut commands: Commands,
    enemies: Query<&Transform, With<Enemy>>,
    mut shadows: Query<(Entity, &FlyingShadow, &mut Transform), Without<Enemy>>,
) {
    for (shadow_entity, shadow, mut shadow_transform) in &mut shadows {
        if let Ok(enemy_transform) = enemies.get(shadow.enemy) {
            shadow_transform.translation.x = enemy_transform.translation.x + 4.0;
            shadow_transform.translation.y = enemy_transform.translation.y - 4.0;
        } else {
            // Enemy despawned, clean up shadow
            commands.entity(shadow_entity).despawn_recursive();
        }
    }
}

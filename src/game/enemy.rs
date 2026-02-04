use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::{
    economy::PlayerEconomy,
    map::GameMap,
    GameEntity,
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveManager>()
            .add_event::<EnemyKilledEvent>()
            .add_event::<EnemyEscapedEvent>()
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
                    handle_enemy_escaped,
                    check_wave_complete,
                    update_death_effects,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Enemy types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Basic,
    Fast,
    Tank,
    Armored,  // Resistant to damage
    Flying,   // Ignores some path (takes shortcuts)
    Boss,     // Big and tough
}

impl EnemyType {
    pub fn health(&self) -> f32 {
        match self {
            EnemyType::Basic => 100.0,
            EnemyType::Fast => 68.0,
            EnemyType::Tank => 340.0,
            EnemyType::Armored => 250.0,
            EnemyType::Flying => 85.0,
            EnemyType::Boss => 2000.0,
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            EnemyType::Basic => 55.0,
            EnemyType::Fast => 100.0,
            EnemyType::Tank => 32.0,
            EnemyType::Armored => 38.0,
            EnemyType::Flying => 85.0,
            EnemyType::Boss => 28.0,
        }
    }

    pub fn reward(&self) -> u32 {
        match self {
            EnemyType::Basic => 8,
            EnemyType::Fast => 12,
            EnemyType::Tank => 30,
            EnemyType::Armored => 25,
            EnemyType::Flying => 15,
            EnemyType::Boss => 175,
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
        }
    }

    pub fn apply_slow(&mut self, duration: f32, slow_factor: f32) {
        self.speed = self.base_speed * slow_factor;
        self.slow_timer = Some(Timer::from_seconds(duration, TimerMode::Once));
    }

    pub fn apply_poison(&mut self, dps: f32, duration: f32) {
        // Stack poison damage
        self.poison_damage += dps;
        // Refresh or extend duration
        self.poison_timer = Some(Timer::from_seconds(duration, TimerMode::Once));
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
    pub initial_size: f32,
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
        }
    }
}

impl WaveManager {
    /// Generate and start the next wave - infinite scaling (gentler curve)
    pub fn start_wave(&mut self) {
        let wave_num = self.current_wave + 1;

        // Calculate health multiplier: stronger mid-game, steeper late-game
        // Wave 1: 1.05x, Wave 10: ~1.85x, Wave 20: ~3.2x, Wave 50: ~7x
        self.health_multiplier = 1.0 + (wave_num as f32).powf(1.25) * 0.05;

        // Calculate spawn delay: starts at 1.0s, decreases to minimum 0.3s
        let spawn_delay = (1.0 - (wave_num as f32 * 0.03)).max(0.3);
        self.spawn_timer = Timer::from_seconds(spawn_delay, TimerMode::Repeating);

        // Generate enemies for this wave
        self.enemies_to_spawn = self.generate_wave_enemies(wave_num);

        self.wave_active = true;
        self.perfect_wave = true;
    }

    /// Procedurally generate enemies for a wave
    fn generate_wave_enemies(&self, wave_num: usize) -> Vec<(EnemyType, f32)> {
        let mut enemies = Vec::new();
        let multiplier = self.health_multiplier;

        // Base enemy count scaling: ramps up more in late game
        // Wave 1: ~8, Wave 10: ~25, Wave 20: ~48, Wave 50: ~115
        let base_count = 5.0 + (wave_num as f32 * 1.4) + (wave_num as f32).powf(1.15) * 1.5;

        // Determine wave type based on wave number
        let wave_type = wave_num % 5;
        let is_boss_wave = wave_num % 10 == 0 && wave_num > 0; // Boss every 10 waves now

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

    pub fn total_waves(&self) -> usize {
        // Infinite mode - just show current progress
        self.current_wave + 1
    }

    /// Calculate wave completion bonus
    pub fn wave_bonus(&self) -> u32 {
        let wave = self.current_wave as u32;
        // Moderate bonus - scales with progression
        let base_bonus = 20 + (wave * 4) + ((wave as f32).sqrt() * 5.0) as u32;
        if self.perfect_wave {
            (base_bonus as f32 * 1.5) as u32  // 50% bonus for no leaks
        } else {
            base_bonus
        }
    }
}

fn reset_wave_manager(mut wave_manager: ResMut<WaveManager>) {
    *wave_manager = WaveManager::default();
}

/// Events
#[derive(Event)]
pub struct EnemyKilledEvent {
    pub enemy: Entity,
    pub reward: u32,
    pub position: Vec3,
    pub size: f32,
}

#[derive(Event)]
pub struct EnemyEscapedEvent {
    pub enemy: Entity,
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
                            transform: Transform::from_translation(pos.extend(3.0)),
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
                        transform: Transform::from_translation(Vec3::new(pos.x, pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET, 3.5)),
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
                        transform: Transform::from_translation(Vec3::new(pos.x, pos.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET, 3.6)),
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
    time: Res<Time>,
    mut escaped_events: EventWriter<EnemyEscapedEvent>,
) {
    for (entity, mut enemy, mut transform) in &mut enemies {
        if enemy.health <= 0.0 || enemy.marked_dead {
            continue;
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
        if enemy.path_index >= map.path.len() - 1 {
            // Enemy reached the end
            escaped_events.send(EnemyEscapedEvent { enemy: entity });
            enemy.marked_dead = true;
            continue;
        }

        let current_pos = transform.translation.truncate();

        // Flying enemies take shortcuts - skip to a waypoint further ahead
        let target_index = if enemy.enemy_type.is_flying() {
            // Skip up to 5 waypoints ahead (creates diagonal movement)
            let skip = 5.min(map.path.len() - 1 - enemy.path_index);
            enemy.path_index + skip
        } else {
            enemy.path_index + 1
        };

        let (next_x, next_y) = map.path[target_index];
        let target_pos = GameMap::grid_to_world(next_x, next_y);

        let direction = (target_pos - current_pos).normalize_or_zero();
        let movement = direction * enemy.speed * time.delta_seconds();

        transform.translation.x += movement.x;
        transform.translation.y += movement.y;

        // Check if reached next waypoint
        let reach_distance = if enemy.enemy_type.is_flying() { 15.0 } else { 5.0 };
        if current_pos.distance(target_pos) < reach_distance {
            enemy.path_index = target_index;
        }
    }
}

/// Apply poison damage over time
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
            enemy.health -= poison_dps * delta;
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
                size: enemy.enemy_type.size(),
            });
        }
    }
}

fn update_health_bars(
    enemies: Query<(&Enemy, &Transform)>,
    mut health_bars: Query<(&HealthBar, &mut Transform, &mut Visibility), Without<Enemy>>,
    mut health_fills: Query<(&HealthBarFill, &mut Transform, &mut Sprite, &mut Visibility), (Without<Enemy>, Without<HealthBar>)>,
) {
    for (health_bar, mut bar_transform, mut visibility) in &mut health_bars {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_bar.enemy) {
            let size = enemy.enemy_type.size();
            let health_pct = enemy.health / enemy.max_health;

            // Cull health bars for full-health enemies (optimization + cleaner visuals)
            *visibility = if health_pct >= 1.0 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };

            bar_transform.translation.x = enemy_transform.translation.x;
            bar_transform.translation.y = enemy_transform.translation.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET;
        }
    }

    for (health_fill, mut fill_transform, mut sprite, mut visibility) in &mut health_fills {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_fill.enemy) {
            let size = enemy.enemy_type.size();
            let health_pct = (enemy.health / enemy.max_health).max(0.0);

            // Cull health bars for full-health enemies
            *visibility = if health_pct >= 1.0 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };

            fill_transform.translation.x = enemy_transform.translation.x;
            fill_transform.translation.y = enemy_transform.translation.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET;

            sprite.custom_size = Some(Vec2::new(size * health_pct, ShapeSizes::HEALTH_BAR_HEIGHT));

            // Color based on health
            sprite.color = if health_pct > 0.6 {
                GameColors::HEALTH_HIGH
            } else if health_pct > 0.3 {
                GameColors::HEALTH_MID
            } else {
                GameColors::HEALTH_LOW
            };
        }
    }
}

fn handle_enemy_killed(
    mut commands: Commands,
    mut events: EventReader<EnemyKilledEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut wave_manager: ResMut<WaveManager>,
    health_bars: Query<(Entity, &HealthBar)>,
    health_fills: Query<(Entity, &HealthBarFill)>,
) {
    for event in events.read() {
        economy.gold += event.reward;
        economy.score += event.reward;
        wave_manager.enemies_alive = wave_manager.enemies_alive.saturating_sub(1);

        // Spawn death effect
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::DEATH_EFFECT,
                    custom_size: Some(Vec2::splat(event.size)),
                    ..default()
                },
                transform: Transform::from_translation(event.position.truncate().extend(3.9)),
                ..default()
            },
            DeathEffect {
                lifetime: Timer::from_seconds(0.3, TimerMode::Once),
                initial_size: event.size,
            },
            GameEntity,
        ));

        // Despawn enemy
        if let Some(entity_commands) = commands.get_entity(event.enemy) {
            entity_commands.despawn_recursive();
        }

        // Despawn health bars
        for (entity, bar) in &health_bars {
            if bar.enemy == event.enemy {
                if let Some(entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn_recursive();
                }
            }
        }
        for (entity, fill) in &health_fills {
            if fill.enemy == event.enemy {
                if let Some(entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn_recursive();
                }
            }
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

        let progress = effect.lifetime.fraction();
        // Expand and fade out
        let scale = 1.0 + progress * 0.5;
        let alpha = 1.0 - progress;

        transform.scale = Vec3::splat(scale);
        sprite.color = GameColors::DEATH_EFFECT.with_alpha(alpha * 0.8);

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
    health_bars: Query<(Entity, &HealthBar)>,
    health_fills: Query<(Entity, &HealthBarFill)>,
) {
    for event in events.read() {
        economy.lives = economy.lives.saturating_sub(1);
        wave_manager.enemies_alive = wave_manager.enemies_alive.saturating_sub(1);
        wave_manager.perfect_wave = false;

        // Despawn enemy
        if let Some(entity_commands) = commands.get_entity(event.enemy) {
            entity_commands.despawn_recursive();
        }

        // Despawn health bars
        for (entity, bar) in &health_bars {
            if bar.enemy == event.enemy {
                if let Some(entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn_recursive();
                }
            }
        }
        for (entity, fill) in &health_fills {
            if fill.enemy == event.enemy {
                if let Some(entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn_recursive();
                }
            }
        }

        if economy.lives == 0 {
            next_state.set(GameState::GameOver);
        }
    }
}

fn check_wave_complete(
    mut wave_manager: ResMut<WaveManager>,
    mut economy: ResMut<PlayerEconomy>,
) {
    if wave_manager.wave_active
        && wave_manager.enemies_to_spawn.is_empty()
        && wave_manager.enemies_alive == 0
    {
        // Award wave completion bonus
        let bonus = wave_manager.wave_bonus();
        economy.gold += bonus;
        economy.score += bonus;

        wave_manager.wave_active = false;
        wave_manager.current_wave += 1;

        // No victory condition - infinite survival mode!
        // Game continues until player loses all lives
    }
}

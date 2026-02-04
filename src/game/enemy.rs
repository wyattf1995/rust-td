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
            EnemyType::Fast => 70.0,
            EnemyType::Tank => 400.0,
            EnemyType::Armored => 280.0,
            EnemyType::Flying => 90.0,
            EnemyType::Boss => 2500.0,
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
            EnemyType::Tank => 25,
            EnemyType::Armored => 20,
            EnemyType::Flying => 15,
            EnemyType::Boss => 150,
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

    /// Armor reduces damage taken (0.0 = no armor, 0.5 = 50% damage reduction)
    pub fn armor(&self) -> f32 {
        match self {
            EnemyType::Armored => 0.5,
            EnemyType::Boss => 0.3,
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
        }
    }

    pub fn apply_slow(&mut self, duration: f32, slow_factor: f32) {
        self.speed = self.base_speed * slow_factor;
        self.slow_timer = Some(Timer::from_seconds(duration, TimerMode::Once));
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

/// Wave configuration
#[derive(Clone)]
pub struct Wave {
    pub enemies: Vec<(EnemyType, u32)>, // (type, count)
    pub spawn_delay: f32,
}

/// Wave manager
#[derive(Resource)]
pub struct WaveManager {
    pub current_wave: usize,
    pub waves: Vec<Wave>,
    pub spawn_timer: Timer,
    pub enemies_to_spawn: Vec<EnemyType>,
    pub wave_active: bool,
    pub enemies_alive: u32,
    pub all_waves_complete: bool,
    pub endless_mode: bool,
    pub perfect_wave: bool,  // No enemies escaped this wave
}

impl Default for WaveManager {
    fn default() -> Self {
        let waves = vec![
            // === EARLY GAME (Waves 1-5) ===
            // Wave 1: Introduction
            Wave {
                enemies: vec![(EnemyType::Basic, 6)],
                spawn_delay: 1.0,
            },
            // Wave 2: A few more
            Wave {
                enemies: vec![(EnemyType::Basic, 10)],
                spawn_delay: 0.9,
            },
            // Wave 3: Fast enemies introduced
            Wave {
                enemies: vec![(EnemyType::Basic, 8), (EnemyType::Fast, 4)],
                spawn_delay: 0.8,
            },
            // Wave 4: Speed pressure
            Wave {
                enemies: vec![(EnemyType::Basic, 10), (EnemyType::Fast, 8)],
                spawn_delay: 0.7,
            },
            // Wave 5: Tanks introduced
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 12),
                    (EnemyType::Fast, 6),
                    (EnemyType::Tank, 3),
                ],
                spawn_delay: 0.7,
            },
            // === MID GAME (Waves 6-10) ===
            // Wave 6: Armored enemies
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 15),
                    (EnemyType::Armored, 5),
                ],
                spawn_delay: 0.6,
            },
            // Wave 7: Flying enemies
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 12),
                    (EnemyType::Fast, 10),
                    (EnemyType::Flying, 6),
                ],
                spawn_delay: 0.6,
            },
            // Wave 8: Tank rush
            Wave {
                enemies: vec![
                    (EnemyType::Tank, 8),
                    (EnemyType::Armored, 4),
                ],
                spawn_delay: 0.7,
            },
            // Wave 9: Speed swarm
            Wave {
                enemies: vec![
                    (EnemyType::Fast, 20),
                    (EnemyType::Flying, 8),
                ],
                spawn_delay: 0.4,
            },
            // Wave 10: Mini-boss
            Wave {
                enemies: vec![
                    (EnemyType::Boss, 1),
                    (EnemyType::Basic, 15),
                    (EnemyType::Fast, 10),
                ],
                spawn_delay: 0.6,
            },
            // === LATE GAME (Waves 11-15) ===
            // Wave 11: All types
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 20),
                    (EnemyType::Fast, 12),
                    (EnemyType::Tank, 5),
                    (EnemyType::Armored, 4),
                    (EnemyType::Flying, 6),
                ],
                spawn_delay: 0.5,
            },
            // Wave 12: Heavy assault
            Wave {
                enemies: vec![
                    (EnemyType::Tank, 10),
                    (EnemyType::Armored, 8),
                    (EnemyType::Flying, 6),
                ],
                spawn_delay: 0.5,
            },
            // Wave 13: Speed nightmare
            Wave {
                enemies: vec![
                    (EnemyType::Fast, 30),
                    (EnemyType::Flying, 15),
                ],
                spawn_delay: 0.3,
            },
            // Wave 14: Armored battalion
            Wave {
                enemies: vec![
                    (EnemyType::Armored, 15),
                    (EnemyType::Tank, 8),
                    (EnemyType::Basic, 10),
                ],
                spawn_delay: 0.5,
            },
            // Wave 15: Second boss
            Wave {
                enemies: vec![
                    (EnemyType::Boss, 2),
                    (EnemyType::Tank, 6),
                    (EnemyType::Armored, 6),
                ],
                spawn_delay: 0.6,
            },
            // === END GAME (Waves 16-20) ===
            // Wave 16: Massive swarm
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 40),
                    (EnemyType::Fast, 20),
                ],
                spawn_delay: 0.25,
            },
            // Wave 17: Elite forces
            Wave {
                enemies: vec![
                    (EnemyType::Tank, 12),
                    (EnemyType::Armored, 12),
                    (EnemyType::Flying, 10),
                ],
                spawn_delay: 0.4,
            },
            // Wave 18: Air superiority
            Wave {
                enemies: vec![
                    (EnemyType::Flying, 25),
                    (EnemyType::Fast, 20),
                ],
                spawn_delay: 0.3,
            },
            // Wave 19: The gauntlet
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 30),
                    (EnemyType::Fast, 25),
                    (EnemyType::Tank, 10),
                    (EnemyType::Armored, 10),
                    (EnemyType::Flying, 10),
                ],
                spawn_delay: 0.3,
            },
            // Wave 20: Final stand - triple boss
            Wave {
                enemies: vec![
                    (EnemyType::Boss, 3),
                    (EnemyType::Tank, 8),
                    (EnemyType::Armored, 8),
                    (EnemyType::Flying, 8),
                    (EnemyType::Fast, 15),
                ],
                spawn_delay: 0.4,
            },
        ];

        Self {
            current_wave: 0,
            waves,
            spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            enemies_to_spawn: Vec::new(),
            wave_active: false,
            enemies_alive: 0,
            all_waves_complete: false,
            endless_mode: false,
            perfect_wave: true,
        }
    }
}

impl WaveManager {
    pub fn start_wave(&mut self) {
        if self.current_wave >= self.waves.len() {
            if self.endless_mode {
                // Generate endless wave
                self.generate_endless_wave();
            } else {
                self.all_waves_complete = true;
                return;
            }
        }

        let wave = &self.waves[self.current_wave];
        self.spawn_timer = Timer::from_seconds(wave.spawn_delay, TimerMode::Repeating);

        // Populate enemies to spawn
        self.enemies_to_spawn.clear();
        for (enemy_type, count) in &wave.enemies {
            for _ in 0..*count {
                self.enemies_to_spawn.push(*enemy_type);
            }
        }

        self.wave_active = true;
        self.perfect_wave = true;
    }

    pub fn total_waves(&self) -> usize {
        if self.endless_mode {
            self.current_wave + 1
        } else {
            self.waves.len()
        }
    }

    /// Calculate wave completion bonus
    pub fn wave_bonus(&self) -> u32 {
        let base_bonus = 15 + (self.current_wave as u32 * 5);
        if self.perfect_wave {
            (base_bonus as f32 * 1.5) as u32  // 50% bonus for perfect wave
        } else {
            base_bonus
        }
    }

    fn generate_endless_wave(&mut self) {
        // Generate progressively harder waves
        let wave_num = self.current_wave + 1;
        let difficulty = wave_num as u32;

        let mut enemies = vec![];

        // Base enemies scale with wave
        enemies.push((EnemyType::Basic, 5 + difficulty * 2));
        enemies.push((EnemyType::Fast, difficulty * 2));

        // Tanks appear more frequently
        if wave_num > 2 {
            enemies.push((EnemyType::Tank, difficulty));
        }

        // Armored
        if wave_num > 4 {
            enemies.push((EnemyType::Armored, difficulty / 2));
        }

        // Flying
        if wave_num > 6 {
            enemies.push((EnemyType::Flying, difficulty / 2));
        }

        // Boss every 5 waves
        if wave_num % 5 == 0 {
            enemies.push((EnemyType::Boss, wave_num as u32 / 5));
        }

        let spawn_delay = (0.8 - (wave_num as f32 * 0.02)).max(0.3);

        self.waves.push(Wave {
            enemies,
            spawn_delay,
        });
    }

    pub fn toggle_endless_mode(&mut self) {
        self.endless_mode = !self.endless_mode;
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
        if let Some(enemy_type) = wave_manager.enemies_to_spawn.pop() {
            // Spawn at path start
            if let Some(&(x, y)) = map.path.first() {
                let pos = GameMap::grid_to_world(x, y);
                let enemy = Enemy::new(enemy_type);
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
    mut next_state: ResMut<NextState<GameState>>,
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

        if wave_manager.current_wave >= wave_manager.waves.len() && !wave_manager.endless_mode {
            wave_manager.all_waves_complete = true;
            next_state.set(GameState::Victory);
        }
    }
}

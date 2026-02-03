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
}

impl EnemyType {
    pub fn health(&self) -> f32 {
        match self {
            EnemyType::Basic => 100.0,
            EnemyType::Fast => 60.0,
            EnemyType::Tank => 300.0,
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            EnemyType::Basic => 50.0,
            EnemyType::Fast => 90.0,
            EnemyType::Tank => 30.0,
        }
    }

    pub fn reward(&self) -> u32 {
        match self {
            EnemyType::Basic => 10,
            EnemyType::Fast => 15,
            EnemyType::Tank => 30,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            EnemyType::Basic => GameColors::ENEMY_BASIC,
            EnemyType::Fast => GameColors::ENEMY_FAST,
            EnemyType::Tank => GameColors::ENEMY_TANK,
        }
    }

    pub fn size(&self) -> f32 {
        match self {
            EnemyType::Basic => ShapeSizes::ENEMY_BASIC,
            EnemyType::Fast => ShapeSizes::ENEMY_FAST,
            EnemyType::Tank => ShapeSizes::ENEMY_TANK,
        }
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
}

impl Default for WaveManager {
    fn default() -> Self {
        let waves = vec![
            Wave {
                enemies: vec![(EnemyType::Basic, 5)],
                spawn_delay: 1.0,
            },
            Wave {
                enemies: vec![(EnemyType::Basic, 8), (EnemyType::Fast, 3)],
                spawn_delay: 0.9,
            },
            Wave {
                enemies: vec![(EnemyType::Basic, 10), (EnemyType::Fast, 5)],
                spawn_delay: 0.8,
            },
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 8),
                    (EnemyType::Fast, 5),
                    (EnemyType::Tank, 2),
                ],
                spawn_delay: 0.8,
            },
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 12),
                    (EnemyType::Fast, 8),
                    (EnemyType::Tank, 4),
                ],
                spawn_delay: 0.7,
            },
            Wave {
                enemies: vec![
                    (EnemyType::Basic, 15),
                    (EnemyType::Fast, 10),
                    (EnemyType::Tank, 6),
                ],
                spawn_delay: 0.6,
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
        }
    }
}

impl WaveManager {
    pub fn start_wave(&mut self) {
        if self.current_wave >= self.waves.len() {
            self.all_waves_complete = true;
            return;
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
    }

    pub fn total_waves(&self) -> usize {
        self.waves.len()
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

        let (next_x, next_y) = map.path[enemy.path_index + 1];
        let target_pos = GameMap::grid_to_world(next_x, next_y);
        let current_pos = transform.translation.truncate();

        let direction = (target_pos - current_pos).normalize_or_zero();
        let movement = direction * enemy.speed * time.delta_seconds();

        transform.translation.x += movement.x;
        transform.translation.y += movement.y;

        // Check if reached next waypoint
        if current_pos.distance(target_pos) < 5.0 {
            enemy.path_index += 1;
        }
    }
}

fn enemy_health_check(
    mut enemies: Query<(Entity, &mut Enemy)>,
    mut killed_events: EventWriter<EnemyKilledEvent>,
) {
    for (entity, mut enemy) in &mut enemies {
        if enemy.health <= 0.0 && !enemy.marked_dead {
            enemy.marked_dead = true;
            killed_events.send(EnemyKilledEvent {
                enemy: entity,
                reward: enemy.reward,
            });
        }
    }
}

fn update_health_bars(
    enemies: Query<(&Enemy, &Transform)>,
    mut health_bars: Query<(&HealthBar, &mut Transform), Without<Enemy>>,
    mut health_fills: Query<(&HealthBarFill, &mut Transform, &mut Sprite), (Without<Enemy>, Without<HealthBar>)>,
) {
    for (health_bar, mut bar_transform) in &mut health_bars {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_bar.enemy) {
            let size = enemy.enemy_type.size();
            bar_transform.translation.x = enemy_transform.translation.x;
            bar_transform.translation.y = enemy_transform.translation.y + size / 2.0 + ShapeSizes::HEALTH_BAR_OFFSET;
        }
    }

    for (health_fill, mut fill_transform, mut sprite) in &mut health_fills {
        if let Ok((enemy, enemy_transform)) = enemies.get(health_fill.enemy) {
            let size = enemy.enemy_type.size();
            let health_pct = (enemy.health / enemy.max_health).max(0.0);

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
        wave_manager.enemies_alive = wave_manager.enemies_alive.saturating_sub(1);

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
    mut next_state: ResMut<NextState<GameState>>,
) {
    if wave_manager.wave_active
        && wave_manager.enemies_to_spawn.is_empty()
        && wave_manager.enemies_alive == 0
    {
        wave_manager.wave_active = false;
        wave_manager.current_wave += 1;

        if wave_manager.current_wave >= wave_manager.waves.len() {
            wave_manager.all_waves_complete = true;
            next_state.set(GameState::Victory);
        }
    }
}

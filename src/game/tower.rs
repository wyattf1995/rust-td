use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::{
    economy::PlayerEconomy,
    enemy::Enemy,
    map::GameMap,
    projectile::SpawnProjectileEvent,
    GameEntity,
};

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedTowerType>()
            .init_resource::<HoveredTower>()
            .add_event::<PlaceTowerEvent>()
            .add_systems(
                Update,
                (
                    handle_tower_placement,
                    tower_targeting,
                    tower_attack,
                    update_tower_visuals,
                    update_range_indicators,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Tower types available
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TowerType {
    #[default]
    Basic,  // Single target, balanced
    Splash, // AOE damage
    Slow,   // Slows enemies
    Sniper, // Long range, high damage, slow
    Rapid,  // Short range, fast attacks
}

impl TowerType {
    pub fn cost(&self) -> u32 {
        match self {
            TowerType::Basic => 50,
            TowerType::Splash => 100,
            TowerType::Slow => 75,
            TowerType::Sniper => 150,
            TowerType::Rapid => 80,
        }
    }

    pub fn damage(&self) -> f32 {
        match self {
            TowerType::Basic => 25.0,
            TowerType::Splash => 15.0,
            TowerType::Slow => 10.0,
            TowerType::Sniper => 80.0,
            TowerType::Rapid => 8.0,
        }
    }

    pub fn range(&self) -> f32 {
        match self {
            TowerType::Basic => 150.0,
            TowerType::Splash => 120.0,
            TowerType::Slow => 130.0,
            TowerType::Sniper => 280.0,
            TowerType::Rapid => 100.0,
        }
    }

    pub fn attack_speed(&self) -> f32 {
        match self {
            TowerType::Basic => 1.0,
            TowerType::Splash => 0.7,
            TowerType::Slow => 1.5,
            TowerType::Sniper => 0.4,
            TowerType::Rapid => 4.0,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            TowerType::Basic => GameColors::TOWER_BASIC,
            TowerType::Splash => GameColors::TOWER_SPLASH,
            TowerType::Slow => GameColors::TOWER_SLOW,
            TowerType::Sniper => GameColors::TOWER_SNIPER,
            TowerType::Rapid => GameColors::TOWER_RAPID,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TowerType::Basic => "Basic",
            TowerType::Splash => "Splash",
            TowerType::Slow => "Slow",
            TowerType::Sniper => "Sniper",
            TowerType::Rapid => "Rapid",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TowerType::Basic => "Balanced single-target damage",
            TowerType::Splash => "Area damage to groups",
            TowerType::Slow => "Slows enemy movement",
            TowerType::Sniper => "Long range, high damage",
            TowerType::Rapid => "Fast attacks, short range",
        }
    }
}

/// Currently selected tower type for placement
#[derive(Resource, Default)]
pub struct SelectedTowerType(pub TowerType);

/// Currently hovered tower (for showing range)
#[derive(Resource, Default)]
pub struct HoveredTower(pub Option<Entity>);

/// Tower component
#[derive(Component)]
pub struct Tower {
    pub tower_type: TowerType,
    pub range: f32,
    pub damage: f32,
    pub attack_cooldown: Timer,
    pub target: Option<Entity>,
    pub grid_x: usize,
    pub grid_y: usize,
}

impl Tower {
    pub fn new(tower_type: TowerType, grid_x: usize, grid_y: usize) -> Self {
        let attack_speed = tower_type.attack_speed();
        Self {
            tower_type,
            range: tower_type.range(),
            damage: tower_type.damage(),
            attack_cooldown: Timer::from_seconds(1.0 / attack_speed, TimerMode::Repeating),
            target: None,
            grid_x,
            grid_y,
        }
    }

    pub fn sell_value(&self) -> u32 {
        // Return 75% of original cost
        (self.tower_type.cost() as f32 * 0.75) as u32
    }
}

/// Range indicator component
#[derive(Component)]
pub struct RangeIndicator {
    pub tower: Entity,
}

/// Tower barrel component
#[derive(Component)]
pub struct TowerBarrel {
    pub tower: Entity,
}

/// Event to place a tower
#[derive(Event)]
pub struct PlaceTowerEvent {
    pub grid_x: usize,
    pub grid_y: usize,
    pub tower_type: TowerType,
}

fn handle_tower_placement(
    mut commands: Commands,
    mut events: EventReader<PlaceTowerEvent>,
    mut map: ResMut<GameMap>,
    mut economy: ResMut<PlayerEconomy>,
) {
    for event in events.read() {
        let cost = event.tower_type.cost();

        // Check if we can afford it and tile is buildable
        if economy.gold < cost || !map.is_buildable(event.grid_x, event.grid_y) {
            continue;
        }

        // Deduct cost
        economy.gold -= cost;

        // Mark tile as occupied
        map.place_tower(event.grid_x, event.grid_y);

        // Spawn tower
        let pos = GameMap::grid_to_world(event.grid_x, event.grid_y);
        let tower = Tower::new(event.tower_type, event.grid_x, event.grid_y);
        let color = event.tower_type.color();
        let range = tower.range;

        let tower_entity = commands
            .spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::splat(ShapeSizes::TOWER)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(2.0)),
                    ..default()
                },
                tower,
                GameEntity,
            ))
            .id();

        // Spawn range indicator (initially hidden)
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::NONE, // Hidden by default
                    custom_size: Some(Vec2::splat(range * 2.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(1.5)),
                ..default()
            },
            RangeIndicator { tower: tower_entity },
            GameEntity,
        ));

        // Spawn tower barrel/turret
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::TOWER_BARREL,
                    custom_size: Some(Vec2::new(ShapeSizes::TOWER_BARREL_WIDTH, ShapeSizes::TOWER_BARREL_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(2.5)),
                ..default()
            },
            TowerBarrel { tower: tower_entity },
            GameEntity,
        ));
    }
}

fn tower_targeting(
    mut towers: Query<(&mut Tower, &Transform)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
) {
    for (mut tower, tower_transform) in &mut towers {
        let tower_pos = tower_transform.translation.truncate();

        // Find closest enemy in range
        let mut best_target: Option<(Entity, f32)> = None;

        for (enemy_entity, enemy_transform, enemy) in &enemies {
            if enemy.health <= 0.0 || enemy.marked_dead {
                continue;
            }

            let enemy_pos = enemy_transform.translation.truncate();
            let distance = tower_pos.distance(enemy_pos);

            if distance <= tower.range {
                // Prefer enemies further along the path (higher path_index)
                if let Some((_, best_index)) = best_target {
                    if enemy.path_index > best_index as usize {
                        best_target = Some((enemy_entity, enemy.path_index as f32));
                    }
                } else {
                    best_target = Some((enemy_entity, enemy.path_index as f32));
                }
            }
        }

        tower.target = best_target.map(|(e, _)| e);

        // Clear target if it's no longer valid
        if let Some(target) = tower.target {
            if enemies.get(target).is_err() {
                tower.target = None;
            }
        }
    }
}

fn tower_attack(
    mut towers: Query<(&mut Tower, &Transform)>,
    enemies: Query<&Transform, With<Enemy>>,
    time: Res<Time>,
    mut projectile_events: EventWriter<SpawnProjectileEvent>,
) {
    for (mut tower, tower_transform) in &mut towers {
        tower.attack_cooldown.tick(time.delta());

        if let Some(target) = tower.target {
            if tower.attack_cooldown.just_finished() {
                if let Ok(_enemy_transform) = enemies.get(target) {
                    let start = tower_transform.translation.truncate();

                    projectile_events.send(SpawnProjectileEvent {
                        start,
                        target,
                        damage: tower.damage,
                        tower_type: tower.tower_type,
                    });
                }
            }
        }
    }
}

fn update_tower_visuals(
    towers: Query<(&Tower, &Transform), Without<TowerBarrel>>,
    enemies: Query<&Transform, (With<Enemy>, Without<Tower>, Without<TowerBarrel>)>,
    mut barrels: Query<(&TowerBarrel, &mut Transform), (Without<Tower>, Without<Enemy>)>,
) {
    for (barrel, mut barrel_transform) in &mut barrels {
        if let Ok((tower, tower_transform)) = towers.get(barrel.tower) {
            // Update barrel position to follow tower
            barrel_transform.translation.x = tower_transform.translation.x;
            barrel_transform.translation.y = tower_transform.translation.y;

            // Rotate barrel toward target
            if let Some(target) = tower.target {
                if let Ok(enemy_transform) = enemies.get(target) {
                    let tower_pos = tower_transform.translation.truncate();
                    let enemy_pos = enemy_transform.translation.truncate();
                    let direction = (enemy_pos - tower_pos).normalize();
                    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
                    barrel_transform.rotation = Quat::from_rotation_z(angle);
                }
            }
        }
    }
}

fn update_range_indicators(
    hovered: Res<HoveredTower>,
    towers: Query<&Transform, (With<Tower>, Without<RangeIndicator>)>,
    mut indicators: Query<(&RangeIndicator, &mut Sprite, &mut Transform), Without<Tower>>,
) {
    for (indicator, mut sprite, mut transform) in &mut indicators {
        // Show range indicator if this tower is hovered
        if hovered.0 == Some(indicator.tower) {
            sprite.color = GameColors::RANGE_INDICATOR;
        } else {
            sprite.color = Color::NONE;
        }

        // Keep indicator position synced with tower
        if let Ok(tower_transform) = towers.get(indicator.tower) {
            transform.translation.x = tower_transform.translation.x;
            transform.translation.y = tower_transform.translation.y;
        }
    }
}

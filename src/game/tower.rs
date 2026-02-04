use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use crate::loading::GameAssets;

use super::{
    economy::PlayerEconomy,
    enemy::Enemy,
    map::GameMap,
    projectile::SpawnProjectileEvent,
    spatial::SpatialGrid,
    GameEntity,
};

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedTowerType>()
            .init_resource::<HoveredTower>()
            .init_resource::<SelectedPlacedTower>()
            .add_event::<PlaceTowerEvent>()
            .add_event::<SellTowerEvent>()
            .add_event::<UpgradeTowerEvent>()
            .add_systems(
                Update,
                (
                    handle_tower_placement,
                    handle_tower_selling,
                    handle_tower_upgrade,
                    tower_targeting,
                    update_buff_auras,
                    tower_attack,
                    update_tower_visuals,
                    update_range_indicators,
                    update_muzzle_flashes,
                    update_level_badges,
                    update_buff_aura_visuals,
                    tower_hotkeys,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Currently selected placed tower for selling/upgrading
#[derive(Resource, Default)]
pub struct SelectedPlacedTower(pub Option<Entity>);

/// Event to sell a tower
#[derive(Event)]
pub struct SellTowerEvent {
    pub tower: Entity,
}

/// Event to upgrade a tower
#[derive(Event)]
pub struct UpgradeTowerEvent {
    pub tower: Entity,
}

/// Muzzle flash effect
#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: Timer,
}

/// Targeting priority for towers
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TargetingPriority {
    #[default]
    First,      // Furthest along the path (default, best for preventing leaks)
    Closest,    // Nearest to tower (best for rapid-fire towers)
    LowestHP,   // Lowest current health (good for finishing enemies)
    HighestHP,  // Highest current health (focus on tanks)
    Fastest,    // Fastest moving (pick off speedsters)
}

impl TargetingPriority {
    pub fn name(&self) -> &'static str {
        match self {
            TargetingPriority::First => "First",
            TargetingPriority::Closest => "Closest",
            TargetingPriority::LowestHP => "Low HP",
            TargetingPriority::HighestHP => "High HP",
            TargetingPriority::Fastest => "Fastest",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TargetingPriority::First => TargetingPriority::Closest,
            TargetingPriority::Closest => TargetingPriority::LowestHP,
            TargetingPriority::LowestHP => TargetingPriority::HighestHP,
            TargetingPriority::HighestHP => TargetingPriority::Fastest,
            TargetingPriority::Fastest => TargetingPriority::First,
        }
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
    Chain,  // Lightning bounces between enemies
    Poison, // Deals damage over time
    Buff,   // Boosts nearby towers (doesn't attack)
}

impl TowerType {
    pub fn cost(&self) -> u32 {
        match self {
            TowerType::Basic => 50,
            TowerType::Splash => 100,
            TowerType::Slow => 75,
            TowerType::Sniper => 150,
            TowerType::Rapid => 80,
            TowerType::Chain => 120,
            TowerType::Poison => 90,
            TowerType::Buff => 200,
        }
    }

    pub fn damage(&self) -> f32 {
        match self {
            TowerType::Basic => 25.0,
            TowerType::Splash => 15.0,
            TowerType::Slow => 10.0,
            TowerType::Sniper => 80.0,
            TowerType::Rapid => 8.0,
            TowerType::Chain => 30.0,   // Per hit, bounces at 70%
            TowerType::Poison => 12.0,  // Initial + DOT
            TowerType::Buff => 0.0,     // Doesn't attack
        }
    }

    pub fn range(&self) -> f32 {
        match self {
            TowerType::Basic => 150.0,
            TowerType::Splash => 120.0,
            TowerType::Slow => 130.0,
            TowerType::Sniper => 280.0,
            TowerType::Rapid => 100.0,
            TowerType::Chain => 140.0,
            TowerType::Poison => 130.0,
            TowerType::Buff => 120.0,   // Aura range
        }
    }

    pub fn attack_speed(&self) -> f32 {
        match self {
            TowerType::Basic => 1.0,
            TowerType::Splash => 0.7,
            TowerType::Slow => 1.5,
            TowerType::Sniper => 0.4,
            TowerType::Rapid => 4.0,
            TowerType::Chain => 0.8,
            TowerType::Poison => 1.2,
            TowerType::Buff => 0.0,     // No attacks
        }
    }

    pub fn color(&self) -> Color {
        match self {
            TowerType::Basic => GameColors::TOWER_BASIC,
            TowerType::Splash => GameColors::TOWER_SPLASH,
            TowerType::Slow => GameColors::TOWER_SLOW,
            TowerType::Sniper => GameColors::TOWER_SNIPER,
            TowerType::Rapid => GameColors::TOWER_RAPID,
            TowerType::Chain => GameColors::TOWER_CHAIN,
            TowerType::Poison => GameColors::TOWER_POISON,
            TowerType::Buff => GameColors::TOWER_BUFF,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TowerType::Basic => "Basic",
            TowerType::Splash => "Splash",
            TowerType::Slow => "Slow",
            TowerType::Sniper => "Sniper",
            TowerType::Rapid => "Rapid",
            TowerType::Chain => "Chain",
            TowerType::Poison => "Poison",
            TowerType::Buff => "Buff",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TowerType::Basic => "Balanced single-target damage",
            TowerType::Splash => "Area damage to groups",
            TowerType::Slow => "Slows enemy movement",
            TowerType::Sniper => "Long range, high damage",
            TowerType::Rapid => "Fast attacks, short range",
            TowerType::Chain => "Lightning bounces 3 times",
            TowerType::Poison => "Deals damage over time",
            TowerType::Buff => "Buffs nearby towers (scales)",
        }
    }

    /// Whether this tower can attack enemies
    pub fn can_attack(&self) -> bool {
        !matches!(self, TowerType::Buff)
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
    pub level: u32,
    pub targeting: TargetingPriority,
}

impl Tower {
    pub fn new(tower_type: TowerType, grid_x: usize, grid_y: usize) -> Self {
        let attack_speed = tower_type.attack_speed();
        // For towers that don't attack (like Buff), use a dummy timer
        let cooldown_secs = if attack_speed > 0.0 { 1.0 / attack_speed } else { 1.0 };
        Self {
            tower_type,
            range: tower_type.range(),
            damage: tower_type.damage(),
            attack_cooldown: Timer::from_seconds(cooldown_secs, TimerMode::Repeating),
            target: None,
            grid_x,
            grid_y,
            level: 1,
            targeting: TargetingPriority::default(),
        }
    }

    pub fn cycle_targeting(&mut self) {
        self.targeting = self.targeting.next();
    }

    pub fn sell_value(&self) -> u32 {
        // Return 75% of total invested cost
        let base_cost = self.tower_type.cost();
        let upgrade_cost = self.upgrade_cost_total();
        ((base_cost + upgrade_cost) as f32 * 0.75) as u32
    }

    pub fn upgrade_cost(&self) -> u32 {
        // Infinite upgrades with exponentially increasing cost
        // Level 2: 50% of base, Level 3: 75%, Level 4: 112%, Level 5: 168%...
        let base = self.tower_type.cost() as f32;
        (base * 0.5 * (1.5_f32).powf((self.level - 1) as f32)) as u32
    }

    fn upgrade_cost_total(&self) -> u32 {
        // Total spent on upgrades
        let mut total = 0;
        let base = self.tower_type.cost() as f32;
        for lvl in 1..self.level {
            total += (base * 0.5 * (1.5_f32).powf((lvl - 1) as f32)) as u32;
        }
        total
    }

    pub fn can_upgrade(&self) -> bool {
        true // Infinite upgrades now!
    }

    pub fn upgrade(&mut self) {
        self.level += 1;
        // Diminishing returns: each level adds less percentage
        // Level 2: +20%, Level 3: +15%, Level 4: +12%, Level 5: +10%...
        let level_bonus = 0.20 / (1.0 + (self.level - 2) as f32 * 0.15);

        // Calculate cumulative multiplier
        let mut damage_mult = 1.0;
        let mut range_mult = 1.0;
        let mut speed_mult = 1.0;
        for lvl in 2..=self.level {
            let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
            damage_mult *= 1.0 + bonus;
            range_mult *= 1.0 + bonus * 0.4;  // Range grows slower
            speed_mult *= 1.0 + bonus * 0.6;  // Speed grows medium
        }

        self.damage = self.tower_type.damage() * damage_mult;
        self.range = self.tower_type.range() * range_mult;
        let attack_speed = self.tower_type.attack_speed() * speed_mult;
        self.attack_cooldown = Timer::from_seconds(1.0 / attack_speed, TimerMode::Repeating);
    }

    /// Calculate stats for next level (for preview)
    pub fn preview_upgrade(&self) -> (f32, f32, f32) {
        let next_level = self.level + 1;
        let mut damage_mult = 1.0;
        let mut range_mult = 1.0;
        let mut speed_mult = 1.0;
        for lvl in 2..=next_level {
            let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
            damage_mult *= 1.0 + bonus;
            range_mult *= 1.0 + bonus * 0.4;
            speed_mult *= 1.0 + bonus * 0.6;
        }
        (
            self.tower_type.damage() * damage_mult,
            self.tower_type.range() * range_mult,
            self.tower_type.attack_speed() * speed_mult,
        )
    }

    /// Get current attack speed
    pub fn attack_speed(&self) -> f32 {
        let mut speed_mult = 1.0;
        for lvl in 2..=self.level {
            let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
            speed_mult *= 1.0 + bonus * 0.6;
        }
        self.tower_type.attack_speed() * speed_mult
    }

    /// Get buff percentage for Buff towers (returns 0 for other types)
    pub fn buff_percentage(&self) -> f32 {
        if self.tower_type != TowerType::Buff {
            return 0.0;
        }
        let level_bonus: f32 = (1..self.level).map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3)).sum();
        0.25 + level_bonus
    }

    /// Get buff percentage at next level
    pub fn buff_percentage_next(&self) -> f32 {
        if self.tower_type != TowerType::Buff {
            return 0.0;
        }
        let next_level = self.level + 1;
        let level_bonus: f32 = (1..next_level).map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3)).sum();
        0.25 + level_bonus
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

/// Tower level badge (shows upgrade level)
#[derive(Component)]
pub struct TowerLevelBadge {
    pub tower: Entity,
}

/// Buff aura visual indicator
#[derive(Component)]
pub struct BuffAuraIndicator {
    pub tower: Entity,
}

/// Tracks if a tower is being buffed
#[derive(Component, Default)]
pub struct BuffedStatus {
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
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
    assets: Res<GameAssets>,
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

        // Spawn level badge (bottom-right corner of tower)
        let badge_offset = Vec2::new(12.0, -12.0);
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "1",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 14.0,
                        color: Color::WHITE,
                    },
                ).with_justify(JustifyText::Center),
                transform: Transform::from_translation(
                    (pos + badge_offset).extend(3.5)
                ),
                ..default()
            },
            TowerLevelBadge { tower: tower_entity },
            GameEntity,
        ));

        // Spawn buff aura indicator for buff towers
        if event.tower_type == TowerType::Buff {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(1.0, 0.85, 0.3, 0.15),
                        custom_size: Some(Vec2::splat(ShapeSizes::BUFF_AURA_RANGE * 2.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(1.2)),
                    ..default()
                },
                BuffAuraIndicator { tower: tower_entity },
                GameEntity,
            ));
        }
    }
}

fn tower_targeting(
    mut towers: Query<(&mut Tower, &Transform)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    spatial_grid: Res<SpatialGrid>,
) {
    for (mut tower, tower_transform) in &mut towers {
        // Buff towers don't target enemies
        if !tower.tower_type.can_attack() {
            tower.target = None;
            continue;
        }

        let tower_pos = tower_transform.translation.truncate();

        // Use spatial grid to get nearby entities (much faster than checking all)
        let nearby = spatial_grid.query_range(tower_pos, tower.range);

        // Find best target based on priority
        let mut best_target: Option<(Entity, f32)> = None;

        for entity in nearby {
            let Ok((enemy_entity, enemy_transform, enemy)) = enemies.get(entity) else {
                continue;
            };

            if enemy.health <= 0.0 || enemy.marked_dead {
                continue;
            }

            let enemy_pos = enemy_transform.translation.truncate();
            let distance = tower_pos.distance(enemy_pos);

            // Double-check distance (spatial grid is approximate)
            if distance > tower.range {
                continue;
            }

            // Calculate priority score based on targeting mode
            let score = match tower.targeting {
                TargetingPriority::First => enemy.path_index as f32 * 1000.0,
                TargetingPriority::Closest => -distance,
                TargetingPriority::LowestHP => -enemy.health,
                TargetingPriority::HighestHP => enemy.health,
                TargetingPriority::Fastest => enemy.base_speed,
            };

            if let Some((_, best_score)) = best_target {
                if score > best_score {
                    best_target = Some((enemy_entity, score));
                }
            } else {
                best_target = Some((enemy_entity, score));
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
    mut commands: Commands,
    mut towers: Query<(&mut Tower, &Transform, Option<&BuffedStatus>)>,
    enemies: Query<&Transform, With<Enemy>>,
    time: Res<Time>,
    mut projectile_events: EventWriter<SpawnProjectileEvent>,
) {
    for (mut tower, tower_transform, buff_status) in &mut towers {
        // Buff towers don't attack
        if !tower.tower_type.can_attack() {
            continue;
        }

        tower.attack_cooldown.tick(time.delta());

        if let Some(target) = tower.target {
            if tower.attack_cooldown.just_finished() {
                if let Ok(_enemy_transform) = enemies.get(target) {
                    let start = tower_transform.translation.truncate();

                    // Apply buff multiplier to damage
                    let damage_mult = buff_status.map(|b| b.damage_multiplier).unwrap_or(1.0);
                    let final_damage = tower.damage * damage_mult;

                    projectile_events.send(SpawnProjectileEvent {
                        start,
                        target,
                        damage: final_damage,
                        tower_type: tower.tower_type,
                    });

                    // Spawn muzzle flash (different color for different tower types)
                    let flash_color = match tower.tower_type {
                        TowerType::Chain => GameColors::PROJECTILE_CHAIN,
                        TowerType::Poison => GameColors::PROJECTILE_POISON,
                        _ => GameColors::MUZZLE_FLASH,
                    };

                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: flash_color,
                                custom_size: Some(Vec2::splat(ShapeSizes::MUZZLE_FLASH)),
                                ..default()
                            },
                            transform: Transform::from_translation(start.extend(2.8)),
                            ..default()
                        },
                        MuzzleFlash {
                            lifetime: Timer::from_seconds(0.08, TimerMode::Once),
                        },
                        GameEntity,
                    ));
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

fn handle_tower_selling(
    mut commands: Commands,
    mut events: EventReader<SellTowerEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut map: ResMut<GameMap>,
    towers: Query<&Tower>,
    range_indicators: Query<(Entity, &RangeIndicator)>,
    barrels: Query<(Entity, &TowerBarrel)>,
    badges: Query<(Entity, &TowerLevelBadge)>,
    buff_indicators: Query<(Entity, &BuffAuraIndicator)>,
) {
    for event in events.read() {
        if let Ok(tower) = towers.get(event.tower) {
            // Refund gold
            economy.gold += tower.sell_value();

            // Clear map tile
            map.remove_tower(tower.grid_x, tower.grid_y);

            // Despawn tower
            commands.entity(event.tower).despawn_recursive();

            // Despawn range indicator
            for (entity, indicator) in &range_indicators {
                if indicator.tower == event.tower {
                    commands.entity(entity).despawn_recursive();
                }
            }

            // Despawn barrel
            for (entity, barrel) in &barrels {
                if barrel.tower == event.tower {
                    commands.entity(entity).despawn_recursive();
                }
            }

            // Despawn level badge
            for (entity, badge) in &badges {
                if badge.tower == event.tower {
                    commands.entity(entity).despawn_recursive();
                }
            }

            // Despawn buff aura indicator
            for (entity, indicator) in &buff_indicators {
                if indicator.tower == event.tower {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}

fn handle_tower_upgrade(
    mut events: EventReader<UpgradeTowerEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut towers: Query<(&mut Tower, &mut Sprite)>,
    mut range_indicators: Query<(&RangeIndicator, &mut Sprite), Without<Tower>>,
) {
    for event in events.read() {
        if let Ok((mut tower, mut sprite)) = towers.get_mut(event.tower) {
            let cost = tower.upgrade_cost();
            if cost > 0 && economy.gold >= cost {
                economy.gold -= cost;
                tower.upgrade();

                // Visual feedback - slightly brighter color
                let base_color = tower.tower_type.color();
                let brightness = 1.0 + 0.15 * (tower.level - 1) as f32;
                sprite.color = Color::srgb(
                    (base_color.to_srgba().red * brightness).min(1.0),
                    (base_color.to_srgba().green * brightness).min(1.0),
                    (base_color.to_srgba().blue * brightness).min(1.0),
                );

                // Update range indicator size
                for (indicator, mut ind_sprite) in &mut range_indicators {
                    if indicator.tower == event.tower {
                        ind_sprite.custom_size = Some(Vec2::splat(tower.range * 2.0));
                    }
                }
            }
        }
    }
}

fn update_muzzle_flashes(
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut MuzzleFlash, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.lifetime.tick(time.delta());

        // Fade out
        let alpha = 1.0 - flash.lifetime.fraction();
        sprite.color = GameColors::MUZZLE_FLASH.with_alpha(alpha);

        if flash.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn tower_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedTowerType>,
) {
    // Number keys 1-8 select tower types
    if keyboard.just_pressed(KeyCode::Digit1) {
        selected.0 = TowerType::Basic;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        selected.0 = TowerType::Splash;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        selected.0 = TowerType::Slow;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        selected.0 = TowerType::Sniper;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        selected.0 = TowerType::Rapid;
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        selected.0 = TowerType::Chain;
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        selected.0 = TowerType::Poison;
    } else if keyboard.just_pressed(KeyCode::Digit8) {
        selected.0 = TowerType::Buff;
    }
}

fn update_level_badges(
    towers: Query<(&Tower, &Transform)>,
    mut badges: Query<(&TowerLevelBadge, &mut Text, &mut Transform), Without<Tower>>,
) {
    for (badge, mut text, mut badge_transform) in &mut badges {
        if let Ok((tower, tower_transform)) = towers.get(badge.tower) {
            // Update badge text to show current level
            text.sections[0].value = format!("{}", tower.level);

            // Keep badge position synced with tower (bottom-right corner)
            let badge_offset = Vec2::new(12.0, -12.0);
            badge_transform.translation.x = tower_transform.translation.x + badge_offset.x;
            badge_transform.translation.y = tower_transform.translation.y + badge_offset.y;
        }
    }
}

/// Update buff status for all towers based on nearby buff towers
fn update_buff_auras(
    mut commands: Commands,
    buff_towers: Query<(&Tower, &Transform)>,
    mut other_towers: Query<(Entity, &Tower, &Transform, Option<&mut BuffedStatus>)>,
) {
    // Collect buff tower positions, ranges, and levels
    let buff_sources: Vec<(Vec2, f32, u32)> = buff_towers
        .iter()
        .filter(|(t, _)| t.tower_type == TowerType::Buff)
        .map(|(t, transform)| (transform.translation.truncate(), t.range, t.level))
        .collect();

    // Update buff status for each tower
    for (entity, tower, transform, buff_status) in &mut other_towers {
        // Skip buff towers themselves
        if tower.tower_type == TowerType::Buff {
            continue;
        }

        let tower_pos = transform.translation.truncate();

        // Check if tower is in range of any buff tower
        let mut total_buff = 0.0;
        for (buff_pos, buff_range, buff_level) in &buff_sources {
            if tower_pos.distance(*buff_pos) <= *buff_range {
                // Base 25% + 5% per level after 1 (with diminishing returns)
                // Level 1: 25%, Level 2: 30%, Level 3: 34%, Level 4: 37%...
                let level_bonus: f32 = (1..*buff_level).map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3)).sum();
                total_buff += 0.25 + level_bonus;
            }
        }

        if total_buff > 0.0 {
            // Tower is being buffed
            let new_status = BuffedStatus {
                damage_multiplier: 1.0 + total_buff,
                speed_multiplier: 1.0 + total_buff * 0.4, // Speed scales too
            };
            if let Some(mut status) = buff_status {
                *status = new_status;
            } else {
                commands.entity(entity).insert(new_status);
            }
        } else if buff_status.is_some() {
            // No longer buffed, remove status
            commands.entity(entity).remove::<BuffedStatus>();
        }
    }
}

/// Visual pulse effect for buff aura indicators
fn update_buff_aura_visuals(
    towers: Query<(&Tower, &Transform)>,
    mut indicators: Query<(&BuffAuraIndicator, &mut Sprite, &mut Transform), Without<Tower>>,
    time: Res<Time>,
) {
    let pulse = (time.elapsed_seconds() * 2.0).sin() * 0.5 + 0.5;

    for (indicator, mut sprite, mut transform) in &mut indicators {
        if let Ok((tower, tower_transform)) = towers.get(indicator.tower) {
            // Update position
            transform.translation.x = tower_transform.translation.x;
            transform.translation.y = tower_transform.translation.y;

            // Update size based on tower range (which increases with upgrades)
            sprite.custom_size = Some(Vec2::splat(tower.range * 2.0));

            // Pulse alpha
            let alpha = 0.1 + pulse * 0.08;
            sprite.color = Color::srgba(1.0, 0.85, 0.3, alpha);
        }
    }
}

use std::collections::HashSet;

use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{CombatConstants, GameColors, ShapeSizes, ZDepth};
use crate::loading::GameAssets;

use crate::persistence::GameSettings;

use super::{
    effects::spawn_hit_sparks,
    enemy::Enemy,
    rand_simple,
    spatial::SpatialGrid,
    stats::GameStats,
    tower::{TowerType, Specialization},
    GameEntity,
};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectileEvent>()
            .add_systems(
                Update,
                (spawn_projectiles, projectile_movement, projectile_collision, update_effects, update_damage_numbers, update_napalm_zones, update_blizzard_zones)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Projectile component
#[derive(Component)]
pub struct Projectile {
    pub target: Entity,
    pub damage: f32,
    pub speed: f32,
    pub tower_type: TowerType,
    pub predicted_pos: Option<Vec2>,  // For leading shots
    pub chain_bounces: u32,           // Remaining bounces for chain lightning
    pub hit_enemies: HashSet<Entity>,  // Enemies already hit (for chain/pierce)
    pub poison_duration_bonus: f32,   // Synergy bonus for poison duration
    pub source_tower: Option<Entity>, // Tower entity that fired this projectile (for stats)
    pub specialization: Option<Specialization>,
    pub pierce_remaining: u32,        // For Railgun: hits before despawning
}

/// Trail particle
#[derive(Component)]
pub struct ProjectileTrail {
    pub lifetime: Timer,
}

/// Splash effect
#[derive(Component)]
pub struct SplashEffect {
    pub lifetime: Timer,
}

/// Floating damage number
#[derive(Component)]
pub struct DamageNumber {
    pub lifetime: Timer,
    pub velocity: Vec2,
}

/// Event to spawn a projectile
#[derive(Event)]
pub struct SpawnProjectileEvent {
    pub start: Vec2,
    pub target: Entity,
    pub damage: f32,
    pub tower_type: TowerType,
    pub extra_chain_bounces: u32,     // From Chain+Chain synergy
    pub poison_duration_bonus: f32,   // From Slow+Poison synergy
    pub source_tower: Option<Entity>, // Tower entity for stats attribution
    pub specialization: Option<Specialization>,
}

/// Napalm ground zone (lingering DOT)
#[derive(Component)]
pub struct NapalmZone {
    pub lifetime: Timer,
    pub damage_per_sec: f32,
    pub radius: f32,
    pub tick_timer: Timer,
    pub source_tower: Option<Entity>,
}

/// Blizzard slow zone
#[derive(Component)]
pub struct BlizzardZone {
    pub lifetime: Timer,
    pub radius: f32,
    pub tick_timer: Timer,
}

/// Data for a pending chain bounce (collected during collision, spawned after)
struct ChainBounce {
    origin: Vec2,
    damage: f32,
    hit_entities: HashSet<Entity>,
    bounces_remaining: u32,
    source_tower: Option<Entity>,
    specialization: Option<Specialization>,
}

fn spawn_projectiles(
    mut commands: Commands,
    mut events: EventReader<SpawnProjectileEvent>,
    enemies: Query<(&Transform, &Enemy)>,
) {
    for event in events.read() {
        let color = event.tower_type.projectile_color();
        let size = event.tower_type.projectile_size();

        let projectile_speed = CombatConstants::PROJECTILE_SPEED;

        // Calculate predicted position (leading the target)
        let predicted_pos = if let Ok((enemy_transform, enemy)) = enemies.get(event.target) {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = event.start.distance(enemy_pos);
            let travel_time = distance / projectile_speed;

            // Predict where enemy will be based on its current velocity direction
            // Approximate by using enemy speed (we don't store velocity directly)
            let prediction_offset = enemy.speed * travel_time * CombatConstants::PREDICTION_INACCURACY;

            // Simple prediction: move in the direction enemy is facing
            // Since enemies follow path, we just add speed in their movement direction
            Some(enemy_pos + Vec2::new(prediction_offset * 0.5, 0.0)) // Rough eastward bias
        } else {
            None
        };

        // Chain lightning starts with 3 bounces (+synergy bonus)
        let chain_bounces = if event.tower_type == TowerType::Chain {
            3 + event.extra_chain_bounces
        } else {
            0
        };

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                transform: Transform::from_translation(event.start.extend(ZDepth::PROJECTILE)),
                ..default()
            },
            Projectile {
                target: event.target,
                damage: event.damage,
                speed: projectile_speed,
                tower_type: event.tower_type,
                predicted_pos,
                chain_bounces,
                hit_enemies: HashSet::new(),
                poison_duration_bonus: event.poison_duration_bonus,
                source_tower: event.source_tower,
                specialization: event.specialization,
                pierce_remaining: if event.specialization == Some(Specialization::Railgun) { 3 } else { 0 },
            },
            GameEntity,
        ));
    }
}

fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform)>,
    enemies: Query<(&Transform, &Enemy), Without<Projectile>>,
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut frame_count: Local<u32>,
) {
    *frame_count = frame_count.wrapping_add(1);
    for (entity, mut projectile, mut transform) in &mut projectiles {
        // Get target position with leading
        let target_pos = if let Ok((enemy_transform, enemy)) = enemies.get(projectile.target) {
            let enemy_pos = enemy_transform.translation.truncate();
            let current_pos = transform.translation.truncate();

            // Update prediction based on current state
            let distance = current_pos.distance(enemy_pos);
            let travel_time = distance / projectile.speed;

            // Lead the target based on enemy speed and predicted travel time
            // This creates a more accurate interception point
            let lead_distance = enemy.speed * travel_time * CombatConstants::LEAD_TARGET_FACTOR;

            // Blend between direct aim and predicted position
            // Closer projectiles aim more directly, farther ones lead more
            let blend = (distance / CombatConstants::LEAD_BLEND_DISTANCE).min(1.0);

            if let Some(predicted) = projectile.predicted_pos {
                // Update prediction as we get closer
                let new_predicted = enemy_pos + Vec2::new(lead_distance, 0.0);
                projectile.predicted_pos = Some(enemy_pos.lerp(new_predicted, blend));
                predicted.lerp(enemy_pos, 1.0 - blend)
            } else {
                enemy_pos
            }
        } else {
            // Target no longer exists, despawn projectile
            commands.entity(entity).despawn_recursive();
            continue;
        };

        let current_pos = transform.translation.truncate();
        let direction = (target_pos - current_pos).normalize_or_zero();
        let movement = direction * projectile.speed * time.delta_seconds();

        transform.translation.x += movement.x;
        transform.translation.y += movement.y;

        // Spawn trail particle (frequency based on particle density setting)
        let trail_skip = settings.particle_density.trail_skip();
        if (*frame_count).is_multiple_of(trail_skip) {
            let trail_alpha = if matches!(projectile.tower_type, TowerType::Chain) { 0.6 } else { 0.5 };
            let trail_color = projectile.tower_type.projectile_color().with_alpha(trail_alpha);

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: trail_color,
                        custom_size: Some(Vec2::splat(ShapeSizes::PROJECTILE_TRAIL)),
                        ..default()
                    },
                    transform: Transform::from_translation(transform.translation),
                    ..default()
                },
                ProjectileTrail {
                    lifetime: Timer::from_seconds(CombatConstants::TRAIL_LIFETIME, TimerMode::Once),
                },
                GameEntity,
            ));
        }
    }
}

fn projectile_collision(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
    assets: Res<GameAssets>,
    spatial_grid: Res<SpatialGrid>,
    settings: Res<GameSettings>,
    mut stats: ResMut<GameStats>,
) {
    let mut chain_bounces: Vec<ChainBounce> = Vec::with_capacity(8);
    // Collect piercing projectile updates (entity, new hit list entry)
    let mut pierce_updates: Vec<Entity> = Vec::with_capacity(8);

    // Check projectile collisions
    for (proj_entity, projectile, proj_transform) in &projectiles {
        let proj_pos = proj_transform.translation.truncate();

        // Check collision with target
        if let Ok((enemy_entity, mut enemy, enemy_transform)) = enemies.get_mut(projectile.target) {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = proj_pos.distance(enemy_pos);

            if distance < CombatConstants::HIT_RADIUS {
                // Assassin crit: 25% chance for 3x damage
                let mut hit_damage = projectile.damage;
                let mut is_crit = false;
                if projectile.specialization == Some(Specialization::Assassin) {
                    let crit_roll = rand_simple(proj_pos.x + proj_pos.y + enemy_pos.x);
                    if crit_roll < 0.25 {
                        hit_damage *= 3.0;
                        is_crit = true;
                    }
                }

                // Calculate damage with armor reduction (skips dead enemies)
                let actual_damage = enemy.apply_armor_damage(hit_damage);

                // Track which tower last hit this enemy (for kill attribution + stats)
                if actual_damage > 0.0 {
                    enemy.last_hit_by = projectile.source_tower;
                    if let Some(tower_entity) = projectile.source_tower {
                        stats.record_damage(tower_entity, actual_damage);
                    }
                    if is_crit {
                        spawn_damage_text(&mut commands, &assets, enemy_pos, actual_damage, true, settings.show_damage_numbers);
                    } else {
                        spawn_damage_text(&mut commands, &assets, enemy_pos, actual_damage, false, settings.show_damage_numbers);
                    }

                    // Hit sparks (density-aware)
                    let spark_count = settings.particle_density.death_particle_count() / 2;
                    if spark_count > 0 {
                        spawn_hit_sparks(&mut commands, enemy_pos, spark_count);
                    }
                }

                // Apply slow effect for slow towers
                if projectile.tower_type == TowerType::Slow {
                    enemy.apply_slow(CombatConstants::SLOW_DURATION, CombatConstants::SLOW_FACTOR);

                    // Cryogenic: 15% chance to freeze (speed = 0 for 1s)
                    if projectile.specialization == Some(Specialization::Cryogenic) {
                        let freeze_roll = rand_simple(proj_pos.x * 7.3 + proj_pos.y);
                        if freeze_roll < 0.12 {
                            enemy.apply_slow(1.0, 0.0); // Full freeze
                        }
                    }

                    // Blizzard: spawn ground slow zone
                    if projectile.specialization == Some(Specialization::Blizzard) {
                        commands.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: GameColors::PROJECTILE_SLOW.with_alpha(0.2),
                                    custom_size: Some(Vec2::splat(60.0)),
                                    ..default()
                                },
                                transform: Transform::from_translation(enemy_pos.extend(ZDepth::ABILITY_EFFECT)),
                                ..default()
                            },
                            BlizzardZone {
                                lifetime: Timer::from_seconds(3.0, TimerMode::Once),
                                radius: 30.0,
                                tick_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                            },
                            GameEntity,
                        ));
                    }
                }

                // Apply poison effect
                if projectile.tower_type == TowerType::Poison {
                    let duration = CombatConstants::POISON_BASE_DURATION * (1.0 + projectile.poison_duration_bonus);
                    enemy.apply_poison(CombatConstants::POISON_DPS, duration);

                    // Spawn poison effect
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: GameColors::PROJECTILE_POISON.with_alpha(0.4),
                                custom_size: Some(Vec2::splat(30.0)),
                                ..default()
                            },
                            transform: Transform::from_translation(enemy_pos.extend(ZDepth::ABILITY_EFFECT)),
                            ..default()
                        },
                        SplashEffect {
                            lifetime: Timer::from_seconds(0.3, TimerMode::Once),
                        },
                        GameEntity,
                    ));
                }

                // Chain lightning bouncing
                if projectile.tower_type == TowerType::Chain && projectile.chain_bounces > 0 {
                    let mut hit_list = projectile.hit_enemies.clone();
                    hit_list.insert(enemy_entity);

                    let bounce_damage = projectile.damage * CombatConstants::CHAIN_DAMAGE_DECAY;
                    let remaining_bounces = projectile.chain_bounces.saturating_sub(1);

                    chain_bounces.push(ChainBounce {
                        origin: enemy_pos,
                        damage: bounce_damage,
                        hit_entities: hit_list,
                        bounces_remaining: remaining_bounces,
                        source_tower: projectile.source_tower,
                        specialization: projectile.specialization,
                    });
                }

                // Splash damage (spatial grid narrows search to nearby enemies)
                if projectile.tower_type == TowerType::Splash {
                    // Shockwave: 40% larger splash radius + knockback primary target
                    let radius_mult = if projectile.specialization == Some(Specialization::Shockwave) { 1.4 } else { 1.0 };
                    let splash_radius = ShapeSizes::SPLASH_RADIUS * radius_mult;
                    let splash_damage = projectile.damage * CombatConstants::SPLASH_DAMAGE_RATIO;

                    // Shockwave: push the primary target before borrowing others
                    if projectile.specialization == Some(Specialization::Shockwave) {
                        enemy.path_index = enemy.path_index.saturating_sub(2);
                    }

                    let nearby = spatial_grid.query_range(enemy_pos, splash_radius);
                    for other_entity in nearby {
                        if other_entity == enemy_entity {
                            continue;
                        }
                        if let Ok((_, mut other_enemy, other_transform)) = enemies.get_mut(other_entity) {
                            let other_pos = other_transform.translation.truncate();
                            if enemy_pos.distance(other_pos) < splash_radius {
                                let other_actual_damage = other_enemy.apply_armor_damage(splash_damage);
                                if other_actual_damage > 0.0 {
                                    spawn_damage_text(&mut commands, &assets, other_pos, other_actual_damage, false, settings.show_damage_numbers);
                                }

                                // Shockwave: push enemies backward along path
                                if projectile.specialization == Some(Specialization::Shockwave) {
                                    other_enemy.path_index = other_enemy.path_index.saturating_sub(2);
                                }
                            }
                        }
                    }

                    // Napalm: spawn lingering ground DOT zone
                    if projectile.specialization == Some(Specialization::Napalm) {
                        commands.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: Color::srgba(1.0, 0.4, 0.1, 0.3),
                                    custom_size: Some(Vec2::splat(splash_radius * 2.0)),
                                    ..default()
                                },
                                transform: Transform::from_translation(enemy_pos.extend(ZDepth::ABILITY_EFFECT)),
                                ..default()
                            },
                            NapalmZone {
                                lifetime: Timer::from_seconds(2.5, TimerMode::Once),
                                damage_per_sec: projectile.damage * 0.25,
                                radius: splash_radius,
                                tick_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                                source_tower: projectile.source_tower,
                            },
                            GameEntity,
                        ));
                    }

                    // Spawn splash effect
                    let splash_color = if projectile.specialization == Some(Specialization::Napalm) {
                        Color::srgba(1.0, 0.4, 0.1, 0.3)
                    } else {
                        GameColors::PROJECTILE_SPLASH.with_alpha(0.3)
                    };
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: splash_color,
                                custom_size: Some(Vec2::splat(splash_radius * 2.0)),
                                ..default()
                            },
                            transform: Transform::from_translation(enemy_pos.extend(ZDepth::ABILITY_EFFECT)),
                            ..default()
                        },
                        SplashEffect {
                            lifetime: Timer::from_seconds(0.2, TimerMode::Once),
                        },
                        GameEntity,
                    ));
                }

                // Railgun: pierce through enemies instead of despawning
                if projectile.specialization == Some(Specialization::Railgun) && projectile.pierce_remaining > 0 {
                    pierce_updates.push(proj_entity);
                } else {
                    commands.entity(proj_entity).despawn_recursive();
                }
            }
        } else {
            // Target doesn't exist anymore
            commands.entity(proj_entity).despawn_recursive();
        }
    }

    // Post-loop: handle pierce continuations and chain bounces
    handle_pierce_continuation(&mut commands, &pierce_updates, &projectiles, &enemies, &spatial_grid);
    handle_chain_bounces(&mut commands, chain_bounces, &mut enemies, &spatial_grid, &assets, &settings);
}

const PIERCE_SEARCH_RANGE: f32 = 80.0;
const ARC_AOE_RADIUS: f32 = 20.0;
const ARC_DAMAGE_RATIO: f32 = 0.35;
const TESLA_RANGE_MULT: f32 = 1.2;

/// Handle Railgun piercing: find next target for each piercing projectile and spawn continuation.
fn handle_pierce_continuation(
    commands: &mut Commands,
    pierce_updates: &[Entity],
    projectiles: &Query<(Entity, &Projectile, &Transform)>,
    enemies: &Query<(Entity, &mut Enemy, &Transform)>,
    spatial_grid: &SpatialGrid,
) {
    for &proj_entity in pierce_updates {
        if let Ok((_, projectile, proj_transform)) = projectiles.get(proj_entity) {
            let proj_pos = proj_transform.translation.truncate();
            let mut hit_list = projectile.hit_enemies.clone();
            hit_list.insert(projectile.target);

            let next_target = find_nearest_unhit_enemy(
                &hit_list, proj_pos, PIERCE_SEARCH_RANGE, enemies, spatial_grid,
            );

            if let Some((target, _)) = next_target {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: projectile.tower_type.projectile_color(),
                            custom_size: Some(Vec2::splat(projectile.tower_type.projectile_size())),
                            ..default()
                        },
                        transform: Transform::from_translation(proj_transform.translation),
                        ..default()
                    },
                    Projectile {
                        target,
                        damage: projectile.damage,
                        speed: projectile.speed,
                        tower_type: projectile.tower_type,
                        predicted_pos: None,
                        chain_bounces: 0,
                        hit_enemies: hit_list,
                        poison_duration_bonus: 0.0,
                        source_tower: projectile.source_tower,
                        specialization: projectile.specialization,
                        pierce_remaining: projectile.pierce_remaining - 1,
                    },
                    GameEntity,
                ));
            }
            commands.entity(proj_entity).despawn_recursive();
        }
    }
}

/// Handle chain bounce projectiles: find next bounce target and spawn continuation.
#[allow(clippy::too_many_arguments)]
fn handle_chain_bounces(
    commands: &mut Commands,
    chain_bounces: Vec<ChainBounce>,
    enemies: &mut Query<(Entity, &mut Enemy, &Transform)>,
    spatial_grid: &SpatialGrid,
    assets: &GameAssets,
    settings: &GameSettings,
) {
    for bounce in chain_bounces {
        let range_mult = if bounce.specialization == Some(Specialization::Tesla) { TESLA_RANGE_MULT } else { 1.0 };
        let bounce_range = ShapeSizes::CHAIN_BOUNCE_RANGE * range_mult;

        let next_target = find_nearest_unhit_enemy(
            &bounce.hit_entities, bounce.origin, bounce_range, enemies, spatial_grid,
        );

        if let Some((target, _)) = next_target {
            if bounce.specialization == Some(Specialization::Arc) {
                handle_arc_aoe(commands, bounce.origin, bounce.damage, enemies, spatial_grid, assets, settings);
            }

            if let Ok((_, _, next_transform)) = enemies.get(target) {
                let next_pos = next_transform.translation.truncate();
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: GameColors::PROJECTILE_CHAIN.with_alpha(0.6),
                            custom_size: Some(Vec2::splat(8.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(bounce.origin.extend(ZDepth::PROJECTILE)),
                        ..default()
                    },
                    Projectile {
                        target,
                        damage: bounce.damage,
                        speed: CombatConstants::CHAIN_BOUNCE_SPEED,
                        tower_type: TowerType::Chain,
                        predicted_pos: Some(next_pos),
                        chain_bounces: bounce.bounces_remaining,
                        hit_enemies: bounce.hit_entities,
                        poison_duration_bonus: 0.0,
                        source_tower: bounce.source_tower,
                        specialization: bounce.specialization,
                        pierce_remaining: 0,
                    },
                    GameEntity,
                ));
            }
        }
    }
}

/// Arc specialization: deal AOE damage at the bounce point.
fn handle_arc_aoe(
    commands: &mut Commands,
    origin_pos: Vec2,
    bounce_damage: f32,
    enemies: &mut Query<(Entity, &mut Enemy, &Transform)>,
    spatial_grid: &SpatialGrid,
    assets: &GameAssets,
    settings: &GameSettings,
) {
    let arc_damage = bounce_damage * ARC_DAMAGE_RATIO;
    let nearby = spatial_grid.query_range(origin_pos, ARC_AOE_RADIUS);
    for entity in nearby {
        if let Ok((_, mut enemy, transform)) = enemies.get_mut(entity) {
            let pos = transform.translation.truncate();
            if origin_pos.distance(pos) < ARC_AOE_RADIUS {
                let actual = enemy.apply_armor_damage(arc_damage);
                if actual > 0.0 {
                    spawn_damage_text(commands, assets, pos, actual, false, settings.show_damage_numbers);
                }
            }
        }
    }
    // Arc visual effect
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: GameColors::PROJECTILE_CHAIN.with_alpha(0.3),
                custom_size: Some(Vec2::splat(ARC_AOE_RADIUS * 2.0)),
                ..default()
            },
            transform: Transform::from_translation(origin_pos.extend(ZDepth::ABILITY_EFFECT)),
            ..default()
        },
        SplashEffect {
            lifetime: Timer::from_seconds(0.15, TimerMode::Once),
        },
        GameEntity,
    ));
}

/// Find the nearest living enemy not in the hit list within the given range.
fn find_nearest_unhit_enemy(
    hit_list: &HashSet<Entity>,
    origin: Vec2,
    range: f32,
    enemies: &Query<(Entity, &mut Enemy, &Transform)>,
    spatial_grid: &SpatialGrid,
) -> Option<(Entity, f32)> {
    let mut best: Option<(Entity, f32)> = None;
    let nearby = spatial_grid.query_range(origin, range);
    for entity in nearby {
        if hit_list.contains(&entity) { continue; }
        if let Ok((_, enemy, transform)) = enemies.get(entity) {
            if enemy.marked_dead || enemy.health <= 0.0 { continue; }
            let dist = origin.distance(transform.translation.truncate());
            if dist <= range && best.is_none_or(|(_, bd)| dist < bd) {
                best = Some((entity, dist));
            }
        }
    }
    best
}

/// Spawn floating damage text (regular or crit).
fn spawn_damage_text(commands: &mut Commands, assets: &GameAssets, pos: Vec2, damage: f32, is_crit: bool, show: bool) {
    if !show {
        return;
    }
    let offset_x = (rand_simple(pos.x) - 0.5) * 20.0;
    let (text, font_size, color, velocity_y) = if is_crit {
        (format!("CRIT {:.0}", damage), ShapeSizes::DAMAGE_TEXT_SIZE + 4.0, Color::srgb(1.0, 0.3, 0.3), 50.0)
    } else {
        (format!("{:.0}", damage), ShapeSizes::DAMAGE_TEXT_SIZE, GameColors::DAMAGE_TEXT, 40.0)
    };

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(text, TextStyle {
                font: assets.font.clone(),
                font_size,
                color,
            }),
            transform: Transform::from_translation(Vec3::new(pos.x, pos.y + 15.0, ZDepth::FLOATING_TEXT)),
            ..default()
        },
        DamageNumber {
            lifetime: Timer::from_seconds(CombatConstants::DAMAGE_NUMBER_LIFETIME, TimerMode::Once),
            velocity: Vec2::new(offset_x, velocity_y),
        },
        GameEntity,
    ));
}

fn update_damage_numbers(
    mut commands: Commands,
    mut numbers: Query<(Entity, &mut DamageNumber, &mut Transform, &mut Text)>,
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
            section.style.color = GameColors::DAMAGE_TEXT.with_alpha(alpha);
        }

        if number.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_effects(
    mut commands: Commands,
    mut trails: Query<(Entity, &mut ProjectileTrail, &mut Sprite)>,
    mut splashes: Query<(Entity, &mut SplashEffect, &mut Sprite), Without<ProjectileTrail>>,
    time: Res<Time>,
) {
    // Update and despawn trails
    for (entity, mut trail, mut sprite) in &mut trails {
        trail.lifetime.tick(time.delta());

        // Fade out
        let alpha = 1.0 - trail.lifetime.fraction();
        sprite.color = sprite.color.with_alpha(alpha * 0.5);

        if trail.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }

    // Update and despawn splash effects
    for (entity, mut effect, mut sprite) in &mut splashes {
        effect.lifetime.tick(time.delta());

        let alpha = 1.0 - effect.lifetime.fraction();
        sprite.color = sprite.color.with_alpha(alpha * 0.3);

        if effect.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_napalm_zones(
    mut commands: Commands,
    mut zones: Query<(Entity, &mut NapalmZone, &Transform, &mut Sprite)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform), Without<NapalmZone>>,
    spatial_grid: Res<SpatialGrid>,
    time: Res<Time>,
) {
    for (entity, mut zone, zone_transform, mut sprite) in &mut zones {
        zone.lifetime.tick(time.delta());
        zone.tick_timer.tick(time.delta());

        // Fade out over lifetime
        let alpha = (1.0 - zone.lifetime.fraction()) * 0.3;
        sprite.color = Color::srgba(1.0, 0.4, 0.1, alpha);

        // Deal damage on tick
        if zone.tick_timer.just_finished() {
            let zone_pos = zone_transform.translation.truncate();
            let tick_damage = zone.damage_per_sec * 0.5; // 0.5s tick interval
            let nearby = spatial_grid.query_range(zone_pos, zone.radius);
            for enemy_entity in nearby {
                if let Ok((_, mut enemy, enemy_transform)) = enemies.get_mut(enemy_entity) {
                    let enemy_pos = enemy_transform.translation.truncate();
                    if zone_pos.distance(enemy_pos) < zone.radius {
                        let actual = enemy.apply_armor_damage(tick_damage);
                        if actual > 0.0 {
                            enemy.last_hit_by = zone.source_tower;
                        }
                    }
                }
            }
        }

        if zone.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_blizzard_zones(
    mut commands: Commands,
    mut zones: Query<(Entity, &mut BlizzardZone, &Transform, &mut Sprite)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform), Without<BlizzardZone>>,
    spatial_grid: Res<SpatialGrid>,
    time: Res<Time>,
) {
    for (entity, mut zone, zone_transform, mut sprite) in &mut zones {
        zone.lifetime.tick(time.delta());
        zone.tick_timer.tick(time.delta());

        // Fade out
        let alpha = (1.0 - zone.lifetime.fraction()) * 0.2;
        sprite.color = GameColors::PROJECTILE_SLOW.with_alpha(alpha);

        // Apply slow on tick
        if zone.tick_timer.just_finished() {
            let zone_pos = zone_transform.translation.truncate();
            let nearby = spatial_grid.query_range(zone_pos, zone.radius);
            for enemy_entity in nearby {
                if let Ok((_, mut enemy, enemy_transform)) = enemies.get_mut(enemy_entity) {
                    let enemy_pos = enemy_transform.translation.truncate();
                    if zone_pos.distance(enemy_pos) < zone.radius {
                        enemy.apply_slow(0.8, CombatConstants::SLOW_FACTOR);
                    }
                }
            }
        }

        if zone.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

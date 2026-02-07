use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{CombatConstants, GameColors, ShapeSizes, ZDepth};
use crate::loading::GameAssets;

use super::{
    enemy::Enemy,
    rand_simple,
    spatial::SpatialGrid,
    tower::TowerType,
    GameEntity,
};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectileEvent>()
            .add_systems(
                Update,
                (spawn_projectiles, projectile_movement, projectile_collision, update_effects, update_damage_numbers)
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
    pub hit_enemies: Vec<Entity>,     // Enemies already hit (for chain)
    pub poison_duration_bonus: f32,   // Synergy bonus for poison duration
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
                hit_enemies: vec![],
                poison_duration_bonus: event.poison_duration_bonus,
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
) {
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

        // Spawn trail particle
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

fn projectile_collision(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
    assets: Res<GameAssets>,
    spatial_grid: Res<SpatialGrid>,
) {
    // Collect chain bounce data to spawn after iteration
    let mut chain_bounces: Vec<(Vec2, f32, Vec<Entity>, u32)> = Vec::new();

    // Check projectile collisions
    for (proj_entity, projectile, proj_transform) in &projectiles {
        let proj_pos = proj_transform.translation.truncate();

        // Check collision with target
        if let Ok((enemy_entity, mut enemy, enemy_transform)) = enemies.get_mut(projectile.target) {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = proj_pos.distance(enemy_pos);

            if distance < CombatConstants::HIT_RADIUS {
                // Calculate damage with armor reduction (skips dead enemies)
                let actual_damage = enemy.apply_armor_damage(projectile.damage);

                // Spawn damage number
                if actual_damage > 0.0 {
                    spawn_damage_number(&mut commands, &assets, enemy_pos, actual_damage);
                }

                // Apply slow effect for slow towers
                if projectile.tower_type == TowerType::Slow {
                    enemy.apply_slow(CombatConstants::SLOW_DURATION, CombatConstants::SLOW_FACTOR);
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
                    hit_list.push(enemy_entity);

                    // Find next target for bounce (30% damage reduction per bounce)
                    let bounce_damage = projectile.damage * CombatConstants::CHAIN_DAMAGE_DECAY;
                    let remaining_bounces = projectile.chain_bounces.saturating_sub(1);

                    chain_bounces.push((enemy_pos, bounce_damage, hit_list, remaining_bounces));
                }

                // Splash damage (spatial grid narrows search to nearby enemies)
                if projectile.tower_type == TowerType::Splash {
                    let splash_radius = ShapeSizes::SPLASH_RADIUS;
                    let splash_damage = projectile.damage * CombatConstants::SPLASH_DAMAGE_RATIO;

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
                                    spawn_damage_number(&mut commands, &assets, other_pos, other_actual_damage);
                                }
                            }
                        }
                    }

                    // Spawn splash effect
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: GameColors::PROJECTILE_SPLASH.with_alpha(0.3),
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

                // Despawn projectile
                commands.entity(proj_entity).despawn_recursive();
            }
        } else {
            // Target doesn't exist anymore
            commands.entity(proj_entity).despawn_recursive();
        }
    }

    // Spawn chain bounce projectiles (spatial grid narrows search to nearby enemies)
    for (origin_pos, bounce_damage, hit_list, remaining_bounces) in chain_bounces {
        // Find nearest enemy not in hit list
        let mut best_target: Option<(Entity, f32)> = None;
        let bounce_range = ShapeSizes::CHAIN_BOUNCE_RANGE;

        let nearby = spatial_grid.query_range(origin_pos, bounce_range);
        for entity in nearby {
            if hit_list.contains(&entity) {
                continue;
            }
            if let Ok((_, enemy, transform)) = enemies.get(entity) {
                if enemy.marked_dead || enemy.health <= 0.0 {
                    continue;
                }

                let enemy_pos = transform.translation.truncate();
                let dist = origin_pos.distance(enemy_pos);

                if dist <= bounce_range {
                    if let Some((_, best_dist)) = best_target {
                        if dist < best_dist {
                            best_target = Some((entity, dist));
                        }
                    } else {
                        best_target = Some((entity, dist));
                    }
                }
            }
        }

        if let Some((next_target, _)) = best_target {
            // Spawn chain lightning visual effect (line from origin to new target)
            if let Ok((_, _, next_transform)) = enemies.get(next_target) {
                let next_pos = next_transform.translation.truncate();

                // Spawn a quick lightning bolt effect
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: GameColors::PROJECTILE_CHAIN.with_alpha(0.6),
                            custom_size: Some(Vec2::splat(8.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(origin_pos.extend(ZDepth::PROJECTILE)),
                        ..default()
                    },
                    Projectile {
                        target: next_target,
                        damage: bounce_damage,
                        speed: CombatConstants::CHAIN_BOUNCE_SPEED,
                        tower_type: TowerType::Chain,
                        predicted_pos: Some(next_pos),
                        chain_bounces: remaining_bounces,
                        hit_enemies: hit_list,
                        poison_duration_bonus: 0.0, // Chain doesn't apply poison
                    },
                    GameEntity,
                ));
            }
        }
    }
}

fn spawn_damage_number(commands: &mut Commands, assets: &GameAssets, pos: Vec2, damage: f32) {
    // Random offset and velocity for variety
    let offset_x = (rand_simple(pos.x) - 0.5) * 20.0;
    let velocity = Vec2::new(offset_x, 40.0);

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                format!("{:.0}", damage),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: ShapeSizes::DAMAGE_TEXT_SIZE,
                    color: GameColors::DAMAGE_TEXT,
                },
            ),
            transform: Transform::from_translation(Vec3::new(pos.x, pos.y + 15.0, ZDepth::FLOATING_TEXT)),
            ..default()
        },
        DamageNumber {
            lifetime: Timer::from_seconds(CombatConstants::DAMAGE_NUMBER_LIFETIME, TimerMode::Once),
            velocity,
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

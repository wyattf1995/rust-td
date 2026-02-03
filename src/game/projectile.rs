use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::{
    enemy::Enemy,
    tower::TowerType,
    GameEntity,
};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectileEvent>()
            .add_systems(
                Update,
                (spawn_projectiles, projectile_movement, projectile_collision, update_effects)
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

/// Event to spawn a projectile
#[derive(Event)]
pub struct SpawnProjectileEvent {
    pub start: Vec2,
    pub target: Entity,
    pub damage: f32,
    pub tower_type: TowerType,
}

fn spawn_projectiles(
    mut commands: Commands,
    mut events: EventReader<SpawnProjectileEvent>,
) {
    for event in events.read() {
        let color = match event.tower_type {
            TowerType::Basic => GameColors::PROJECTILE_BASIC,
            TowerType::Splash => GameColors::PROJECTILE_SPLASH,
            TowerType::Slow => GameColors::PROJECTILE_SLOW,
            TowerType::Sniper => GameColors::PROJECTILE_SNIPER,
            TowerType::Rapid => GameColors::PROJECTILE_RAPID,
        };

        let size = match event.tower_type {
            TowerType::Basic => ShapeSizes::PROJECTILE_BASIC,
            TowerType::Splash => ShapeSizes::PROJECTILE_SPLASH,
            TowerType::Slow => ShapeSizes::PROJECTILE_SLOW,
            TowerType::Sniper => 14.0,  // Larger sniper bullet
            TowerType::Rapid => 5.0,    // Small rapid bullets
        };

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                transform: Transform::from_translation(event.start.extend(4.0)),
                ..default()
            },
            Projectile {
                target: event.target,
                damage: event.damage,
                speed: 400.0,
                tower_type: event.tower_type,
            },
            GameEntity,
        ));
    }
}

fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &Projectile, &mut Transform)>,
    enemies: Query<&Transform, (With<Enemy>, Without<Projectile>)>,
    time: Res<Time>,
) {
    for (entity, projectile, mut transform) in &mut projectiles {
        // Get target position
        let target_pos = if let Ok(enemy_transform) = enemies.get(projectile.target) {
            enemy_transform.translation.truncate()
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
        let trail_color = match projectile.tower_type {
            TowerType::Basic => GameColors::PROJECTILE_BASIC.with_alpha(0.5),
            TowerType::Splash => GameColors::PROJECTILE_SPLASH.with_alpha(0.5),
            TowerType::Slow => GameColors::PROJECTILE_SLOW.with_alpha(0.5),
            TowerType::Sniper => GameColors::PROJECTILE_SNIPER.with_alpha(0.5),
            TowerType::Rapid => GameColors::PROJECTILE_RAPID.with_alpha(0.5),
        };

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
                lifetime: Timer::from_seconds(0.15, TimerMode::Once),
            },
            GameEntity,
        ));
    }
}

fn projectile_collision(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
) {
    // Check projectile collisions
    for (proj_entity, projectile, proj_transform) in &projectiles {
        let proj_pos = proj_transform.translation.truncate();

        // Check collision with target
        if let Ok((enemy_entity, mut enemy, enemy_transform)) = enemies.get_mut(projectile.target) {
            let enemy_pos = enemy_transform.translation.truncate();
            let distance = proj_pos.distance(enemy_pos);

            if distance < 20.0 {
                // Apply damage
                enemy.health -= projectile.damage;

                // Apply slow effect for slow towers
                if projectile.tower_type == TowerType::Slow {
                    enemy.apply_slow(2.0, 0.5);
                }

                // Splash damage
                if projectile.tower_type == TowerType::Splash {
                    let splash_radius = ShapeSizes::SPLASH_RADIUS;
                    let splash_damage = projectile.damage * 0.5;

                    for (other_entity, mut other_enemy, other_transform) in &mut enemies {
                        if other_entity == enemy_entity {
                            continue;
                        }

                        let other_pos = other_transform.translation.truncate();
                        if enemy_pos.distance(other_pos) < splash_radius {
                            other_enemy.health -= splash_damage;
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
                            transform: Transform::from_translation(enemy_pos.extend(3.8)),
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

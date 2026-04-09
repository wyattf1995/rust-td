use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ZDepth};
use crate::persistence::GameSettings;

use super::{
    ease_out,
    enemy::Enemy,
    rand_simple,
    tower::{Tower, TowerCore},
    GameEntity,
};

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_hit_sparks,
                update_speed_streaks,
                update_armor_shimmer,
                update_tower_idle_glow,
                update_path_energy_pulse,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Hit spark particle (spawned on projectile impact)
#[derive(Component)]
pub struct HitSpark {
    pub lifetime: Timer,
    pub velocity: Vec2,
}

/// Speed streak behind fast enemies
#[derive(Component)]
pub struct SpeedStreak {
    pub lifetime: Timer,
}

/// Spawn hit sparks at an impact position
pub fn spawn_hit_sparks(
    commands: &mut Commands,
    pos: Vec2,
    count: usize,
) {
    for i in 0..count {
        let seed = pos.x + pos.y * 3.7 + i as f32;
        let angle = rand_simple(seed) * std::f32::consts::TAU;
        let speed = 120.0 + rand_simple(seed + 50.0) * 60.0;
        let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(1.0, 0.95, 0.8, 0.9),
                    custom_size: Some(Vec2::splat(3.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(ZDepth::DEATH_EFFECT + 0.1)),
                ..default()
            },
            HitSpark {
                lifetime: Timer::from_seconds(0.12, TimerMode::Once),
                velocity,
            },
            GameEntity,
        ));
    }
}

fn update_hit_sparks(
    mut commands: Commands,
    mut sparks: Query<(Entity, &mut HitSpark, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut spark, mut transform, mut sprite) in &mut sparks {
        spark.lifetime.tick(time.delta());

        // Move with deceleration
        transform.translation.x += spark.velocity.x * time.delta_seconds();
        transform.translation.y += spark.velocity.y * time.delta_seconds();
        spark.velocity *= 0.85_f32.powf(time.delta_seconds() * 60.0);

        // Fade out
        let alpha = (1.0 - ease_out(spark.lifetime.fraction())) * 0.9;
        sprite.color = Color::srgba(1.0, 0.95, 0.8, alpha);

        if spark.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_speed_streaks(
    mut commands: Commands,
    enemies: Query<(&Enemy, &Transform)>,
    mut streaks: Query<(Entity, &mut SpeedStreak, &mut Sprite)>,
    settings: Res<GameSettings>,
    time: Res<Time>,
    mut frame_count: Local<u32>,
) {
    *frame_count = frame_count.wrapping_add(1);

    // Spawn streaks for fast enemies
    let streak_skip = settings.particle_density.trail_skip() * 2;
    #[allow(clippy::manual_is_multiple_of)] // is_multiple_of is unstable (Rust 1.85)
    if streak_skip > 0 && *frame_count % streak_skip == 0 {
        for (enemy, transform) in &enemies {
            if enemy.speed > 80.0 && !enemy.marked_dead && enemy.health > 0.0 {
                let pos = transform.translation.truncate();
                let color = enemy.enemy_type.color().with_alpha(0.3);

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color,
                            custom_size: Some(Vec2::new(8.0, 2.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(pos.extend(ZDepth::DEATH_EFFECT - 0.1)),
                        ..default()
                    },
                    SpeedStreak {
                        lifetime: Timer::from_seconds(0.15, TimerMode::Once),
                    },
                    GameEntity,
                ));
            }
        }
    }

    // Update and despawn streaks
    for (entity, mut streak, mut sprite) in &mut streaks {
        streak.lifetime.tick(time.delta());
        let alpha = (1.0 - ease_out(streak.lifetime.fraction())) * 0.3;
        sprite.color = sprite.color.with_alpha(alpha);

        if streak.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_armor_shimmer(
    mut enemies: Query<(&Enemy, &mut Sprite)>,
    settings: Res<GameSettings>,
    time: Res<Time>,
) {
    // Skip on low density
    if settings.particle_density == crate::persistence::ParticleDensity::Low {
        return;
    }
    if settings.reduce_motion { return; }

    let elapsed = time.elapsed_seconds();

    for (enemy, mut sprite) in &mut enemies {
        if (enemy.enemy_type.armor() > 0.0 || enemy.bonus_armor > 0.0) && !enemy.marked_dead {
            // Periodic shimmer using sin wave
            let shimmer = ((elapsed * 4.0 + enemy.base_speed).sin() * 0.5 + 0.5).powf(8.0) * 0.15;
            let base_color = enemy.enemy_type.color();
            let srgba = base_color.to_srgba();
            sprite.color = Color::srgb(
                (srgba.red + shimmer).min(1.0),
                (srgba.green + shimmer).min(1.0),
                (srgba.blue + shimmer).min(1.0),
            );
        }
    }
}

fn update_tower_idle_glow(
    towers: Query<(&Tower, &Transform)>,
    mut cores: Query<(&TowerCore, &mut Sprite)>,
    settings: Res<GameSettings>,
    time: Res<Time>,
) {
    // Skip on low density
    if settings.particle_density == crate::persistence::ParticleDensity::Low {
        return;
    }
    if settings.reduce_motion { return; }

    let elapsed = time.elapsed_seconds();

    for (core, mut sprite) in &mut cores {
        if let Ok((tower, _)) = towers.get(core.tower) {
            let base_color = tower.accent_color();
            let brightness = 1.0 + 0.12 * (tower.level - 1) as f32;

            // Phase offset per tower grid position so they don't sync
            let phase = (tower.grid_x as f32 * 1.7 + tower.grid_y as f32 * 2.3) % std::f32::consts::TAU;
            let pulse = ((elapsed * 1.5 + phase).sin() * 0.5 + 0.5) * 0.05;

            let srgba = base_color.to_srgba();
            sprite.color = Color::srgb(
                (srgba.red * brightness + pulse).min(1.0),
                (srgba.green * brightness + pulse).min(1.0),
                (srgba.blue * brightness + pulse).min(1.0),
            );
        }
    }
}

fn update_path_energy_pulse(
    mut dots: Query<(&super::map::PathFlowDot, &mut Sprite)>,
    settings: Res<GameSettings>,
    time: Res<Time>,
) {
    // Skip on low density
    if settings.particle_density == crate::persistence::ParticleDensity::Low {
        return;
    }
    if settings.reduce_motion { return; }

    let elapsed = time.elapsed_seconds();

    for (dot, mut sprite) in &mut dots {
        // Occasional bright flash using modulo condition
        let flash_phase = (elapsed * 2.0 + dot.progress * 10.0).sin();
        if flash_phase > 0.95 {
            sprite.color = GameColors::PATH_INDICATOR.with_alpha(0.5);
        }
        // Normal alpha handled by update_path_flow_dots, we only override on flash
    }
}

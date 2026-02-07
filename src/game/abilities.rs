use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ZDepth};

use super::{
    enemy::{Enemy, WaveManager},
    ui::UiZones,
    GameEntity,
};

// Ability tuning constants
const FREEZE_COOLDOWN: f32 = 45.0;
const FREEZE_DURATION: f32 = 3.0;
const GOLD_RUSH_COOLDOWN: f32 = 60.0;
const GOLD_RUSH_DURATION: f32 = 10.0;
const ARTILLERY_COOLDOWN: f32 = 90.0;
const ARTILLERY_DAMAGE: f32 = 400.0;
const ARTILLERY_RADIUS: f32 = 100.0;

pub struct AbilitiesPlugin;

impl Plugin for AbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerAbilities>()
            .add_event::<FreezeAbilityEvent>()
            .add_event::<GoldRushAbilityEvent>()
            .add_event::<ArtilleryAbilityEvent>()
            .add_systems(OnEnter(GameState::Playing), reset_abilities)
            .add_systems(
                Update,
                (
                    ability_cooldowns,
                    ability_input,
                    handle_freeze_ability,
                    handle_gold_rush_ability,
                    handle_artillery_ability,
                    update_freeze_effects,
                    update_gold_rush_status,
                    update_artillery_target,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Player's active abilities
#[derive(Resource)]
pub struct PlayerAbilities {
    pub freeze_cooldown: Timer,
    pub freeze_ready: bool,
    pub gold_rush_cooldown: Timer,
    pub gold_rush_ready: bool,
    pub gold_rush_active: Option<Timer>,
    pub artillery_cooldown: Timer,
    pub artillery_ready: bool,
    pub artillery_targeting: bool, // Waiting for click to fire
}

impl Default for PlayerAbilities {
    fn default() -> Self {
        Self {
            freeze_cooldown: Timer::from_seconds(FREEZE_COOLDOWN, TimerMode::Once),
            freeze_ready: true,
            gold_rush_cooldown: Timer::from_seconds(GOLD_RUSH_COOLDOWN, TimerMode::Once),
            gold_rush_ready: true,
            gold_rush_active: None,
            artillery_cooldown: Timer::from_seconds(ARTILLERY_COOLDOWN, TimerMode::Once),
            artillery_ready: true,
            artillery_targeting: false,
        }
    }
}

/// Events
#[derive(Event)]
pub struct FreezeAbilityEvent;

#[derive(Event)]
pub struct GoldRushAbilityEvent;

#[derive(Event)]
pub struct ArtilleryAbilityEvent {
    pub position: Vec2,
}

/// Component for freeze visual effect
#[derive(Component)]
pub struct FreezeEffect {
    pub lifetime: Timer,
}

/// Component for frozen enemies
#[derive(Component)]
pub struct Frozen {
    pub timer: Timer,
    pub original_speed: f32,
}

/// Component for artillery strike visual
#[derive(Component)]
pub struct ArtilleryStrike {
    pub lifetime: Timer,
}

/// Component for artillery targeting cursor
#[derive(Component)]
pub struct ArtilleryTargetCursor;

/// Tick a cooldown timer and mark ready when finished.
fn tick_cooldown(ready: &mut bool, cooldown: &mut Timer, delta: std::time::Duration) {
    if !*ready {
        cooldown.tick(delta);
        if cooldown.finished() {
            *ready = true;
            cooldown.reset();
        }
    }
}

fn reset_abilities(mut abilities: ResMut<PlayerAbilities>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    *abilities = PlayerAbilities::default();
}

fn ability_cooldowns(
    mut abilities: ResMut<PlayerAbilities>,
    time: Res<Time>,
    wave_manager: Res<super::enemy::WaveManager>,
) {
    if !wave_manager.wave_active { return; }
    let delta = time.delta();
    let a = &mut *abilities;
    tick_cooldown(&mut a.freeze_ready, &mut a.freeze_cooldown, delta);
    tick_cooldown(&mut a.gold_rush_ready, &mut a.gold_rush_cooldown, delta);
    tick_cooldown(&mut a.artillery_ready, &mut a.artillery_cooldown, delta);
}

fn ability_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    pointer: Res<super::PointerState>,
    mut abilities: ResMut<PlayerAbilities>,
    mut freeze_events: EventWriter<FreezeAbilityEvent>,
    mut gold_rush_events: EventWriter<GoldRushAbilityEvent>,
    mut artillery_events: EventWriter<ArtilleryAbilityEvent>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    ui_zones: Res<UiZones>,
) {
    // Freeze - Q key
    if keyboard.just_pressed(KeyCode::KeyQ) && abilities.freeze_ready {
        freeze_events.send(FreezeAbilityEvent);
        abilities.freeze_ready = false;
    }

    // Gold Rush - W key
    if keyboard.just_pressed(KeyCode::KeyW) && abilities.gold_rush_ready {
        gold_rush_events.send(GoldRushAbilityEvent);
        abilities.gold_rush_ready = false;
    }

    // Artillery - E key to activate targeting, click to fire
    if keyboard.just_pressed(KeyCode::KeyE) && abilities.artillery_ready {
        abilities.artillery_targeting = true;
    }

    // Cancel artillery targeting with Escape
    if keyboard.just_pressed(KeyCode::Escape) && abilities.artillery_targeting {
        abilities.artillery_targeting = false;
    }

    // Fire artillery on click/touch while targeting
    if abilities.artillery_targeting && pointer.just_pressed {
        if let Ok(window) = windows.get_single() {
            if let Ok((camera, camera_transform)) = camera_q.get_single() {
                if let Some(cursor_pos) = pointer.position {
                    // Don't fire if clicking on UI areas (dynamic zone boundaries)
                    let window_height = window.height();
                    if cursor_pos.y <= window_height - ui_zones.bottom_bar_height - 10.0 && cursor_pos.y >= ui_zones.top_bar_height + 10.0 {
                        if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                            artillery_events.send(ArtilleryAbilityEvent { position: world_pos });
                            abilities.artillery_targeting = false;
                            abilities.artillery_ready = false;
                        }
                    }
                }
            }
        }
    }
}

fn handle_freeze_ability(
    mut commands: Commands,
    mut events: EventReader<FreezeAbilityEvent>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform), Without<Frozen>>,
    wave_manager: Res<WaveManager>,
) {
    for _ in events.read() {
        // Freeze scales with wave: 3s base → 5s max
        let wave = wave_manager.current_wave as f32;
        let duration = FREEZE_DURATION + (wave * 0.1).min(2.0);

        for (entity, mut enemy, transform) in &mut enemies {
            let original_speed = enemy.speed;
            enemy.speed = 0.0;

            commands.entity(entity).try_insert(Frozen {
                timer: Timer::from_seconds(duration, TimerMode::Once),
                original_speed,
            });

            // Spawn freeze visual effect on each enemy
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::ABILITY_FREEZE.with_alpha(0.4),
                        custom_size: Some(Vec2::splat(enemy.enemy_type.size() + 10.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(transform.translation.truncate().extend(ZDepth::FREEZE_EFFECT)),
                    ..default()
                },
                FreezeEffect {
                    lifetime: Timer::from_seconds(duration, TimerMode::Once),
                },
                GameEntity,
            ));
        }
    }
}

fn handle_gold_rush_ability(
    mut events: EventReader<GoldRushAbilityEvent>,
    mut abilities: ResMut<PlayerAbilities>,
) {
    for _ in events.read() {
        // Activate gold rush for 10 seconds
        abilities.gold_rush_active = Some(Timer::from_seconds(GOLD_RUSH_DURATION, TimerMode::Once));
    }
}

fn handle_artillery_ability(
    mut commands: Commands,
    mut events: EventReader<ArtilleryAbilityEvent>,
    mut enemies: Query<(&mut Enemy, &Transform)>,
    wave_manager: Res<WaveManager>,
) {
    // Artillery scales with wave: damage +15%/wave, radius +2/wave (capped at 160)
    let wave = wave_manager.current_wave as f32;
    let artillery_damage = ARTILLERY_DAMAGE * (1.0 + wave * 0.15);
    let artillery_radius = (ARTILLERY_RADIUS + wave * 2.0).min(160.0);

    for event in events.read() {
        // Damage all enemies in radius
        for (mut enemy, transform) in &mut enemies {
            let enemy_pos = transform.translation.truncate();
            let distance = event.position.distance(enemy_pos);

            if distance <= artillery_radius {
                // Full damage at center, falloff at edges
                let damage_falloff = 1.0 - (distance / artillery_radius) * 0.5;
                enemy.apply_armor_damage(artillery_damage * damage_falloff);
            }
        }

        // Spawn visual effect
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::ABILITY_NUKE.with_alpha(0.6),
                    custom_size: Some(Vec2::splat(artillery_radius * 2.0)),
                    ..default()
                },
                transform: Transform::from_translation(event.position.extend(ZDepth::ABILITY_EFFECT)),
                ..default()
            },
            ArtilleryStrike {
                lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            },
            GameEntity,
        ));
    }
}

fn update_freeze_effects(
    mut commands: Commands,
    mut effects: Query<(Entity, &mut FreezeEffect, &mut Sprite)>,
    mut frozen_enemies: Query<(Entity, &mut Frozen, &mut Enemy)>,
    time: Res<Time>,
) {
    // Update freeze visual effects
    for (entity, mut effect, mut sprite) in &mut effects {
        effect.lifetime.tick(time.delta());
        let alpha = (1.0 - effect.lifetime.fraction()) * 0.4;
        sprite.color = GameColors::ABILITY_FREEZE.with_alpha(alpha);

        if effect.lifetime.finished() {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }

    // Update frozen enemies
    for (entity, mut frozen, mut enemy) in &mut frozen_enemies {
        frozen.timer.tick(time.delta());

        if frozen.timer.finished() {
            // Restore speed
            enemy.speed = frozen.original_speed;
            if let Some(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.remove::<Frozen>();
            }
        }
    }
}

fn update_gold_rush_status(
    mut abilities: ResMut<PlayerAbilities>,
    time: Res<Time>,
) {
    if let Some(ref mut timer) = abilities.gold_rush_active {
        timer.tick(time.delta());
        if timer.finished() {
            abilities.gold_rush_active = None;
        }
    }
}

fn update_artillery_target(
    mut commands: Commands,
    abilities: Res<PlayerAbilities>,
    mut cursor_query: Query<(Entity, &mut Transform), With<ArtilleryTargetCursor>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    pointer: Res<super::PointerState>,
    mut artillery_effects: Query<(Entity, &mut ArtilleryStrike, &mut Sprite, &mut Transform), Without<ArtilleryTargetCursor>>,
    time: Res<Time>,
) {
    // Update artillery strike visual effects
    for (entity, mut strike, mut sprite, mut transform) in &mut artillery_effects {
        strike.lifetime.tick(time.delta());
        let progress = strike.lifetime.fraction();

        // Expand and fade
        let scale = 1.0 + progress * 0.3;
        transform.scale = Vec3::splat(scale);

        let alpha = (1.0 - progress) * 0.6;
        sprite.color = GameColors::ABILITY_NUKE.with_alpha(alpha);

        if strike.lifetime.finished() {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }

    // Show/hide targeting cursor
    if abilities.artillery_targeting {
        if let Ok((camera, camera_transform)) = camera_q.get_single() {
            if let Some(cursor_pos) = pointer.position {
                if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    // Update or spawn cursor
                    if let Ok((_, mut transform)) = cursor_query.get_single_mut() {
                        transform.translation = world_pos.extend(ZDepth::ARTILLERY_CURSOR);
                    } else {
                        // Spawn targeting cursor
                        commands.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: GameColors::ABILITY_NUKE.with_alpha(0.3),
                                    custom_size: Some(Vec2::splat(200.0)), // Preview radius
                                    ..default()
                                },
                                transform: Transform::from_translation(world_pos.extend(ZDepth::ARTILLERY_CURSOR)),
                                ..default()
                            },
                            ArtilleryTargetCursor,
                            GameEntity,
                        ));
                    }
                }
            }
        }
    } else {
        // Remove cursor when not targeting
        for (entity, _) in &cursor_query {
            if let Some(entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn_recursive();
            }
        }
    }
}

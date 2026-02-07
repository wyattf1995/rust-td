use bevy::prelude::*;
use bevy::input::touch::Touches;

pub mod abilities;
pub mod economy;
pub mod enemy;
pub mod map;
pub mod projectile;
pub mod spatial;
pub mod tower;
pub mod ui;

use crate::analytics::{Analytics, track_with_context};
use crate::GameState;
use crate::graphics::shapes::GameColors;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            map::MapPlugin,
            tower::TowerPlugin,
            enemy::EnemyPlugin,
            projectile::ProjectilePlugin,
            economy::EconomyPlugin,
            ui::GameUiPlugin,
            spatial::SpatialPlugin,
            abilities::AbilitiesPlugin,
        ))
        .init_resource::<ScreenShake>()
        .init_resource::<PointerState>()
        .add_systems(Update, update_pointer_state)
        .add_systems(OnEnter(GameState::Playing), setup_game)
        // Note: Don't cleanup on exit Playing - would destroy game when pausing
        // Cleanup happens when entering Menu instead
        .add_systems(OnEnter(GameState::Menu), cleanup_game)
        .add_systems(OnEnter(GameState::GameOver), setup_game_over)
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over)
        .add_systems(OnEnter(GameState::Victory), setup_victory)
        .add_systems(OnExit(GameState::Victory), cleanup_victory)
        .add_systems(
            Update,
            update_screen_shake.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (restart_button_system, restart_interaction)
                .run_if(in_state(GameState::GameOver).or_else(in_state(GameState::Victory))),
        );
    }
}

#[derive(Component)]
pub struct GameEntity;

/// Screen shake resource for camera trauma
#[derive(Resource, Default)]
pub struct ScreenShake {
    pub trauma: f32,
}

/// Marker resource: present while a game session is active.
/// Prevents OnEnter(Playing) init systems from re-running when resuming from pause.
#[derive(Resource)]
pub struct GameActive;

/// Unified pointer state: reads from both mouse cursor and touch input.
/// Game-world systems should use this instead of window.cursor_position() directly.
#[derive(Resource, Default)]
pub struct PointerState {
    /// Current pointer position in window/viewport coordinates
    pub position: Option<Vec2>,
    /// Whether a "click" (mouse left-click or touch-start) happened this frame
    pub just_pressed: bool,
}

#[derive(Component)]
struct GameOverScreen;

#[derive(Component)]
struct VictoryScreen;

#[derive(Component)]
struct RestartButton;

fn setup_game(mut _commands: Commands) {
    // Camera is already spawned in loading state and persists
}

fn cleanup_game(mut commands: Commands, query: Query<Entity, With<GameEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<GameActive>();
}

fn setup_game_over(
    mut commands: Commands,
    assets: Res<crate::loading::GameAssets>,
    analytics: Res<Analytics>,
    wave_manager: Res<enemy::WaveManager>,
    economy: Res<economy::PlayerEconomy>,
) {
    // Track game over event with stats
    let wave_reached = wave_manager.current_wave.to_string();
    let score = economy.score.to_string();
    track_with_context(
        &analytics,
        "game_over",
        &[
            ("wave_reached", &wave_reached),
            ("score", &score),
        ],
    );

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: GameColors::GAME_OVER_OVERLAY.into(),
                ..default()
            },
            GameOverScreen,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "GAME OVER",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 72.0,
                    color: GameColors::PRIMARY,
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            }));

            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(200.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::PRIMARY.into(),
                        ..default()
                    },
                    RestartButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "RESTART",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 28.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn cleanup_game_over(mut commands: Commands, query: Query<Entity, With<GameOverScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_victory(mut commands: Commands, assets: Res<crate::loading::GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: GameColors::GAME_OVER_OVERLAY.into(),
                ..default()
            },
            VictoryScreen,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "VICTORY!",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 72.0,
                    color: GameColors::SUCCESS,
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            }));

            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(200.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::SUCCESS.into(),
                        ..default()
                    },
                    RestartButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "PLAY AGAIN",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 28.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn cleanup_victory(mut commands: Commands, query: Query<Entity, With<VictoryScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn restart_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RestartButton>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = GameColors::SECONDARY.into();
            }
            Interaction::Hovered => {
                *color = GameColors::PRIMARY.with_alpha(0.8).into();
            }
            Interaction::None => {
                *color = GameColors::PRIMARY.into();
            }
        }
    }
}

fn update_pointer_state(
    mut pointer: ResMut<PointerState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
) {
    pointer.just_pressed = false;

    // Touch takes priority (mobile)
    if let Some(touch) = touches.iter_just_pressed().next() {
        pointer.position = Some(touch.position());
        pointer.just_pressed = true;
    } else if let Some(touch) = touches.iter().next() {
        // Finger is held down — update position for hover/targeting
        pointer.position = Some(touch.position());
    } else if let Ok(window) = windows.get_single() {
        // Fallback to mouse cursor (desktop)
        pointer.position = window.cursor_position();
        if mouse_button.just_pressed(MouseButton::Left) {
            pointer.just_pressed = true;
        }
    }

    // No touch AND no cursor → clear position (touch-only devices)
    if touches.iter().count() == 0 {
        if let Ok(window) = windows.get_single() {
            if window.cursor_position().is_none() {
                pointer.position = None;
            }
        }
    }
}

fn update_screen_shake(
    mut screen_shake: ResMut<ScreenShake>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    if screen_shake.trauma > 0.01 {
        // Decay trauma
        screen_shake.trauma *= 0.92_f32.powf(time.delta_seconds() * 60.0);

        let t = time.elapsed_seconds();
        let offset_x = screen_shake.trauma * (t * 50.0).sin() * 3.0;
        let offset_y = screen_shake.trauma * (t * 40.0).cos() * 3.0;

        // Clamp offset
        let offset_x = offset_x.clamp(-5.0, 5.0);
        let offset_y = offset_y.clamp(-5.0, 5.0);

        for mut transform in &mut camera_query {
            transform.translation.x = offset_x;
            transform.translation.y = offset_y;
        }
    } else {
        screen_shake.trauma = 0.0;
        // Reset camera to origin
        for mut transform in &mut camera_query {
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
    }
}

fn restart_interaction(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Button click restart
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Menu);
        }
    }

    // Quick restart with R key
    if keyboard.just_pressed(KeyCode::KeyR) {
        next_state.set(GameState::Menu);
    }
}

use bevy::prelude::*;
use bevy::input::touch::Touches;

pub mod abilities;
pub mod economy;
pub mod effects;
pub mod enemy;
pub mod map;
pub mod projectile;
pub mod spatial;
pub mod stats;
pub mod tower;
pub mod ui;

use crate::analytics::{Analytics, track_with_context};
use crate::persistence::{GameSettings, HighScores, save_highscores};
use crate::GameState;
use crate::graphics::shapes::GameColors;

/// Simple pseudo-random based on position (deterministic)
pub(crate) fn rand_simple(seed: f32) -> f32 {
    ((seed * 12.9898).sin() * 43758.5453).fract()
}

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
            stats::StatsPlugin,
            effects::EffectsPlugin,
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
    selected_map: Res<map::SelectedMap>,
    mut high_scores: ResMut<HighScores>,
    mut game_stats: ResMut<stats::GameStats>,
) {
    let map_name = selected_map.0.name();
    let wave = wave_manager.current_wave;
    let score = economy.score;

    // Finalize stats
    game_stats.finalize(score, wave);

    // Check and save high score
    let is_new_best = high_scores.update_if_better(map_name, wave, score);
    if is_new_best {
        save_highscores(&high_scores);
    }

    // Track game over event with stats
    let wave_reached = wave.to_string();
    let score_str = score.to_string();
    track_with_context(
        &analytics,
        "game_over",
        &[
            ("wave_reached", &wave_reached),
            ("score", &score_str),
            ("map", map_name),
        ],
    );

    // Build stats summary text
    let summary = format!(
        "Waves: {}   Score: {}\nKills: {}   Escaped: {}   Max Combo: {}\nGold Earned: {}   Gold Spent: {}",
        game_stats.waves_survived,
        game_stats.total_score,
        game_stats.total_enemies_killed,
        game_stats.total_enemies_escaped,
        game_stats.max_combo,
        game_stats.total_gold_earned,
        game_stats.total_gold_spent,
    );

    // Build kill breakdown
    let mut kill_breakdown = String::new();
    let mut kills_sorted: Vec<(u8, u32)> = game_stats.kills_by_type.iter().map(|(&k, &v)| (k, v)).collect();
    kills_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for (key, count) in &kills_sorted {
        if let Some(etype) = stats::GameStats::enemy_type_from_key(*key) {
            let name = match etype {
                enemy::EnemyType::Basic => "Basic",
                enemy::EnemyType::Fast => "Fast",
                enemy::EnemyType::Tank => "Tank",
                enemy::EnemyType::Armored => "Armored",
                enemy::EnemyType::Flying => "Flying",
                enemy::EnemyType::Boss => "Boss",
                enemy::EnemyType::Splitter => "Splitter",
                enemy::EnemyType::MiniSplitter => "Mini",
            };
            kill_breakdown.push_str(&format!("  {} x{}\n", name, count));
        }
    }

    // Best tower info
    let best_tower_text = if let Some(best) = game_stats.best_tower() {
        format!(
            "{} Lv{} — {:.0} dmg, {} kills",
            best.tower_type.name(), best.max_level, best.total_damage, best.total_kills
        )
    } else {
        "No towers placed".to_string()
    };

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
            // "NEW BEST!" text above game over (only if new record)
            if is_new_best {
                parent.spawn(TextBundle::from_section(
                    "NEW BEST!",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 32.0,
                        color: Color::srgb(1.0, 0.85, 0.2),
                    },
                ).with_style(Style {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                }));
            }

            parent.spawn(TextBundle::from_section(
                "GAME OVER",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 56.0,
                    color: GameColors::PRIMARY,
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(16.0)),
                ..default()
            }));

            // Stats panel
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                // Left column: Summary stats
                row.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.08, 0.08, 0.12, 0.8).into(),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                }).with_children(|col| {
                    col.spawn(TextBundle::from_section(
                        &summary,
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 12.0,
                            color: Color::WHITE,
                        },
                    ));

                    if !kill_breakdown.is_empty() {
                        col.spawn(TextBundle::from_section(
                            "Kills by Type:",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 11.0,
                                color: GameColors::PRIMARY,
                            },
                        ).with_style(Style {
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        }));

                        col.spawn(TextBundle::from_section(
                            &kill_breakdown,
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 11.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                            },
                        ));
                    }
                });

                // Right column: Best tower + gold timeline
                row.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.08, 0.08, 0.12, 0.8).into(),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                }).with_children(|col| {
                    col.spawn(TextBundle::from_section(
                        "MVP Tower:",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 11.0,
                            color: GameColors::PRIMARY,
                        },
                    ));
                    col.spawn(TextBundle::from_section(
                        &best_tower_text,
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 12.0,
                            color: Color::WHITE,
                        },
                    ));

                    // Gold per wave mini bar chart
                    if !game_stats.wave_gold.is_empty() {
                        col.spawn(TextBundle::from_section(
                            "Gold Timeline:",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 11.0,
                                color: GameColors::PRIMARY,
                            },
                        ).with_style(Style {
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        }));

                        // Bar chart container
                        col.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::FlexEnd,
                                column_gap: Val::Px(2.0),
                                height: Val::Px(40.0),
                                ..default()
                            },
                            ..default()
                        }).with_children(|chart| {
                            let max_gold = game_stats.wave_gold.iter()
                                .map(|w| w.gold_from_kills + w.gold_from_bonus + w.gold_from_interest + w.gold_from_early_send)
                                .max()
                                .unwrap_or(1)
                                .max(1);

                            // Show up to 20 bars
                            let waves_to_show = game_stats.wave_gold.len().min(20);
                            for record in game_stats.wave_gold.iter().take(waves_to_show) {
                                let total = record.gold_from_kills + record.gold_from_bonus + record.gold_from_interest + record.gold_from_early_send;
                                let height_pct = (total as f32 / max_gold as f32) * 36.0;
                                chart.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Px(6.0),
                                        height: Val::Px(height_pct.max(2.0)),
                                        ..default()
                                    },
                                    background_color: GameColors::GOLD.into(),
                                    border_radius: BorderRadius::all(Val::Px(1.0)),
                                    ..default()
                                });
                            }
                        });
                    }
                });
            });

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

            // Hint
            parent.spawn(TextBundle::from_section(
                "Press R to restart",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            }));
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
    settings: Res<GameSettings>,
) {
    if !settings.screen_shake {
        // Still decay trauma so it doesn't accumulate while disabled
        screen_shake.trauma = 0.0;
        for mut transform in &mut camera_query {
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
        return;
    }

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

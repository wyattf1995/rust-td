use bevy::prelude::*;
use rand::Rng;

use crate::{
    analytics::{Analytics, track_with_context},
    game::map::{GameMap, MapPreset, SelectedMap, GRID_WIDTH, GRID_HEIGHT},
    loading::GameAssets,
    persistence::HighScores,
    GameState,
};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(
                Update,
                (
                    button_system,
                    map_button_hover,
                    button_interaction,
                    map_select_interaction,
                    animate_projectiles,
                    animate_title_glow,
                )
                    .run_if(in_state(GameState::Menu)),
            )
            .add_systems(OnExit(GameState::Menu), cleanup_menu);
    }
}

#[derive(Component)]
struct MenuScreen;

#[derive(Component)]
struct MenuCamera;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct MapSelectButton(MapPreset);

#[derive(Component)]
struct MenuProjectile {
    velocity: Vec2,
}

#[derive(Component)]
struct TitleText;

const BUTTON_NORMAL: Color = Color::srgb(0.91, 0.27, 0.38);
const BUTTON_HOVER: Color = Color::srgb(1.0, 0.37, 0.48);
const BUTTON_PRESSED: Color = Color::srgb(0.71, 0.17, 0.28);

// Neon projectile colors
const NEON_COLORS: [(f32, f32, f32); 8] = [
    (0.4, 0.85, 1.0),   // Cyan (Basic)
    (1.0, 0.5, 0.2),    // Orange (Splash)
    (0.3, 0.9, 1.0),    // Ice blue (Slow)
    (1.0, 0.3, 0.4),    // Red (Sniper)
    (1.0, 0.9, 0.3),    // Yellow (Rapid)
    (0.7, 0.4, 1.0),    // Purple (Chain)
    (0.4, 1.0, 0.5),    // Green (Poison)
    (1.0, 0.85, 0.4),   // Gold (Buff)
];

fn setup_menu(mut commands: Commands, assets: Res<GameAssets>, selected_map: Res<SelectedMap>, high_scores: Res<HighScores>) {
    // Spawn menu camera
    commands.spawn((Camera2dBundle::default(), MenuCamera, MenuScreen));

    // Spawn background projectiles
    let mut rng = rand::thread_rng();
    for _ in 0..25 {
        spawn_projectile(&mut commands, &mut rng, true);
    }

    // Spawn menu UI
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
                background_color: Color::srgba(0.06, 0.06, 0.1, 0.85).into(),
                z_index: ZIndex::Global(10),
                ..default()
            },
            MenuScreen,
        ))
        .with_children(|parent| {
            // Title with glow effect
            parent.spawn((
                TextBundle::from_section(
                    "NEON COMMAND",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 82.0,
                        color: Color::srgb(0.91, 0.27, 0.38),
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(15.0)),
                    ..default()
                }),
                TitleText,
            ));

            // Subtitle
            parent.spawn(
                TextBundle::from_section(
                    "TACTICAL TOWER DEFENSE",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 22.0,
                        color: Color::srgba(0.4, 0.85, 1.0, 0.7),
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                }),
            );

            // Play button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(220.0),
                            height: Val::Px(65.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BUTTON_NORMAL.into(),
                        border_color: Color::srgba(1.0, 1.0, 1.0, 0.3).into(),
                        ..default()
                    },
                    PlayButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "PLAY",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 32.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Map selection row
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    let presets = [MapPreset::Random, MapPreset::Serpentine, MapPreset::Sprint, MapPreset::Spiral];
                    for preset in presets {
                        let is_selected = preset == selected_map.0;
                        let border_color = if is_selected {
                            Color::srgb(0.0, 0.85, 0.95)
                        } else {
                            Color::srgba(1.0, 1.0, 1.0, 0.15)
                        };

                        row.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(130.0),
                                    height: Val::Px(95.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    padding: UiRect::all(Val::Px(4.0)),
                                    ..default()
                                },
                                background_color: Color::srgb(0.14, 0.15, 0.18).into(),
                                border_color: border_color.into(),
                                ..default()
                            },
                            MapSelectButton(preset),
                        ))
                        .with_children(|btn| {
                            // Preview container
                            let path = GameMap::generate_path_for(preset);
                            btn.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(90.0),
                                    height: Val::Px(55.0),
                                    position_type: PositionType::Relative,
                                    ..default()
                                },
                                background_color: Color::srgb(0.08, 0.08, 0.12).into(),
                                ..default()
                            })
                            .with_children(|preview| {
                                let dot_color = Color::srgba(0.0, 0.85, 0.95, 0.7);
                                for &(px, py) in &path {
                                    if px < GRID_WIDTH && py < GRID_HEIGHT {
                                        preview.spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Px(4.0),
                                                height: Val::Px(4.0),
                                                position_type: PositionType::Absolute,
                                                left: Val::Px(px as f32 * 5.0),
                                                top: Val::Px(py as f32 * 5.0),
                                                ..default()
                                            },
                                            background_color: dot_color.into(),
                                            ..default()
                                        });
                                    }
                                }
                            });

                            // Preset name
                            btn.spawn(TextBundle::from_section(
                                preset.name(),
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 12.0,
                                    color: if is_selected {
                                        Color::srgb(0.0, 0.85, 0.95)
                                    } else {
                                        Color::WHITE
                                    },
                                },
                            ));

                            // Description
                            btn.spawn(TextBundle::from_section(
                                preset.description(),
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 9.0,
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                                },
                            ));

                            // Best wave (if record exists)
                            if let Some(entry) = high_scores.get(preset.name()) {
                                btn.spawn(TextBundle::from_section(
                                    format!("Best: Wave {}", entry.wave),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 9.0,
                                        color: Color::srgba(1.0, 0.85, 0.2, 0.5),
                                    },
                                ));
                            }
                        });
                    }
                });

            // Features list
            parent.spawn(
                TextBundle::from_section(
                    "8 Towers • Synergy Combos • Infinite Waves",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 16.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                }),
            );

            // Controls hint
            parent.spawn(
                TextBundle::from_section(
                    "Keys 1-8 to select towers • Click to place",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 14.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.35),
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(15.0)),
                    ..default()
                }),
            );
        });
}

fn spawn_projectile(commands: &mut Commands, rng: &mut impl Rng, random_x: bool) {
    let (r, g, b) = NEON_COLORS[rng.gen_range(0..NEON_COLORS.len())];

    // Random position - spawn from left side (or random x for initial setup)
    let x = if random_x {
        rng.gen_range(-700.0..700.0)
    } else {
        -750.0 + rng.gen_range(-50.0..0.0)
    };
    let y = rng.gen_range(-400.0..400.0);

    // Velocity - always moving right with some vertical variation
    let speed = rng.gen_range(150.0..400.0);
    let angle: f32 = rng.gen_range(-0.2..0.2);
    let velocity = Vec2::new(speed * angle.cos(), speed * angle.sin());

    // Size based on speed (faster = smaller, like distance)
    let size = if speed > 300.0 {
        rng.gen_range(4.0..8.0)
    } else if speed > 200.0 {
        rng.gen_range(6.0..12.0)
    } else {
        rng.gen_range(10.0..18.0)
    };

    // Alpha based on speed (faster = dimmer, like distance)
    let alpha = if speed > 300.0 {
        rng.gen_range(0.3..0.5)
    } else if speed > 200.0 {
        rng.gen_range(0.5..0.7)
    } else {
        rng.gen_range(0.7..0.9)
    };

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(r, g, b, alpha),
                custom_size: Some(Vec2::new(size * 2.5, size)), // Elongated like a streak
                ..default()
            },
            transform: Transform::from_xyz(x, y, -10.0),
            ..default()
        },
        MenuProjectile {
            velocity,
        },
        MenuScreen,
    ));
}

fn animate_projectiles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &MenuProjectile)>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();

    for (entity, mut transform, projectile) in &mut query {
        // Move projectile
        transform.translation.x += projectile.velocity.x * time.delta_seconds();
        transform.translation.y += projectile.velocity.y * time.delta_seconds();

        // If off screen, respawn
        if transform.translation.x > 800.0 || transform.translation.y.abs() > 450.0 {
            commands.entity(entity).despawn();
            spawn_projectile(&mut commands, &mut rng, false);
        }
    }
}

fn animate_title_glow(mut query: Query<&mut Text, With<TitleText>>, time: Res<Time>) {
    for mut text in &mut query {
        // Pulsing glow effect
        let t = time.elapsed_seconds();
        let pulse = (t * 2.0).sin() * 0.15 + 0.85;

        if let Some(section) = text.sections.first_mut() {
            section.style.color = Color::srgb(0.91 * pulse, 0.27 * pulse, 0.38 * pulse);
        }
    }
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<PlayButton>, Without<MapSelectButton>),
    >,
) {
    for (interaction, mut color, mut border) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BUTTON_PRESSED.into();
                *border = Color::srgba(1.0, 1.0, 1.0, 0.5).into();
            }
            Interaction::Hovered => {
                *color = BUTTON_HOVER.into();
                *border = Color::srgba(1.0, 1.0, 1.0, 0.6).into();
            }
            Interaction::None => {
                *color = BUTTON_NORMAL.into();
                *border = Color::srgba(1.0, 1.0, 1.0, 0.3).into();
            }
        }
    }
}

fn map_button_hover(
    mut interaction_query: Query<
        (&Interaction, &MapSelectButton, &mut BackgroundColor),
        (Changed<Interaction>, With<MapSelectButton>),
    >,
) {
    for (interaction, _btn, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.18, 0.2, 0.24).into();
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.2, 0.22, 0.26).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.14, 0.15, 0.18).into();
            }
        }
    }
}

fn button_interaction(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    analytics: Res<Analytics>,
    selected_map: Res<SelectedMap>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            let map_name = selected_map.0.name();
            track_with_context(&analytics, "game_started", &[("map", map_name)]);
            next_state.set(GameState::Playing);
        }
    }
}

fn map_select_interaction(
    interaction_query: Query<(&Interaction, &MapSelectButton), Changed<Interaction>>,
    mut selected_map: ResMut<SelectedMap>,
    mut all_buttons: Query<(&MapSelectButton, &mut BorderColor, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let mut new_selection = None;

    for (interaction, map_btn) in &interaction_query {
        if *interaction == Interaction::Pressed {
            new_selection = Some(map_btn.0);
        }
    }

    if let Some(preset) = new_selection {
        selected_map.0 = preset;

        // Update all button visuals
        for (btn, mut border, children) in &mut all_buttons {
            let is_selected = btn.0 == preset;
            *border = if is_selected {
                Color::srgb(0.0, 0.85, 0.95).into()
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.15).into()
            };

            // Update the name text color (second child, after preview container)
            if let Some(&name_child) = children.iter().nth(1) {
                if let Ok(mut text) = text_query.get_mut(name_child) {
                    if let Some(section) = text.sections.first_mut() {
                        section.style.color = if is_selected {
                            Color::srgb(0.0, 0.85, 0.95)
                        } else {
                            Color::WHITE
                        };
                    }
                }
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

use bevy::prelude::*;

use crate::{loading::GameAssets, GameState};
use crate::graphics::shapes::GameColors;

use super::{
    economy::PlayerEconomy,
    enemy::WaveManager,
    map::GameMap,
    tower::{PlaceTowerEvent, SelectedTowerType, TowerType},
    GameEntity,
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(
                Update,
                (
                    update_gold_display,
                    update_lives_display,
                    update_wave_display,
                    tower_button_system,
                    start_wave_button,
                    handle_tile_click,
                    update_tower_selection,
                    update_info_panel,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
struct GoldText;

#[derive(Component)]
struct LivesText;

#[derive(Component)]
struct WaveText;

#[derive(Component)]
struct TowerButton(TowerType);

#[derive(Component)]
struct StartWaveButton;

#[derive(Component)]
struct TowerButtonBorder(TowerType);

#[derive(Component)]
struct InfoPanel;

#[derive(Component)]
struct InfoPanelText;

fn setup_ui(mut commands: Commands, assets: Res<GameAssets>) {
    // Top bar - HUD
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    padding: UiRect::horizontal(Val::Px(20.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    ..default()
                },
                background_color: GameColors::UI_OVERLAY.into(),
                ..default()
            },
            GameEntity,
        ))
        .with_children(|parent| {
            // Left side - Gold
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Gold icon (yellow square)
                    parent.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            ..default()
                        },
                        background_color: GameColors::GOLD.into(),
                        ..default()
                    });
                    // Gold text
                    parent.spawn((
                        TextBundle::from_section(
                            "200",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 28.0,
                                color: GameColors::GOLD,
                            },
                        ),
                        GoldText,
                    ));
                });

            // Center - Wave info
            parent.spawn((
                TextBundle::from_section(
                    "Wave 1 / 6",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 24.0,
                        color: Color::WHITE,
                    },
                ),
                WaveText,
            ));

            // Right side - Lives
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Heart icon (red square)
                    parent.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            ..default()
                        },
                        background_color: GameColors::PRIMARY.into(),
                        ..default()
                    });
                    // Lives text
                    parent.spawn((
                        TextBundle::from_section(
                            "20",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 28.0,
                                color: GameColors::PRIMARY,
                            },
                        ),
                        LivesText,
                    ));
                });
        });

    // Bottom bar - Tower selection
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(140.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(15.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Px(0.0),
                    ..default()
                },
                background_color: GameColors::UI_OVERLAY.into(),
                ..default()
            },
            GameEntity,
        ))
        .with_children(|parent| {
            // Tower buttons with detailed info
            for tower_type in [TowerType::Basic, TowerType::Splash, TowerType::Slow, TowerType::Sniper, TowerType::Rapid] {
                parent
                    .spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(110.0),
                                height: Val::Px(115.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            border_color: BorderColor(Color::NONE),
                            ..default()
                        },
                        TowerButtonBorder(tower_type),
                    ))
                    .with_children(|parent| {
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Column,
                                        justify_content: JustifyContent::SpaceEvenly,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::all(Val::Px(8.0)),
                                        ..default()
                                    },
                                    background_color: GameColors::BUTTON_NORMAL.into(),
                                    ..default()
                                },
                                TowerButton(tower_type),
                            ))
                            .with_children(|parent| {
                                // Top row: Icon + Name
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(8.0),
                                            ..default()
                                        },
                                        ..default()
                                    })
                                    .with_children(|parent| {
                                        // Tower icon
                                        parent.spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Px(24.0),
                                                height: Val::Px(24.0),
                                                ..default()
                                            },
                                            background_color: tower_type.color().into(),
                                            ..default()
                                        });
                                        // Tower name
                                        parent.spawn(TextBundle::from_section(
                                            tower_type.name(),
                                            TextStyle {
                                                font: assets.font.clone(),
                                                font_size: 16.0,
                                                color: Color::WHITE,
                                            },
                                        ));
                                    });

                                // Cost row
                                parent.spawn(TextBundle::from_section(
                                    format!("{} gold", tower_type.cost()),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 18.0,
                                        color: GameColors::GOLD,
                                    },
                                ));

                                // Stats row
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            row_gap: Val::Px(2.0),
                                            ..default()
                                        },
                                        ..default()
                                    })
                                    .with_children(|parent| {
                                        // Damage
                                        parent.spawn(TextBundle::from_section(
                                            format!("DMG: {:.0}", tower_type.damage()),
                                            TextStyle {
                                                font: assets.font.clone(),
                                                font_size: 11.0,
                                                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                                            },
                                        ));
                                        // Range & Speed
                                        parent.spawn(TextBundle::from_section(
                                            format!("RNG: {:.0} | SPD: {:.1}", tower_type.range(), tower_type.attack_speed()),
                                            TextStyle {
                                                font: assets.font.clone(),
                                                font_size: 10.0,
                                                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                                            },
                                        ));
                                    });
                            });
                    });
            }

            // Spacer
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(20.0),
                    ..default()
                },
                ..default()
            });

            // Info panel for selected tower
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(180.0),
                            height: Val::Px(120.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                        background_color: Color::srgba(0.1, 0.1, 0.15, 0.9).into(),
                        ..default()
                    },
                    InfoPanel,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(
                                "Selected: Basic\n",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 14.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "Balanced single-target\ndamage tower",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 12.0,
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                                },
                            ),
                        ]),
                        InfoPanelText,
                    ));
                });

            // Spacer
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(20.0),
                    ..default()
                },
                ..default()
            });

            // Start Wave button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(100.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_START.into(),
                        ..default()
                    },
                    StartWaveButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "START\nWAVE",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 16.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn update_gold_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<GoldText>>,
) {
    if economy.is_changed() {
        for mut text in &mut query {
            text.sections[0].value = format!("{}", economy.gold);
        }
    }
}

fn update_lives_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<LivesText>>,
) {
    if economy.is_changed() {
        for mut text in &mut query {
            text.sections[0].value = format!("{}", economy.lives);
        }
    }
}

fn update_wave_display(
    wave_manager: Res<WaveManager>,
    mut query: Query<&mut Text, With<WaveText>>,
) {
    for mut text in &mut query {
        let current = wave_manager.current_wave + 1;
        let total = wave_manager.total_waves();
        let status = if wave_manager.wave_active { " (Active)" } else { "" };
        text.sections[0].value = format!("Wave {} / {}{}", current.min(total), total, status);
    }
}

fn update_info_panel(
    selected: Res<SelectedTowerType>,
    mut query: Query<&mut Text, With<InfoPanelText>>,
) {
    if selected.is_changed() {
        for mut text in &mut query {
            let tower_type = selected.0;
            text.sections[0].value = format!("Selected: {}\n", tower_type.name());
            text.sections[1].value = tower_type.description().to_string();
        }
    }
}

fn tower_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &TowerButton),
        Changed<Interaction>,
    >,
    mut selected: ResMut<SelectedTowerType>,
) {
    for (interaction, mut color, tower_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected.0 = tower_button.0;
                *color = GameColors::BUTTON_SELECTED.into();
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_HOVER.into();
            }
            Interaction::None => {
                if selected.0 == tower_button.0 {
                    *color = GameColors::BUTTON_SELECTED.into();
                } else {
                    *color = GameColors::BUTTON_NORMAL.into();
                }
            }
        }
    }
}

fn update_tower_selection(
    selected: Res<SelectedTowerType>,
    mut borders: Query<(&TowerButtonBorder, &mut BorderColor)>,
) {
    if selected.is_changed() {
        for (border, mut color) in &mut borders {
            if border.0 == selected.0 {
                *color = BorderColor(Color::WHITE);
            } else {
                *color = BorderColor(Color::NONE);
            }
        }
    }
}

fn start_wave_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StartWaveButton>),
    >,
    mut wave_manager: ResMut<WaveManager>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = GameColors::BUTTON_START_PRESSED.into();
                if !wave_manager.wave_active {
                    wave_manager.start_wave();
                }
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_START_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_START.into();
            }
        }
    }
}

fn handle_tile_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    map: Res<GameMap>,
    selected: Res<SelectedTowerType>,
    economy: Res<PlayerEconomy>,
    mut place_events: EventWriter<PlaceTowerEvent>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Convert to world coordinates
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Convert to grid coordinates
    let Some((grid_x, grid_y)) = GameMap::world_to_grid(world_pos) else {
        return;
    };

    // Check if buildable and affordable
    if !map.is_buildable(grid_x, grid_y) {
        return;
    }

    if economy.gold < selected.0.cost() {
        return;
    }

    // Check if click is in UI area (bottom 140px or top 50px)
    let window_height = window.height();
    if cursor_pos.y > window_height - 140.0 || cursor_pos.y < 50.0 {
        return;
    }

    place_events.send(PlaceTowerEvent {
        grid_x,
        grid_y,
        tower_type: selected.0,
    });
}

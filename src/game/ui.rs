use bevy::prelude::*;

use crate::{loading::GameAssets, GameState, GameSpeed};
use crate::graphics::shapes::GameColors;

use super::{
    economy::PlayerEconomy,
    enemy::WaveManager,
    map::{GameMap, HoveredTile, TileType},
    tower::{PlaceTowerEvent, SelectedTowerType, SelectedPlacedTower, SellTowerEvent, UpgradeTowerEvent, Tower, TowerType},
    GameEntity,
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    update_gold_display,
                    update_lives_display,
                    update_wave_display,
                    update_score_display,
                    tower_button_system,
                    start_wave_button,
                    handle_tile_click,
                    update_tower_selection,
                    update_info_panel,
                    speed_button_system,
                    pause_input,
                    update_tower_context_menu,
                    tower_context_buttons,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (pause_menu_buttons, pause_input_resume)
                    .run_if(in_state(GameState::Paused)),
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

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct SpeedButton(f32);

#[derive(Component)]
struct PauseMenu;

#[derive(Component)]
struct ResumeButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct TowerContextMenu;

#[derive(Component)]
struct SellButton;

#[derive(Component)]
struct UpgradeButton;

#[derive(Component)]
struct UpgradeCostText;

#[derive(Component)]
struct SellValueText;

#[derive(Component)]
struct EndlessModeButton;

#[derive(Component)]
struct CloseMenuButton;

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

            // Center section - Wave info and Score
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
                    parent.spawn((
                        TextBundle::from_section(
                            "Wave 1 / 10",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 22.0,
                                color: Color::WHITE,
                            },
                        ),
                        WaveText,
                    ));
                    parent.spawn((
                        TextBundle::from_section(
                            "Score: 0",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                            },
                        ),
                        ScoreText,
                    ));
                });

            // Speed controls
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for speed in [1.0, 2.0, 3.0] {
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Px(32.0),
                                        height: Val::Px(28.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    background_color: if speed == 1.0 {
                                        GameColors::BUTTON_SELECTED.into()
                                    } else {
                                        GameColors::BUTTON_NORMAL.into()
                                    },
                                    ..default()
                                },
                                SpeedButton(speed),
                            ))
                            .with_children(|parent| {
                                parent.spawn(TextBundle::from_section(
                                    format!("{}x", speed as u32),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 14.0,
                                        color: Color::WHITE,
                                    },
                                ));
                            });
                    }
                });

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

    // Tower context menu (initially hidden) - for upgrade/sell
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(130.0),
                    height: Val::Px(120.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                background_color: Color::srgba(0.1, 0.1, 0.15, 0.95).into(),
                ..default()
            },
            TowerContextMenu,
            GameEntity,
        ))
        .with_children(|parent| {
            // Close button (X)
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            position_type: PositionType::Absolute,
                            right: Val::Px(4.0),
                            top: Val::Px(4.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgba(0.5, 0.2, 0.2, 0.8).into(),
                        ..default()
                    },
                    CloseMenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "X",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 12.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Upgrade button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(20.0)),
                            ..default()
                        },
                        background_color: GameColors::BUTTON_NORMAL.into(),
                        ..default()
                    },
                    UpgradeButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            "Upgrade [U]",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 12.0,
                                color: Color::WHITE,
                            },
                        ),
                        UpgradeCostText,
                    ));
                });

            // Sell button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.6, 0.2, 0.2).into(),
                        ..default()
                    },
                    SellButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            "Sell [S]",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 12.0,
                                color: Color::WHITE,
                            },
                        ),
                        SellValueText,
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

fn update_score_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<ScoreText>>,
) {
    if economy.is_changed() {
        for mut text in &mut query {
            text.sections[0].value = format!("Score: {}", economy.score);
        }
    }
}

fn speed_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &SpeedButton),
        Changed<Interaction>,
    >,
    mut game_speed: ResMut<GameSpeed>,
    mut all_speed_buttons: Query<(&SpeedButton, &mut BackgroundColor), Without<Interaction>>,
    mut time: ResMut<Time<Virtual>>,
) {
    for (interaction, mut color, speed_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                game_speed.0 = speed_button.0;
                time.set_relative_speed(speed_button.0);
                *color = GameColors::BUTTON_SELECTED.into();

                // Update all button colors
                for (btn, mut btn_color) in &mut all_speed_buttons {
                    if btn.0 == speed_button.0 {
                        *btn_color = GameColors::BUTTON_SELECTED.into();
                    } else {
                        *btn_color = GameColors::BUTTON_NORMAL.into();
                    }
                }
            }
            Interaction::Hovered => {
                if game_speed.0 != speed_button.0 {
                    *color = GameColors::BUTTON_HOVER.into();
                }
            }
            Interaction::None => {
                if game_speed.0 == speed_button.0 {
                    *color = GameColors::BUTTON_SELECTED.into();
                } else {
                    *color = GameColors::BUTTON_NORMAL.into();
                }
            }
        }
    }
}

fn pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    selected_tower: Res<SelectedPlacedTower>,
) {
    // Only pause if no tower is selected (Escape deselects tower first)
    if keyboard.just_pressed(KeyCode::Escape) && selected_tower.0.is_none() {
        next_state.set(GameState::Paused);
    }
}

fn pause_input_resume(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Playing);
    }
}

fn setup_pause_menu(mut commands: Commands, assets: Res<GameAssets>) {
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
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            },
            PauseMenu,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "PAUSED",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 64.0,
                    color: Color::WHITE,
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            }));

            // Resume button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        background_color: GameColors::BUTTON_START.into(),
                        ..default()
                    },
                    ResumeButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "RESUME",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Quit button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(200.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.5, 0.2, 0.2).into(),
                        ..default()
                    },
                    QuitButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "QUIT TO MENU",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Hint text
            parent.spawn(TextBundle::from_section(
                "Press ESC to resume",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 16.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(30.0)),
                ..default()
            }));
        });
}

fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn pause_menu_buttons(
    mut resume_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ResumeButton>),
    >,
    mut quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitButton>, Without<ResumeButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color) in &mut resume_query {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_START_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_START.into();
            }
        }
    }

    for (interaction, mut color) in &mut quit_query {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameState::Menu);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.6, 0.3, 0.3).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.5, 0.2, 0.2).into();
            }
        }
    }
}

fn update_tower_context_menu(
    hovered_tile: Res<HoveredTile>,
    map: Res<GameMap>,
    towers: Query<(Entity, &Tower)>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut context_menu: Query<(&mut Style, &Children), With<TowerContextMenu>>,
    mut upgrade_text: Query<&mut Text, (With<UpgradeCostText>, Without<SellValueText>)>,
    mut sell_text: Query<&mut Text, With<SellValueText>>,
    windows: Query<&Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    economy: Res<PlayerEconomy>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
) {
    // Escape to deselect
    if keyboard.just_pressed(KeyCode::Escape) && selected_tower.0.is_some() {
        selected_tower.0 = None;
        return;
    }

    // Keyboard shortcuts when tower is selected
    if let Some(tower_entity) = selected_tower.0 {
        if let Ok((_, tower)) = towers.get(tower_entity) {
            // U to upgrade
            if keyboard.just_pressed(KeyCode::KeyU) {
                if tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
                    upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                }
            }
            // S to sell
            if keyboard.just_pressed(KeyCode::KeyS) {
                sell_events.send(SellTowerEvent { tower: tower_entity });
                selected_tower.0 = None;
                return;
            }
        }
    }

    // Left-click to select/deselect towers
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some((hx, hy)) = hovered_tile.position {
            if map.tiles[hx][hy] == TileType::Tower {
                // Find the tower at this position
                for (entity, tower) in &towers {
                    if tower.grid_x == hx && tower.grid_y == hy {
                        // Toggle selection - if already selected, deselect
                        if selected_tower.0 == Some(entity) {
                            selected_tower.0 = None;
                        } else {
                            selected_tower.0 = Some(entity);
                        }
                        break;
                    }
                }
            } else {
                // Clicked on non-tower tile - deselect
                selected_tower.0 = None;
            }
        }
    }

    // Update context menu visibility and position
    for (mut style, _) in &mut context_menu {
        if let Some(tower_entity) = selected_tower.0 {
            if let Ok((_, tower)) = towers.get(tower_entity) {
                style.display = Display::Flex;

                // Position near the tower
                if let Ok(window) = windows.get_single() {
                    let world_pos = GameMap::grid_to_world(tower.grid_x, tower.grid_y);
                    // Convert to screen space (approximate)
                    let screen_x = world_pos.x + window.width() / 2.0 + 40.0;
                    let screen_y = window.height() / 2.0 - world_pos.y - 45.0;
                    style.left = Val::Px(screen_x.clamp(0.0, window.width() - 130.0));
                    style.top = Val::Px(screen_y.clamp(0.0, window.height() - 100.0));
                }

                // Update text
                for mut text in &mut upgrade_text {
                    if tower.can_upgrade() {
                        text.sections[0].value = format!("Upgrade [U] ({}g)", tower.upgrade_cost());
                    } else {
                        text.sections[0].value = "MAX LEVEL".to_string();
                    }
                }
                for mut text in &mut sell_text {
                    text.sections[0].value = format!("Sell [S] ({}g)", tower.sell_value());
                }
            } else {
                style.display = Display::None;
                selected_tower.0 = None;
            }
        } else {
            style.display = Display::None;
        }
    }
}

fn tower_context_buttons(
    mut upgrade_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<UpgradeButton>, Without<SellButton>, Without<CloseMenuButton>),
    >,
    mut sell_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SellButton>, Without<UpgradeButton>, Without<CloseMenuButton>),
    >,
    mut close_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<CloseMenuButton>, Without<UpgradeButton>, Without<SellButton>),
    >,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    towers: Query<&Tower>,
    economy: Res<PlayerEconomy>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
) {
    // Close button
    for (interaction, mut color) in &mut close_query {
        match *interaction {
            Interaction::Pressed => {
                selected_tower.0 = None;
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.7, 0.3, 0.3).into();
            }
            Interaction::None => {
                *color = Color::srgba(0.5, 0.2, 0.2, 0.8).into();
            }
        }
    }

    // Upgrade button
    for (interaction, mut color) in &mut upgrade_query {
        match *interaction {
            Interaction::Pressed => {
                if let Some(tower_entity) = selected_tower.0 {
                    if let Ok(tower) = towers.get(tower_entity) {
                        if tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
                            upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                        }
                    }
                }
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_NORMAL.into();
            }
        }
    }

    // Sell button
    for (interaction, mut color) in &mut sell_query {
        match *interaction {
            Interaction::Pressed => {
                if let Some(tower_entity) = selected_tower.0 {
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                }
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.7, 0.3, 0.3).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.6, 0.2, 0.2).into();
            }
        }
    }
}

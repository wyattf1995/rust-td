use bevy::prelude::*;

use crate::{loading::GameAssets, GameState, GameSpeed};
use crate::graphics::shapes::GameColors;

use super::{
    abilities::PlayerAbilities,
    economy::{PlayerEconomy, KillStreak},
    enemy::{WaveManager, WaveModifier},
    map::{GameMap, HoveredTile, TileType},
    tower::{PlaceTowerEvent, SelectedTowerType, SelectedPlacedTower, SellTowerEvent, UpgradeTowerEvent, Tower, TowerType, TowerSynergies},
    GameEntity,
};

pub struct GameUiPlugin;

/// Resource to track if UI was clicked this frame (prevents click-through to game)
#[derive(Resource, Default)]
pub struct UiClicked(pub bool);

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiClicked>()
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    // Phase 1: Reset UI click flag
                    reset_ui_clicked,
                    // Phase 2: UI button interactions (set flag if clicked)
                    (
                        tower_button_system,
                        start_wave_button,
                        tower_context_shortcuts,
                        update_tower_context_menu,
                        tower_context_buttons,
                        speed_button_system,
                    ),
                    // Phase 3: Game world clicks (check flag before placing)
                    handle_tile_click,
                    // Phase 4: Display updates (order doesn't matter)
                    (
                        update_gold_display,
                        update_lives_display,
                        update_wave_display,
                        update_score_display,
                        update_combo_display,
                        update_ability_display,
                        update_ability_tooltips,
                        update_tower_selection,
                        update_info_panel,
                        pause_input,
                    ),
                )
                    .chain()
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
struct InfoPanelName;

#[derive(Component)]
struct InfoPanelDesc;

#[derive(Component)]
struct InfoPanelStats;

#[derive(Component)]
struct InfoPanelCost;

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

#[derive(Component)]
struct TowerStatsText;

#[derive(Component)]
struct TowerUpgradePreview;

#[derive(Component)]
struct ComboDisplay;

#[derive(Component)]
struct ComboText;

#[derive(Component)]
struct WaveModifierText;

#[derive(Component)]
struct TargetingButton;

#[derive(Component)]
struct TargetingText;

#[derive(Component)]
struct SynergyText;

#[derive(Component)]
struct AbilityBar;

#[derive(Component)]
struct AbilityButton(AbilityType);

#[derive(Component)]
struct AbilityCooldownText(AbilityType);

#[derive(Component)]
struct AbilityTooltip;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AbilityType {
    Freeze,
    GoldRush,
    Artillery,
}

impl AbilityType {
    fn tooltip(&self) -> (&'static str, &'static str) {
        match self {
            AbilityType::Freeze => ("Freeze [Q]", "Freeze all enemies for 3s\nCooldown: 45s"),
            AbilityType::GoldRush => ("Gold Rush [W]", "2x gold from kills for 10s\nCooldown: 60s"),
            AbilityType::Artillery => ("Artillery [E]", "Click to bomb an area for 400 dmg\nCooldown: 90s"),
        }
    }
}

fn setup_ui(mut commands: Commands, assets: Res<GameAssets>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
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

    // Bottom bar - Tower selection (touch-friendly)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(122.0),
                    padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(6.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
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
            // Tower buttons - compact for mobile (8 towers now)
            for tower_type in [TowerType::Basic, TowerType::Splash, TowerType::Slow, TowerType::Sniper, TowerType::Rapid, TowerType::Chain, TowerType::Poison, TowerType::Buff] {
                parent
                    .spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(70.0),
                                height: Val::Px(108.0),
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
                                        padding: UiRect::all(Val::Px(4.0)),
                                        ..default()
                                    },
                                    background_color: GameColors::BUTTON_NORMAL.into(),
                                    ..default()
                                },
                                TowerButton(tower_type),
                            ))
                            .with_children(|parent| {
                                // Tower icon
                                parent.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Px(28.0),
                                        height: Val::Px(28.0),
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
                                        font_size: 12.0,
                                        color: Color::WHITE,
                                    },
                                ));

                                // Cost
                                parent.spawn(TextBundle::from_section(
                                    format!("{}g", tower_type.cost()),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 13.0,
                                        color: GameColors::GOLD,
                                    },
                                ));

                                // 3-stat rows (or 2 for Buff tower)
                                if tower_type == TowerType::Buff {
                                    parent.spawn(TextBundle::from_section(
                                        format!("BUFF +25%\nRNG {:.0}", tower_type.range()),
                                        TextStyle {
                                            font: assets.font.clone(),
                                            font_size: 9.0,
                                            color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                                        },
                                    ));
                                } else {
                                    parent.spawn(TextBundle::from_section(
                                        format!(
                                            "DMG {:.0}\nRNG {:.0}\nSPD {:.1}",
                                            tower_type.damage(),
                                            tower_type.range(),
                                            tower_type.attack_speed()
                                        ),
                                        TextStyle {
                                            font: assets.font.clone(),
                                            font_size: 9.0,
                                            color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                                        },
                                    ));
                                }
                            });
                    });
            }

            // Info panel for selected tower
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(175.0),
                            height: Val::Px(108.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceEvenly,
                            align_items: AlignItems::FlexStart,
                            padding: UiRect::all(Val::Px(8.0)),
                            margin: UiRect::horizontal(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::srgba(0.1, 0.1, 0.15, 0.9).into(),
                        ..default()
                    },
                    InfoPanel,
                ))
                .with_children(|parent| {
                    // Tower name in accent color
                    parent.spawn((
                        TextBundle::from_section(
                            "Basic",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 13.0,
                                color: TowerType::Basic.color(),
                            },
                        ),
                        InfoPanelName,
                    ));
                    // Description
                    parent.spawn((
                        TextBundle::from_section(
                            "Balanced single-target damage",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 9.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                            },
                        ),
                        InfoPanelDesc,
                    ));
                    // Stats
                    parent.spawn((
                        TextBundle::from_section(
                            "DMG: 25  RNG: 150\nSPD: 1.0/s",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 10.0,
                                color: Color::srgb(0.7, 0.8, 0.95),
                            },
                        ),
                        InfoPanelStats,
                    ));
                    // Cost
                    parent.spawn((
                        TextBundle::from_section(
                            "Cost: 50g",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 11.0,
                                color: GameColors::GOLD,
                            },
                        ),
                        InfoPanelCost,
                    ));
                });

            // Start Wave button (touch-friendly)
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(80.0),
                            height: Val::Px(55.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_START.into(),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    StartWaveButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "START",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 14.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });

    // Tower context menu (initially hidden) - for upgrade/sell with stats
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(Val::Px(12.0)),
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: Color::srgba(0.08, 0.08, 0.12, 0.98).into(),
                border_color: BorderColor(Color::srgba(0.3, 0.3, 0.4, 0.8)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            TowerContextMenu,
            GameEntity,
        ))
        .with_children(|parent| {
            // Header with tower name and close button
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Tower name/level will be part of stats text
                    parent.spawn(NodeBundle::default()); // Placeholder

                    // Close button (X)
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(22.0),
                                    height: Val::Px(22.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: Color::srgba(0.4, 0.15, 0.15, 0.9).into(),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            CloseMenuButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                "X",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 14.0,
                                    color: Color::WHITE,
                                },
                            ));
                        });
                });

            // Tower stats section with background
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    background_color: Color::srgba(0.15, 0.15, 0.2, 0.8).into(),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(
                                "Basic Tower Lv1\n",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 13.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "DMG: 25  RNG: 150  SPD: 1.0/s",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 11.0,
                                    color: Color::srgba(0.7, 0.8, 0.9, 1.0),
                                },
                            ),
                        ]),
                        TowerStatsText,
                    ));
                });

            // Upgrade preview section
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    background_color: Color::srgba(0.1, 0.2, 0.15, 0.8).into(),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(
                                "Next Level\n",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 11.0,
                                    color: GameColors::SUCCESS,
                                },
                            ),
                            TextSection::new(
                                "DMG: 30 (+5)\nRNG: 165 (+15)\nSPD: 1.2/s (+0.2)",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 10.0,
                                    color: Color::srgba(0.6, 0.95, 0.6, 1.0),
                                },
                            ),
                        ]),
                        TowerUpgradePreview,
                    ));
                });

            // Synergy display (hidden when no synergies active)
            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 11.0,
                        color: Color::srgb(0.9, 0.7, 1.0), // Light purple for synergies
                    },
                ).with_style(Style {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                }),
                SynergyText,
            ));

            // Targeting mode button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(6.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.3, 0.3, 0.5).into(),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    TargetingButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            "Target: First [T]",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 12.0,
                                color: Color::WHITE,
                            },
                        ),
                        TargetingText,
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
                            margin: UiRect::bottom(Val::Px(6.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.5, 0.3).into(),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    UpgradeButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            "Upgrade [U] - 30g",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 13.0,
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
                        background_color: Color::srgb(0.5, 0.2, 0.2).into(),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    SellButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle::from_section(
                            "Sell [S] +37g",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 13.0,
                                color: Color::WHITE,
                            },
                        ),
                        SellValueText,
                    ));
                });
        });

    // Combo display (centered, initially hidden)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(80.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-100.0)), // Center the 200px wide box
                    width: Val::Px(200.0),
                    height: Val::Px(60.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    display: Display::None, // Hidden by default
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.7).into(),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            ComboDisplay,
            GameEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "COMBO x1",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 32.0,
                        color: GameColors::GOLD,
                    },
                ),
                ComboText,
            ));
        });

    // Ability bar (left side, vertical)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(60.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.6).into(),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            AbilityBar,
            GameEntity,
        ))
        .with_children(|parent| {
            // Freeze ability [Q]
            spawn_ability_button(parent, &assets, AbilityType::Freeze, "Q", "Freeze", GameColors::ABILITY_FREEZE);
            // Gold Rush ability [W]
            spawn_ability_button(parent, &assets, AbilityType::GoldRush, "W", "Gold", GameColors::ABILITY_GOLD_RUSH);
            // Artillery ability [E]
            spawn_ability_button(parent, &assets, AbilityType::Artillery, "E", "Arty", GameColors::ABILITY_NUKE);
        });
}

fn spawn_ability_button(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    ability: AbilityType,
    key: &str,
    name: &str,
    color: Color,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(66.0),
                    height: Val::Px(58.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(4.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                background_color: color.with_alpha(0.3).into(),
                border_color: BorderColor(Color::NONE),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            AbilityButton(ability),
        ))
        .with_children(|btn| {
            // Top row: key badge left-aligned in dark chip
            btn.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                row.spawn(NodeBundle {
                    style: Style {
                        padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(1.0), Val::Px(1.0)),
                        ..default()
                    },
                    background_color: Color::srgba(0.0, 0.0, 0.0, 0.5).into(),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                }).with_children(|chip| {
                    chip.spawn(TextBundle::from_section(
                        key,
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 11.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                        },
                    ));
                });
            });
            // Center: ability name
            btn.spawn(TextBundle::from_section(
                name,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color,
                },
            ));
            // Bottom: status text
            btn.spawn((
                TextBundle::from_section(
                    "Ready",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 11.0,
                        color: GameColors::SUCCESS,
                    },
                ),
                AbilityCooldownText(ability),
            ));
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
        let status = if wave_manager.wave_active { " ⚔" } else { "" };
        // Show health multiplier as difficulty indicator
        let difficulty = format!(" ({:.1}x)", wave_manager.health_multiplier);

        // Show wave modifier if active
        let modifier: &str = match wave_manager.current_modifier {
            WaveModifier::None => "",
            WaveModifier::SpeedBoost => " [SPEED]",
            WaveModifier::ArmoredWave => " [ARMOR]",
            WaveModifier::Swarm => " [SWARM]",
            WaveModifier::Regen => " [REGEN]",
            WaveModifier::GoldRush => " [GOLD!]",
        };

        text.sections[0].value = format!("Wave {}{}{}{}", current, difficulty, modifier, status);
    }
}

fn update_combo_display(
    kill_streak: Res<KillStreak>,
    mut display_query: Query<&mut Style, With<ComboDisplay>>,
    mut text_query: Query<&mut Text, With<ComboText>>,
) {
    for mut style in &mut display_query {
        // Show combo display when streak is 3+
        style.display = if kill_streak.count >= 3 && kill_streak.active {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut text in &mut text_query {
        if kill_streak.count >= 3 {
            text.sections[0].value = format!("COMBO x{}", kill_streak.count);

            // Color changes based on combo level
            text.sections[0].style.color = if kill_streak.count >= 10 {
                Color::srgb(1.0, 0.3, 0.3) // Red for 10+
            } else if kill_streak.count >= 6 {
                Color::srgb(1.0, 0.6, 0.0) // Orange for 6+
            } else {
                GameColors::GOLD // Yellow for 3+
            };
        }
    }
}

fn update_ability_display(
    abilities: Res<PlayerAbilities>,
    mut button_query: Query<(&AbilityButton, &mut BackgroundColor, &mut BorderColor)>,
    mut cooldown_text_query: Query<(&AbilityCooldownText, &mut Text)>,
) {
    for (button, mut bg_color, mut border_color) in &mut button_query {
        let (ready, active, remaining) = match button.0 {
            AbilityType::Freeze => (
                abilities.freeze_ready,
                false,
                abilities.freeze_cooldown.remaining_secs(),
            ),
            AbilityType::GoldRush => (
                abilities.gold_rush_ready,
                abilities.gold_rush_active.is_some(),
                if abilities.gold_rush_active.is_some() {
                    abilities.gold_rush_active.as_ref().unwrap().remaining_secs()
                } else {
                    abilities.gold_rush_cooldown.remaining_secs()
                },
            ),
            AbilityType::Artillery => (
                abilities.artillery_ready,
                abilities.artillery_targeting,
                abilities.artillery_cooldown.remaining_secs(),
            ),
        };

        // Update button opacity based on ready state
        let base_color = match button.0 {
            AbilityType::Freeze => GameColors::ABILITY_FREEZE,
            AbilityType::GoldRush => GameColors::ABILITY_GOLD_RUSH,
            AbilityType::Artillery => GameColors::ABILITY_NUKE,
        };

        if ready {
            *bg_color = base_color.with_alpha(0.5).into();
            *border_color = BorderColor(base_color.with_alpha(0.6));
        } else if active {
            *bg_color = base_color.with_alpha(0.8).into();
            *border_color = BorderColor(base_color.with_alpha(0.8));
        } else {
            *bg_color = base_color.with_alpha(0.2).into();
            *border_color = BorderColor(Color::NONE);
        }
    }

    for (cooldown_text, mut text) in &mut cooldown_text_query {
        let (ready, active, remaining) = match cooldown_text.0 {
            AbilityType::Freeze => (
                abilities.freeze_ready,
                false,
                abilities.freeze_cooldown.remaining_secs(),
            ),
            AbilityType::GoldRush => (
                abilities.gold_rush_ready,
                abilities.gold_rush_active.is_some(),
                if abilities.gold_rush_active.is_some() {
                    abilities.gold_rush_active.as_ref().unwrap().remaining_secs()
                } else {
                    abilities.gold_rush_cooldown.remaining_secs()
                },
            ),
            AbilityType::Artillery => (
                abilities.artillery_ready,
                abilities.artillery_targeting,
                abilities.artillery_cooldown.remaining_secs(),
            ),
        };

        if ready {
            text.sections[0].value = "Ready".to_string();
            text.sections[0].style.color = GameColors::SUCCESS;
        } else if active {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = GameColors::GOLD;
        } else {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = Color::srgba(1.0, 1.0, 1.0, 0.5);
        }
    }
}

fn update_ability_tooltips(
    mut commands: Commands,
    assets: Res<GameAssets>,
    button_query: Query<(&Interaction, &AbilityButton)>,
    tooltip_query: Query<Entity, With<AbilityTooltip>>,
) {
    // Find which ability (if any) is hovered
    let mut hovered: Option<AbilityType> = None;
    for (interaction, button) in &button_query {
        if *interaction == Interaction::Hovered {
            hovered = Some(button.0);
        }
    }

    // Despawn existing tooltip
    for entity in &tooltip_query {
        commands.entity(entity).despawn_recursive();
    }

    // Spawn new tooltip if hovering
    if let Some(ability) = hovered {
        let (title, desc) = ability.tooltip();

        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(60.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    max_width: Val::Px(200.0),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.85).into(),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            AbilityTooltip,
            GameEntity,
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                title,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: Color::WHITE,
                },
            ));
            parent.spawn(TextBundle::from_section(
                desc,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                },
            ));
        });
    }
}

fn update_info_panel(
    selected: Res<SelectedTowerType>,
    mut name_query: Query<&mut Text, (With<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelStats>, Without<InfoPanelCost>)>,
    mut desc_query: Query<&mut Text, (With<InfoPanelDesc>, Without<InfoPanelName>, Without<InfoPanelStats>, Without<InfoPanelCost>)>,
    mut stats_query: Query<&mut Text, (With<InfoPanelStats>, Without<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelCost>)>,
    mut cost_query: Query<&mut Text, (With<InfoPanelCost>, Without<InfoPanelName>, Without<InfoPanelDesc>, Without<InfoPanelStats>)>,
) {
    if selected.is_changed() {
        let tower_type = selected.0;

        for mut text in &mut name_query {
            text.sections[0].value = tower_type.name().to_string();
            text.sections[0].style.color = tower_type.color();
        }

        for mut text in &mut desc_query {
            text.sections[0].value = tower_type.description().to_string();
        }

        for mut text in &mut stats_query {
            if tower_type == TowerType::Buff {
                text.sections[0].value = format!(
                    "BUFF: +25%  RNG: {:.0}",
                    tower_type.range()
                );
            } else {
                text.sections[0].value = format!(
                    "DMG: {:.0}  RNG: {:.0}\nSPD: {:.1}/s",
                    tower_type.damage(),
                    tower_type.range(),
                    tower_type.attack_speed()
                );
            }
        }

        for mut text in &mut cost_query {
            text.sections[0].value = format!("Cost: {}g", tower_type.cost());
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
                *color = BorderColor(border.0.color());
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

/// Reset UI clicked flag at the start of each frame
fn reset_ui_clicked(mut ui_clicked: ResMut<UiClicked>) {
    ui_clicked.0 = false;
}

fn handle_tile_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    map: Res<GameMap>,
    selected: Res<SelectedTowerType>,
    economy: Res<PlayerEconomy>,
    mut place_events: EventWriter<PlaceTowerEvent>,
    ui_clicked: Res<UiClicked>,
) {
    // Don't place tower if UI was clicked
    if ui_clicked.0 {
        return;
    }

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

/// Handle keyboard shortcuts for tower context menu (separate system to stay within param limit)
fn tower_context_shortcuts(
    towers: Query<&Tower>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
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
        if let Ok(tower) = towers.get(tower_entity) {
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
            }
        }
    }
}

fn update_tower_context_menu(
    hovered_tile: Res<HoveredTile>,
    map: Res<GameMap>,
    towers: Query<(Entity, &Tower, Option<&TowerSynergies>)>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut context_menu: Query<(&mut Style, &Children), With<TowerContextMenu>>,
    mut upgrade_text: Query<&mut Text, (With<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut sell_text: Query<&mut Text, (With<SellValueText>, Without<UpgradeCostText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut stats_text: Query<&mut Text, (With<TowerStatsText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut preview_text: Query<&mut Text, (With<TowerUpgradePreview>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TargetingText>, Without<SynergyText>)>,
    mut targeting_text: Query<&mut Text, (With<TargetingText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<SynergyText>)>,
    mut synergy_text: Query<&mut Text, (With<SynergyText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>)>,
    windows: Query<&Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    economy: Res<PlayerEconomy>,
) {
    // Left-click to select/deselect towers
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some((hx, hy)) = hovered_tile.position {
            if map.tiles[hx][hy] == TileType::Tower {
                // Find the tower at this position
                for (entity, tower, _) in &towers {
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
            if let Ok((_, tower, synergies)) = towers.get(tower_entity) {
                style.display = Display::Flex;

                // Position near the tower
                if let Ok(window) = windows.get_single() {
                    let world_pos = GameMap::grid_to_world(tower.grid_x, tower.grid_y);
                    // Convert to screen space (approximate)
                    let screen_x = world_pos.x + window.width() / 2.0 + 40.0;
                    let screen_y = window.height() / 2.0 - world_pos.y - 60.0;
                    style.left = Val::Px(screen_x.clamp(0.0, window.width() - 180.0));
                    style.top = Val::Px(screen_y.clamp(0.0, window.height() - 220.0));
                }

                // Get current attack speed
                let attack_speed = tower.attack_speed();
                let is_buff_tower = tower.tower_type == TowerType::Buff;

                // Update stats text
                for mut text in &mut stats_text {
                    text.sections[0].value = format!("{} Lv{}\n", tower.tower_type.name(), tower.level);
                    if is_buff_tower {
                        let buff_pct = tower.buff_percentage() * 100.0;
                        text.sections[1].value = format!(
                            "BUFF: +{:.0}%  RNG: {:.0}",
                            buff_pct,
                            tower.range
                        );
                    } else {
                        text.sections[1].value = format!(
                            "DMG: {:.0}  RNG: {:.0}  SPD: {:.1}/s",
                            tower.damage,
                            tower.range,
                            attack_speed
                        );
                    }
                }

                // Update upgrade preview text using the tower's preview method
                for mut text in &mut preview_text {
                    let (next_damage, next_range, next_speed) = tower.preview_upgrade();
                    text.sections[0].value = format!("Level {} Preview\n", tower.level + 1);
                    if is_buff_tower {
                        let curr_buff = tower.buff_percentage() * 100.0;
                        let next_buff = tower.buff_percentage_next() * 100.0;
                        text.sections[1].value = format!(
                            "BUFF: +{:.0}% (+{:.0}%)\nRNG: {:.0} (+{:.0})",
                            next_buff, next_buff - curr_buff,
                            next_range, next_range - tower.range
                        );
                    } else {
                        text.sections[1].value = format!(
                            "DMG: {:.0} (+{:.0})\nRNG: {:.0} (+{:.0})\nSPD: {:.2}/s (+{:.2})",
                            next_damage, next_damage - tower.damage,
                            next_range, next_range - tower.range,
                            next_speed, next_speed - attack_speed
                        );
                    }
                }

                // Update button text
                let upgrade_cost = tower.upgrade_cost();
                let can_afford = economy.gold >= upgrade_cost;
                for mut text in &mut upgrade_text {
                    text.sections[0].value = if can_afford {
                        format!("Upgrade [U] - {}g", upgrade_cost)
                    } else {
                        format!("Need {}g (have {})", upgrade_cost, economy.gold)
                    };
                }
                for mut text in &mut sell_text {
                    text.sections[0].value = format!("Sell [S] +{}g", tower.sell_value());
                }
                // Update targeting text to show current mode
                for mut text in &mut targeting_text {
                    text.sections[0].value = format!("Target: {} [T]", tower.targeting.name());
                }
                // Update synergy text
                for mut text in &mut synergy_text {
                    if let Some(syn) = synergies {
                        if !syn.active.is_empty() {
                            use std::fmt::Write;
                            let mut result = String::new();
                            for (i, s) in syn.active.iter().enumerate() {
                                if i > 0 { result.push('\n'); }
                                let _ = write!(result, "\u{26A1} {}: {}", s.name(), s.description());
                            }
                            text.sections[0].value = result;
                        } else {
                            text.sections[0].value = String::new();
                        }
                    } else {
                        text.sections[0].value = String::new();
                    }
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
        (With<UpgradeButton>, Without<SellButton>, Without<CloseMenuButton>, Without<TargetingButton>),
    >,
    mut sell_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<SellButton>, Without<UpgradeButton>, Without<CloseMenuButton>, Without<TargetingButton>),
    >,
    mut close_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<CloseMenuButton>, Without<UpgradeButton>, Without<SellButton>, Without<TargetingButton>),
    >,
    mut targeting_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<TargetingButton>, Without<UpgradeButton>, Without<SellButton>, Without<CloseMenuButton>),
    >,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut towers: Query<&mut Tower>,
    economy: Res<PlayerEconomy>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
    mut targeting_text: Query<&mut Text, With<TargetingText>>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    // Close button
    for (interaction, mut color) in &mut close_query {
        match *interaction {
            Interaction::Pressed => {
                selected_tower.0 = None;
                ui_clicked.0 = true;
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.6, 0.25, 0.25).into();
                ui_clicked.0 = true;
            }
            Interaction::None => {
                *color = Color::srgba(0.4, 0.15, 0.15, 0.9).into();
            }
        }
    }

    // Check if we can afford upgrade
    let can_afford = if let Some(tower_entity) = selected_tower.0 {
        if let Ok(tower) = towers.get(tower_entity) {
            economy.gold >= tower.upgrade_cost()
        } else {
            false
        }
    } else {
        false
    };

    // Upgrade button - color based on affordability
    for (interaction, mut color) in &mut upgrade_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                if let Some(tower_entity) = selected_tower.0 {
                    if let Ok(tower) = towers.get(tower_entity) {
                        if economy.gold >= tower.upgrade_cost() {
                            upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                        }
                    }
                }
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                if can_afford {
                    *color = Color::srgb(0.25, 0.65, 0.4).into();
                } else {
                    *color = Color::srgb(0.35, 0.35, 0.4).into();
                }
            }
            Interaction::None => {
                if can_afford {
                    *color = Color::srgb(0.2, 0.5, 0.3).into();
                } else {
                    *color = Color::srgb(0.25, 0.25, 0.3).into();
                }
            }
        }
    }

    // Sell button
    for (interaction, mut color) in &mut sell_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                if let Some(tower_entity) = selected_tower.0 {
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                }
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                *color = Color::srgb(0.65, 0.3, 0.3).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.5, 0.2, 0.2).into();
            }
        }
    }

    // Handle targeting button click or [T] key
    let mut should_cycle_targeting = false;

    for (interaction, mut color) in &mut targeting_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                should_cycle_targeting = true;
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
                *color = Color::srgb(0.4, 0.4, 0.65).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.3, 0.3, 0.5).into();
            }
        }
    }

    // Keyboard shortcut [T]
    if keyboard.just_pressed(KeyCode::KeyT) && selected_tower.0.is_some() {
        should_cycle_targeting = true;
    }

    // Apply targeting cycle
    if should_cycle_targeting {
        if let Some(tower_entity) = selected_tower.0 {
            if let Ok(mut tower) = towers.get_mut(tower_entity) {
                tower.cycle_targeting();

                // Update targeting text
                for mut text in &mut targeting_text {
                    text.sections[0].value = format!("Target: {} [T]", tower.targeting.name());
                }
            }
        }
    }
}

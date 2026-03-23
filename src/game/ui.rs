use bevy::prelude::*;

use crate::{loading::GameAssets, GameState, GameSpeed, ScreenInfo};
use crate::graphics::shapes::GameColors;
use crate::persistence::SettingsOpen;
use crate::synergy_ui::SynergyPanelOpen;

use super::{
    abilities::{PlayerAbilities, FreezeAbilityEvent, GoldRushAbilityEvent},
    economy::{PlayerEconomy, KillStreak},
    enemy::{EnemyType, WaveManager, WaveModifier},
    map::{GameMap, HoveredTile, TileType},
    tower::{PlaceTowerEvent, SelectedTowerType, SelectedPlacedTower, SellTowerEvent, UpgradeTowerEvent, SpecializeTowerEvent, Specialization, Tower, TowerType, TowerSynergies},
    GameEntity,
};

pub struct GameUiPlugin;

/// Resource to track if UI was clicked this frame (prevents click-through to game)
#[derive(Resource, Default)]
pub struct UiClicked(pub bool);

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiClicked>()
            .init_resource::<UiZones>()
            .init_resource::<LifeLostFlash>()
            .init_resource::<SellPending>()
            .init_resource::<PlacementRejectFlash>()
            .init_resource::<GoldRejectFlash>()
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    // Phase 1: Reset UI click flag + update zone sizes
                    (reset_ui_clicked, update_ui_zones),
                    // Phase 2: UI button interactions (set flag if clicked)
                    (
                        tower_button_system,
                        start_wave_button,
                        tower_context_shortcuts,
                        update_tower_context_menu,
                        update_spec_panel,
                        tower_context_buttons,
                        spec_button_system,
                        speed_button_system,
                        ability_button_activate,
                    ),
                    // Phase 3: Game world clicks (check flag before placing)
                    handle_tile_click,
                    // Phase 4: Display updates (order doesn't matter)
                    (
                        update_gold_display,
                        update_gold_reject_flash,
                        update_gold_rush_indicator,
                        update_lives_display,
                        update_life_flash,
                        update_wave_display,
                        update_score_display,
                        update_combo_display,
                        update_ability_display,
                        update_ability_tooltips,
                        adapt_ability_bar_layout,
                        update_tower_selection,
                        update_tower_affordability,
                        update_wave_announcement,
                        update_start_button_text,
                        update_info_panel,
                        update_wave_preview,
                        pause_input,
                    ),
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (pause_menu_buttons, pause_settings_button, pause_synergy_button, pause_input_resume)
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
struct TowerCostText;

#[derive(Component)]
struct HudPauseButton;

#[derive(Component)]
struct HudSynergyButton;

#[derive(Component)]
struct PauseSynergyButton;

#[derive(Component)]
struct StartWaveButton;

#[derive(Component)]
struct StartWaveButtonText;

#[derive(Component)]
struct TowerButtonBorder(TowerType);

#[derive(Component)]
struct InfoPanel;

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
struct GoldRushIndicator;

#[derive(Component)]
struct SpeedButton(f32);

#[derive(Component)]
struct PauseMenu;

#[derive(Component)]
struct ResumeButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct PauseSettingsButton;

#[derive(Component)]
struct WavePreviewPanel;

#[derive(Component)]
struct WavePreviewText;

#[derive(Component)]
struct TowerDetailPanel;

#[derive(Component)]
struct SellButton;

#[derive(Component)]
struct UpgradeButton;

#[derive(Component)]
struct UpgradeCostText;

#[derive(Component)]
struct SellValueText;


#[derive(Component)]
struct TowerStatsText;

#[derive(Component)]
struct TowerUpgradePreview;

#[derive(Component)]
struct ComboDisplay;

#[derive(Component)]
struct ComboText;

#[derive(Component)]
struct TargetingButton;

#[derive(Component)]
struct TargetingText;

#[derive(Component)]
struct SynergyText;

#[derive(Component)]
struct SpecChoicePanel;

#[derive(Component)]
struct SpecButton(Specialization);

#[derive(Component)]
struct SpecButtonText;

#[derive(Component)]
pub struct TopHudBar;

#[derive(Component)]
pub struct BottomTowerBar;

/// Dynamic UI zone boundaries (actual rendered pixel heights)
#[derive(Resource, Default)]
pub struct UiZones {
    pub top_bar_height: f32,
    pub bottom_bar_height: f32,
}

/// Wave announcement overlay
#[derive(Component)]
struct WaveAnnouncement {
    lifetime: Timer,
}

/// Resource to track life-lost flash effect
#[derive(Resource, Default)]
struct LifeLostFlash {
    timer: Option<Timer>,
    previous_lives: u32,
}

/// Resource to flash a tile red when placement is rejected
#[derive(Resource, Default)]
pub struct PlacementRejectFlash {
    pub timer: Option<Timer>,
    pub grid_pos: Option<(usize, usize)>,
}

/// Resource to flash gold text red when can't afford
#[derive(Resource, Default)]
struct GoldRejectFlash {
    timer: Option<Timer>,
}

/// Resource to track sell confirmation state (double-tap to sell)
#[derive(Resource, Default)]
struct SellPending {
    timer: Option<Timer>,
    tower: Option<Entity>,
}

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

/// Returns (ready, active, remaining_secs) for a given ability type.
fn ability_state(abilities: &PlayerAbilities, ability: AbilityType) -> (bool, bool, f32) {
    match ability {
        AbilityType::Freeze => (
            abilities.freeze_ready,
            false,
            abilities.freeze_cooldown.remaining_secs(),
        ),
        AbilityType::GoldRush => (
            abilities.gold_rush_ready,
            abilities.gold_rush_active.is_some(),
            abilities.gold_rush_active.as_ref()
                .map(|t| t.remaining_secs())
                .unwrap_or_else(|| abilities.gold_rush_cooldown.remaining_secs()),
        ),
        AbilityType::Artillery => (
            abilities.artillery_ready,
            abilities.artillery_targeting,
            abilities.artillery_cooldown.remaining_secs(),
        ),
    }
}

fn setup_ui(mut commands: Commands, assets: Res<GameAssets>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    setup_top_bar(&mut commands, &assets);
    setup_bottom_bar(&mut commands, &assets);
    setup_wave_preview(&mut commands, &assets);
    setup_tower_detail_panel(&mut commands, &assets);
    setup_combo_display(&mut commands, &assets);
    setup_ability_bar(&mut commands, &assets);
}

fn setup_top_bar(commands: &mut Commands, assets: &GameAssets) {
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
            TopHudBar,
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
                    // Gold Rush indicator (hidden by default)
                    parent.spawn((
                        TextBundle::from_section(
                            "2x",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 18.0,
                                color: GameColors::ABILITY_GOLD_RUSH,
                            },
                        ).with_style(Style {
                            display: Display::None,
                            ..default()
                        }),
                        GoldRushIndicator,
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
                                color: Color::srgba(1.0, 1.0, 1.0, 0.85),
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
                        column_gap: Val::Px(3.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for speed in [1.0, 2.0, 3.0, 4.0, 5.0] {
                        let is_default = speed == 1.0;
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Px(38.0),
                                        height: Val::Px(38.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    background_color: if is_default {
                                        Color::srgb(0.15, 0.35, 0.45).into()
                                    } else {
                                        GameColors::BUTTON_NORMAL.into()
                                    },
                                    border_color: BorderColor(if is_default {
                                        GameColors::PRIMARY
                                    } else {
                                        Color::NONE
                                    }),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                SpeedButton(speed),
                            ))
                            .with_children(|parent| {
                                parent.spawn(TextBundle::from_section(
                                    format!("{}x", speed as u32),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 12.0,
                                        color: if is_default {
                                            GameColors::PRIMARY
                                        } else {
                                            Color::srgba(1.0, 1.0, 1.0, 0.65)
                                        },
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
                    // Pause button
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(38.0),
                                height: Val::Px(38.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::left(Val::Px(8.0)),
                                ..default()
                            },
                            background_color: GameColors::BUTTON_NORMAL.into(),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        HudPauseButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "| |",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.65),
                            },
                        ));
                    });
                    // Synergy reference button
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(38.0),
                                height: Val::Px(38.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::left(Val::Px(4.0)),
                                ..default()
                            },
                            background_color: GameColors::BUTTON_NORMAL.into(),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        HudSynergyButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "?",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 16.0,
                                color: Color::srgb(0.9, 0.7, 1.0),
                            },
                        ));
                    });
                });
        });
}

fn setup_bottom_bar(commands: &mut Commands, assets: &GameAssets) {
    // Bottom bar - Tower selection (touch-friendly)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(65.0),
                    max_height: Val::Px(220.0),
                    padding: UiRect::all(Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Px(0.0),
                    ..default()
                },
                background_color: GameColors::UI_OVERLAY.into(),
                ..default()
            },
            BottomTowerBar,
            GameEntity,
        ))
        .with_children(|parent| {
            // Tower buttons - compact for mobile (8 towers now)
            for (idx, tower_type) in [TowerType::Basic, TowerType::Splash, TowerType::Slow, TowerType::Sniper, TowerType::Rapid, TowerType::Chain, TowerType::Poison, TowerType::Buff].iter().enumerate() {
                let tower_type = *tower_type;
                parent
                    .spawn((
                        NodeBundle {
                            style: Style {
                                min_width: Val::Px(56.0),
                                max_width: Val::Px(75.0),
                                flex_basis: Val::Px(65.0),
                                flex_shrink: 1.0,
                                height: Val::Px(100.0),
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
                                // Keyboard shortcut label
                                parent.spawn(TextBundle::from_section(
                                    format!("{}", idx + 1),
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 10.0,
                                        color: Color::srgba(1.0, 1.0, 1.0, 0.55),
                                    },
                                ).with_style(Style {
                                    align_self: AlignSelf::FlexStart,
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(2.0),
                                    left: Val::Px(4.0),
                                    ..default()
                                }));

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
                                parent.spawn((
                                    TextBundle::from_section(
                                        format!("{}g", tower_type.cost()),
                                        TextStyle {
                                            font: assets.font.clone(),
                                            font_size: 13.0,
                                            color: GameColors::GOLD,
                                        },
                                    ),
                                    TowerCostText,
                                ));

                                // 3-stat rows (or 2 for Buff tower)
                                if tower_type == TowerType::Buff {
                                    parent.spawn(TextBundle::from_section(
                                        format!("BUFF +25%\nRNG {:.0}", tower_type.range()),
                                        TextStyle {
                                            font: assets.font.clone(),
                                            font_size: 9.0,
                                            color: Color::srgba(1.0, 1.0, 1.0, 0.85),
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
                                            color: Color::srgba(1.0, 1.0, 1.0, 0.85),
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
                            min_width: Val::Px(140.0),
                            max_width: Val::Px(180.0),
                            flex_basis: Val::Px(160.0),
                            flex_shrink: 1.0,
                            height: Val::Px(100.0),
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
                                color: Color::srgba(1.0, 1.0, 1.0, 0.75),
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
                    parent.spawn((
                        TextBundle::from_section(
                            "START",
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 14.0,
                                color: Color::WHITE,
                            },
                        ),
                        StartWaveButtonText,
                    ));
                });
        });
}

fn setup_wave_preview(commands: &mut Commands, assets: &GameAssets) {
    // Wave preview panel (positioned above START button, shown between waves)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    position_type: PositionType::Absolute,
                    right: Val::Px(8.0),
                    bottom: Val::Px(72.0),
                    display: Display::None,
                    ..default()
                },
                background_color: Color::srgba(0.06, 0.06, 0.1, 0.9).into(),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            WavePreviewPanel,
            GameEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 11.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                    },
                ),
                WavePreviewText,
            ));
        });
}

fn setup_tower_detail_panel(commands: &mut Commands, assets: &GameAssets) {
    // Tower detail panel (fixed right side, initially hidden) - for upgrade/sell with stats
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(150.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(Val::Px(8.0)),
                    position_type: PositionType::Absolute,
                    right: Val::Px(10.0),
                    top: Val::Px(60.0),
                    display: Display::None,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.6).into(),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            TowerDetailPanel,
            GameEntity,
        ))
        .with_children(|parent| {
            // Tower stats section with background
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(6.0)),
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
                                    font_size: 12.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "DMG: 25  RNG: 150  SPD: 1.0/s",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 10.0,
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
                        padding: UiRect::all(Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(6.0)),
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
                                    font_size: 10.0,
                                    color: GameColors::SUCCESS,
                                },
                            ),
                            TextSection::new(
                                "DMG: 30 (+5)\nRNG: 165 (+15)\nSPD: 1.2/s (+0.2)",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 9.0,
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
                        font_size: 10.0,
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
                            height: Val::Px(44.0),
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
                                font_size: 11.0,
                                color: Color::WHITE,
                            },
                        ),
                        TargetingText,
                    ));
                });

            // Specialization choice panel (hidden by default, shown when needs_specialization)
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            display: Display::None,
                            margin: UiRect::bottom(Val::Px(6.0)),
                            ..default()
                        },
                        ..default()
                    },
                    SpecChoicePanel,
                ))
                .with_children(|parent| {
                    // Header
                    parent.spawn(TextBundle::from_section(
                        "Choose Specialization:",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 11.0,
                            color: GameColors::PRIMARY,
                        },
                    ).with_style(Style {
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    }));

                    // Two spec buttons (text updated dynamically)
                    for i in 0..2 {
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Percent(100.0),
                                        min_height: Val::Px(36.0),
                                        padding: UiRect::all(Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::bottom(Val::Px(4.0)),
                                        ..default()
                                    },
                                    background_color: Color::srgb(0.2, 0.3, 0.5).into(),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                // Default to Marksman, will be updated dynamically
                                SpecButton(if i == 0 { Specialization::Marksman } else { Specialization::Gunner }),
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    TextBundle::from_section(
                                        "",
                                        TextStyle {
                                            font: assets.font.clone(),
                                            font_size: 10.0,
                                            color: Color::WHITE,
                                        },
                                    ),
                                    SpecButtonText,
                                ));
                            });
                    }
                });

            // Upgrade button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(44.0),
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
                                font_size: 11.0,
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
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_SELL.into(),
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
                                font_size: 11.0,
                                color: Color::WHITE,
                            },
                        ),
                        SellValueText,
                    ));
                });
        });
}

fn setup_combo_display(commands: &mut Commands, assets: &GameAssets) {
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
}

fn setup_ability_bar(commands: &mut Commands, assets: &GameAssets) {
    // Ability bar (left side, vertical)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(60.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
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
            spawn_ability_button(parent, assets, AbilityType::Freeze, "Q", "Freeze",
                "All enemies 3s", "CD: 45s",
                GameColors::ABILITY_FREEZE);
            spawn_ability_button(parent, assets, AbilityType::GoldRush, "W", "Gold Rush",
                "2x gold 10s", "CD: 60s",
                GameColors::ABILITY_GOLD_RUSH);
            spawn_ability_button(parent, assets, AbilityType::Artillery, "E", "Artillery",
                "400 dmg AOE", "CD: 90s",
                GameColors::ABILITY_NUKE);
        });
}

fn spawn_ability_button(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    ability: AbilityType,
    key: &str,
    name: &str,
    stat_line: &str,
    cooldown_line: &str,
    color: Color,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(110.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(6.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                background_color: color.with_alpha(0.2).into(),
                border_color: BorderColor(Color::NONE),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            AbilityButton(ability),
        ))
        .with_children(|btn| {
            // Top row: key badge + ability name
            btn.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                // Key badge
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
                // Ability name
                row.spawn(TextBundle::from_section(
                    name,
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 13.0,
                        color,
                    },
                ));
            });

            // Stat line
            btn.spawn(TextBundle::from_section(
                stat_line,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 9.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.75),
                },
            ));

            // Cooldown + status row
            btn.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                row.spawn(TextBundle::from_section(
                    cooldown_line,
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 9.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.65),
                    },
                ));
                row.spawn((
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

fn update_gold_reject_flash(
    mut flash: ResMut<GoldRejectFlash>,
    mut query: Query<&mut Text, With<GoldText>>,
    time: Res<Time>,
) {
    if let Some(ref mut timer) = flash.timer {
        timer.tick(time.delta());
        let t = timer.fraction();
        let flash_color = GameColors::HEALTH_LOW.mix(&GameColors::GOLD, t);
        for mut text in &mut query {
            text.sections[0].style.color = flash_color;
        }
        if timer.finished() {
            flash.timer = None;
        }
    }
}

fn update_gold_rush_indicator(
    abilities: Res<PlayerAbilities>,
    mut indicator_query: Query<(&mut Text, &mut Style), With<GoldRushIndicator>>,
    mut gold_text_query: Query<&mut Text, (With<GoldText>, Without<GoldRushIndicator>)>,
    time: Res<Time>,
) {
    let is_active = abilities.gold_rush_active.is_some();
    let remaining = abilities.gold_rush_active.as_ref().map(|t| t.remaining_secs()).unwrap_or(0.0);

    for (mut text, mut style) in &mut indicator_query {
        if is_active {
            style.display = Display::Flex;
            text.sections[0].value = format!("2x {:.0}s", remaining);

            // Pulse the indicator color - brighter when more time left
            let pulse = (time.elapsed_seconds() * 4.0).sin() * 0.5 + 0.5;
            let alpha = 0.7 + pulse * 0.3;
            text.sections[0].style.color = GameColors::ABILITY_GOLD_RUSH.with_alpha(alpha);
        } else {
            style.display = Display::None;
        }
    }

    // Pulse the gold amount text while Gold Rush is active
    for mut text in &mut gold_text_query {
        if is_active {
            let pulse = (time.elapsed_seconds() * 3.0).sin() * 0.5 + 0.5;
            let r = GameColors::GOLD.to_srgba().red * (0.85 + pulse * 0.15);
            let g = GameColors::GOLD.to_srgba().green * (0.85 + pulse * 0.15);
            let b = GameColors::GOLD.to_srgba().blue * (0.85 + pulse * 0.15);
            text.sections[0].style.color = Color::srgb(r.min(1.0), g.min(1.0), b.min(1.0));
        } else {
            text.sections[0].style.color = GameColors::GOLD;
        }
    }
}

fn update_lives_display(
    economy: Res<PlayerEconomy>,
    mut query: Query<&mut Text, With<LivesText>>,
    mut flash: ResMut<LifeLostFlash>,
) {
    if economy.is_changed() {
        // Detect life loss and trigger flash
        if flash.previous_lives > 0 && economy.lives < flash.previous_lives {
            flash.timer = Some(Timer::from_seconds(0.4, TimerMode::Once));
        }
        flash.previous_lives = economy.lives;

        for mut text in &mut query {
            text.sections[0].value = format!("{}", economy.lives);
        }
    }
}

fn update_life_flash(
    mut flash: ResMut<LifeLostFlash>,
    mut query: Query<&mut Text, With<LivesText>>,
    time: Res<Time>,
) {
    if let Some(ref mut timer) = flash.timer {
        timer.tick(time.delta());
        let progress = timer.fraction();

        for mut text in &mut query {
            // Lerp from bright red back to primary color
            let flash_color = GameColors::HEALTH_LOW;
            let normal_color = GameColors::PRIMARY;
            let r = flash_color.to_srgba().red + (normal_color.to_srgba().red - flash_color.to_srgba().red) * progress;
            let g = flash_color.to_srgba().green + (normal_color.to_srgba().green - flash_color.to_srgba().green) * progress;
            let b = flash_color.to_srgba().blue + (normal_color.to_srgba().blue - flash_color.to_srgba().blue) * progress;
            text.sections[0].style.color = Color::srgb(r, g, b);
        }

        if timer.finished() {
            flash.timer = None;
        }
    }
}

fn update_wave_display(
    wave_manager: Res<WaveManager>,
    mut query: Query<&mut Text, With<WaveText>>,
) {
    if !wave_manager.is_changed() { return; }
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
    if !kill_streak.is_changed() { return; }
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
        let (ready, active, _remaining) = ability_state(&abilities, button.0);

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
        let (ready, active, remaining) = ability_state(&abilities, cooldown_text.0);

        if ready {
            text.sections[0].value = "Ready".to_string();
            text.sections[0].style.color = GameColors::SUCCESS;
        } else if active {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = GameColors::GOLD;
        } else {
            text.sections[0].value = format!("{:.0}s", remaining);
            text.sections[0].style.color = Color::srgba(1.0, 1.0, 1.0, 0.65);
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
                    color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                },
            ));
        });
    }
}

fn ability_button_activate(
    button_query: Query<(&Interaction, &AbilityButton), Changed<Interaction>>,
    mut abilities: ResMut<PlayerAbilities>,
    mut freeze_events: EventWriter<FreezeAbilityEvent>,
    mut gold_rush_events: EventWriter<GoldRushAbilityEvent>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, button) in &button_query {
        match *interaction {
            Interaction::Pressed => {
                ui_clicked.0 = true;
                match button.0 {
                    AbilityType::Freeze => {
                        if abilities.freeze_ready {
                            freeze_events.send(FreezeAbilityEvent);
                            abilities.freeze_ready = false;
                        }
                    }
                    AbilityType::GoldRush => {
                        if abilities.gold_rush_ready {
                            gold_rush_events.send(GoldRushAbilityEvent);
                            abilities.gold_rush_ready = false;
                        }
                    }
                    AbilityType::Artillery => {
                        if abilities.artillery_ready {
                            abilities.artillery_targeting = !abilities.artillery_targeting;
                        }
                    }
                }
            }
            Interaction::Hovered => {
                ui_clicked.0 = true;
            }
            Interaction::None => {}
        }
    }
}

fn adapt_ability_bar_layout(
    screen_info: Res<ScreenInfo>,
    mut ability_bar: Query<&mut Style, With<AbilityBar>>,
    mut ability_buttons: Query<&mut Style, (With<AbilityButton>, Without<AbilityBar>)>,
) {
    if !screen_info.is_changed() { return; }

    let Ok(mut bar_style) = ability_bar.get_single_mut() else { return };

    if screen_info.is_landscape {
        // Landscape mobile: horizontal bar at top-right to avoid blocking grid
        bar_style.flex_direction = FlexDirection::Row;
        bar_style.column_gap = Val::Px(6.0);
        bar_style.row_gap = Val::Px(0.0);
        bar_style.left = Val::Auto;
        bar_style.right = Val::Px(10.0);
        bar_style.top = Val::Px(60.0);

        // Compact ability buttons for landscape
        for mut btn_style in &mut ability_buttons {
            btn_style.width = Val::Px(90.0);
        }
    } else {
        // Default: vertical column on the left
        bar_style.flex_direction = FlexDirection::Column;
        bar_style.column_gap = Val::Px(0.0);
        bar_style.row_gap = Val::Px(6.0);
        bar_style.left = Val::Px(10.0);
        bar_style.right = Val::Auto;
        bar_style.top = Val::Px(60.0);

        for mut btn_style in &mut ability_buttons {
            btn_style.width = Val::Px(110.0);
        }
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
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, mut color, tower_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected.0 = tower_button.0;
                *color = GameColors::BUTTON_SELECTED.into();
                ui_clicked.0 = true;
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_HOVER.into();
                ui_clicked.0 = true;
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

fn update_tower_affordability(
    economy: Res<PlayerEconomy>,
    button_query: Query<(&TowerButton, &Interaction, &Children)>,
    mut cost_text_query: Query<&mut Text, With<TowerCostText>>,
) {
    for (tower_button, interaction, children) in &button_query {
        let can_afford = economy.gold >= tower_button.0.cost();

        if *interaction != Interaction::None {
            continue;
        }

        for &child in children.iter() {
            if let Ok(mut text) = cost_text_query.get_mut(child) {
                text.sections[0].style.color = if can_afford {
                    GameColors::GOLD
                } else {
                    GameColors::HEALTH_LOW
                };
            }
        }
    }
}

fn start_wave_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StartWaveButton>),
    >,
    mut wave_manager: ResMut<WaveManager>,
    mut economy: ResMut<PlayerEconomy>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    existing_announcements: Query<Entity, With<WaveAnnouncement>>,
    mut ui_clicked: ResMut<UiClicked>,
    mut stats: ResMut<super::stats::GameStats>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = GameColors::BUTTON_START_PRESSED.into();
                ui_clicked.0 = true;
                if !wave_manager.wave_active {
                    // Calculate early-send bonus before starting wave
                    let early_bonus = wave_manager.early_send_bonus(time.elapsed_seconds_f64());

                    wave_manager.start_wave();

                    // Award early-send bonus
                    if early_bonus > 0 {
                        economy.gold += early_bonus;
                        economy.score += early_bonus;
                        stats.record_early_send(early_bonus);
                    }

                    // Despawn any existing announcement
                    for entity in &existing_announcements {
                        commands.entity(entity).despawn_recursive();
                    }

                    // Spawn wave announcement
                    let wave_num = wave_manager.current_wave + 1;
                    let modifier_name = wave_manager.current_modifier.name();

                    commands.spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                top: Val::Percent(35.0),
                                left: Val::Percent(50.0),
                                margin: UiRect::left(Val::Px(-120.0)),
                                width: Val::Px(240.0),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(12.0)),
                                ..default()
                            },
                            background_color: Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                            ..default()
                        },
                        WaveAnnouncement {
                            lifetime: Timer::from_seconds(1.5, TimerMode::Once),
                        },
                        GameEntity,
                    )).with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            format!("WAVE {}", wave_num),
                            TextStyle {
                                font: assets.font.clone(),
                                font_size: 40.0,
                                color: GameColors::PRIMARY,
                            },
                        ));
                        if early_bonus > 0 {
                            parent.spawn(TextBundle::from_section(
                                format!("+{}g EARLY BONUS", early_bonus),
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 16.0,
                                    color: GameColors::GOLD,
                                },
                            ).with_style(Style {
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            }));
                        }
                        if !modifier_name.is_empty() {
                            parent.spawn(TextBundle::from_section(
                                modifier_name,
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 18.0,
                                    color: GameColors::GOLD,
                                },
                            ).with_style(Style {
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            }));
                        }
                    });
                }
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_START_HOVER.into();
                ui_clicked.0 = true;
            }
            Interaction::None => {
                *color = GameColors::BUTTON_START.into();
            }
        }
    }
}

fn update_wave_preview(
    wave_manager: Res<WaveManager>,
    mut panel_query: Query<&mut Style, With<WavePreviewPanel>>,
    mut text_query: Query<&mut Text, With<WavePreviewText>>,
    mut cached_wave: Local<usize>,
    mut cached_text: Local<String>,
) {
    for mut style in &mut panel_query {
        if wave_manager.wave_active {
            style.display = Display::None;
            continue;
        }

        style.display = Display::Flex;

        // Only recompute preview when wave changes
        let next_wave = wave_manager.current_wave + 1;
        if *cached_wave != next_wave {
            *cached_wave = next_wave;

            let preview = wave_manager.preview_next_wave();
            let mut text = format!("NEXT: Wave {}", next_wave);

            if preview.modifier != WaveModifier::None {
                text.push_str(&format!("\n{}", preview.modifier.name()));
            }

            // Show fuzzy counts per enemy type with brief descriptions
            for (etype, count) in &preview.counts {
                let fuzzy = fuzzy_count(*count);
                let name = match etype {
                    EnemyType::Basic => "Basic",
                    EnemyType::Fast => "Fast (quick)",
                    EnemyType::Tank => "Tank (tough)",
                    EnemyType::Armored => "Armored (resists)",
                    EnemyType::Flying => "Flying (skips path)",
                    EnemyType::Boss => "BOSS (high HP)",
                    EnemyType::Splitter => "Splitter",
                    EnemyType::MiniSplitter => "Mini",
                };
                text.push_str(&format!("\n  {} x{}", name, fuzzy));
            }

            *cached_text = text;
        }

        for mut text in &mut text_query {
            text.sections[0].value = cached_text.clone();
        }
    }
}

/// Display fuzzy counts: ~5, ~10, ~15, ~25, 30+
fn fuzzy_count(count: usize) -> &'static str {
    match count {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4..=7 => "~5",
        8..=12 => "~10",
        13..=18 => "~15",
        19..=27 => "~25",
        _ => "30+",
    }
}

fn update_start_button_text(
    wave_manager: Res<WaveManager>,
    time: Res<Time>,
    mut text_query: Query<&mut Text, With<StartWaveButtonText>>,
) {
    for mut text in &mut text_query {
        let new_value = if wave_manager.wave_active {
            "START".to_string()
        } else {
            let bonus = wave_manager.early_send_bonus(time.elapsed_seconds_f64());
            if bonus > 0 { format!("START +{}g", bonus) } else { "START".into() }
        };
        if text.sections[0].value != new_value {
            text.sections[0].value = new_value;
        }
    }
}

fn update_wave_announcement(
    mut commands: Commands,
    mut announcements: Query<(Entity, &mut WaveAnnouncement, &Children)>,
    mut text_query: Query<&mut Text, Without<StartWaveButtonText>>,
    time: Res<Time>,
) {
    for (entity, mut announcement, children) in &mut announcements {
        announcement.lifetime.tick(time.delta());
        let progress = announcement.lifetime.fraction();

        // Fade out text alpha
        let alpha = if progress < 0.3 {
            // Fade in quickly
            progress / 0.3
        } else if progress > 0.7 {
            // Fade out
            1.0 - (progress - 0.7) / 0.3
        } else {
            1.0
        };

        // Update all child text alphas
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                if let Some(section) = text.sections.get_mut(0) {
                    let base_color = section.style.color;
                    section.style.color = base_color.with_alpha(alpha);
                }
            }
        }

        if announcement.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Reset UI clicked flag at the start of each frame
fn reset_ui_clicked(mut ui_clicked: ResMut<UiClicked>) {
    ui_clicked.0 = false;
}

/// Update UI zone boundaries from actual rendered node sizes
fn update_ui_zones(
    top_bar: Query<&Node, With<TopHudBar>>,
    bottom_bar: Query<&Node, With<BottomTowerBar>>,
    mut zones: ResMut<UiZones>,
) {
    if let Ok(node) = top_bar.get_single() {
        zones.top_bar_height = node.size().y;
    }
    if let Ok(node) = bottom_bar.get_single() {
        zones.bottom_bar_height = node.size().y;
    }
}

fn handle_tile_click(
    pointer: Res<super::PointerState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    map: Res<GameMap>,
    selected: Res<SelectedTowerType>,
    economy: Res<PlayerEconomy>,
    mut place_events: EventWriter<PlaceTowerEvent>,
    ui_clicked: Res<UiClicked>,
    ui_zones: Res<UiZones>,
    mut tile_flash: ResMut<PlacementRejectFlash>,
    mut gold_flash: ResMut<GoldRejectFlash>,
) {
    if ui_clicked.0 || !pointer.just_pressed {
        return;
    }

    let Ok(window) = windows.get_single() else { return; };
    let Ok((camera, camera_transform)) = camera_q.get_single() else { return; };
    let Some(cursor_pos) = pointer.position else { return; };
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return; };
    let Some((grid_x, grid_y)) = GameMap::world_to_grid(world_pos) else { return; };

    // UI zone check FIRST — clicks in HUD area should not trigger feedback
    let window_height = window.height();
    if cursor_pos.y > window_height - ui_zones.bottom_bar_height - 10.0 || cursor_pos.y < ui_zones.top_bar_height + 10.0 {
        return;
    }

    // Buildability check — flash tile red on rejection
    if !map.is_buildable(grid_x, grid_y) {
        tile_flash.timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
        tile_flash.grid_pos = Some((grid_x, grid_y));
        return;
    }

    // Affordability check — flash gold text red on rejection
    if economy.gold < selected.0.cost() {
        gold_flash.timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
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
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor, &SpeedButton, &Children),
        With<SpeedButton>,
    >,
    mut game_speed: ResMut<GameSpeed>,
    mut text_query: Query<&mut Text>,
    mut time: ResMut<Time<Virtual>>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    let active_bg: BackgroundColor = Color::srgb(0.15, 0.35, 0.45).into();
    let inactive_bg: BackgroundColor = GameColors::BUTTON_NORMAL.into();
    let hover_bg: BackgroundColor = GameColors::BUTTON_HOVER.into();

    // First pass: detect press and update game speed
    let mut new_speed: Option<f32> = None;
    for (interaction, _, _, speed_button, _) in &button_query {
        if *interaction == Interaction::Pressed {
            new_speed = Some(speed_button.0);
            ui_clicked.0 = true;
        } else if *interaction == Interaction::Hovered {
            ui_clicked.0 = true;
        }
    }

    if let Some(speed) = new_speed {
        game_speed.0 = speed;
        time.set_relative_speed(speed);
    }

    // Second pass: update all button visuals based on current game speed
    for (interaction, mut color, mut border, speed_button, children) in &mut button_query {
        let is_active = speed_button.0 == game_speed.0;

        if is_active {
            *color = active_bg;
            *border = BorderColor(GameColors::PRIMARY);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = GameColors::PRIMARY;
                }
            }
        } else if *interaction == Interaction::Hovered {
            *color = hover_bg;
            *border = BorderColor(Color::NONE);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = Color::srgba(1.0, 1.0, 1.0, 0.65);
                }
            }
        } else {
            *color = inactive_bg;
            *border = BorderColor(Color::NONE);
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color = Color::srgba(1.0, 1.0, 1.0, 0.65);
                }
            }
        }
    }
}

fn pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    selected_tower: Res<SelectedPlacedTower>,
    pause_button: Query<&Interaction, (Changed<Interaction>, With<HudPauseButton>)>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
    synergy_button: Query<&Interaction, (Changed<Interaction>, With<HudSynergyButton>)>,
) {
    // "?" button opens synergy panel
    for interaction in &synergy_button {
        if *interaction == Interaction::Pressed {
            synergy_open.0 = !synergy_open.0;
        }
    }
    // ESC: close synergy panel first, then deselect tower, then pause
    if keyboard.just_pressed(KeyCode::Escape) {
        if synergy_open.0 {
            synergy_open.0 = false;
        } else if selected_tower.0.is_none() {
            next_state.set(GameState::Paused);
        }
        return;
    }
    // Pause via HUD button click
    for interaction in &pause_button {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Paused);
        }
    }
}

fn pause_input_resume(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if synergy_open.0 {
            synergy_open.0 = false;
        } else {
            next_state.set(GameState::Playing);
        }
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

            // Settings button
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
                        background_color: Color::srgba(1.0, 1.0, 1.0, 0.1).into(),
                        ..default()
                    },
                    PauseSettingsButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "SETTINGS",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                        },
                    ));
                });

            // Synergies button
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
                        background_color: Color::srgba(1.0, 1.0, 1.0, 0.1).into(),
                        ..default()
                    },
                    PauseSynergyButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "SYNERGIES",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            color: Color::srgb(0.9, 0.7, 1.0),
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
                        background_color: GameColors::BUTTON_SELL.into(),
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
                    color: Color::srgba(1.0, 1.0, 1.0, 0.65),
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

/// Apply standard button hover/unhover colors. Returns true if pressed.
fn button_interaction(
    interaction: &Interaction,
    color: &mut BackgroundColor,
    normal: Color,
    hover: Color,
) -> bool {
    match *interaction {
        Interaction::Pressed => true,
        Interaction::Hovered => {
            *color = hover.into();
            false
        }
        Interaction::None => {
            *color = normal.into();
            false
        }
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
        if button_interaction(interaction, &mut color, GameColors::BUTTON_START, GameColors::BUTTON_START_HOVER) {
            next_state.set(GameState::Playing);
        }
    }

    for (interaction, mut color) in &mut quit_query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_SELL, GameColors::BUTTON_SELL_HOVER) {
            next_state.set(GameState::Menu);
        }
    }
}

fn pause_settings_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PauseSettingsButton>)>,
    mut settings_open: ResMut<SettingsOpen>,
) {
    for (interaction, mut color) in &mut query {
        if button_interaction(interaction, &mut color, Color::srgba(1.0, 1.0, 1.0, 0.1), Color::srgba(1.0, 1.0, 1.0, 0.2)) {
            settings_open.0 = true;
        }
    }
}

fn pause_synergy_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<PauseSynergyButton>)>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
) {
    for (interaction, mut color) in &mut query {
        if button_interaction(interaction, &mut color, Color::srgba(1.0, 1.0, 1.0, 0.1), Color::srgba(1.0, 1.0, 1.0, 0.2)) {
            synergy_open.0 = true;
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
    mut sell_pending: ResMut<SellPending>,
    time: Res<Time>,
) {
    // Tick sell confirmation timer
    if let Some(ref mut timer) = sell_pending.timer {
        timer.tick(time.delta());
        if timer.finished() {
            sell_pending.timer = None;
            sell_pending.tower = None;
        }
    }

    // Escape to deselect (also cancels pending sell)
    if keyboard.just_pressed(KeyCode::Escape) && selected_tower.0.is_some() {
        selected_tower.0 = None;
        sell_pending.timer = None;
        sell_pending.tower = None;
        return;
    }

    // Keyboard shortcuts when tower is selected
    if let Some(tower_entity) = selected_tower.0 {
        if let Ok(tower) = towers.get(tower_entity) {
            // U to upgrade
            if keyboard.just_pressed(KeyCode::KeyU)
                && tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
                    upgrade_events.send(UpgradeTowerEvent { tower: tower_entity });
                }
            // S to sell — requires double-tap within 1.5 seconds
            if keyboard.just_pressed(KeyCode::KeyS) {
                if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                    // Second press — confirm sell
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                    sell_pending.timer = None;
                    sell_pending.tower = None;
                } else {
                    // First press — enter pending state
                    sell_pending.tower = Some(tower_entity);
                    sell_pending.timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
                }
            }
        }
    }
}

fn update_tower_context_menu(
    hovered_tile: Res<HoveredTile>,
    map: Res<GameMap>,
    towers: Query<(Entity, &Tower, Option<&TowerSynergies>)>,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut context_menu: Query<&mut Style, With<TowerDetailPanel>>,
    mut upgrade_text: Query<&mut Text, (With<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut sell_text: Query<&mut Text, (With<SellValueText>, Without<UpgradeCostText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut stats_text: Query<&mut Text, (With<TowerStatsText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerUpgradePreview>, Without<TargetingText>, Without<SynergyText>)>,
    mut preview_text: Query<&mut Text, (With<TowerUpgradePreview>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TargetingText>, Without<SynergyText>)>,
    mut targeting_text: Query<&mut Text, (With<TargetingText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<SynergyText>)>,
    mut synergy_text: Query<&mut Text, (With<SynergyText>, Without<UpgradeCostText>, Without<SellValueText>, Without<TowerStatsText>, Without<TowerUpgradePreview>, Without<TargetingText>)>,
    pointer: Res<super::PointerState>,
    economy: Res<PlayerEconomy>,
    sell_pending: Res<SellPending>,
) {
    // Left-click or touch to select/deselect towers
    if pointer.just_pressed {
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

    // Update detail panel visibility
    for mut style in &mut context_menu {
        if let Some(tower_entity) = selected_tower.0 {
            if let Ok((_, tower, synergies)) = towers.get(tower_entity) {
                style.display = Display::Flex;

                // Get current attack speed
                let attack_speed = tower.attack_speed();
                let is_buff_tower = tower.tower_type == TowerType::Buff;

                // Update stats text
                for mut text in &mut stats_text {
                    let spec_label = tower.specialization
                        .map(|s| format!(" [{}]", s.name()))
                        .unwrap_or_default();
                    text.sections[0].value = format!("{}{} Lv{}\n", tower.tower_type.name(), spec_label, tower.level);
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

                let needs_spec = tower.needs_specialization();

                if needs_spec {
                    // Clear upgrade button/preview when spec choice is needed
                    for mut text in &mut upgrade_text {
                        text.sections[0].value = String::new();
                    }
                    for mut text in &mut preview_text {
                        text.sections[0].value = "Choose Specialization\n".to_string();
                        text.sections[1].value = String::new();
                    }
                } else {
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
                            let spec_label = tower.specialization
                                .map(|s| format!(" ({})", s.name()))
                                .unwrap_or_default();
                            text.sections[1].value = format!(
                                "DMG: {:.0} (+{:.0})\nRNG: {:.0} (+{:.0})\nSPD: {:.2}/s (+{:.2}){}",
                                next_damage, next_damage - tower.damage,
                                next_range, next_range - tower.range,
                                next_speed, next_speed - attack_speed,
                                spec_label
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
                }
                for mut text in &mut sell_text {
                    if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                        text.sections[0].value = "Confirm [S]?".into();
                        text.sections[0].style.color = Color::srgb(1.0, 0.4, 0.4);
                    } else {
                        text.sections[0].value = format!("Sell [S] +{}g", tower.sell_value());
                        text.sections[0].style.color = Color::WHITE;
                    }
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

fn update_spec_panel(
    selected_tower: Res<SelectedPlacedTower>,
    towers: Query<&Tower>,
    mut spec_panel: Query<&mut Style, With<SpecChoicePanel>>,
    mut spec_buttons: Query<(&mut SpecButton, &Children)>,
    mut spec_texts: Query<&mut Text, With<SpecButtonText>>,
) {
    let Some(tower_entity) = selected_tower.0 else {
        for mut style in &mut spec_panel {
            style.display = Display::None;
        }
        return;
    };

    let Ok(tower) = towers.get(tower_entity) else {
        for mut style in &mut spec_panel {
            style.display = Display::None;
        }
        return;
    };

    let needs_spec = tower.needs_specialization();

    for mut style in &mut spec_panel {
        style.display = if needs_spec { Display::Flex } else { Display::None };
    }

    if needs_spec {
        if let Some(choices) = Specialization::choices_for(tower.tower_type) {
            let upgrade_cost = tower.upgrade_cost();
            let mut button_iter = spec_buttons.iter_mut();
            for spec in choices.iter() {
                if let Some((mut spec_btn, children)) = button_iter.next() {
                    spec_btn.0 = *spec;
                    for child in children.iter() {
                        if let Ok(mut text) = spec_texts.get_mut(*child) {
                            text.sections[0].value = format!(
                                "{}: {}\n{}g",
                                spec.name(), spec.description(), upgrade_cost
                            );
                        }
                    }
                }
            }
        }
    }
}

fn tower_context_buttons(
    mut upgrade_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<UpgradeButton>, Without<SellButton>, Without<TargetingButton>),
    >,
    mut sell_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<SellButton>, Without<UpgradeButton>, Without<TargetingButton>),
    >,
    mut targeting_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<TargetingButton>, Without<UpgradeButton>, Without<SellButton>),
    >,
    mut selected_tower: ResMut<SelectedPlacedTower>,
    mut towers: Query<&mut Tower>,
    economy: Res<PlayerEconomy>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut upgrade_events: EventWriter<UpgradeTowerEvent>,
    mut sell_events: EventWriter<SellTowerEvent>,
    mut targeting_text: Query<&mut Text, With<TargetingText>>,
    mut ui_clicked: ResMut<UiClicked>,
    mut sell_pending: ResMut<SellPending>,
) {
    // Check if we can afford upgrade and it's allowed
    let can_afford = if let Some(tower_entity) = selected_tower.0 {
        if let Ok(tower) = towers.get(tower_entity) {
            tower.can_upgrade() && economy.gold >= tower.upgrade_cost()
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
                        if tower.can_upgrade() && economy.gold >= tower.upgrade_cost() {
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

    // Sell button — requires double-click to confirm
    for (interaction, mut color) in &mut sell_query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, GameColors::BUTTON_SELL, GameColors::BUTTON_SELL_HOVER) {
            if let Some(tower_entity) = selected_tower.0 {
                if sell_pending.tower == Some(tower_entity) && sell_pending.timer.is_some() {
                    sell_events.send(SellTowerEvent { tower: tower_entity });
                    selected_tower.0 = None;
                    sell_pending.timer = None;
                    sell_pending.tower = None;
                } else {
                    sell_pending.tower = Some(tower_entity);
                    sell_pending.timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
                }
            }
        }
    }

    // Handle targeting button click or [T] key
    let mut should_cycle_targeting = false;

    for (interaction, mut color) in &mut targeting_query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, Color::srgb(0.3, 0.3, 0.5), Color::srgb(0.4, 0.4, 0.65)) {
            should_cycle_targeting = true;
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

fn spec_button_system(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &SpecButton),
        Changed<Interaction>,
    >,
    selected_tower: Res<SelectedPlacedTower>,
    towers: Query<&Tower>,
    economy: Res<PlayerEconomy>,
    mut spec_events: EventWriter<SpecializeTowerEvent>,
    mut ui_clicked: ResMut<UiClicked>,
) {
    for (interaction, mut color, spec_btn) in &mut query {
        if *interaction != Interaction::None { ui_clicked.0 = true; }
        if button_interaction(interaction, &mut color, Color::srgb(0.2, 0.3, 0.5), Color::srgb(0.3, 0.4, 0.65)) {
            if let Some(tower_entity) = selected_tower.0 {
                if let Ok(tower) = towers.get(tower_entity) {
                    if tower.needs_specialization() && economy.gold >= tower.upgrade_cost() {
                        spec_events.send(SpecializeTowerEvent {
                            tower: tower_entity,
                            specialization: spec_btn.0,
                        });
                    }
                }
            }
        }
    }
}

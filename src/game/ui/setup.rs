use super::*;

pub(super) fn setup_ui(mut commands: Commands, assets: Res<GameAssets>, active: Option<Res<super::super::GameActive>>) {
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
                                color: GameColors::TEXT_BRIGHT,
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
                                        width: Val::Px(44.0),
                                        height: Val::Px(44.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    background_color: if is_default {
                                        GameColors::BUTTON_ACTIVE.into()
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
                                            GameColors::TEXT_MEDIUM
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
                                width: Val::Px(44.0),
                                height: Val::Px(44.0),
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
                                color: GameColors::TEXT_MEDIUM,
                            },
                        ));
                    });
                    // Synergy reference button
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(44.0),
                                height: Val::Px(44.0),
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
                                color: GameColors::SYNERGY,
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
                                        color: GameColors::TEXT_MEDIUM,
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
                                            font_size: 10.0,
                                            color: GameColors::TEXT_BRIGHT,
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
                                            font_size: 10.0,
                                            color: GameColors::TEXT_BRIGHT,
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
                                font_size: 10.0,
                                color: GameColors::TEXT_SECONDARY,
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
                background_color: GameColors::PANEL_BG.into(),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            TowerDetailPanel,
            GameEntity,
        ))
        .with_children(|parent| {
            spawn_stats_section(parent, assets);
            spawn_upgrade_preview(parent, assets);

            // Synergy display (hidden when no synergies active)
            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 10.0,
                        color: GameColors::SYNERGY, // Light purple for synergies
                    },
                ).with_style(Style {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                }),
                SynergyText,
            ));

            spawn_spec_choice_panel(parent, assets);
            spawn_action_buttons(parent, assets);
        });
}

fn spawn_stats_section(parent: &mut ChildBuilder, assets: &GameAssets) {
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
}

fn spawn_upgrade_preview(parent: &mut ChildBuilder, assets: &GameAssets) {
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
                            font_size: 10.0,
                            color: Color::srgba(0.6, 0.95, 0.6, 1.0),
                        },
                    ),
                ]),
                TowerUpgradePreview,
            ));
        });
}

fn spawn_spec_choice_panel(parent: &mut ChildBuilder, assets: &GameAssets) {
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
}

fn spawn_action_buttons(parent: &mut ChildBuilder, assets: &GameAssets) {
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
                background_color: GameColors::BUTTON_TARGETING.into(),
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
                background_color: GameColors::PANEL_BG.into(),
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
                            color: GameColors::TEXT_BRIGHT,
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
                    font_size: 10.0,
                    color: GameColors::TEXT_SECONDARY,
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
                        font_size: 10.0,
                        color: GameColors::TEXT_SECONDARY,
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

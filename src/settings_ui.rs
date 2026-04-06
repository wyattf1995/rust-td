use bevy::prelude::*;

use crate::loading::GameAssets;
use crate::persistence::{
    GameSettings, HighScores, SettingsOpen,
    save_settings, save_highscores,
};
use crate::graphics::shapes::GameColors;
use crate::game::ui::button_interaction;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            toggle_settings_overlay,
            settings_button_system,
            settings_close_button,
            reset_highscores_button,
            reset_confirm_buttons,
        ));
    }
}

#[derive(Component)]
struct SettingsOverlay;

#[derive(Component)]
struct SettingsCloseButton;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingField {
    ScreenShake,
    ParticleDensity,
    DamageNumbers,
    RangeOnHover,
    ReduceMotion,
}

#[derive(Component)]
struct SettingsToggle(SettingField);

#[derive(Component)]
struct SettingsValueText(SettingField);

#[derive(Component)]
struct ResetHighScoresButton;

#[derive(Component)]
struct ResetConfirmPanel;

#[derive(Component)]
struct ResetConfirmYes;

#[derive(Component)]
struct ResetConfirmNo;

const PANEL_BG: Color = Color::srgba(0.06, 0.06, 0.1, 0.95);
const TOGGLE_ON: Color = Color::srgb(0.0, 0.75, 0.85);
const TOGGLE_OFF: Color = Color::srgb(0.35, 0.35, 0.4);

fn toggle_settings_overlay(
    mut commands: Commands,
    settings_open: Res<SettingsOpen>,
    existing: Query<Entity, With<SettingsOverlay>>,
    assets: Res<GameAssets>,
    settings: Res<GameSettings>,
) {
    if !settings_open.is_changed() {
        return;
    }

    if settings_open.0 {
        // Don't double-spawn
        if !existing.is_empty() { return; }
        spawn_settings_overlay(&mut commands, &assets, &settings);
    } else {
        for entity in &existing {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_settings_overlay(commands: &mut Commands, assets: &GameAssets, settings: &GameSettings) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.5).into(),
            z_index: ZIndex::Global(20),
            ..default()
        },
        SettingsOverlay,
    )).with_children(|parent| {
        // Panel
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(90.0),
                max_width: Val::Px(340.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            background_color: PANEL_BG.into(),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            // Header row
            panel.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                row.spawn(TextBundle::from_section(
                    "SETTINGS",
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 22.0,
                        color: GameColors::PRIMARY,
                    },
                ));
                // Close button
                row.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(40.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_GHOST.into(),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    SettingsCloseButton,
                )).with_children(|btn| {
                    btn.spawn(TextBundle::from_section("X", TextStyle {
                        font: assets.font.clone(),
                        font_size: 16.0,
                        color: Color::WHITE,
                    }));
                });
            });

            // Setting rows
            spawn_toggle_row(panel, assets, "Screen Shake", SettingField::ScreenShake,
                if settings.screen_shake { "ON" } else { "OFF" });
            spawn_cycle_row(panel, assets, "Particles", SettingField::ParticleDensity,
                settings.particle_density.name());
            spawn_toggle_row(panel, assets, "Damage Numbers", SettingField::DamageNumbers,
                if settings.show_damage_numbers { "ON" } else { "OFF" });
            spawn_toggle_row(panel, assets, "Range on Hover", SettingField::RangeOnHover,
                if settings.show_range_on_hover { "ON" } else { "OFF" });
            spawn_toggle_row(panel, assets, "Reduce Motion", SettingField::ReduceMotion,
                if settings.reduce_motion { "ON" } else { "OFF" });

            // Reset high scores button
            panel.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    background_color: GameColors::BUTTON_SELL.into(),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                ResetHighScoresButton,
            )).with_children(|btn| {
                btn.spawn(TextBundle::from_section("RESET HIGH SCORES", TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: Color::WHITE,
                }));
            });
        });
    });
}

fn spawn_toggle_row(parent: &mut ChildBuilder, assets: &GameAssets, label: &str, field: SettingField, value: &str) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Px(36.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(8.0)),
            ..default()
        },
        background_color: GameColors::ROW_BG.into(),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }).with_children(|row| {
        row.spawn(TextBundle::from_section(label, TextStyle {
            font: assets.font.clone(),
            font_size: 13.0,
            color: Color::WHITE,
        }));
        let is_on = value == "ON";
        row.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(50.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: if is_on { TOGGLE_ON } else { TOGGLE_OFF }.into(),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            SettingsToggle(field),
        )).with_children(|btn| {
            btn.spawn((
                TextBundle::from_section(value, TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
                    color: Color::WHITE,
                }),
                SettingsValueText(field),
            ));
        });
    });
}

fn spawn_cycle_row(parent: &mut ChildBuilder, assets: &GameAssets, label: &str, field: SettingField, value: &str) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Px(36.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(8.0)),
            ..default()
        },
        background_color: GameColors::ROW_BG.into(),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }).with_children(|row| {
        row.spawn(TextBundle::from_section(label, TextStyle {
            font: assets.font.clone(),
            font_size: 13.0,
            color: Color::WHITE,
        }));
        row.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(50.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: TOGGLE_ON.into(),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            SettingsToggle(field),
        )).with_children(|btn| {
            btn.spawn((
                TextBundle::from_section(value, TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
                    color: Color::WHITE,
                }),
                SettingsValueText(field),
            ));
        });
    });
}

fn settings_button_system(
    mut toggle_query: Query<
        (&Interaction, &SettingsToggle, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut settings: ResMut<GameSettings>,
    mut text_query: Query<(&mut Text, &SettingsValueText)>,
) {
    let mut changed = false;

    for (interaction, toggle, mut color) in &mut toggle_query {
        match *interaction {
            Interaction::Pressed => {
                changed = true;
                match toggle.0 {
                    SettingField::ScreenShake => {
                        settings.screen_shake = !settings.screen_shake;
                    }
                    SettingField::ParticleDensity => {
                        settings.particle_density = settings.particle_density.next();
                    }
                    SettingField::DamageNumbers => {
                        settings.show_damage_numbers = !settings.show_damage_numbers;
                    }
                    SettingField::RangeOnHover => {
                        settings.show_range_on_hover = !settings.show_range_on_hover;
                    }
                    SettingField::ReduceMotion => {
                        settings.reduce_motion = !settings.reduce_motion;
                    }
                }
            }
            Interaction::Hovered => {
                // Brighten current on/off color for hover feedback
                let is_on = match toggle.0 {
                    SettingField::ScreenShake => settings.screen_shake,
                    SettingField::ParticleDensity => true,
                    SettingField::DamageNumbers => settings.show_damage_numbers,
                    SettingField::RangeOnHover => settings.show_range_on_hover,
                    SettingField::ReduceMotion => settings.reduce_motion,
                };
                let base = if is_on { TOGGLE_ON } else { TOGGLE_OFF };
                let srgba = base.to_srgba();
                *color = Color::srgb(
                    (srgba.red + 0.15).min(1.0),
                    (srgba.green + 0.15).min(1.0),
                    (srgba.blue + 0.15).min(1.0),
                ).into();
            }
            Interaction::None => {
                // Restore correct on/off color
                let is_on = match toggle.0 {
                    SettingField::ScreenShake => settings.screen_shake,
                    SettingField::ParticleDensity => true,
                    SettingField::DamageNumbers => settings.show_damage_numbers,
                    SettingField::RangeOnHover => settings.show_range_on_hover,
                    SettingField::ReduceMotion => settings.reduce_motion,
                };
                *color = if is_on { TOGGLE_ON } else { TOGGLE_OFF }.into();
            }
        }
    }

    if changed {
        save_settings(&settings);

        // Update all value texts and button colors
        for (mut text, value_text) in &mut text_query {
            let (label, _is_on) = match value_text.0 {
                SettingField::ScreenShake => (
                    if settings.screen_shake { "ON" } else { "OFF" },
                    settings.screen_shake,
                ),
                SettingField::ParticleDensity => (
                    settings.particle_density.name(),
                    true, // always "on" color for cycle
                ),
                SettingField::DamageNumbers => (
                    if settings.show_damage_numbers { "ON" } else { "OFF" },
                    settings.show_damage_numbers,
                ),
                SettingField::RangeOnHover => (
                    if settings.show_range_on_hover { "ON" } else { "OFF" },
                    settings.show_range_on_hover,
                ),
                SettingField::ReduceMotion => (
                    if settings.reduce_motion { "ON" } else { "OFF" },
                    settings.reduce_motion,
                ),
            };
            text.sections[0].value = label.to_string();
        }

        // Update toggle button colors
        for (_, toggle, mut color) in &mut toggle_query {
            let is_on = match toggle.0 {
                SettingField::ScreenShake => settings.screen_shake,
                SettingField::ParticleDensity => true,
                SettingField::DamageNumbers => settings.show_damage_numbers,
                SettingField::RangeOnHover => settings.show_range_on_hover,
                SettingField::ReduceMotion => settings.reduce_motion,
            };
            *color = if is_on { TOGGLE_ON } else { TOGGLE_OFF }.into();
        }
    }
}

fn settings_close_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<SettingsCloseButton>)>,
    mut settings_open: ResMut<SettingsOpen>,
) {
    for (interaction, mut color) in &mut query {
        if button_interaction(interaction, &mut color, GameColors::BUTTON_GHOST, GameColors::BUTTON_GHOST_HOVER) {
            settings_open.0 = false;
        }
    }
}

fn reset_highscores_button(
    mut commands: Commands,
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ResetHighScoresButton>)>,
    existing_confirm: Query<Entity, With<ResetConfirmPanel>>,
    assets: Res<GameAssets>,
) {
    for (interaction, mut color) in &mut query {
        if !button_interaction(interaction, &mut color, GameColors::BUTTON_SELL, GameColors::BUTTON_SELL_HOVER) {
            continue;
        }
            // Don't double-spawn
            if !existing_confirm.is_empty() { return; }

            // Spawn confirmation dialog
            commands.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: GameColors::PANEL_BG.into(),
                    z_index: ZIndex::Global(30),
                    ..default()
                },
                ResetConfirmPanel,
            )).with_children(|parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(260.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(16.0)),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    background_color: PANEL_BG.into(),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                }).with_children(|panel| {
                    panel.spawn(TextBundle::from_section("Reset all high scores? This cannot be undone.", TextStyle {
                        font: assets.font.clone(),
                        font_size: 16.0,
                        color: Color::WHITE,
                    }));

                    panel.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        ..default()
                    }).with_children(|row| {
                        row.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(80.0),
                                    height: Val::Px(32.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: Color::srgb(0.7, 0.2, 0.2).into(),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            ResetConfirmYes,
                        )).with_children(|btn| {
                            btn.spawn(TextBundle::from_section("Reset Scores", TextStyle {
                                font: assets.font.clone(),
                                font_size: 14.0,
                                color: Color::WHITE,
                            }));
                        });

                        row.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(80.0),
                                    height: Val::Px(32.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: Color::srgb(0.3, 0.3, 0.35).into(),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            ResetConfirmNo,
                        )).with_children(|btn| {
                            btn.spawn(TextBundle::from_section("Keep Scores", TextStyle {
                                font: assets.font.clone(),
                                font_size: 14.0,
                                color: Color::WHITE,
                            }));
                        });
                    });
                });
            });
    }
}

fn reset_confirm_buttons(
    mut commands: Commands,
    mut yes_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ResetConfirmYes>)>,
    mut no_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ResetConfirmNo>, Without<ResetConfirmYes>)>,
    confirm_panel: Query<Entity, With<ResetConfirmPanel>>,
    mut high_scores: ResMut<HighScores>,
) {
    let mut dismiss = false;

    for (interaction, mut color) in &mut yes_query {
        if button_interaction(interaction, &mut color, Color::srgb(0.7, 0.2, 0.2), Color::srgb(0.85, 0.3, 0.3)) {
            high_scores.scores.clear();
            save_highscores(&high_scores);
            dismiss = true;
        }
    }

    for (interaction, mut color) in &mut no_query {
        if button_interaction(interaction, &mut color, Color::srgb(0.3, 0.3, 0.35), Color::srgb(0.4, 0.4, 0.45)) {
            dismiss = true;
        }
    }

    if dismiss {
        for entity in &confirm_panel {
            commands.entity(entity).despawn_recursive();
        }
    }
}

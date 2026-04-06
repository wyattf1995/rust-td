use bevy::prelude::*;

use crate::loading::GameAssets;
use crate::graphics::shapes::GameColors;
use crate::game::tower::{SynergyType, TowerType};

/// Resource to toggle synergy reference panel visibility
#[derive(Resource, Default)]
pub struct SynergyPanelOpen(pub bool);

pub struct SynergyUiPlugin;

impl Plugin for SynergyUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SynergyPanelOpen>()
            .add_systems(Update, (
                toggle_synergy_panel,
                synergy_close_button,
            ));
    }
}

#[derive(Component)]
struct SynergyOverlay;

#[derive(Component)]
struct SynergyCloseButton;

const PANEL_BG: Color = Color::srgba(0.06, 0.06, 0.1, 0.95);
fn toggle_synergy_panel(
    mut commands: Commands,
    synergy_open: Res<SynergyPanelOpen>,
    existing: Query<Entity, With<SynergyOverlay>>,
    assets: Res<GameAssets>,
) {
    if !synergy_open.is_changed() {
        return;
    }

    if synergy_open.0 {
        if !existing.is_empty() { return; }
        spawn_synergy_panel(&mut commands, &assets);
    } else {
        for entity in &existing {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_synergy_panel(commands: &mut Commands, assets: &GameAssets) {
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
            z_index: ZIndex::Global(21),
            ..default()
        },
        SynergyOverlay,
    )).with_children(|parent| {
        // Panel container
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(90.0),
                max_width: Val::Px(380.0),
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
                    "TOWER SYNERGIES",
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
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: GameColors::BUTTON_GHOST.into(),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    SynergyCloseButton,
                )).with_children(|btn| {
                    btn.spawn(TextBundle::from_section(
                        "X",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 16.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                        },
                    ));
                });
            });

            // Synergy rows
            for synergy in SynergyType::ALL {
                let (tower_a, tower_b) = synergy.towers_required();
                spawn_synergy_row(panel, assets, synergy, tower_a, tower_b);
            }

            // Footer explanation
            panel.spawn(TextBundle::from_section(
                "Place towers adjacent (including diagonals) to activate",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 10.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.45),
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            }));
        });
    });
}

fn spawn_synergy_row(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    synergy: SynergyType,
    tower_a: TowerType,
    tower_b: TowerType,
) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(10.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        background_color: GameColors::ROW_BG.into(),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }).with_children(|row| {
        // Top line: synergy name
        row.spawn(TextBundle::from_section(
            format!("  {}  {}", synergy.name(), synergy.description()),
            TextStyle {
                font: assets.font.clone(),
                font_size: 14.0,
                color: GameColors::SYNERGY,
            },
        ));
        // Bottom line: tower requirement with colored indicators
        row.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        }).with_children(|req| {
            // Tower A color swatch
            req.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                background_color: tower_a.color().into(),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            });
            req.spawn(TextBundle::from_section(
                tower_a.name(),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                },
            ));
            req.spawn(TextBundle::from_section(
                " + ",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                },
            ));
            // Tower B color swatch
            req.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                background_color: tower_b.color().into(),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            });
            req.spawn(TextBundle::from_section(
                tower_b.name(),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                },
            ));
        });
    });
}

fn synergy_close_button(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<SynergyCloseButton>)>,
    mut synergy_open: ResMut<SynergyPanelOpen>,
) {
    for (interaction, mut color) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                synergy_open.0 = false;
            }
            Interaction::Hovered => {
                *color = GameColors::BUTTON_GHOST_HOVER.into();
            }
            Interaction::None => {
                *color = GameColors::BUTTON_GHOST.into();
            }
        }
    }
}

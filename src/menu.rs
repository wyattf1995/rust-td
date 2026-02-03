use bevy::prelude::*;

use crate::{loading::GameAssets, GameState};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(Update, (button_system, button_interaction).run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), cleanup_menu);
    }
}

#[derive(Component)]
struct MenuScreen;

#[derive(Component)]
struct PlayButton;

const BUTTON_NORMAL: Color = Color::srgb(0.91, 0.27, 0.38);
const BUTTON_HOVER: Color = Color::srgb(1.0, 0.37, 0.48);
const BUTTON_PRESSED: Color = Color::srgb(0.71, 0.17, 0.28);

fn setup_menu(mut commands: Commands, assets: Res<GameAssets>) {
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
                background_color: Color::srgb(0.1, 0.1, 0.18).into(),
                ..default()
            },
            MenuScreen,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "TOWER DEFENSE",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 72.0,
                    color: Color::srgb(0.91, 0.27, 0.38),
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            }));

            // Subtitle
            parent.spawn(TextBundle::from_section(
                "Built with Rust + Bevy",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 24.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(60.0)),
                ..default()
            }));

            // Play button
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
                        background_color: BUTTON_NORMAL.into(),
                        ..default()
                    },
                    PlayButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "PLAY",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 28.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // Instructions
            parent.spawn(TextBundle::from_section(
                "Click to place towers • Defend against waves of enemies",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 16.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(40.0)),
                ..default()
            }));
        });
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BUTTON_PRESSED.into();
            }
            Interaction::Hovered => {
                *color = BUTTON_HOVER.into();
            }
            Interaction::None => {
                *color = BUTTON_NORMAL.into();
            }
        }
    }
}

fn button_interaction(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Playing);
        }
    }
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

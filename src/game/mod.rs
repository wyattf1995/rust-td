use bevy::prelude::*;

pub mod economy;
pub mod enemy;
pub mod map;
pub mod pool;
pub mod projectile;
pub mod spatial;
pub mod tower;
pub mod ui;

use crate::GameState;
use crate::graphics::shapes::GameColors;

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
            pool::PoolPlugin,
        ))
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
            (restart_button_system, restart_interaction)
                .run_if(in_state(GameState::GameOver).or_else(in_state(GameState::Victory))),
        );
    }
}

#[derive(Component)]
pub struct GameEntity;

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
}

fn setup_game_over(mut commands: Commands, assets: Res<crate::loading::GameAssets>) {
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
            parent.spawn(TextBundle::from_section(
                "GAME OVER",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 72.0,
                    color: GameColors::PRIMARY,
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

fn restart_interaction(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Menu);
        }
    }
}

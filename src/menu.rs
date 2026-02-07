use bevy::prelude::*;
use rand::Rng;

use crate::{analytics::{Analytics, track_with_context}, loading::GameAssets, GameState};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(
                Update,
                (
                    button_system,
                    button_interaction,
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

fn setup_menu(mut commands: Commands, assets: Res<GameAssets>) {
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
        (Changed<Interaction>, With<Button>),
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

fn button_interaction(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    analytics: Res<Analytics>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            track_with_context(&analytics, "game_started", &[]);
            next_state.set(GameState::Playing);
        }
    }
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::GameColors;

/// Challenge parameters parsed from URL query string (?c=mapname,wave,score).
/// Populated once at startup; empty if no challenge URL was provided.
#[derive(Resource, Default)]
pub struct ChallengeParams {
    pub map_name: Option<String>,
    pub wave: Option<usize>,
    pub score: Option<u32>,
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .init_resource::<ChallengeParams>()
            .add_systems(Startup, read_challenge_params)
            .add_systems(OnEnter(GameState::Loading), setup_loading)
            .add_systems(Update, check_loading.run_if(in_state(GameState::Loading)))
            .add_systems(OnExit(GameState::Loading), cleanup_loading);
    }
}

/// Read challenge parameters from the URL query string once at startup.
fn read_challenge_params(mut params: ResMut<ChallengeParams>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(location) = window.location().search() {
                // Parse ?c=mapname,wave,score
                if let Some(c_param) = location.strip_prefix("?c=") {
                    let parts: Vec<&str> = c_param.split(',').collect();
                    if parts.len() >= 2 {
                        params.map_name = Some(parts[0].to_string());
                        params.wave = parts.get(1).and_then(|s| s.parse().ok());
                        params.score = parts.get(2).and_then(|s| s.parse().ok());
                    }
                }
            }
        }
    }
    // Suppress unused variable warning on non-WASM targets
    let _ = &mut *params;
}

/// Holds all loaded game assets
#[derive(Resource, Default)]
pub struct GameAssets {
    pub font: Handle<Font>,
    pub loaded: bool,
}

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct LoadingBar;

fn setup_loading(mut commands: Commands, asset_server: Res<AssetServer>, mut assets: ResMut<GameAssets>) {
    // Spawn camera for UI rendering
    commands.spawn(Camera2dBundle::default());

    // Load font
    assets.font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // Spawn loading UI
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
            LoadingScreen,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "TOWER DEFENSE",
                TextStyle {
                    font_size: 48.0,
                    color: GameColors::BRAND,
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            }));

            // Loading bar background
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(300.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    background_color: GameColors::BUTTON_GHOST.into(),
                    ..default()
                })
                .with_children(|parent| {
                    // Loading bar fill
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: GameColors::BRAND.into(),
                            ..default()
                        },
                        LoadingBar,
                    ));
                });

            // Loading text
            parent.spawn(TextBundle::from_section(
                "Loading assets...",
                TextStyle {
                    font_size: 16.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            }));
        });
}

fn check_loading(
    asset_server: Res<AssetServer>,
    mut assets: ResMut<GameAssets>,
    mut loading_bar: Query<&mut Style, With<LoadingBar>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    use bevy::asset::LoadState;

    // Use regular LoadState which is more reliable for fonts
    let font_state = asset_server.get_load_state(&assets.font);

    let (progress, ready) = match font_state {
        Some(LoadState::Loaded) => (1.0, true),
        Some(LoadState::Loading) => (0.5, false),
        Some(LoadState::Failed(_)) => {
            // Font failed to load - proceed anyway with default font
            bevy::log::warn!("Font failed to load, proceeding with default");
            (1.0, true)
        }
        _ => (0.0, false),
    };

    // Update loading bar
    if let Ok(mut bar) = loading_bar.get_single_mut() {
        bar.width = Val::Percent(progress * 100.0);
    }

    // Check if ready to proceed
    if ready {
        assets.loaded = true;

        // Hide HTML loading screen
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;

            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = window)]
                fn hideLoadingScreen();
            }

            hideLoadingScreen();
        }

        next_state.set(GameState::Menu);
    }
}

fn cleanup_loading(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

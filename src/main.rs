use bevy::prelude::*;

mod game;
mod graphics;
mod loading;
mod menu;

/// Game states
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Playing,
    Paused,
    GameOver,
    Victory,
}

/// Game speed multiplier
#[derive(Resource)]
pub struct GameSpeed(pub f32);

impl Default for GameSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

fn main() {
    // Set up panic hook for better error messages in browser
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Rust Tower Defense".into(),
                        resolution: (1280.0, 720.0).into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.18)))
        .init_state::<GameState>()
        .init_resource::<GameSpeed>()
        .add_plugins((
            loading::LoadingPlugin,
            menu::MenuPlugin,
            game::GamePlugin,
            graphics::GraphicsPlugin,
        ))
        .run();
}

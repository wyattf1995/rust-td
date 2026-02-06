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

/// UI scale factor for responsive design (mobile vs desktop)
#[derive(Resource)]
pub struct UiScale {
    pub factor: f32,
    pub is_mobile: bool,
}

impl Default for UiScale {
    fn default() -> Self {
        Self {
            factor: 1.0,
            is_mobile: false,
        }
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
                        title: "Neon Command".into(),
                        resolution: (1280.0, 720.0).into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true, // Better for mobile
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.18)))
        .init_state::<GameState>()
        .init_resource::<GameSpeed>()
        .init_resource::<UiScale>()
        .add_plugins((
            loading::LoadingPlugin,
            menu::MenuPlugin,
            game::GamePlugin,
            graphics::GraphicsPlugin,
        ))
        .add_systems(Update, detect_screen_size)
        .run();
}

/// Detect screen size and update UI scale for mobile
fn detect_screen_size(
    windows: Query<&Window>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    let width = window.width();
    let height = window.height();

    // Consider mobile if width < 800 or if in portrait mode
    let is_mobile = width < 800.0 || (height > width);

    // Calculate scale factor based on screen width
    // Base design is 1280px wide
    let base_width = 1280.0;
    let scale = if is_mobile {
        // On mobile, scale up UI elements for touch
        (width / base_width).max(0.6) * 1.3
    } else {
        (width / base_width).max(0.7)
    };

    if (ui_scale.factor - scale).abs() > 0.01 || ui_scale.is_mobile != is_mobile {
        ui_scale.factor = scale;
        ui_scale.is_mobile = is_mobile;
    }
}

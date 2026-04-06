#![deny(clippy::unwrap_used)]

use bevy::prelude::*;
#[cfg(debug_assertions)]
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

mod analytics;
mod game;
mod graphics;
mod loading;
mod menu;
mod persistence;
mod settings_ui;
mod synergy_ui;
mod tips;

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

/// Screen info for responsive design (mobile vs desktop)
#[derive(Resource)]
pub struct ScreenInfo {
    pub factor: f32,
    pub is_mobile: bool,
    pub is_landscape: bool,
}

impl Default for ScreenInfo {
    fn default() -> Self {
        Self {
            factor: 1.0,
            is_mobile: false,
            is_landscape: false,
        }
    }
}

fn main() {
    // Set up panic hook for better error messages in browser
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    app.add_plugins(
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
        .init_resource::<ScreenInfo>()
        .add_plugins((
            analytics::AnalyticsPlugin,
            persistence::PersistencePlugin,
            loading::LoadingPlugin,
            menu::MenuPlugin,
            game::GamePlugin,
            graphics::GraphicsPlugin,
            settings_ui::SettingsPlugin,
            synergy_ui::SynergyUiPlugin,
            tips::TipsPlugin,
        ))
        .add_systems(Update, (detect_screen_size, update_camera_projection));

    #[cfg(debug_assertions)]
    app.add_plugins((FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin::default()));

    app.run();
}

/// Detect screen size and update UI scale for responsive layout
fn detect_screen_size(
    windows: Query<&Window>,
    mut screen_info: ResMut<ScreenInfo>,
    mut bevy_ui_scale: ResMut<bevy::ui::UiScale>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    let width = window.width();
    let height = window.height();

    const MOBILE_WIDTH: f32 = 800.0;
    const MOBILE_HEIGHT: f32 = 500.0;
    const BASE_DESIGN_WIDTH: f32 = 1280.0;

    let is_mobile = width < MOBILE_WIDTH || height < MOBILE_HEIGHT || (height > width);
    let is_landscape = is_mobile && width > height;

    let scale = if is_mobile {
        ((width / BASE_DESIGN_WIDTH) * 1.3).clamp(0.5, 1.3)
    } else {
        (width / BASE_DESIGN_WIDTH).clamp(0.6, 1.5)
    };

    if (screen_info.factor - scale).abs() > 0.01
        || screen_info.is_mobile != is_mobile
        || screen_info.is_landscape != is_landscape
    {
        screen_info.factor = scale;
        screen_info.is_mobile = is_mobile;
        screen_info.is_landscape = is_landscape;
        // Bevy's built-in UiScale globally multiplies all Val::Px values
        bevy_ui_scale.0 = scale;
    }
}

/// Scale camera projection so the full grid + UI margins always fit in the viewport
fn update_camera_projection(
    windows: Query<&Window>,
    mut camera_q: Query<&mut OrthographicProjection, With<Camera2d>>,
    screen_info: Res<ScreenInfo>,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok(mut projection) = camera_q.get_single_mut() else { return };

    let window_w = window.width();
    let window_h = window.height();
    if window_w <= 0.0 || window_h <= 0.0 { return; }

    const CAMERA_TARGET_W: f32 = 940.0;  // Grid (900) + small margin
    const CAMERA_TARGET_H: f32 = 750.0;  // Grid (550) + HUD + bottom bar
    const CAMERA_TARGET_H_LANDSCAPE: f32 = 650.0;  // Smaller for landscape mobile

    let target_w = CAMERA_TARGET_W;
    let target_h = if screen_info.is_landscape { CAMERA_TARGET_H_LANDSCAPE } else { CAMERA_TARGET_H };

    // Scale to whichever dimension is tighter
    let scale_x = target_w / window_w;
    let scale_y = target_h / window_h;
    projection.scale = scale_x.max(scale_y).max(1.0);
}

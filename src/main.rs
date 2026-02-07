use bevy::prelude::*;

mod analytics;
mod game;
mod graphics;
mod loading;
mod menu;
mod persistence;
mod settings_ui;

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
        .init_resource::<ScreenInfo>()
        .add_plugins((
            analytics::AnalyticsPlugin,
            persistence::PersistencePlugin,
            loading::LoadingPlugin,
            menu::MenuPlugin,
            game::GamePlugin,
            graphics::GraphicsPlugin,
            settings_ui::SettingsPlugin,
        ))
        .add_systems(Update, (detect_screen_size, update_camera_projection))
        .run();
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

    // Consider mobile if either dimension is small (catches landscape phones)
    let is_mobile = width < 800.0 || height < 500.0 || (height > width);
    let is_landscape = is_mobile && width > height;

    // Calculate scale factor based on screen width
    // Base design is 1280px wide
    let base_width = 1280.0;
    let scale = if is_mobile {
        // On mobile, scale up UI elements for touch
        ((width / base_width) * 1.3).clamp(0.5, 1.3)
    } else {
        (width / base_width).clamp(0.6, 1.5)
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

    // Game grid: 18*50=900 wide, 11*50=550 tall
    // Plus UI margins: ~50px top HUD, ~122px bottom bar
    let target_w = 940.0;  // Grid width + small margin
    let target_h = if screen_info.is_landscape {
        // Landscape mobile: UI bars are smaller, need less vertical margin
        650.0
    } else {
        750.0  // Default: includes full HUD + bottom bar margins
    };

    // Scale to whichever dimension is tighter
    let scale_x = target_w / window_w;
    let scale_y = target_h / window_h;
    projection.scale = scale_x.max(scale_y).max(1.0);
}

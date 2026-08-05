//! Lightweight analytics module for Neon Command
//! Privacy-first: anonymous sessions, no PII, no cookies

use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Analytics resource holding build info
#[derive(Resource)]
pub struct Analytics {
    pub game_version: &'static str,
}

impl Default for Analytics {
    fn default() -> Self {
        Self {
            game_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

// JavaScript interop for WASM builds — `catch` attribute converts JS exceptions
// to Result instead of panicking, so a missing window.neonAnalytics won't crash the game.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "neonAnalytics"], js_name = trackEvent)]
    fn js_track_event(event_name: &str, properties_json: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, js_namespace = ["window", "neonAnalytics"], js_name = isEnabled)]
    fn js_analytics_enabled() -> Result<bool, JsValue>;
}

/// Track an analytics event
#[allow(unused_variables)]
pub fn track_event(event: &str, properties: &[(&str, &str)]) {
    #[cfg(target_arch = "wasm32")]
    {
        // Check if analytics is available and enabled (silently skip if JS object missing)
        if js_analytics_enabled().unwrap_or(false) {
            // Build JSON manually to avoid serde dependency
            let mut json = String::from("{");
            for (i, (key, value)) in properties.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str(&format!("\"{}\":\"{}\"", key, value));
            }
            json.push('}');

            let _ = js_track_event(event, &json);
        }
    }

    // In non-WASM builds, just log to console for debugging
    #[cfg(not(target_arch = "wasm32"))]
    {
        bevy::log::info!("[Analytics] {} {:?}", event, properties);
    }
}

/// Track event with the analytics resource context
pub fn track_with_context(analytics: &Analytics, event: &str, extra_props: &[(&str, &str)]) {
    // Session identity comes from Umami itself, so only the build version is worth
    // attaching to every event.
    let mut all_props: Vec<(&str, &str)> = vec![("game_version", analytics.game_version)];
    all_props.extend_from_slice(extra_props);

    track_event(event, &all_props);
}

/// Plugin to initialize analytics
pub struct AnalyticsPlugin;

impl Plugin for AnalyticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Analytics>()
            .add_systems(Startup, track_session_start);
    }
}

/// Track session start on app initialization
fn track_session_start(analytics: Res<Analytics>) {
    track_with_context(&analytics, "session_started", &[]);
}

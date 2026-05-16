use bevy::prelude::*;

use crate::{loading::GameAssets, GameState, GameSpeed, ScreenInfo};
use crate::graphics::shapes::GameColors;
use crate::persistence::SettingsOpen;
use crate::synergy_ui::SynergyPanelOpen;

use super::{
    abilities::{PlayerAbilities, FreezeAbilityEvent, GoldRushAbilityEvent},
    economy::{PlayerEconomy, KillStreak},
    enemy::{EnemyType, WaveManager, WaveModifier},
    map::{GameMap, HoveredTile, TileType},
    tower::{PlaceTowerEvent, RecentPlacement, SelectedTowerType, SelectedPlacedTower, SellTowerEvent, UpgradeTowerEvent, SpecializeTowerEvent, Specialization, Tower, TowerType, TowerSynergies},
    GameEntity,
};

pub mod setup;
pub mod hud;
pub mod tower_panel;
pub mod abilities;
pub mod wave;
pub mod pause;

pub struct GameUiPlugin;

/// Resource to track if UI was clicked this frame (prevents click-through to game)
#[derive(Resource, Default)]
pub struct UiClicked(pub bool);

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiClicked>()
            .init_resource::<UiZones>()
            .init_resource::<LifeLostFlash>()
            .init_resource::<SellPending>()
            .init_resource::<PlacementRejectFlash>()
            .init_resource::<GoldRejectFlash>()
            .add_systems(OnEnter(GameState::Playing), setup::setup_ui)
            .add_systems(OnEnter(GameState::Paused), pause::setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), pause::cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    // Phase 1: Reset UI click flag + update zone sizes
                    (reset_ui_clicked, update_ui_zones),
                    // Phase 2: UI button interactions (set flag if clicked)
                    (
                        tower_panel::tower_button_system,
                        wave::start_wave_button,
                        tower_panel::tower_context_shortcuts,
                        tower_panel::update_tower_context_menu,
                        tower_panel::update_spec_panel,
                        tower_panel::tower_context_buttons,
                        tower_panel::spec_button_system,
                        hud::speed_button_system,
                        abilities::ability_button_activate,
                    ),
                    // Phase 3: Game world clicks (check flag before placing)
                    handle_tile_click,
                    // Phase 4: Display updates (order doesn't matter)
                    (
                        hud::update_gold_display,
                        hud::update_gold_reject_flash,
                        hud::update_gold_rush_indicator,
                        hud::update_lives_display,
                        hud::update_life_flash,
                        hud::update_wave_display,
                        hud::update_score_display,
                        hud::update_combo_display,
                        abilities::update_ability_display,
                        abilities::update_ability_tooltips,
                        abilities::adapt_ability_bar_layout,
                        tower_panel::update_tower_selection,
                        tower_panel::update_tower_affordability,
                        wave::update_wave_announcement,
                        wave::update_start_button_text,
                        tower_panel::update_info_panel,
                        wave::update_wave_preview,
                        hud::update_undo_hint,
                        pause::pause_input,
                        pause::pause_on_blur,
                    ),
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (pause::pause_menu_buttons, pause::pause_settings_button, pause::pause_synergy_button, pause::pause_input_resume)
                    .run_if(in_state(GameState::Paused)),
            );
    }
}

#[derive(Component)]
pub(super) struct GoldText;

#[derive(Component)]
pub(super) struct LivesText;

#[derive(Component)]
pub(super) struct WaveText;

#[derive(Component)]
pub(super) struct TowerButton(pub(super) TowerType);

#[derive(Component)]
pub(super) struct TowerCostText;

#[derive(Component)]
pub(super) struct HudPauseButton;

#[derive(Component)]
pub(super) struct HudSynergyButton;

#[derive(Component)]
pub(super) struct PauseSynergyButton;

#[derive(Component)]
pub(super) struct StartWaveButton;

#[derive(Component)]
pub(super) struct StartWaveButtonText;

#[derive(Component)]
pub(super) struct TowerButtonBorder(pub(super) TowerType);

#[derive(Component)]
pub(super) struct InfoPanel;

#[derive(Component)]
pub(super) struct InfoPanelName;

#[derive(Component)]
pub(super) struct InfoPanelDesc;

#[derive(Component)]
pub(super) struct InfoPanelStats;

#[derive(Component)]
pub(super) struct InfoPanelCost;

#[derive(Component)]
pub(super) struct ScoreText;

#[derive(Component)]
pub(super) struct GoldRushIndicator;

#[derive(Component)]
pub(super) struct SpeedButton(pub(super) f32);

#[derive(Component)]
pub(super) struct PauseMenu;

#[derive(Component)]
pub(super) struct ResumeButton;

#[derive(Component)]
pub(super) struct QuitButton;

#[derive(Component)]
pub(super) struct PauseSettingsButton;

#[derive(Component)]
pub(super) struct WavePreviewPanel;

#[derive(Component)]
pub(super) struct WavePreviewText;

#[derive(Component)]
pub(super) struct TowerDetailPanel;

#[derive(Component)]
pub(super) struct SellButton;

#[derive(Component)]
pub(super) struct UpgradeButton;

#[derive(Component)]
pub(super) struct UpgradeCostText;

#[derive(Component)]
pub(super) struct SellValueText;


#[derive(Component)]
pub(super) struct TowerStatsText;

#[derive(Component)]
pub(super) struct TowerUpgradePreview;

#[derive(Component)]
pub(super) struct ComboDisplay;

#[derive(Component)]
pub(super) struct ComboText;

#[derive(Component)]
pub(super) struct TargetingButton;

#[derive(Component)]
pub(super) struct TargetingText;

#[derive(Component)]
pub(super) struct SynergyText;

#[derive(Component)]
pub(super) struct SpecChoicePanel;

#[derive(Component)]
pub(super) struct SpecButton(pub(super) Specialization);

#[derive(Component)]
pub(super) struct SpecButtonText;

#[derive(Component)]
pub struct TopHudBar;

#[derive(Component)]
pub struct BottomTowerBar;

/// Dynamic UI zone boundaries (actual rendered pixel heights)
#[derive(Resource, Default)]
pub struct UiZones {
    pub top_bar_height: f32,
    pub bottom_bar_height: f32,
}

/// Wave announcement overlay
#[derive(Component)]
pub(super) struct WaveAnnouncement {
    pub(super) lifetime: Timer,
}

/// Resource to track life-lost flash effect
#[derive(Resource, Default)]
pub(super) struct LifeLostFlash {
    pub(super) timer: Option<Timer>,
    pub(super) previous_lives: u32,
}

/// Resource to flash a tile red when placement is rejected
#[derive(Resource, Default)]
pub struct PlacementRejectFlash {
    pub timer: Option<Timer>,
    pub grid_pos: Option<(usize, usize)>,
}

/// Resource to flash gold text red when can't afford
#[derive(Resource, Default)]
pub(super) struct GoldRejectFlash {
    pub(super) timer: Option<Timer>,
}

/// Resource to track sell confirmation state (double-tap to sell)
#[derive(Resource, Default)]
pub(super) struct SellPending {
    pub(super) timer: Option<Timer>,
    pub(super) tower: Option<Entity>,
}

#[derive(Component)]
pub(super) struct UndoHintText;

#[derive(Component)]
pub(super) struct AbilityBar;

#[derive(Component)]
pub(super) struct AbilityButton(pub(super) AbilityType);

#[derive(Component)]
pub(super) struct AbilityCooldownText(pub(super) AbilityType);

#[derive(Component)]
pub(super) struct AbilityTooltip;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AbilityType {
    Freeze,
    GoldRush,
    Artillery,
}

impl AbilityType {
    pub(super) fn tooltip(&self) -> (&'static str, &'static str) {
        match self {
            AbilityType::Freeze => ("Freeze [Q]", "Freeze all enemies for 3s\nCooldown: 45s"),
            AbilityType::GoldRush => ("Gold Rush [W]", "2x gold from kills for 10s\nCooldown: 60s"),
            AbilityType::Artillery => ("Artillery [E]", "Click to bomb an area for 400 dmg\nCooldown: 90s"),
        }
    }
}

/// Returns (ready, active, remaining_secs) for a given ability type.
pub(super) fn ability_state(abilities: &PlayerAbilities, ability: AbilityType) -> (bool, bool, f32) {
    match ability {
        AbilityType::Freeze => (
            abilities.freeze_ready,
            false,
            abilities.freeze_cooldown.remaining_secs(),
        ),
        AbilityType::GoldRush => (
            abilities.gold_rush_ready,
            abilities.gold_rush_active.is_some(),
            abilities.gold_rush_active.as_ref()
                .map(|t| t.remaining_secs())
                .unwrap_or_else(|| abilities.gold_rush_cooldown.remaining_secs()),
        ),
        AbilityType::Artillery => (
            abilities.artillery_ready,
            abilities.artillery_targeting,
            abilities.artillery_cooldown.remaining_secs(),
        ),
    }
}

/// Reset UI clicked flag at the start of each frame
fn reset_ui_clicked(mut ui_clicked: ResMut<UiClicked>) {
    ui_clicked.0 = false;
}

/// Update UI zone boundaries from actual rendered node sizes
fn update_ui_zones(
    top_bar: Query<&ComputedNode, With<TopHudBar>>,
    bottom_bar: Query<&ComputedNode, With<BottomTowerBar>>,
    mut zones: ResMut<UiZones>,
) {
    if let Ok(node) = top_bar.get_single() {
        zones.top_bar_height = node.size().y;
    }
    if let Ok(node) = bottom_bar.get_single() {
        zones.bottom_bar_height = node.size().y;
    }
}

fn handle_tile_click(
    pointer: Res<super::PointerState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    map: Res<GameMap>,
    selected: Res<SelectedTowerType>,
    economy: Res<PlayerEconomy>,
    mut place_events: EventWriter<PlaceTowerEvent>,
    ui_clicked: Res<UiClicked>,
    ui_zones: Res<UiZones>,
    mut tile_flash: ResMut<PlacementRejectFlash>,
    mut gold_flash: ResMut<GoldRejectFlash>,
) {
    if ui_clicked.0 || !pointer.just_pressed {
        return;
    }

    let Ok(window) = windows.get_single() else { return; };
    let Ok((camera, camera_transform)) = camera_q.get_single() else { return; };
    let Some(cursor_pos) = pointer.position else { return; };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return; };
    let Some((grid_x, grid_y)) = GameMap::world_to_grid(world_pos) else { return; };

    // UI zone check FIRST — clicks in HUD area should not trigger feedback
    let window_height = window.height();
    if cursor_pos.y > window_height - ui_zones.bottom_bar_height - 10.0 || cursor_pos.y < ui_zones.top_bar_height + 10.0 {
        return;
    }

    // Buildability check — flash tile red on rejection
    if !map.is_buildable(grid_x, grid_y) {
        tile_flash.timer = Some(Timer::from_seconds(0.4, TimerMode::Once));
        tile_flash.grid_pos = Some((grid_x, grid_y));
        return;
    }

    // Affordability check — flash gold text red on rejection
    if economy.gold < selected.0.cost() {
        gold_flash.timer = Some(Timer::from_seconds(0.3, TimerMode::Once));
        return;
    }

    place_events.send(PlaceTowerEvent {
        grid_x,
        grid_y,
        tower_type: selected.0,
    });
}

/// Apply standard button hover/unhover colors. Returns true if pressed.
pub fn button_interaction(
    interaction: &Interaction,
    color: &mut BackgroundColor,
    normal: Color,
    hover: Color,
) -> bool {
    match *interaction {
        Interaction::Pressed => true,
        Interaction::Hovered => {
            *color = hover.into();
            false
        }
        Interaction::None => {
            *color = normal.into();
            false
        }
    }
}

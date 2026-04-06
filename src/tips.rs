//! Contextual first-session tips that appear at key gameplay moments.
//! Each tip shows once per player (tracked in LocalStorage via TipsShown).

use bevy::prelude::*;

use crate::game::enemy::WaveManager;
use crate::game::tower::Tower;
use crate::graphics::shapes::GameColors;
use crate::persistence::{LifetimeStats, TipsShown, save_tips};
use crate::GameState;

pub struct TipsPlugin;

impl Plugin for TipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveTip>()
            .add_systems(
                Update,
                (check_tip_triggers, update_tip_display)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_tips);
    }
}

#[derive(Resource, Default)]
struct ActiveTip {
    current: Option<TipKind>,
    timer: Option<Timer>,
}

#[derive(Clone, Copy, PartialEq)]
enum TipKind {
    Welcome,
    Economy,
    Specialization,
    Synergy,
    EarlySend,
    Targeting,
}

impl TipKind {
    fn text(self) -> &'static str {
        match self {
            TipKind::Welcome => "Survive as many waves as you can!\nPlace towers (1-8) to stop enemies from reaching the exit.\nSpend gold wisely — you earn interest on savings between waves.",
            TipKind::Economy => "You earned interest on your saved gold!\nHold gold between waves for 10% interest (max 50g).\nSend waves early for a bonus — but don't fall behind!",
            TipKind::Specialization => "Towers branch into specializations at Level 3 — choose wisely!",
            TipKind::Synergy => "Place matching towers adjacent to each other for synergy bonuses. Press ? to see all synergies.",
            TipKind::EarlySend => "Send the next wave early for a gold bonus! Press Space between waves.",
            TipKind::Targeting => "Press T on a selected tower to cycle targeting: First, Closest, Lowest HP, Highest HP, Fastest.",
        }
    }
}

#[derive(Component)]
struct TipOverlay;

fn check_tip_triggers(
    mut tips_shown: ResMut<TipsShown>,
    mut active: ResMut<ActiveTip>,
    towers: Query<&Tower>,
    wave_manager: Res<WaveManager>,
    lifetime_stats: Res<LifetimeStats>,
) {
    // Don't trigger a new tip if one is already showing
    if active.current.is_some() {
        return;
    }

    // Tip: Welcome — first frame of first-ever game
    if !tips_shown.welcome {
        if lifetime_stats.total_games == 0 {
            active.current = Some(TipKind::Welcome);
            tips_shown.welcome = true;
            save_tips(&tips_shown);
            return;
        }
    }

    // Tip: Economy — after wave 1 completes (between waves)
    if !tips_shown.economy {
        if wave_manager.current_wave >= 1 && !wave_manager.wave_active {
            active.current = Some(TipKind::Economy);
            tips_shown.economy = true;
            save_tips(&tips_shown);
            return;
        }
    }

    // Tip: Specialization — when any tower reaches Level 2
    if !tips_shown.specialization {
        for tower in &towers {
            if tower.level >= 2 {
                active.current = Some(TipKind::Specialization);
                tips_shown.specialization = true;
                save_tips(&tips_shown);
                return;
            }
        }
    }

    // Tip: Synergy — when 3+ towers placed (likely adjacent pairs)
    if !tips_shown.synergy {
        if towers.iter().count() >= 3 {
            active.current = Some(TipKind::Synergy);
            tips_shown.synergy = true;
            save_tips(&tips_shown);
            return;
        }
    }

    // Tip: Early send — after completing wave 2 (player has seen the basics)
    if !tips_shown.early_send {
        if wave_manager.current_wave >= 2 && !wave_manager.wave_active {
            active.current = Some(TipKind::EarlySend);
            tips_shown.early_send = true;
            save_tips(&tips_shown);
            return;
        }
    }

    // Tip: Targeting — after wave 5 (player is established)
    if !tips_shown.targeting {
        if wave_manager.current_wave >= 5 {
            active.current = Some(TipKind::Targeting);
            tips_shown.targeting = true;
            save_tips(&tips_shown);
        }
    }
}

fn update_tip_display(
    mut commands: Commands,
    mut active: ResMut<ActiveTip>,
    existing: Query<Entity, With<TipOverlay>>,
    assets: Res<crate::loading::GameAssets>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Tick timer and auto-dismiss
    if let Some(ref mut timer) = active.timer {
        timer.tick(time.delta());
        if timer.finished() || keyboard.just_pressed(KeyCode::Escape) {
            active.current = None;
            active.timer = None;
            for entity in &existing {
                commands.entity(entity).despawn_recursive();
            }
            return;
        }
    }

    // Spawn tip overlay if we have a new tip and no overlay exists
    if let Some(kind) = active.current {
        if existing.is_empty() {
            active.timer = Some(Timer::from_seconds(6.0, TimerMode::Once));

            commands.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(8.0),
                        left: Val::Percent(50.0),
                        margin: UiRect::left(Val::Px(-200.0)),
                        width: Val::Px(400.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::srgba(0.06, 0.12, 0.2, 0.92).into(),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    z_index: ZIndex::Global(50),
                    ..default()
                },
                TipOverlay,
            )).with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    kind.text(),
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 13.0,
                        color: GameColors::SECONDARY,
                    },
                ));
            });
        }
    }
}

fn cleanup_tips(
    mut commands: Commands,
    query: Query<Entity, With<TipOverlay>>,
    mut active: ResMut<ActiveTip>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
    active.current = None;
    active.timer = None;
}

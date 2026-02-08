use bevy::prelude::*;

use crate::GameState;
use super::rules;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerEconomy>()
            .init_resource::<KillStreak>()
            .add_systems(OnEnter(GameState::Playing), (reset_economy, reset_kill_streak))
            .add_systems(Update, update_kill_streak.run_if(in_state(GameState::Playing)));
    }
}

/// Player's economy state
#[derive(Resource)]
pub struct PlayerEconomy {
    pub gold: u32,
    pub lives: u32,
    pub score: u32,
    pub interest_rate: f32,
}

impl Default for PlayerEconomy {
    fn default() -> Self {
        Self {
            gold: 160,
            lives: 10,
            score: 0,
            interest_rate: 0.10, // 10% interest
        }
    }
}

impl PlayerEconomy {
    /// Calculate interest earned on current gold (capped at 50)
    pub fn calculate_interest(&self) -> u32 {
        rules::calculate_interest(self.gold, self.interest_rate)
    }
}

fn reset_economy(mut economy: ResMut<PlayerEconomy>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    *economy = PlayerEconomy::default();
}

/// Kill streak tracking for combo system
#[derive(Resource)]
pub struct KillStreak {
    pub count: u32,
    pub timer: Timer,
    pub max_reached: u32,
    pub active: bool,
}

impl Default for KillStreak {
    fn default() -> Self {
        Self {
            count: 0,
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            max_reached: 0,
            active: false,
        }
    }
}

impl KillStreak {
    /// Register a kill - increases combo and resets timer
    pub fn register_kill(&mut self) {
        self.count += 1;
        self.timer.reset();
        self.active = true;
        if self.count > self.max_reached {
            self.max_reached = self.count;
        }
    }

    /// Get the gold multiplier based on current streak
    pub fn multiplier(&self) -> f32 {
        rules::kill_streak_multiplier(self.count)
    }

}

fn reset_kill_streak(mut streak: ResMut<KillStreak>, active: Option<Res<super::GameActive>>) {
    if active.is_some() { return; }
    *streak = KillStreak::default();
}

/// Update kill streak timer - combo breaks after 2 seconds without kills
fn update_kill_streak(mut streak: ResMut<KillStreak>, time: Res<Time>) {
    if streak.active {
        streak.timer.tick(time.delta());
        if streak.timer.finished() {
            streak.count = 0;
            streak.active = false;
        }
    }
}

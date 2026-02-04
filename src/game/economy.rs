use bevy::prelude::*;

use crate::GameState;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerEconomy>()
            .add_systems(OnEnter(GameState::Playing), reset_economy);
    }
}

/// Player's economy state
#[derive(Resource)]
pub struct PlayerEconomy {
    pub gold: u32,
    pub lives: u32,
    pub score: u32,
}

impl Default for PlayerEconomy {
    fn default() -> Self {
        Self {
            gold: 100,
            lives: 10,
            score: 0,
        }
    }
}

fn reset_economy(mut economy: ResMut<PlayerEconomy>) {
    *economy = PlayerEconomy::default();
}

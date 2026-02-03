use bevy::prelude::*;

pub mod shapes;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, _app: &mut App) {
        // Graphics systems will be added here as needed
    }
}

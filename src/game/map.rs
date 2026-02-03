use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::GameEntity;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMap>()
            .add_systems(OnEnter(GameState::Playing), setup_map);
    }
}

/// Grid dimensions
pub const GRID_WIDTH: usize = 16;
pub const GRID_HEIGHT: usize = 12;
pub const TILE_SIZE: f32 = ShapeSizes::TILE;

/// Tile types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileType {
    Empty,     // Can build towers
    Path,      // Enemy path
    Blocked,   // Cannot build
    Tower,     // Has a tower
}

/// The game map
#[derive(Resource)]
pub struct GameMap {
    pub tiles: [[TileType; GRID_HEIGHT]; GRID_WIDTH],
    pub path: Vec<(usize, usize)>,
}

impl Default for GameMap {
    fn default() -> Self {
        let mut tiles = [[TileType::Empty; GRID_HEIGHT]; GRID_WIDTH];

        // Define a longer, more complex winding path
        let path = vec![
            // Start from left side, middle-ish
            (0, 10),
            (1, 10),
            (2, 10),
            (2, 9),
            (2, 8),
            (2, 7),
            (3, 7),
            (4, 7),
            (5, 7),
            (5, 8),
            (5, 9),
            (5, 10),
            (6, 10),
            (7, 10),
            (7, 9),
            (7, 8),
            (7, 7),
            (7, 6),
            (7, 5),
            (8, 5),
            (9, 5),
            (10, 5),
            (10, 6),
            (10, 7),
            (10, 8),
            (11, 8),
            (12, 8),
            (12, 7),
            (12, 6),
            (12, 5),
            (12, 4),
            (12, 3),
            (11, 3),
            (10, 3),
            (9, 3),
            (9, 2),
            (9, 1),
            (10, 1),
            (11, 1),
            (12, 1),
            (13, 1),
            (14, 1),
            (14, 2),
            (14, 3),
            (14, 4),
            (14, 5),
            (15, 5),
        ];

        // Mark path tiles
        for &(x, y) in &path {
            tiles[x][y] = TileType::Path;
        }

        // Add some blocked/obstacle tiles for visual interest
        let blocked_tiles = [
            (3, 9), (4, 9), (3, 8),
            (6, 8), (6, 9),
            (8, 7), (9, 7), (8, 6),
            (11, 5), (11, 6), (11, 7),
            (13, 3), (13, 4),
            (10, 2), (11, 2),
        ];

        for &(x, y) in &blocked_tiles {
            tiles[x][y] = TileType::Blocked;
        }

        Self { tiles, path }
    }
}

impl GameMap {
    /// Convert grid coordinates to world position (center of tile)
    pub fn grid_to_world(x: usize, y: usize) -> Vec2 {
        let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
        let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;

        Vec2::new(
            x as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_x,
            y as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_y,
        )
    }

    /// Convert world position to grid coordinates
    pub fn world_to_grid(pos: Vec2) -> Option<(usize, usize)> {
        let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
        let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;

        let x = ((pos.x + offset_x) / TILE_SIZE).floor() as i32;
        let y = ((pos.y + offset_y) / TILE_SIZE).floor() as i32;

        if x >= 0 && x < GRID_WIDTH as i32 && y >= 0 && y < GRID_HEIGHT as i32 {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    /// Check if a tile is buildable
    pub fn is_buildable(&self, x: usize, y: usize) -> bool {
        if x >= GRID_WIDTH || y >= GRID_HEIGHT {
            return false;
        }
        self.tiles[x][y] == TileType::Empty
    }

    /// Mark tile as having a tower
    pub fn place_tower(&mut self, x: usize, y: usize) {
        if x < GRID_WIDTH && y < GRID_HEIGHT {
            self.tiles[x][y] = TileType::Tower;
        }
    }

    /// Remove tower from tile
    pub fn remove_tower(&mut self, x: usize, y: usize) {
        if x < GRID_WIDTH && y < GRID_HEIGHT && self.tiles[x][y] == TileType::Tower {
            self.tiles[x][y] = TileType::Empty;
        }
    }
}

#[derive(Component)]
pub struct MapTile {
    pub x: usize,
    pub y: usize,
}

fn setup_map(mut commands: Commands, map: Res<GameMap>) {
    // Spawn grid tiles
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let pos = GameMap::grid_to_world(x, y);
            let tile_type = map.tiles[x][y];

            let color = match tile_type {
                TileType::Path => GameColors::PATH,
                TileType::Empty => GameColors::TILE_EMPTY,
                TileType::Blocked => GameColors::TILE_BLOCKED,
                TileType::Tower => GameColors::TILE_EMPTY,
            };

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::splat(TILE_SIZE - ShapeSizes::TILE_GAP)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(0.0)),
                    ..default()
                },
                MapTile { x, y },
                GameEntity,
            ));
        }
    }

    // Draw path direction indicators
    for i in 0..map.path.len() - 1 {
        let (x1, y1) = map.path[i];
        let (x2, y2) = map.path[i + 1];

        let pos1 = GameMap::grid_to_world(x1, y1);
        let pos2 = GameMap::grid_to_world(x2, y2);
        let mid = (pos1 + pos2) / 2.0;

        // Small indicator dot along path
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::PATH_INDICATOR,
                    custom_size: Some(Vec2::splat(ShapeSizes::PATH_INDICATOR)),
                    ..default()
                },
                transform: Transform::from_translation(mid.extend(0.5)),
                ..default()
            },
            GameEntity,
        ));
    }

    // Spawn entry point indicator
    if let Some(&(x, y)) = map.path.first() {
        let pos = GameMap::grid_to_world(x, y);
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::PRIMARY,
                    custom_size: Some(Vec2::splat(ShapeSizes::SPAWN_INDICATOR)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(1.0)),
                ..default()
            },
            GameEntity,
        ));
    }

    // Spawn exit point indicator
    if let Some(&(x, y)) = map.path.last() {
        let pos = GameMap::grid_to_world(x, y);
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::SUCCESS,
                    custom_size: Some(Vec2::splat(ShapeSizes::SPAWN_INDICATOR)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(1.0)),
                ..default()
            },
            GameEntity,
        ));
    }
}

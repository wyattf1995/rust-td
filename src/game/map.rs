use bevy::prelude::*;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::GameEntity;
use super::tower::{SelectedTowerType, Tower};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMap>()
            .init_resource::<HoveredTile>()
            .add_systems(OnEnter(GameState::Playing), setup_map)
            .add_systems(
                Update,
                (tile_hover_system, update_tile_visuals)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Currently hovered tile
#[derive(Resource, Default)]
pub struct HoveredTile {
    pub position: Option<(usize, usize)>,
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
    pub base_color: Color,
}

/// Range preview circle when hovering over buildable tile
#[derive(Component)]
pub struct TileRangePreview;

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
                MapTile { x, y, base_color: color },
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

    // Spawn range preview circle (initially hidden)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::NONE,
                custom_size: Some(Vec2::splat(300.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.8)),
            ..default()
        },
        TileRangePreview,
        GameEntity,
    ));
}

fn tile_hover_system(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut hovered_tile: ResMut<HoveredTile>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        hovered_tile.position = None;
        return;
    };

    // Check if cursor is in UI area
    let window_height = window.height();
    if cursor_pos.y > window_height - 140.0 || cursor_pos.y < 50.0 {
        hovered_tile.position = None;
        return;
    }

    // Convert to world coordinates
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        hovered_tile.position = None;
        return;
    };

    // Convert to grid coordinates
    hovered_tile.position = GameMap::world_to_grid(world_pos);
}

fn update_tile_visuals(
    hovered_tile: Res<HoveredTile>,
    map: Res<GameMap>,
    selected_tower: Res<SelectedTowerType>,
    mut tiles: Query<(&MapTile, &mut Sprite)>,
    mut range_preview: Query<(&mut Sprite, &mut Transform), (With<TileRangePreview>, Without<MapTile>)>,
    towers: Query<&Tower>,
) {
    // Update tile colors based on hover
    for (tile, mut sprite) in &mut tiles {
        if let Some((hx, hy)) = hovered_tile.position {
            if tile.x == hx && tile.y == hy {
                // Tile is hovered
                let tile_type = map.tiles[tile.x][tile.y];
                if tile_type == TileType::Empty {
                    // Buildable - show green tint
                    sprite.color = Color::srgba(0.3, 0.6, 0.3, 1.0);
                } else if tile_type == TileType::Tower {
                    // Has tower - show selection highlight
                    sprite.color = Color::srgba(0.4, 0.4, 0.6, 1.0);
                } else {
                    // Not buildable - show red tint
                    sprite.color = Color::srgba(0.6, 0.3, 0.3, 1.0);
                }
            } else {
                sprite.color = tile.base_color;
            }
        } else {
            sprite.color = tile.base_color;
        }
    }

    // Update range preview
    for (mut preview_sprite, mut preview_transform) in &mut range_preview {
        if let Some((hx, hy)) = hovered_tile.position {
            let tile_type = map.tiles[hx][hy];

            if tile_type == TileType::Empty {
                // Show range preview for placing new tower
                let pos = GameMap::grid_to_world(hx, hy);
                preview_transform.translation = pos.extend(0.8);
                let range = selected_tower.0.range();
                preview_sprite.custom_size = Some(Vec2::splat(range * 2.0));
                preview_sprite.color = GameColors::RANGE_INDICATOR;
            } else if tile_type == TileType::Tower {
                // Show range of existing tower
                let pos = GameMap::grid_to_world(hx, hy);
                preview_transform.translation = pos.extend(0.8);

                // Find the tower at this position
                let mut found_range = 150.0;
                for tower in &towers {
                    if tower.grid_x == hx && tower.grid_y == hy {
                        found_range = tower.range;
                        break;
                    }
                }
                preview_sprite.custom_size = Some(Vec2::splat(found_range * 2.0));
                preview_sprite.color = GameColors::RANGE_INDICATOR;
            } else {
                preview_sprite.color = Color::NONE;
            }
        } else {
            preview_sprite.color = Color::NONE;
        }
    }
}

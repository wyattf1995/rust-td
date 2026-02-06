use bevy::prelude::*;
use rand::Rng;
use std::collections::HashSet;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes};

use super::GameEntity;
use super::tower::{SelectedTowerType, Tower};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredTile>()
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

/// Grid dimensions - larger grid for more strategic options
pub const GRID_WIDTH: usize = 18;
pub const GRID_HEIGHT: usize = 11;
pub const TILE_SIZE: f32 = ShapeSizes::TILE;
/// Vertical offset to center grid between header (50px) and footer (~115px)
pub const GRID_Y_OFFSET: f32 = 28.0;

/// Terrain types for visual variety
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerrainType {
    Rock,
    Water,
    Forest,
    Crystal,
}

impl TerrainType {
    pub fn color(&self, variant: bool) -> Color {
        match self {
            TerrainType::Rock => {
                if variant { GameColors::TERRAIN_ROCK } else { GameColors::TERRAIN_ROCK_DARK }
            }
            TerrainType::Water => {
                if variant { GameColors::TERRAIN_WATER } else { GameColors::TERRAIN_WATER_LIGHT }
            }
            TerrainType::Forest => {
                if variant { GameColors::TERRAIN_FOREST } else { GameColors::TERRAIN_FOREST_LIGHT }
            }
            TerrainType::Crystal => {
                if variant { GameColors::TERRAIN_CRYSTAL } else { GameColors::TERRAIN_LAVA }
            }
        }
    }
}

/// Tile types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileType {
    Empty,     // Can build towers
    Path,      // Enemy path
    Blocked(TerrainType),   // Cannot build - has terrain
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
        Self::generate_random()
    }
}

impl GameMap {
    /// Generate a random map with procedural path and obstacles
    pub fn generate_random() -> Self {
        let mut tiles = [[TileType::Empty; GRID_HEIGHT]; GRID_WIDTH];

        // Generate a winding path using simple random walk algorithm
        let path = Self::generate_path();

        // Mark path tiles
        let path_set: HashSet<(usize, usize)> = path.iter().cloned().collect();
        for &(x, y) in &path {
            tiles[x][y] = TileType::Path;
        }

        // Generate random terrain obstacles
        Self::generate_obstacles(&mut tiles, &path_set);

        Self { tiles, path }
    }

    /// Generate a winding path from left to right with guaranteed snake pattern
    fn generate_path() -> Vec<(usize, usize)> {
        let mut rng = rand::thread_rng();
        let mut path = Vec::with_capacity(120);
        let mut path_set = HashSet::with_capacity(120); // O(1) membership checks

        // Start on left edge
        let start_y = rng.gen_range(2..(GRID_HEIGHT - 2));
        path.push((0, start_y));
        path_set.insert((0, start_y));

        let mut x = 0usize;
        let mut y = start_y;

        // Track consecutive horizontal moves to force vertical wandering
        let mut horizontal_streak = 0;
        let max_horizontal = 2; // Force vertical move after this many horizontal moves

        // Track current vertical direction for snake pattern
        let mut going_up = rng.gen_bool(0.5);

        while x < GRID_WIDTH - 1 {
            let mut moved = false;

            // If we've moved horizontally too much, force a vertical move
            if horizontal_streak >= max_horizontal {
                // Try to move vertically in current direction
                let target_y = if going_up { y + 1 } else { y.saturating_sub(1) };

                // Check if we can move in preferred direction
                let can_move_preferred = if going_up {
                    target_y < GRID_HEIGHT - 1 && !path_set.contains(&(x, target_y))
                } else {
                    y > 1 && !path_set.contains(&(x, target_y))
                };

                if can_move_preferred {
                    y = target_y;
                    path.push((x, y));
                    path_set.insert((x, y));
                    horizontal_streak = 0;
                    moved = true;
                } else {
                    // Hit edge, reverse direction and move the other way
                    going_up = !going_up;
                    let new_target_y = if going_up { y + 1 } else { y.saturating_sub(1) };
                    let can_reverse = if going_up {
                        new_target_y < GRID_HEIGHT - 1 && !path_set.contains(&(x, new_target_y))
                    } else {
                        y > 1 && !path_set.contains(&(x, new_target_y))
                    };

                    if can_reverse {
                        y = new_target_y;
                        path.push((x, y));
                        path_set.insert((x, y));
                        horizontal_streak = 0;
                        moved = true;
                    }
                }
            }

            // If didn't move vertically, decide between horizontal and vertical
            if !moved {
                // Weight: 40% horizontal, 60% vertical to encourage winding
                let move_horizontal = rng.gen_bool(0.4) || horizontal_streak == 0;

                if move_horizontal && x + 1 < GRID_WIDTH {
                    x += 1;
                    path.push((x, y));
                    path_set.insert((x, y));
                    horizontal_streak += 1;

                    // After moving right, sometimes do a vertical run
                    if rng.gen_bool(0.5) {
                        let run_length = rng.gen_range(2..5);
                        for _ in 0..run_length {
                            let target_y = if going_up { y + 1 } else { y.saturating_sub(1) };
                            let can_move = if going_up {
                                target_y < GRID_HEIGHT - 1 && !path_set.contains(&(x, target_y))
                            } else {
                                y > 1 && !path_set.contains(&(x, target_y))
                            };

                            if can_move {
                                y = target_y;
                                path.push((x, y));
                                path_set.insert((x, y));
                            } else {
                                going_up = !going_up;
                                break;
                            }
                        }
                        horizontal_streak = 0;
                    }
                } else {
                    // Try vertical move
                    let target_y = if going_up { y + 1 } else { y.saturating_sub(1) };
                    let can_move = if going_up {
                        target_y < GRID_HEIGHT - 1 && !path_set.contains(&(x, target_y))
                    } else {
                        y > 1 && !path_set.contains(&(x, target_y))
                    };

                    if can_move {
                        y = target_y;
                        path.push((x, y));
                        path_set.insert((x, y));
                        horizontal_streak = 0;
                    } else {
                        // Can't move vertically, try reversing
                        going_up = !going_up;
                        // Force horizontal if stuck
                        if x + 1 < GRID_WIDTH {
                            x += 1;
                            path.push((x, y));
                            path_set.insert((x, y));
                            horizontal_streak += 1;
                        }
                    }
                }
            }

            // Prevent infinite loops
            if path.len() > 150 {
                break;
            }
        }

        // Ensure we end at the right edge
        while x < GRID_WIDTH - 1 {
            x += 1;
            path.push((x, y));
            path_set.insert((x, y));
        }

        path
    }

    /// Generate random terrain obstacles avoiding the path
    fn generate_obstacles(tiles: &mut [[TileType; GRID_HEIGHT]; GRID_WIDTH], path_set: &HashSet<(usize, usize)>) {
        let mut rng = rand::thread_rng();
        let terrain_types = [TerrainType::Rock, TerrainType::Water, TerrainType::Forest, TerrainType::Crystal];

        // Choose 2-5 terrain clusters (reduced for more buildable space)
        let num_clusters = rng.gen_range(2..6);

        for _ in 0..num_clusters {
            // Pick a random center point
            let center_x = rng.gen_range(2..(GRID_WIDTH - 2));
            let center_y = rng.gen_range(1..(GRID_HEIGHT - 1));

            // Pick a terrain type
            let terrain = terrain_types[rng.gen_range(0..terrain_types.len())];

            // Create a cluster of 3-8 tiles (reduced ~20%)
            let cluster_size = rng.gen_range(3..9);
            let mut placed = 0;
            let mut attempts = 0;

            while placed < cluster_size && attempts < 50 {
                // Random offset from center
                let offset_x = rng.gen_range(-2..3);
                let offset_y = rng.gen_range(-2..3);

                let tx = (center_x as i32 + offset_x) as usize;
                let ty = (center_y as i32 + offset_y) as usize;

                // Check bounds and not on path
                if tx < GRID_WIDTH && ty < GRID_HEIGHT
                    && !path_set.contains(&(tx, ty))
                    && !Self::is_adjacent_to_path(tx, ty, path_set)
                    && tiles[tx][ty] == TileType::Empty
                {
                    tiles[tx][ty] = TileType::Blocked(terrain);
                    placed += 1;
                }
                attempts += 1;
            }
        }

        // Add some scattered individual obstacles (reduced ~20%)
        let scattered = rng.gen_range(6..12);
        for _ in 0..scattered {
            let x = rng.gen_range(0..GRID_WIDTH);
            let y = rng.gen_range(0..GRID_HEIGHT);
            let terrain = terrain_types[rng.gen_range(0..terrain_types.len())];

            if !path_set.contains(&(x, y))
                && !Self::is_adjacent_to_path(x, y, path_set)
                && tiles[x][y] == TileType::Empty
            {
                tiles[x][y] = TileType::Blocked(terrain);
            }
        }
    }

    /// Check if a tile is directly adjacent to the path (buffer zone)
    fn is_adjacent_to_path(x: usize, y: usize, path_set: &HashSet<(usize, usize)>) -> bool {
        let neighbors = [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ];

        for (nx, ny) in neighbors {
            if path_set.contains(&(nx, ny)) {
                return true;
            }
        }
        false
    }

    /// Convert grid coordinates to world position (center of tile)
    pub fn grid_to_world(x: usize, y: usize) -> Vec2 {
        let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
        let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;

        Vec2::new(
            x as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_x,
            y as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_y + GRID_Y_OFFSET,
        )
    }

    /// Convert world position to grid coordinates
    pub fn world_to_grid(pos: Vec2) -> Option<(usize, usize)> {
        let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
        let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;

        let x = ((pos.x + offset_x) / TILE_SIZE).floor() as i32;
        let y = ((pos.y + offset_y - GRID_Y_OFFSET) / TILE_SIZE).floor() as i32;

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

/// Terrain decoration sprite (second layer for visual depth)
#[derive(Component)]
pub struct TerrainDecoration;

/// Range preview circle when hovering over buildable tile
#[derive(Component)]
pub struct TileRangePreview;

fn setup_map(mut commands: Commands, active: Option<Res<super::GameActive>>) {
    // Skip if resuming from pause (game already active)
    if active.is_some() {
        return;
    }
    commands.insert_resource(super::GameActive);

    // Generate a new random map each game
    let map = GameMap::generate_random();

    // Simple deterministic variation based on position
    let variation_seed = |x: usize, y: usize| -> bool {
        ((x * 7 + y * 13) % 3) == 0
    };

    // Spawn grid tiles with borders
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let pos = GameMap::grid_to_world(x, y);
            let tile_type = map.tiles[x][y];

            // Tile border (slightly larger, behind main tile)
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::TILE_BORDER,
                        custom_size: Some(Vec2::splat(TILE_SIZE - 1.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(-0.1)),
                    ..default()
                },
                GameEntity,
            ));

            let color = match tile_type {
                TileType::Path => GameColors::PATH,
                TileType::Empty => GameColors::TILE_EMPTY,
                TileType::Blocked(terrain) => terrain.color(variation_seed(x, y)),
                TileType::Tower => GameColors::TILE_EMPTY,
            };

            // Main tile sprite (inner area)
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::splat(ShapeSizes::TILE_INNER)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(0.0)),
                    ..default()
                },
                MapTile { x, y, base_color: color },
                GameEntity,
            ));


            // Add decoration overlay for terrain tiles
            if let TileType::Blocked(terrain) = tile_type {
                let decoration_color = match terrain {
                    TerrainType::Rock => Color::srgba(0.3, 0.28, 0.25, 0.25),
                    TerrainType::Water => Color::srgba(0.2, 0.4, 0.6, 0.2),
                    TerrainType::Forest => Color::srgba(0.15, 0.3, 0.18, 0.25),
                    TerrainType::Crystal => Color::srgba(0.4, 0.25, 0.5, 0.25),
                };

                // Inner decoration (smaller highlight)
                let deco_size = TILE_SIZE * 0.4;
                let offset_x = if variation_seed(x, y) { 6.0 } else { -6.0 };
                let offset_y = if variation_seed(y, x) { 6.0 } else { -6.0 };

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: decoration_color,
                            custom_size: Some(Vec2::splat(deco_size)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x + offset_x, pos.y + offset_y, 0.1)
                        ),
                        ..default()
                    },
                    TerrainDecoration,
                    GameEntity,
                ));
            }
        }
    }

    // Draw path lane markings based on actual path sequence
    let dash_length = TILE_SIZE * 0.35;
    let dash_offset = TILE_SIZE * 0.22;

    for i in 0..map.path.len() {
        let (x, y) = map.path[i];
        let pos = GameMap::grid_to_world(x, y);

        // Check previous node in path
        if i > 0 {
            let (px, py) = map.path[i - 1];
            let dx = px as i32 - x as i32;
            let dy = py as i32 - y as i32;

            // Draw dash toward previous
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::PATH_LANE,
                        custom_size: Some(if dx != 0 {
                            Vec2::new(dash_length, ShapeSizes::PATH_LANE_WIDTH)
                        } else {
                            Vec2::new(ShapeSizes::PATH_LANE_WIDTH, dash_length)
                        }),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        pos.x + dx as f32 * dash_offset,
                        pos.y + dy as f32 * dash_offset,
                        0.05,
                    )),
                    ..default()
                },
                GameEntity,
            ));
        }

        // Check next node in path
        if i + 1 < map.path.len() {
            let (nx, ny) = map.path[i + 1];
            let dx = nx as i32 - x as i32;
            let dy = ny as i32 - y as i32;

            // Draw dash toward next
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::PATH_LANE,
                        custom_size: Some(if dx != 0 {
                            Vec2::new(dash_length, ShapeSizes::PATH_LANE_WIDTH)
                        } else {
                            Vec2::new(ShapeSizes::PATH_LANE_WIDTH, dash_length)
                        }),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        pos.x + dx as f32 * dash_offset,
                        pos.y + dy as f32 * dash_offset,
                        0.05,
                    )),
                    ..default()
                },
                GameEntity,
            ));
        }
    }

    // Draw path direction indicators (dots between tiles)
    for i in 0..map.path.len().saturating_sub(1) {
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

    // Spawn entry point indicator (enemy spawn)
    if let Some(&(x, y)) = map.path.first() {
        let pos = GameMap::grid_to_world(x, y);
        // Outer glow
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::SECONDARY.with_alpha(0.3),
                    custom_size: Some(Vec2::splat(ShapeSizes::SPAWN_INDICATOR + 8.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(0.9)),
                ..default()
            },
            GameEntity,
        ));
        // Inner marker
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::SECONDARY,
                    custom_size: Some(Vec2::splat(ShapeSizes::SPAWN_INDICATOR)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(1.0)),
                ..default()
            },
            GameEntity,
        ));
    }

    // Spawn exit point indicator (goal)
    if let Some(&(x, y)) = map.path.last() {
        let pos = GameMap::grid_to_world(x, y);
        // Outer glow
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::ACCENT.with_alpha(0.3),
                    custom_size: Some(Vec2::splat(ShapeSizes::EXIT_INDICATOR + 8.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(0.9)),
                ..default()
            },
            GameEntity,
        ));
        // Inner marker
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::ACCENT,
                    custom_size: Some(Vec2::splat(ShapeSizes::EXIT_INDICATOR)),
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

    // Insert the generated map as a resource
    commands.insert_resource(map);
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

    // Check if cursor is in UI area (top HUD or bottom panel)
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
                    sprite.color = GameColors::TILE_HOVER_BUILD;
                } else if tile_type == TileType::Tower {
                    // Has tower - show selection highlight
                    sprite.color = GameColors::TILE_HOVER_TOWER;
                } else {
                    // Not buildable - show red tint
                    sprite.color = GameColors::TILE_HOVER_BLOCKED;
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

use bevy::prelude::*;
use rand::Rng;
use std::collections::HashSet;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes, ZDepth};

use super::GameEntity;
use super::tower::{SelectedTowerType, Tower};
use super::ui::UiZones;

/// Map preset variants for strategic variety
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MapPreset {
    #[default]
    Random,
    Serpentine,
    Sprint,
    Spiral,
}

impl MapPreset {
    pub fn name(&self) -> &'static str {
        match self {
            MapPreset::Random => "Random",
            MapPreset::Serpentine => "Serpentine",
            MapPreset::Sprint => "Sprint",
            MapPreset::Spiral => "Spiral",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MapPreset::Random => "Procedural path",
            MapPreset::Serpentine => "Long winding road",
            MapPreset::Sprint => "Short & direct",
            MapPreset::Spiral => "Inward spiral",
        }
    }
}

/// Currently selected map preset (persists between menu and game)
#[derive(Resource, Default)]
pub struct SelectedMap(pub MapPreset);

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredTile>()
            .init_resource::<SelectedMap>()
            .add_systems(OnEnter(GameState::Playing), setup_map)
            .add_systems(
                Update,
                (tile_hover_system, update_tile_visuals, update_path_flow_dots, update_spawn_exit_markers)
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
        Self::generate(MapPreset::Random)
    }
}

impl GameMap {
    /// Generate just the path for a preset (for menu previews).
    pub fn generate_path_for(preset: MapPreset) -> Vec<(usize, usize)> {
        match preset {
            MapPreset::Random => Self::generate_path(),
            MapPreset::Serpentine => Self::generate_serpentine_path(),
            MapPreset::Sprint => Self::generate_sprint_path(),
            MapPreset::Spiral => Self::generate_spiral_path(),
        }
    }

    /// Generate a map using the given preset
    pub fn generate(preset: MapPreset) -> Self {
        let path = match preset {
            MapPreset::Random => Self::generate_path(),
            MapPreset::Serpentine => Self::generate_serpentine_path(),
            MapPreset::Sprint => Self::generate_sprint_path(),
            MapPreset::Spiral => Self::generate_spiral_path(),
        };

        let mut tiles = [[TileType::Empty; GRID_HEIGHT]; GRID_WIDTH];
        let path_set: HashSet<(usize, usize)> = path.iter().cloned().collect();
        for &(x, y) in &path {
            tiles[x][y] = TileType::Path;
        }
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

    /// Generate a serpentine path: full-width snake with 3 horizontal runs
    fn generate_serpentine_path() -> Vec<(usize, usize)> {
        let mut rng = rand::thread_rng();
        let mut path = Vec::with_capacity(150);

        // Three horizontal rows with vertical connectors
        // Row positions with minor randomization
        let row1 = rng.gen_range(1..3);           // top area
        let row2 = rng.gen_range(4..7);           // middle area
        let row3 = rng.gen_range(8..(GRID_HEIGHT - 1)); // bottom area

        // Run 1: left to right along row1
        for x in 0..GRID_WIDTH - 1 {
            path.push((x, row1));
        }

        // Drop down from row1 to row2 at right side
        let drop_x = GRID_WIDTH - 2;
        let (start, end) = if row1 < row2 { (row1, row2) } else { (row2, row1) };
        for y in (start + 1)..=end {
            path.push((drop_x, y));
        }

        // Run 2: right to left along row2
        for x in (1..=drop_x).rev() {
            path.push((x, row2));
        }

        // Drop down from row2 to row3 at left side
        let drop_x2 = 1;
        let (start, end) = if row2 < row3 { (row2, row3) } else { (row3, row2) };
        for y in (start + 1)..=end {
            path.push((drop_x2, y));
        }

        // Run 3: left to right along row3 to exit
        for x in (drop_x2 + 1)..GRID_WIDTH {
            path.push((x, row3));
        }

        path
    }

    /// Generate a sprint path: mostly horizontal through center with minor jogs
    fn generate_sprint_path() -> Vec<(usize, usize)> {
        let mut rng = rand::thread_rng();
        let mut path = Vec::with_capacity(40);

        let mut y = GRID_HEIGHT / 2; // Start near center
        let mut x = 0;

        path.push((x, y));

        while x < GRID_WIDTH - 1 {
            // Move right 2-4 columns
            let run = rng.gen_range(2..5).min(GRID_WIDTH - 1 - x);
            for _ in 0..run {
                x += 1;
                path.push((x, y));
            }

            if x >= GRID_WIDTH - 1 {
                break;
            }

            // Small vertical jog of 1-2 tiles
            let jog = rng.gen_range(1..3);
            let direction = if y <= 2 {
                1 // must go down
            } else if y >= GRID_HEIGHT - 3 {
                -1 // must go up
            } else if rng.gen_bool(0.5) {
                1
            } else {
                -1
            };

            for _ in 0..jog {
                let new_y = (y as i32 + direction) as usize;
                if new_y > 0 && new_y < GRID_HEIGHT - 1 {
                    y = new_y;
                    path.push((x, y));
                }
            }
        }

        path
    }

    /// Generate a spiral path: clockwise inward from top-left, exits right.
    /// Loops are spaced 2 apart to leave buildable columns between them.
    fn generate_spiral_path() -> Vec<(usize, usize)> {
        let mut path = Vec::with_capacity(120);

        let mut left = 0usize;
        let mut right = GRID_WIDTH - 2; // Leave last column for exit
        let mut top = 0usize;
        let mut bottom = GRID_HEIGHT - 1;

        // Start at top-left
        let mut x = 0;
        let mut y = 0;
        path.push((x, y));

        // Spiral inward clockwise, shrinking by 2 each loop to leave gaps
        loop {
            // Move right along top
            while x < right {
                x += 1;
                path.push((x, y));
            }
            top += 2;
            if top > bottom { break; }

            // Move down along right
            while y < bottom {
                y += 1;
                path.push((x, y));
            }
            right = right.saturating_sub(2);
            if left > right { break; }

            // Move left along bottom
            while x > left {
                x -= 1;
                path.push((x, y));
            }
            bottom = bottom.saturating_sub(2);
            if top > bottom { break; }

            // Move up along left
            while y > top {
                y -= 1;
                path.push((x, y));
            }
            left += 2;
            if left > right { break; }
        }

        // Break out rightward to exit at column GRID_WIDTH-1
        while x < GRID_WIDTH - 1 {
            x += 1;
            path.push((x, y));
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

/// Animated dot flowing along the path
#[derive(Component)]
pub struct PathFlowDot {
    pub progress: f32,  // 0.0 to 1.0 along the path
    pub speed: f32,
}

/// Spawn/exit marker for pulsing animation
#[derive(Component)]
pub struct SpawnExitMarker {
    pub is_spawn: bool,
}

fn setup_map(mut commands: Commands, active: Option<Res<super::GameActive>>, selected_map: Res<SelectedMap>) {
    // Skip if resuming from pause (game already active)
    if active.is_some() {
        return;
    }
    commands.insert_resource(super::GameActive);

    // Generate map based on selected preset
    let map = GameMap::generate(selected_map.0);

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
                    transform: Transform::from_translation(pos.extend(ZDepth::TILE_BORDER)),
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
                    transform: Transform::from_translation(pos.extend(ZDepth::TILE)),
                    ..default()
                },
                MapTile { x, y, base_color: color },
                GameEntity,
            ));


            // Grid crosshair dot on buildable tiles
            if tile_type == TileType::Empty {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgba(0.25, 0.28, 0.35, 0.2),
                            custom_size: Some(Vec2::splat(3.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(pos.extend(ZDepth::GRID_DOT)),
                        ..default()
                    },
                    GameEntity,
                ));
            }

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
                            Vec3::new(pos.x + offset_x, pos.y + offset_y, ZDepth::TERRAIN_DECORATION)
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
                        ZDepth::PATH_LANE,
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
                        ZDepth::PATH_LANE,
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
                transform: Transform::from_translation(mid.extend(ZDepth::PATH_INDICATOR)),
                ..default()
            },
            GameEntity,
        ));
    }

    // Path edge glow lines along path-to-non-path boundaries
    {
        let path_set: HashSet<(usize, usize)> = map.path.iter().cloned().collect();
        for &(px, py) in &path_set {
            let pos = GameMap::grid_to_world(px, py);
            let half_tile = TILE_SIZE / 2.0 - 3.0; // inset from tile edge
            let glow_color = Color::srgba(0.35, 0.4, 0.55, 0.15);

            // Check 4 cardinal neighbors
            // Up (+y)
            if py + 1 >= GRID_HEIGHT || !path_set.contains(&(px, py + 1)) {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: glow_color,
                            custom_size: Some(Vec2::new(44.0, 1.5)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x, pos.y + half_tile, ZDepth::PATH_EDGE_GLOW)
                        ),
                        ..default()
                    },
                    GameEntity,
                ));
            }
            // Down (-y)
            if py == 0 || !path_set.contains(&(px, py - 1)) {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: glow_color,
                            custom_size: Some(Vec2::new(44.0, 1.5)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x, pos.y - half_tile, ZDepth::PATH_EDGE_GLOW)
                        ),
                        ..default()
                    },
                    GameEntity,
                ));
            }
            // Right (+x)
            if px + 1 >= GRID_WIDTH || !path_set.contains(&(px + 1, py)) {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: glow_color,
                            custom_size: Some(Vec2::new(1.5, 44.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x + half_tile, pos.y, ZDepth::PATH_EDGE_GLOW)
                        ),
                        ..default()
                    },
                    GameEntity,
                ));
            }
            // Left (-x)
            if px == 0 || !path_set.contains(&(px - 1, py)) {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: glow_color,
                            custom_size: Some(Vec2::new(1.5, 44.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x - half_tile, pos.y, ZDepth::PATH_EDGE_GLOW)
                        ),
                        ..default()
                    },
                    GameEntity,
                ));
            }
        }
    }

    // Spawn entry point indicator (enemy spawn)
    if let Some(&(x, y)) = map.path.first() {
        let pos = GameMap::grid_to_world(x, y);
        spawn_point_marker(&mut commands, pos, GameColors::SECONDARY, ShapeSizes::SPAWN_INDICATOR, true);
    }

    // Spawn exit point indicator (goal)
    if let Some(&(x, y)) = map.path.last() {
        let pos = GameMap::grid_to_world(x, y);
        spawn_point_marker(&mut commands, pos, GameColors::ACCENT, ShapeSizes::EXIT_INDICATOR, false);
    }

    // Spawn range preview circle (initially hidden)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::NONE,
                custom_size: Some(Vec2::splat(300.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, ZDepth::RANGE_PREVIEW)),
            ..default()
        },
        TileRangePreview,
        GameEntity,
    ));

    // Spawn path flow dots (animated indicators showing enemy direction)
    for i in 0..5 {
        let progress = i as f32 / 5.0;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: GameColors::PATH_INDICATOR.with_alpha(0.2),
                    custom_size: Some(Vec2::splat(4.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, ZDepth::PATH_FLOW_DOT)),
                ..default()
            },
            PathFlowDot {
                progress,
                speed: 0.15,
            },
            GameEntity,
        ));
    }

    // Feature #11: Add extra terrain accent sprites for visual variety
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            if let TileType::Blocked(terrain) = map.tiles[x][y] {
                let pos = GameMap::grid_to_world(x, y);
                // Position-based pseudo-random for subtle color variation
                let seed = (x * 17 + y * 31) as f32;
                let r_var = (((seed * 12.9898).sin() * 43758.5453).fract() - 0.5) * 0.04;
                let g_var = (((seed * 78.233).sin() * 43758.5453).fract() - 0.5) * 0.04;
                let b_var = (((seed * 39.346).sin() * 43758.5453).fract() - 0.5) * 0.04;

                // Small accent sprite for extra depth
                let accent_color = match terrain {
                    TerrainType::Crystal => Color::srgba(0.5 + r_var, 0.3 + g_var, 0.6 + b_var, 0.15),
                    TerrainType::Rock => Color::srgba(0.25 + r_var, 0.22 + g_var, 0.2 + b_var, 0.15),
                    TerrainType::Water => Color::srgba(0.15 + r_var, 0.35 + g_var, 0.55 + b_var, 0.12),
                    TerrainType::Forest => Color::srgba(0.12 + r_var, 0.25 + g_var, 0.15 + b_var, 0.15),
                };

                let off_x = (((seed * 5.7).sin() * 100.0).fract() - 0.5) * 16.0;
                let off_y = (((seed * 3.2).cos() * 100.0).fract() - 0.5) * 16.0;

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: accent_color,
                            custom_size: Some(Vec2::splat(TILE_SIZE * 0.25)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            Vec3::new(pos.x + off_x, pos.y + off_y, ZDepth::TERRAIN_ACCENT)
                        ),
                        ..default()
                    },
                    TerrainDecoration,
                    GameEntity,
                ));
            }
        }
    }

    // Insert the generated map as a resource
    commands.insert_resource(map);
}

fn tile_hover_system(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut hovered_tile: ResMut<HoveredTile>,
    ui_zones: Res<UiZones>,
    pointer: Res<super::PointerState>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = pointer.position else {
        hovered_tile.position = None;
        return;
    };

    // Check if cursor is in UI area (dynamic zone boundaries)
    let window_height = window.height();
    if cursor_pos.y > window_height - ui_zones.bottom_bar_height - 10.0 || cursor_pos.y < ui_zones.top_bar_height + 10.0 {
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
    abilities: Res<super::abilities::PlayerAbilities>,
) {
    // Update tile colors based on hover (suppress during artillery targeting)
    for (tile, mut sprite) in &mut tiles {
        if abilities.artillery_targeting {
            sprite.color = tile.base_color;
        } else if let Some((hx, hy)) = hovered_tile.position {
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

    // Update range preview (hide during artillery targeting)
    for (mut preview_sprite, mut preview_transform) in &mut range_preview {
        if abilities.artillery_targeting {
            preview_sprite.color = Color::NONE;
        } else if let Some((hx, hy)) = hovered_tile.position {
            let tile_type = map.tiles[hx][hy];

            if tile_type == TileType::Empty {
                // Show range preview for placing new tower
                let pos = GameMap::grid_to_world(hx, hy);
                preview_transform.translation = pos.extend(ZDepth::RANGE_PREVIEW);
                let range = selected_tower.0.range();
                preview_sprite.custom_size = Some(Vec2::splat(range * 2.0));
                preview_sprite.color = GameColors::RANGE_INDICATOR;
            } else if tile_type == TileType::Tower {
                // Show range of existing tower
                let pos = GameMap::grid_to_world(hx, hy);
                preview_transform.translation = pos.extend(ZDepth::RANGE_PREVIEW);

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

/// Animate path flow dots traveling along the path
fn update_path_flow_dots(
    map: Res<GameMap>,
    mut dots: Query<(&mut PathFlowDot, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    if map.path.len() < 2 {
        return;
    }

    let path_len = map.path.len() - 1;

    for (mut dot, mut transform, mut sprite) in &mut dots {
        // Advance progress
        dot.progress += dot.speed * time.delta_seconds();
        if dot.progress >= 1.0 {
            dot.progress -= 1.0;
        }

        // Interpolate position along path
        let path_pos = dot.progress * path_len as f32;
        let segment = (path_pos as usize).min(path_len - 1);
        let frac = path_pos - segment as f32;

        let (x1, y1) = map.path[segment];
        let (x2, y2) = map.path[(segment + 1).min(path_len)];

        let pos1 = GameMap::grid_to_world(x1, y1);
        let pos2 = GameMap::grid_to_world(x2, y2);

        let world_pos = pos1 + (pos2 - pos1) * frac;
        transform.translation.x = world_pos.x;
        transform.translation.y = world_pos.y;

        // Pulse alpha
        let alpha = 0.2 + 0.15 * (time.elapsed_seconds() * 3.0 + dot.progress * 6.28).sin();
        sprite.color = GameColors::PATH_INDICATOR.with_alpha(alpha);
    }
}

/// Pulse spawn and exit markers for visibility
fn spawn_point_marker(
    commands: &mut Commands,
    pos: Vec2,
    color: Color,
    size: f32,
    is_spawn: bool,
) {
    // Outer glow
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: color.with_alpha(0.3),
                custom_size: Some(Vec2::splat(size + 8.0)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(ZDepth::SPAWN_EXIT_GLOW)),
            ..default()
        },
        SpawnExitMarker { is_spawn },
        GameEntity,
    ));
    // Inner marker
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(ZDepth::SPAWN_EXIT_MARKER)),
            ..default()
        },
        SpawnExitMarker { is_spawn },
        GameEntity,
    ));
}

fn update_spawn_exit_markers(
    mut markers: Query<(&SpawnExitMarker, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    for (marker, mut transform, mut sprite) in &mut markers {
        let t = time.elapsed_seconds();

        // Gentle scale pulse
        let scale_pulse = 1.0 + 0.15 * (t * 2.5).sin();
        transform.scale = Vec3::splat(scale_pulse);

        // Pulse glow alpha for outer markers (lower z = glow)
        if transform.translation.z < 1.0 {
            let base_alpha = if marker.is_spawn { 0.3 } else { 0.3 };
            let alpha = base_alpha * (0.6 + 0.4 * (t * 2.0).sin());
            let base_color = if marker.is_spawn {
                GameColors::SECONDARY
            } else {
                GameColors::ACCENT
            };
            sprite.color = base_color.with_alpha(alpha);
        }
    }
}

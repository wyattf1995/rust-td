use bevy::prelude::*;
use std::collections::HashMap;

use super::enemy::Enemy;
use crate::GameState;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .add_systems(
                Update,
                update_spatial_grid.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Grid cell size for spatial partitioning
const CELL_SIZE: f32 = 64.0;

/// Spatial hash grid for efficient range queries
#[derive(Resource, Default)]
pub struct SpatialGrid {
    /// Maps cell coordinates to entities in that cell
    cells: HashMap<(i32, i32), Vec<Entity>>,
    /// Maps entity to its current cell (for efficient updates)
    entity_cells: HashMap<Entity, (i32, i32)>,
}

impl SpatialGrid {
    /// Convert world position to cell coordinates
    pub fn world_to_cell(pos: Vec2) -> (i32, i32) {
        (
            (pos.x / CELL_SIZE).floor() as i32,
            (pos.y / CELL_SIZE).floor() as i32,
        )
    }

    /// Get all entities within a given range of a position
    pub fn query_range(&self, center: Vec2, range: f32) -> Vec<Entity> {
        let mut result = Vec::new();

        // Calculate cell range to check
        let cells_to_check = (range / CELL_SIZE).ceil() as i32 + 1;
        let center_cell = Self::world_to_cell(center);

        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                let cell = (center_cell.0 + dx, center_cell.1 + dy);
                if let Some(entities) = self.cells.get(&cell) {
                    result.extend(entities.iter().copied());
                }
            }
        }

        result
    }

    /// Clear the grid (called at start of each update)
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// Insert an entity at a position
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let cell = Self::world_to_cell(pos);
        self.cells.entry(cell).or_default().push(entity);
        self.entity_cells.insert(entity, cell);
    }

    /// Remove an entity
    pub fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(entities) = self.cells.get_mut(&cell) {
                entities.retain(|&e| e != entity);
            }
        }
    }
}

/// Update spatial grid with enemy positions
fn update_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
) {
    grid.clear();

    for (entity, transform, enemy) in &enemies {
        if enemy.health > 0.0 && !enemy.marked_dead {
            grid.insert(entity, transform.translation.truncate());
        }
    }
}

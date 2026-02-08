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
        // Calculate cell range to check
        let cells_to_check = (range / CELL_SIZE).ceil() as i32 + 1;
        let side = (cells_to_check * 2 + 1) as usize;
        let mut result = Vec::with_capacity(side * side * 2);
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
    /// Keeps allocated memory in cell Vecs to avoid reallocation
    pub fn clear(&mut self) {
        for entities in self.cells.values_mut() {
            entities.clear();
        }
        self.entity_cells.clear();
    }

    /// Insert an entity at a position
    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let cell = Self::world_to_cell(pos);
        self.cells.entry(cell).or_default().push(entity);
        self.entity_cells.insert(entity, cell);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    #[test]
    fn test_world_to_cell_origin() {
        assert_eq!(SpatialGrid::world_to_cell(Vec2::ZERO), (0, 0));
    }

    #[test]
    fn test_world_to_cell_positive() {
        // CELL_SIZE = 64.0, so (100, 100) → (1, 1)
        assert_eq!(SpatialGrid::world_to_cell(Vec2::new(100.0, 100.0)), (1, 1));
    }

    #[test]
    fn test_world_to_cell_negative() {
        assert_eq!(SpatialGrid::world_to_cell(Vec2::new(-10.0, -10.0)), (-1, -1));
    }

    #[test]
    fn test_world_to_cell_boundary() {
        // Exactly on cell boundary → next cell
        assert_eq!(SpatialGrid::world_to_cell(Vec2::new(64.0, 0.0)), (1, 0));
        // Just below boundary → previous cell
        assert_eq!(SpatialGrid::world_to_cell(Vec2::new(63.9, 0.0)), (0, 0));
    }

    #[test]
    fn test_insert_and_query_basic() {
        let mut grid = SpatialGrid::default();
        let e = entity(1);
        grid.insert(e, Vec2::new(32.0, 32.0));

        let result = grid.query_range(Vec2::new(32.0, 32.0), 10.0);
        assert!(result.contains(&e));
    }

    #[test]
    fn test_query_empty_grid() {
        let grid = SpatialGrid::default();
        let result = grid.query_range(Vec2::ZERO, 100.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_finds_nearby_entities() {
        let mut grid = SpatialGrid::default();
        let e1 = entity(1);
        let e2 = entity(2);
        let e3 = entity(3);
        // All within 64px of each other
        grid.insert(e1, Vec2::new(10.0, 10.0));
        grid.insert(e2, Vec2::new(30.0, 10.0));
        grid.insert(e3, Vec2::new(10.0, 30.0));

        let result = grid.query_range(Vec2::new(20.0, 20.0), 100.0);
        assert!(result.contains(&e1));
        assert!(result.contains(&e2));
        assert!(result.contains(&e3));
    }

    #[test]
    fn test_query_range_includes_adjacent_cells() {
        let mut grid = SpatialGrid::default();
        let e1 = entity(1);
        let e2 = entity(2);
        // Place in different cells
        grid.insert(e1, Vec2::new(10.0, 10.0));   // cell (0,0)
        grid.insert(e2, Vec2::new(100.0, 100.0));  // cell (1,1)

        // Query from e1's position with range that covers e2's cell
        let result = grid.query_range(Vec2::new(10.0, 10.0), 200.0);
        assert!(result.contains(&e1));
        assert!(result.contains(&e2));
    }

    #[test]
    fn test_query_range_excludes_distant_cells() {
        let mut grid = SpatialGrid::default();
        let e1 = entity(1);
        let e2 = entity(2);
        // Far apart: 10 cells distance
        grid.insert(e1, Vec2::new(0.0, 0.0));
        grid.insert(e2, Vec2::new(1000.0, 0.0));

        // Small range query from e1's position
        let result = grid.query_range(Vec2::ZERO, 50.0);
        assert!(result.contains(&e1));
        assert!(!result.contains(&e2));
    }

    #[test]
    fn test_multiple_entities_same_cell() {
        let mut grid = SpatialGrid::default();
        let e1 = entity(1);
        let e2 = entity(2);
        let e3 = entity(3);
        grid.insert(e1, Vec2::new(5.0, 5.0));
        grid.insert(e2, Vec2::new(10.0, 10.0));
        grid.insert(e3, Vec2::new(15.0, 15.0));

        let result = grid.query_range(Vec2::new(10.0, 10.0), 1.0);
        assert_eq!(result.len(), 3); // All in cell (0,0)
    }

    #[test]
    fn test_clear_removes_entities() {
        let mut grid = SpatialGrid::default();
        grid.insert(entity(1), Vec2::new(10.0, 10.0));
        grid.insert(entity(2), Vec2::new(20.0, 20.0));

        grid.clear();

        let result = grid.query_range(Vec2::ZERO, 1000.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_clear_preserves_cell_allocation() {
        let mut grid = SpatialGrid::default();
        grid.insert(entity(1), Vec2::new(10.0, 10.0));
        let cell = SpatialGrid::world_to_cell(Vec2::new(10.0, 10.0));

        grid.clear();

        // Cell still exists in map (capacity preserved), just empty
        assert!(grid.cells.contains_key(&cell));
        assert!(grid.cells[&cell].is_empty());
    }

    #[test]
    fn test_insert_after_clear() {
        let mut grid = SpatialGrid::default();
        grid.insert(entity(1), Vec2::new(10.0, 10.0));
        grid.clear();
        grid.insert(entity(2), Vec2::new(10.0, 10.0));

        let result = grid.query_range(Vec2::new(10.0, 10.0), 1.0);
        assert_eq!(result.len(), 1);
        assert!(result.contains(&entity(2)));
    }

    #[test]
    fn test_zero_range_returns_same_cell() {
        let mut grid = SpatialGrid::default();
        grid.insert(entity(1), Vec2::new(10.0, 10.0));

        // Zero range still queries the center cell + adjacent due to ceil+1
        let result = grid.query_range(Vec2::new(10.0, 10.0), 0.0);
        assert!(result.contains(&entity(1)));
    }

    #[test]
    fn test_negative_positions() {
        let mut grid = SpatialGrid::default();
        let e = entity(1);
        grid.insert(e, Vec2::new(-200.0, -300.0));

        let result = grid.query_range(Vec2::new(-200.0, -300.0), 10.0);
        assert!(result.contains(&e));
    }

    #[test]
    fn test_entity_cells_tracking() {
        let mut grid = SpatialGrid::default();
        let e = entity(1);
        grid.insert(e, Vec2::new(100.0, 100.0));

        assert_eq!(grid.entity_cells.get(&e), Some(&(1, 1)));
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

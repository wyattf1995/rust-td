# Algorithm Anti-Patterns Catalog

**Project**: Neon Command (rust-td)
**Date**: 2026-02-06

This document catalogs the recurring anti-patterns found during the algorithm audit, with codebase-specific examples and standard fixes.

---

## Anti-Pattern 1: Linear Scan for Grid-Based Lookup

**Occurrences Found**: 1

**Locations**:
- `src/game/tower.rs:1040-1044`

**Pattern Description**:
When data is organized on a fixed grid (towers placed on tiles), iterating over ALL elements to find which one occupies a specific grid cell:
```rust
// Scans all towers to find the one at (check_x, check_y)
for (other_entity, other_x, other_y, other_type) in &tower_data {
    if *other_x == check_x as usize && *other_y == check_y as usize {
        adjacent_types.push(*other_type);
    }
}
```

**Why It's Problematic**:
Grid-based data has natural spatial indexing. A `HashMap<(usize, usize), TowerType>` provides O(1) lookup by coordinates. Scanning all elements is O(n) per lookup, and when done inside a loop over all towers x 8 neighbors, becomes O(n²).

**Standard Fix**:
Build a HashMap indexed by grid coordinates before the query loop:
```rust
let tower_grid: HashMap<(usize, usize), TowerType> = towers
    .iter()
    .map(|(_, t, _)| ((t.grid_x, t.grid_y), t.tower_type))
    .collect();

// O(1) lookup per neighbor
if let Some(&neighbor_type) = tower_grid.get(&(check_x as usize, check_y as usize)) {
    adjacent_types.push(neighbor_type);
}
```

**Codebase-Specific Notes**:
The grid is fixed at 18x11 tiles. A 2D array `[[Option<TowerType>; 11]; 18]` would be even faster than a HashMap (no hashing overhead), but HashMap is simpler and the grid is small enough that it doesn't matter.

---

## Anti-Pattern 2: Bypassing Existing Spatial Index

**Occurrences Found**: 2

**Locations**:
- `src/game/projectile.rs:297-309` (splash damage)
- `src/game/projectile.rs:344-361` (chain bounce target)

**Pattern Description**:
A spatial index exists (`SpatialGrid`) and is used in some systems (`tower_targeting`), but other systems that need range queries iterate over all entities instead:
```rust
// Splash: iterates ALL enemies to find those in radius
for (other_entity, mut other_enemy, other_transform) in &mut enemies {
    if enemy_pos.distance(other_pos) < splash_radius {
        // ...
    }
}
```

**Why It's Problematic**:
The `SpatialGrid` already partitions enemies into 64px cells for efficient range queries. Not using it in `projectile_collision` means O(E) scans per splash/chain event, when O(nearby) would suffice. As enemy count grows in later waves (40-80+), this becomes the bottleneck.

**Standard Fix**:
Inject the `SpatialGrid` resource into the system and use `query_range()`:
```rust
fn projectile_collision(
    // ... existing params ...
    spatial_grid: Res<SpatialGrid>,  // Add this
) {
    // Replace full enemy scan with spatial query
    let nearby = spatial_grid.query_range(enemy_pos, splash_radius);
    for entity in nearby {
        if let Ok((_, mut enemy, transform)) = enemies.get_mut(entity) {
            // ... apply splash damage
        }
    }
}
```

**Codebase-Specific Notes**:
The `SpatialGrid` is already a `Resource` updated every frame. Adding it as a system parameter is a one-line change. The only consideration is that `projectile_collision` needs `&mut enemies` for damage application, while `SpatialGrid` returns Entity IDs — use `enemies.get_mut(entity)` to bridge the two.

---

## Anti-Pattern 3: Recalculating Derived State on Every Access

**Occurrences Found**: 5

**Locations**:
- `src/game/tower.rs:298-303` (`upgrade()` — calculates multipliers)
- `src/game/tower.rs:319-324` (`preview_upgrade()` — recalculates from scratch)
- `src/game/tower.rs:335-338` (`attack_speed()` — recalculates speed multiplier)
- `src/game/tower.rs:347` (`buff_percentage()` — recalculates buff)
- `src/game/tower.rs:357` (`buff_percentage_next()` — recalculates buff)

**Pattern Description**:
Methods that recompute a value from first principles (iterating over all levels) every time they're called, even though the value only changes on discrete events (upgrades):
```rust
pub fn attack_speed(&self) -> f32 {
    let mut speed_mult = 1.0;
    for lvl in 2..=self.level {  // Re-derives from scratch every call
        let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
        speed_mult *= 1.0 + bonus * 0.6;
    }
    self.tower_type.attack_speed() * speed_mult
}
```

**Why It's Problematic**:
- `buff_percentage()` is called inside `update_buff_auras`, which runs every frame for every tower pair
- `attack_speed()` is read whenever tower stats are displayed
- The value only changes when `upgrade()` is called (rare event)
- O(L) per call instead of O(1) with caching

**Standard Fix**:
Cache the computed value in the struct and update it only when the input changes:
```rust
pub struct Tower {
    // Cache derived values
    cached_attack_speed: f32,
    cached_buff_pct: f32,
}

impl Tower {
    pub fn upgrade(&mut self) {
        self.level += 1;
        // ... calculate multipliers ...
        self.cached_attack_speed = self.tower_type.attack_speed() * speed_mult;
        self.cached_buff_pct = self.compute_buff_pct();
    }

    pub fn attack_speed(&self) -> f32 {
        self.cached_attack_speed  // O(1)
    }
}
```

**Codebase-Specific Notes**:
Bevy's component model makes cached fields natural — just add fields to the `Tower` component. The `preview_upgrade()` method still needs to compute from scratch (it's for preview of a level that hasn't happened yet), but it's only called when the context menu is open, so that's acceptable.

---

## Anti-Pattern 4: Vec::contains() for Membership Checks

**Occurrences Found**: 2 distinct patterns (10+ individual call sites)

**Locations**:
- `src/game/map.rs:137,139,152,154,182,184,201,203` (path generation)
- `src/game/tower.rs:1052,1059,1066,1073,1080` (synergy type checks)

**Pattern Description**:
Using `Vec::contains()` for repeated membership testing:
```rust
// Path generation: O(P) per check, called ~8x per iteration
if !path.contains(&(x, target_y)) {
    // ...
}

// Synergy checking: O(A) per check, 5 checks (A = adjacent count, max 8)
if adjacent_types.contains(&TowerType::Sniper) {
    // ...
}
```

**Why It's Problematic**:
`Vec::contains()` is O(n) linear search. When called repeatedly as the collection grows, cumulative cost is O(n²). For the path generation case, with ~100 nodes and ~8 checks per iteration, this is ~80,000 comparisons.

**Standard Fix**:
Maintain a parallel `HashSet` for O(1) membership checks:
```rust
let mut path = Vec::with_capacity(120);
let mut path_set = HashSet::with_capacity(120);

path.push(pos);
path_set.insert(pos);

// O(1) check
if !path_set.contains(&(x, target_y)) { ... }
```

**Codebase-Specific Notes**:
For the tower synergy case (`adjacent_types`), the Vec has max 8 elements, so linear search is likely faster than HashSet overhead. The fix here is only worthwhile for the path generation case (50-150 elements). The synergy `adjacent_types` Vec is fine as-is — 5 calls x max 8 items = 40 comparisons, well below HashSet's constant factor overhead.

---

## Anti-Pattern 5: Heap Allocation for Static Content

**Occurrences Found**: 5

**Locations**:
- `src/game/ui.rs:951` (`" [SPEED]".to_string()`)
- `src/game/ui.rs:952` (`" [ARMOR]".to_string()`)
- `src/game/ui.rs:953` (`" [SWARM]".to_string()`)
- `src/game/ui.rs:954` (`" [REGEN]".to_string()`)
- `src/game/ui.rs:955` (`" [GOLD!]".to_string()`)

**Pattern Description**:
Converting string literals to owned `String` when a borrowed `&str` suffices:
```rust
let modifier = match wave_manager.current_modifier {
    WaveModifier::SpeedBoost => " [SPEED]".to_string(),  // Heap allocation
    // ...
};
```

**Why It's Problematic**:
Each `.to_string()` allocates on the heap, copies bytes, and later requires deallocation. For static content known at compile time, this is pure waste. The `modifier` variable is used only in a `format!()` call immediately after, which accepts `&str`.

**Standard Fix**:
```rust
let modifier: &str = match wave_manager.current_modifier {
    WaveModifier::SpeedBoost => " [SPEED]",
    // ...
};
```

**Codebase-Specific Notes**:
This is called in `update_wave_text` which updates every frame when wave state changes. While the individual allocation is tiny (~8 bytes), it's a heap round-trip that's trivially avoidable. Rust's `format!()` macro accepts `&str` and `String` interchangeably via the `Display` trait.

---

## Anti-Pattern 6: Missing Vec Pre-allocation

**Occurrences Found**: 3

**Locations**:
- `src/game/enemy.rs:318` (`Vec::new()` for 10-80 enemies)
- `src/game/map.rs:111` (`Vec::new()` for 50-150 path nodes)
- `src/game/spatial.rs:42` (`Vec::new()` in query_range, called per tower per frame)

**Pattern Description**:
Creating a Vec with unknown-but-estimable size without providing a capacity hint:
```rust
let mut enemies = Vec::new();  // Starts at capacity 0
for _ in 0..count {
    enemies.push((EnemyType::Basic, multiplier));  // Grows 0->1->2->4->8->16->32->64
}
```

**Why It's Problematic**:
Vec doubles its capacity on each reallocation. For 60 elements, that's 7 reallocations, each copying all existing elements. With `with_capacity(60)`, there are zero reallocations.

**Standard Fix**:
```rust
let mut enemies = Vec::with_capacity(base_count as usize + 10);
```

**Codebase-Specific Notes**:
- `enemy.rs`: `base_count` is calculated before the Vec is populated — use it directly
- `map.rs`: Path length is bounded by grid size (18x11 = 198 max), use `with_capacity(120)`
- `spatial.rs`: `query_range` is the hottest path — estimate based on cell count and typical density

---

## Anti-Pattern 7: Intermediate Collection for Immediate Consumption

**Occurrences Found**: 1

**Locations**:
- `src/game/ui.rs:1665-1668`

**Pattern Description**:
Collecting iterator results into a Vec just to call `.join()`:
```rust
let synergy_strs: Vec<String> = syn.active.iter()
    .map(|s| format!("⚡ {}: {}", s.name(), s.description()))
    .collect();
text.sections[0].value = synergy_strs.join("\n");
```

**Why It's Problematic**:
Creates n+1 String allocations (n format! results + 1 join result) when a single String built incrementally would suffice. The intermediate Vec exists only to call `.join()`.

**Standard Fix**:
```rust
use std::fmt::Write;

let mut result = String::new();
for (i, s) in syn.active.iter().enumerate() {
    if i > 0 { result.push('\n'); }
    write!(result, "⚡ {}: {}", s.name(), s.description()).unwrap();
}
text.sections[0].value = result;
```

**Codebase-Specific Notes**:
With only 1-4 synergies max, the absolute savings are minimal. But the pattern is worth fixing as it demonstrates unnecessary allocations. Using `write!()` instead of `format!()` avoids intermediate String creation.

---

## Summary Table

| Anti-Pattern | Occurrences | Severity | Fix Effort |
|-------------|-------------|----------|------------|
| Linear scan for grid lookup | 1 | Critical | Small |
| Bypassing spatial index | 2 | High | Small |
| Recalculating derived state | 5 | Medium | Medium |
| Vec::contains() for membership | 10+ sites | Low-Med | Small |
| Heap allocation for static strings | 5 | Low | Trivial |
| Missing Vec pre-allocation | 3 | Low | Trivial |
| Intermediate collection | 1 | Low | Trivial |

# Algorithm Complexity & Data Structure Audit

**Project**: Neon Command (rust-td)
**Date**: 2026-02-06
**Codebase**: ~7,500 lines of Rust (Bevy ECS game engine, compiled to WASM)
**Context**: Browser-based tower defense game running at 60fps

---

## Executive Summary

The codebase is generally well-optimized for a real-time game. The spatial hash grid for enemy lookups is a strong architectural choice. However, there are **2 critical issues** (tower synergy O(T x 8 x T) nested loop and splash damage O(E) scan per hit) that run every frame and will degrade as entity counts grow. The remaining findings are medium/low priority — stat recalculations, missing pre-allocations, and minor data structure mismatches.

### Top 5 Worst-Case Complexity Functions

| Rank | Function | Location | Complexity | Frequency |
|------|----------|----------|------------|-----------|
| 1 | `update_tower_synergies` | tower.rs:1004 | O(T x 8 x T) ≈ O(T²) | Every frame |
| 2 | Splash damage scan | projectile.rs:297 | O(E) per splash hit | Per splash projectile collision |
| 3 | Chain bounce target search | projectile.rs:344 | O(E) per bounce | Per chain projectile collision |
| 4 | `update_buff_auras` | tower.rs:948 | O(B x T x L) | Every frame |
| 5 | `generate_path` (.contains) | map.rs:109 | O(P²) cumulative | Once at game start |

*T = towers, E = enemies, B = buff towers, L = buff level, P = path length*

---

## Issue 1: Tower Synergy Adjacency Check — Triple Nested Loop

**Location**: `src/game/tower.rs:1004-1087`

**Function**: `update_tower_synergies`

**Current Code**:
```rust
let tower_data: Vec<(Entity, usize, usize, TowerType)> = towers
    .iter()
    .map(|(e, t, _)| (e, t.grid_x, t.grid_y, t.tower_type))
    .collect();

for (entity, tower, mut synergies) in &mut towers {
    // ...
    let mut adjacent_types: Vec<TowerType> = Vec::new();
    for (ox, oy) in adjacent_offsets {                          // 8 iterations
        let check_x = grid_x as i32 + ox;
        let check_y = grid_y as i32 + oy;
        if check_x >= 0 && check_y >= 0 {
            for (other_entity, other_x, other_y, other_type) in &tower_data {  // T iterations
                if *other_entity != entity
                    && *other_x == check_x as usize
                    && *other_y == check_y as usize
                {
                    adjacent_types.push(*other_type);
                }
            }
        }
    }
    // Then 5x .contains() calls on adjacent_types...
}
```

**Current Complexity**:
- Time: O(T x 8 x T) = O(T²) where T = number of towers
- Space: O(T) for tower_data + O(8) for adjacent_types

**Typical Input Size**: T = 20-50 towers in a typical game

**Call Frequency**: Every frame (60fps)

**Estimated Impact**: At 30 towers: 30 x 8 x 30 = 7,200 comparisons per frame. At 50 towers: 50 x 8 x 50 = 20,000 comparisons per frame.

**Problem Description**:
For each tower, the inner loop scans ALL towers to find which ones are at each of the 8 adjacent grid positions. This is a classic "join without index" anti-pattern. The grid is already a spatial structure (18x11 tiles), but we're doing a linear scan instead of a direct lookup.

**Recommended Fix**:
```rust
fn update_tower_synergies(
    mut towers: Query<(Entity, &Tower, &mut TowerSynergies)>,
) {
    // Build a grid-indexed HashMap: O(T)
    let mut tower_grid: HashMap<(usize, usize), TowerType> = HashMap::new();
    for (_, tower, _) in towers.iter() {
        tower_grid.insert((tower.grid_x, tower.grid_y), tower.tower_type);
    }

    for (entity, tower, mut synergies) in &mut towers {
        synergies.active.clear();
        synergies.range_bonus = 0.0;
        synergies.poison_duration_bonus = 0.0;
        synergies.speed_buff_multiplier = 1.0;
        synergies.extra_chain_bounces = 0;

        let adjacent_offsets: [(i32, i32); 8] = [
            (-1, -1), (0, -1), (1, -1),
            (-1, 0),           (1, 0),
            (-1, 1),  (0, 1),  (1, 1),
        ];

        // O(1) lookup per neighbor instead of O(T)
        let mut adjacent_types: Vec<TowerType> = Vec::with_capacity(8);
        for (ox, oy) in adjacent_offsets {
            let check_x = tower.grid_x as i32 + ox;
            let check_y = tower.grid_y as i32 + oy;
            if check_x >= 0 && check_y >= 0 {
                if let Some(&tower_type) = tower_grid.get(&(check_x as usize, check_y as usize)) {
                    adjacent_types.push(tower_type);
                }
            }
        }

        // Synergy matching (unchanged)
        match tower.tower_type {
            TowerType::Sniper => {
                if adjacent_types.contains(&TowerType::Sniper) {
                    synergies.active.push(SynergyType::SniperPair);
                    synergies.range_bonus = 0.15;
                }
            }
            // ... rest unchanged
        }
    }
}
```

**New Complexity**:
- Time: O(T) to build grid + O(T x 8 x 1) = O(T) total
- Space: O(T) for grid HashMap

**Expected Improvement**: At 50 towers: from 20,000 ops/frame to ~450 ops/frame (~44x faster)

**Priority**: CRITICAL

**Dependencies**: None. Drop-in replacement.

---

## Issue 2: Splash Damage Full Enemy Scan

**Location**: `src/game/projectile.rs:293-309`

**Function**: `projectile_collision`

**Current Code**:
```rust
// Splash damage
if projectile.tower_type == TowerType::Splash {
    let splash_radius = ShapeSizes::SPLASH_RADIUS;
    let splash_damage = projectile.damage * 0.5;

    for (other_entity, mut other_enemy, other_transform) in &mut enemies {
        if other_entity == enemy_entity {
            continue;
        }
        let other_pos = other_transform.translation.truncate();
        if enemy_pos.distance(other_pos) < splash_radius {
            // ... apply damage
        }
    }
}
```

**Current Complexity**:
- Time: O(E) per splash collision, where E = total alive enemies
- Space: O(1)

**Typical Input Size**: E = 20-60 enemies on screen

**Call Frequency**: Per splash projectile collision (multiple per frame with splash towers)

**Estimated Impact**: With 3 splash towers at 0.7 attack speed: ~2 hits/sec each = ~6 splash scans/sec. At wave 15 with 40 enemies: 240 distance checks per second. Manageable but avoidable.

**Problem Description**:
Splash checks iterate over ALL enemies to find those in the splash radius. The SpatialGrid already exists for tower targeting but isn't used here.

**Recommended Fix**:
```rust
if projectile.tower_type == TowerType::Splash {
    let splash_radius = ShapeSizes::SPLASH_RADIUS;
    let splash_damage = projectile.damage * 0.5;

    // Use spatial grid for splash range query
    let nearby = spatial_grid.query_range(enemy_pos, splash_radius);
    for other_entity in nearby {
        if other_entity == enemy_entity { continue; }
        if let Ok((_, mut other_enemy, other_transform)) = enemies.get_mut(other_entity) {
            let other_pos = other_transform.translation.truncate();
            if enemy_pos.distance(other_pos) < splash_radius {
                let other_armor = other_enemy.total_armor();
                let other_actual_damage = splash_damage * (1.0 - other_armor);
                other_enemy.health = (other_enemy.health - other_actual_damage).max(0.0);
                spawn_damage_number(&mut commands, &assets, other_pos, other_actual_damage);
            }
        }
    }
}
```

**New Complexity**:
- Time: O(nearby) where nearby << E (typically 3-8 enemies in splash radius)
- Space: O(nearby) for query result vec

**Expected Improvement**: With 40 enemies on screen, checking ~5 nearby instead of 40 = ~8x fewer distance calculations

**Priority**: HIGH

**Dependencies**: Requires passing `SpatialGrid` as parameter to `projectile_collision` system (currently not injected).

---

## Issue 3: Chain Bounce Target Search — Full Enemy Scan

**Location**: `src/game/projectile.rs:338-361`

**Function**: `projectile_collision` (chain bounce section)

**Current Code**:
```rust
for (origin_pos, bounce_damage, hit_list, remaining_bounces) in chain_bounces {
    let mut best_target: Option<(Entity, f32)> = None;
    let bounce_range = ShapeSizes::CHAIN_BOUNCE_RANGE;

    for (entity, enemy, transform) in &enemies {
        if hit_list.contains(&entity) || enemy.marked_dead || enemy.health <= 0.0 {
            continue;
        }
        let enemy_pos = transform.translation.truncate();
        let dist = origin_pos.distance(enemy_pos);
        if dist <= bounce_range {
            if let Some((_, best_dist)) = best_target {
                if dist < best_dist {
                    best_target = Some((entity, dist));
                }
            } else {
                best_target = Some((entity, dist));
            }
        }
    }
}
```

**Current Complexity**:
- Time: O(E) per bounce, up to 3-4 bounces = O(3E) per chain projectile
- Space: O(1) search + O(bounces) for hit_list

**Typical Input Size**: E = 20-60 enemies

**Call Frequency**: Per chain projectile collision (multiple per frame with chain towers)

**Problem Description**:
Same pattern as splash — full enemy scan when spatial grid would narrow the search. Additionally, `hit_list` is a `Vec<Entity>` using `.contains()` for O(n) membership checks (though n is small, max 4).

**Recommended Fix**:
```rust
// Use spatial grid for bounce range query
let nearby = spatial_grid.query_range(origin_pos, bounce_range);
for entity in nearby {
    if hit_list.contains(&entity) { continue; }
    if let Ok((_, enemy, transform)) = enemies.get(entity) {
        if enemy.marked_dead || enemy.health <= 0.0 { continue; }
        // ... same nearest-target logic
    }
}
```

**New Complexity**:
- Time: O(nearby) per bounce where nearby << E
- Space: Same

**Expected Improvement**: ~5-10x fewer checks per bounce

**Priority**: HIGH

**Dependencies**: Same as Issue 2 — requires `SpatialGrid` in `projectile_collision`.

---

## Issue 4: Repeated Stat Multiplier Calculations

**Location**: `src/game/tower.rs:288-359`

**Functions**: `upgrade()`, `preview_upgrade()`, `attack_speed()`, `buff_percentage()`, `buff_percentage_next()`

**Current Code**:
```rust
// upgrade() - lines 298-303
for lvl in 2..=self.level {
    let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
    damage_mult *= 1.0 + bonus;
    range_mult *= 1.0 + bonus * 0.4;
    speed_mult *= 1.0 + bonus * 0.6;
}

// preview_upgrade() - lines 319-324 (nearly identical)
for lvl in 2..=next_level {
    let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
    damage_mult *= 1.0 + bonus;
    range_mult *= 1.0 + bonus * 0.4;
    speed_mult *= 1.0 + bonus * 0.6;
}

// attack_speed() - lines 335-338 (duplicated partial calc)
for lvl in 2..=self.level {
    let bonus = 0.20 / (1.0 + (lvl - 2) as f32 * 0.15);
    speed_mult *= 1.0 + bonus * 0.6;
}

// buff_percentage() - line 347
let level_bonus: f32 = (1..self.level).map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3)).sum();

// buff_percentage_next() - line 357 (same but next_level)
let level_bonus: f32 = (1..next_level).map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3)).sum();
```

**Current Complexity**:
- Time: O(L) per call where L = tower level. Called multiple times per frame for UI display.
- Space: O(1)

**Typical Input Size**: L = 1-15 (typical tower levels)

**Call Frequency**: `attack_speed()` indirectly used every frame; `buff_percentage()` called every frame in `update_buff_auras`; `preview_upgrade()` called when context menu is open.

**Problem Description**:
Five methods recalculate multipliers from scratch by iterating from level 2 to current level. The tower's `upgrade()` method already computes these — the results should be cached in the Tower struct rather than recalculated on every access.

**Recommended Fix**:
```rust
pub struct Tower {
    // ... existing fields ...
    // Cached multipliers (updated on upgrade)
    cached_speed_mult: f32,
    cached_buff_pct: f32,
}

pub fn upgrade(&mut self) {
    self.level += 1;
    // ... existing loop calculates damage_mult, range_mult, speed_mult ...
    self.damage = self.tower_type.damage() * damage_mult;
    self.range = self.tower_type.range() * range_mult;
    self.cached_speed_mult = speed_mult;

    if self.tower_type == TowerType::Buff {
        let level_bonus: f32 = (1..self.level)
            .map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3))
            .sum();
        self.cached_buff_pct = 0.25 + level_bonus;
    }

    let attack_speed = self.tower_type.attack_speed() * speed_mult;
    let cooldown_secs = if attack_speed > 0.0 { 1.0 / attack_speed } else { 1.0 };
    self.attack_cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Repeating);
}

pub fn attack_speed(&self) -> f32 {
    self.tower_type.attack_speed() * self.cached_speed_mult  // O(1) now
}

pub fn buff_percentage(&self) -> f32 {
    if self.tower_type != TowerType::Buff { return 0.0; }
    self.cached_buff_pct  // O(1) now
}
```

**New Complexity**:
- Time: O(1) for all stat lookups; O(L) only on upgrade (called once per upgrade event)
- Space: O(1) — two extra f32 fields per tower

**Expected Improvement**: Eliminates redundant O(L) loops from every-frame systems. At level 10 with 30 towers, saves ~300 iterations/frame from `update_buff_auras` alone.

**Priority**: MEDIUM

**Dependencies**: Need to set `cached_speed_mult = 1.0` and `cached_buff_pct = 0.25` in Tower::new/default.

---

## Issue 5: Buff Aura Level Bonus Recalculated Per-Tower-Per-Buff

**Location**: `src/game/tower.rs:970-977`

**Function**: `update_buff_auras`

**Current Code**:
```rust
for (buff_pos, buff_range, buff_level) in &buff_sources {
    if tower_pos.distance(*buff_pos) <= *buff_range {
        // Recalculates from scratch for each tower in range
        let level_bonus: f32 = (1..*buff_level)
            .map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3))
            .sum();
        total_buff += 0.25 + level_bonus;
    }
}
```

**Current Complexity**:
- Time: O(B x T x L) where B = buff towers, T = non-buff towers, L = buff level
- Space: O(B) for buff_sources

**Problem Description**:
The buff percentage calculation iterates over levels for EVERY tower checked against EVERY buff source. The buff percentage depends only on the buff tower's level — not on which tower is being buffed. It should be calculated once per buff source.

**Recommended Fix**:
```rust
// Pre-calculate buff percentages: O(B x L)
let buff_sources: Vec<(Vec2, f32, f32)> = buff_towers
    .iter()
    .filter(|(t, _)| t.tower_type == TowerType::Buff)
    .map(|(t, transform)| {
        let pct = t.buff_percentage();  // Or use cached value from Issue 4 fix
        (transform.translation.truncate(), t.range, pct)
    })
    .collect();

// Now inner loop is O(B x T) with no level iteration
for (buff_pos, buff_range, buff_pct) in &buff_sources {
    if tower_pos.distance(*buff_pos) <= *buff_range {
        total_buff += buff_pct;
    }
}
```

**New Complexity**:
- Time: O(B x L) + O(B x T) = O(B x T) (L term eliminated from hot loop)
- Space: Same

**Expected Improvement**: With 3 buff towers at level 5, 25 non-buff towers: from 375 level iterations/frame to 15.

**Priority**: MEDIUM

**Dependencies**: Benefits from Issue 4 fix (cached buff_percentage).

---

## Issue 6: Path Generation Uses Vec.contains() Instead of HashSet

**Location**: `src/game/map.rs:109-236`

**Function**: `generate_path`

**Current Code**:
```rust
let mut path = Vec::new();
// ...
// Called 8+ times throughout the function:
if !path.contains(&(x, target_y)) {  // O(P) each time
    // ...
}
```

**Current Complexity**:
- Time: O(P) per `.contains()` call, called ~8 times per loop iteration, over ~100 iterations = O(P² x 8)
- Space: O(P)

**Typical Input Size**: P = 50-150 path nodes

**Call Frequency**: Once at game start (map generation)

**Problem Description**:
`Vec::contains()` is O(n) for each call. During path generation, this is called repeatedly as the path grows. A `HashSet` alongside the Vec would provide O(1) membership checks.

**Recommended Fix**:
```rust
fn generate_path() -> Vec<(usize, usize)> {
    let mut rng = rand::thread_rng();
    let mut path = Vec::with_capacity(120);
    let mut path_set = HashSet::with_capacity(120);  // Shadow set for O(1) lookups

    let start_y = rng.gen_range(2..(GRID_HEIGHT - 2));
    path.push((0, start_y));
    path_set.insert((0, start_y));

    // ... replace all path.contains(&pos) with path_set.contains(&pos)
    // ... on every path.push(pos), also path_set.insert(pos)
}
```

**New Complexity**:
- Time: O(P) total (O(1) per contains check)
- Space: O(P) — duplicated in HashSet

**Expected Improvement**: At 100 path nodes: from ~80,000 comparisons to ~1,000. ~80x faster path generation.

**Priority**: LOW (only runs once at game start, but easy to fix)

**Dependencies**: Add `use std::collections::HashSet;` (already imported in map.rs for obstacles).

---

## Issue 7: Enemy Wave Generation Without Pre-allocation

**Location**: `src/game/enemy.rs:317-456`

**Function**: `generate_wave_enemies`

**Current Code**:
```rust
fn generate_wave_enemies(&self, wave_num: usize) -> Vec<(EnemyType, f32)> {
    let mut enemies = Vec::new();  // No capacity hint
    // ... many .push() calls (20-80+ enemies)
}
```

**Current Complexity**:
- Time: O(n) with ~log₂(n) reallocations due to Vec growth
- Space: O(n) with wasted capacity from doubling strategy

**Typical Input Size**: 10-80 enemies per wave

**Call Frequency**: Once per wave start (~every 30-60 seconds)

**Problem Description**:
Vec starts at capacity 0 and doubles on each reallocation. For 60 enemies: 0 -> 1 -> 2 -> 4 -> 8 -> 16 -> 32 -> 64 = 7 reallocations with copies. The `base_count` is already calculated and could be used.

**Recommended Fix**:
```rust
fn generate_wave_enemies(&self, wave_num: usize) -> Vec<(EnemyType, f32)> {
    // ... calculate base_count as before ...
    let estimated_size = (base_count * 1.5) as usize + 5;  // Conservative overestimate
    let mut enemies = Vec::with_capacity(estimated_size);
    // ... rest unchanged
}
```

**New Complexity**:
- Time: O(n) with 0 reallocations
- Space: O(n) with minimal waste

**Expected Improvement**: Eliminates 5-7 reallocations per wave. Negligible in absolute time but good practice.

**Priority**: LOW

**Dependencies**: None.

---

## Issue 8: Static Strings Allocated as Owned String

**Location**: `src/game/ui.rs:949-956`

**Function**: `update_wave_text`

**Current Code**:
```rust
let modifier = match wave_manager.current_modifier {
    WaveModifier::None => String::new(),
    WaveModifier::SpeedBoost => " [SPEED]".to_string(),
    WaveModifier::ArmoredWave => " [ARMOR]".to_string(),
    WaveModifier::Swarm => " [SWARM]".to_string(),
    WaveModifier::Regen => " [REGEN]".to_string(),
    WaveModifier::GoldRush => " [GOLD!]".to_string(),
};
```

**Current Complexity**:
- Time: O(1) per allocation, but heap allocation for static content
- Space: One heap allocation per frame when wave text updates

**Problem Description**:
These are constant strings being allocated on the heap every time the wave text updates. Should use `&str` literals.

**Recommended Fix**:
```rust
let modifier: &str = match wave_manager.current_modifier {
    WaveModifier::None => "",
    WaveModifier::SpeedBoost => " [SPEED]",
    WaveModifier::ArmoredWave => " [ARMOR]",
    WaveModifier::Swarm => " [SWARM]",
    WaveModifier::Regen => " [REGEN]",
    WaveModifier::GoldRush => " [GOLD!]",
};
```

**New Complexity**: Zero heap allocations for static content

**Priority**: LOW

**Dependencies**: None.

---

## Issue 9: Synergy Display Creates Intermediate Vec<String>

**Location**: `src/game/ui.rs:1665-1668`

**Function**: `update_tower_context_menu`

**Current Code**:
```rust
let synergy_strs: Vec<String> = syn.active.iter()
    .map(|s| format!("⚡ {}: {}", s.name(), s.description()))
    .collect();
text.sections[0].value = synergy_strs.join("\n");
```

**Current Complexity**:
- Time: O(n) with n String allocations + 1 join allocation
- Space: O(n) intermediate Vec<String>

**Problem Description**:
Creates a Vec of Strings just to join them. For 1-4 synergies, this allocates 2-5 Strings (the Vec elements plus the final join). Can be done in a single String.

**Recommended Fix**:
```rust
let mut synergy_text = String::new();
for (i, s) in syn.active.iter().enumerate() {
    if i > 0 { synergy_text.push('\n'); }
    synergy_text.push_str(&format!("⚡ {}: {}", s.name(), s.description()));
}
text.sections[0].value = synergy_text;
```

**New Complexity**: O(n) with 1 String allocation instead of n+1

**Priority**: LOW

**Dependencies**: None.

---

## Issue 10: SpatialGrid.query_range Returns Unfiltered Results

**Location**: `src/game/spatial.rs:41-58`

**Function**: `query_range`

**Current Code**:
```rust
pub fn query_range(&self, center: Vec2, range: f32) -> Vec<Entity> {
    let mut result = Vec::new();
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
```

**Current Complexity**:
- Time: O(C²) where C = (range/64 + 1) * 2 + 1. For range=150: C=7, 49 cell lookups
- Space: O(result_count) — allocates a new Vec every call

**Problem Description**:
1. Allocates a new Vec on every call (called per tower per frame in `tower_targeting`)
2. Checks a square of cells but range is circular — includes corners that are definitely out of range
3. No pre-allocation hint for result Vec

**Recommended Fix**:
```rust
pub fn query_range(&self, center: Vec2, range: f32) -> Vec<Entity> {
    let cells_to_check = (range / CELL_SIZE).ceil() as i32 + 1;
    let center_cell = Self::world_to_cell(center);
    let estimated = (cells_to_check * 2 + 1).pow(2) as usize * 2;  // Rough estimate
    let mut result = Vec::with_capacity(estimated);

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
```

**New Complexity**: Same asymptotic, but eliminates Vec reallocations

**Priority**: LOW

**Dependencies**: None.

---

## Issue 11: SpatialGrid Rebuilt from Scratch Every Frame

**Location**: `src/game/spatial.rs:84-95`

**Function**: `update_spatial_grid`

**Current Code**:
```rust
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
```

**Current Complexity**:
- Time: O(E) per frame to rebuild (clear all cells + re-insert all enemies)
- Space: O(E) — HashMap entries recreated every frame

**Problem Description**:
The grid is cleared and rebuilt every frame. For a game where most enemies move smoothly, incremental updates (only moving entities that changed cells) would be more efficient. However, Bevy's ECS makes change detection easy and the current approach is idiomatic. The bigger issue is that `clear()` deallocates all the inner Vecs. Using `retain` + `drain` or resetting counts without deallocating would avoid repeated allocation.

**Recommended Fix**:
```rust
pub fn clear(&mut self) {
    // Keep allocated memory, just clear contents
    for entities in self.cells.values_mut() {
        entities.clear();
    }
    self.entity_cells.clear();
}
```

**New Complexity**: Same O(E), but avoids HashMap entry deallocation/reallocation

**Priority**: LOW

**Dependencies**: None.

---

## Prioritized Fix List

| Priority | Issue | Est. Improvement | Effort |
|----------|-------|------------------|--------|
| CRITICAL | #1 Tower synergy grid lookup | 44x fewer ops at 50 towers | Small (15 min) |
| HIGH | #2 Splash uses spatial grid | 8x fewer distance checks | Small (10 min) |
| HIGH | #3 Chain bounce uses spatial grid | 5-10x fewer checks | Small (10 min) |
| MEDIUM | #4 Cache stat multipliers | Eliminate O(L) per frame | Medium (30 min) |
| MEDIUM | #5 Pre-calc buff percentages | 25x fewer level iterations | Small (10 min) |
| LOW | #6 Path gen HashSet | 80x faster (one-time) | Small (15 min) |
| LOW | #7 Enemy Vec pre-allocation | Eliminate reallocations | Trivial (2 min) |
| LOW | #8 Static string refs | Eliminate heap allocs | Trivial (2 min) |
| LOW | #9 Synergy display string | Fewer allocations | Trivial (5 min) |
| LOW | #10 SpatialGrid pre-alloc | Fewer Vec reallocations | Trivial (5 min) |
| LOW | #11 SpatialGrid incremental clear | Fewer HashMap ops | Small (10 min) |

---

## Analysis Checklist

### Nested Loops
- [x] All nested loops identified (Issues #1, #2, #3, spatial grid)
- [x] Each assessed for necessity
- [x] Map/HashMap alternatives evaluated (Issue #1)
- [x] Input sizes documented

### Data Structures
- [x] All array/Vec lookups reviewed
- [x] Frequent membership checks evaluated for Set usage (Issues #1 synergy, #6 path)
- [x] Key-value access reviewed for HashMap (Issue #1)
- [x] Data structure matches access pattern (spatial grid: good; tower synergy: needs fix)

### Searching & Sorting
- [x] No unnecessary sorting found (deterministic shuffle is fine)
- [x] No sorted data requiring binary search
- [x] Search results not cached but spatial grid provides locality
- [x] Partial results not used where possible (splash scans all enemies)

### Recursion
- [x] Only Bevy's `despawn_recursive()` — framework method, appropriate usage
- [x] No user-written recursion found
- [x] No memoization needed
- [x] No stack depth concerns

### String Operations
- [x] No string concat in hot loops (analytics is cold path)
- [x] No regex used in codebase
- [x] format! in UI guarded by change detection (mostly acceptable)
- [x] Static string heap allocation identified (Issue #8)

### Hot Paths
- [x] `tower_targeting` analyzed — uses spatial grid (good)
- [x] `update_tower_synergies` analyzed — O(T²) (Issue #1, critical)
- [x] `update_buff_auras` analyzed — redundant calc (Issues #4, #5)
- [x] `projectile_collision` analyzed — missing spatial grid (Issues #2, #3)
- [x] `update_spatial_grid` analyzed — full rebuild per frame (Issue #11)

---

## Positive Findings

These architectural decisions are well done:

1. **SpatialGrid for tower targeting** (spatial.rs) — Excellent O(nearby) instead of O(E) for finding targets
2. **Bevy change detection guards** — UI updates only fire when resources change (`.is_changed()`)
3. **ECS architecture** — Components are small value types, cache-friendly iteration
4. **No unnecessary cloning** — Font handle clones are cheap (reference-counted handles)
5. **Object pooling infrastructure** (pool.rs) — Ready for entity reuse
6. **No unsafe code** — Entire codebase is safe Rust
7. **Timer-based cooldowns** — Avoids per-frame conditional checks for attack timing

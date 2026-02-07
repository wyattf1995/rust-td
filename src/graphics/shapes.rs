use bevy::prelude::*;

/// =============================================================================
/// NEON COMMAND - Visual Design System
/// =============================================================================
/// Theme: Military-tech with dark metallic bases and distinct neon accents
/// - All towers share a dark gunmetal base for cohesion
/// - Each tower type has a unique neon accent color
/// - Grid uses subtle borders and lane-marked paths
/// - Enemies use warm/threat colors (reds, oranges, purples)
/// =============================================================================

/// Color palette for the game
pub struct GameColors;

impl GameColors {
    // ==========================================================================
    // CORE PALETTE - Foundation colors used throughout
    // ==========================================================================

    /// Bright accent for important UI elements
    pub const PRIMARY: Color = Color::srgb(0.0, 0.85, 0.95);      // Cyan
    pub const SECONDARY: Color = Color::srgb(0.95, 0.3, 0.4);     // Coral red
    pub const ACCENT: Color = Color::srgb(0.4, 0.95, 0.5);        // Mint green

    // ==========================================================================
    // TOWER COLORS - Dark base + Neon accent per type
    // ==========================================================================
    // Each tower uses TOWER_BASE as primary, these are the accent/glow colors

    /// Tower base color (shared by all towers)
    pub const TOWER_BASE: Color = Color::srgb(0.18, 0.2, 0.24);

    /// Tower accent colors (unique per type)
    pub const TOWER_BASIC: Color = Color::srgb(0.3, 0.7, 1.0);    // Sky blue
    pub const TOWER_SPLASH: Color = Color::srgb(1.0, 0.5, 0.2);   // Orange
    pub const TOWER_SLOW: Color = Color::srgb(0.4, 0.9, 0.95);    // Ice cyan
    pub const TOWER_SNIPER: Color = Color::srgb(0.85, 0.3, 0.85); // Magenta
    pub const TOWER_RAPID: Color = Color::srgb(1.0, 0.85, 0.2);   // Gold
    pub const TOWER_CHAIN: Color = Color::srgb(0.3, 0.6, 1.0);    // Electric blue
    pub const TOWER_POISON: Color = Color::srgb(0.5, 1.0, 0.3);   // Toxic green
    pub const TOWER_BUFF: Color = Color::srgb(1.0, 0.9, 0.5);     // Warm gold

    /// Tower barrel (darker than base for depth)
    pub const TOWER_BARREL: Color = Color::srgb(0.1, 0.1, 0.12);

    // ==========================================================================
    // GRID & MAP COLORS
    // ==========================================================================

    /// Empty buildable tile
    pub const TILE_EMPTY: Color = Color::srgb(0.1, 0.11, 0.14);

    /// Tile border/grid lines
    pub const TILE_BORDER: Color = Color::srgb(0.16, 0.17, 0.2);

    /// Path base color (warmer, more visible "road")
    pub const PATH: Color = Color::srgb(0.18, 0.17, 0.20);

    /// Path lane markings
    pub const PATH_LANE: Color = Color::srgb(0.30, 0.30, 0.36);

    /// Path direction indicators
    pub const PATH_INDICATOR: Color = Color::srgba(0.45, 0.48, 0.55, 0.65);

    /// Hover states
    pub const TILE_HOVER_BUILD: Color = Color::srgb(0.15, 0.35, 0.2);   // Green tint
    pub const TILE_HOVER_BLOCKED: Color = Color::srgb(0.35, 0.15, 0.15); // Red tint
    pub const TILE_HOVER_TOWER: Color = Color::srgb(0.2, 0.25, 0.35);   // Blue tint

    /// Range indicator
    pub const RANGE_INDICATOR: Color = Color::srgba(0.3, 0.8, 1.0, 0.15);

    // ==========================================================================
    // TERRAIN COLORS - Muted, doesn't compete with gameplay elements
    // ==========================================================================

    pub const TERRAIN_ROCK: Color = Color::srgb(0.16, 0.15, 0.14);
    pub const TERRAIN_ROCK_DARK: Color = Color::srgb(0.11, 0.1, 0.09);
    pub const TERRAIN_WATER: Color = Color::srgb(0.08, 0.14, 0.22);
    pub const TERRAIN_WATER_LIGHT: Color = Color::srgb(0.1, 0.18, 0.28);
    pub const TERRAIN_FOREST: Color = Color::srgb(0.08, 0.14, 0.1);
    pub const TERRAIN_FOREST_LIGHT: Color = Color::srgb(0.1, 0.18, 0.12);
    pub const TERRAIN_CRYSTAL: Color = Color::srgb(0.14, 0.1, 0.18);
    pub const TERRAIN_LAVA: Color = Color::srgb(0.2, 0.08, 0.06);

    // ==========================================================================
    // ENEMY COLORS - Warm threat colors, clearly distinct from towers
    // ==========================================================================

    pub const ENEMY_BASIC: Color = Color::srgb(0.9, 0.35, 0.3);    // Red
    pub const ENEMY_FAST: Color = Color::srgb(1.0, 0.75, 0.25);    // Amber
    pub const ENEMY_TANK: Color = Color::srgb(0.55, 0.4, 0.55);    // Dusty purple
    pub const ENEMY_ARMORED: Color = Color::srgb(0.5, 0.55, 0.6);  // Steel gray
    pub const ENEMY_FLYING: Color = Color::srgb(0.7, 0.85, 1.0);   // Light sky
    pub const ENEMY_BOSS: Color = Color::srgb(0.8, 0.15, 0.2);     // Deep crimson
    pub const ENEMY_SPLITTER: Color = Color::srgb(0.7, 0.35, 0.8); // Violet
    pub const ENEMY_MINI_SPLITTER: Color = Color::srgb(0.8, 0.5, 0.9); // Light violet

    // ==========================================================================
    // PROJECTILE COLORS - Matching tower accents but brighter/glowing
    // ==========================================================================

    pub const PROJECTILE_BASIC: Color = Color::srgb(0.5, 0.85, 1.0);
    pub const PROJECTILE_SPLASH: Color = Color::srgb(1.0, 0.6, 0.3);
    pub const PROJECTILE_SLOW: Color = Color::srgb(0.6, 0.95, 1.0);
    pub const PROJECTILE_SNIPER: Color = Color::srgb(1.0, 0.5, 1.0);
    pub const PROJECTILE_RAPID: Color = Color::srgb(1.0, 0.9, 0.4);
    pub const PROJECTILE_CHAIN: Color = Color::srgb(0.5, 0.8, 1.0);
    pub const PROJECTILE_POISON: Color = Color::srgb(0.6, 1.0, 0.4);

    // ==========================================================================
    // UI COLORS
    // ==========================================================================

    pub const GOLD: Color = Color::srgb(1.0, 0.85, 0.25);
    pub const HEALTH_HIGH: Color = Color::srgb(0.3, 0.9, 0.4);
    pub const HEALTH_MID: Color = Color::srgb(0.95, 0.75, 0.25);
    pub const HEALTH_LOW: Color = Color::srgb(0.95, 0.3, 0.25);
    pub const HEALTH_BAR_BG: Color = Color::srgb(0.12, 0.12, 0.14);
    pub const SUCCESS: Color = Color::srgb(0.6, 1.0, 0.7);

    pub const BUTTON_NORMAL: Color = Color::srgb(0.14, 0.15, 0.18);
    pub const BUTTON_HOVER: Color = Color::srgb(0.2, 0.22, 0.26);
    pub const BUTTON_SELECTED: Color = Color::srgb(0.25, 0.28, 0.34);
    pub const BUTTON_START: Color = Color::srgb(0.15, 0.4, 0.25);
    pub const BUTTON_START_HOVER: Color = Color::srgb(0.2, 0.5, 0.3);
    pub const BUTTON_START_PRESSED: Color = Color::srgb(0.1, 0.3, 0.18);

    pub const UI_OVERLAY: Color = Color::srgba(0.04, 0.04, 0.06, 0.85);
    pub const GAME_OVER_OVERLAY: Color = Color::srgba(0.02, 0.02, 0.03, 0.9);
    // ==========================================================================
    // EFFECT COLORS
    // ==========================================================================

    pub const DAMAGE_TEXT: Color = Color::srgb(1.0, 0.4, 0.35);
    pub const GOLD_TEXT: Color = Color::srgb(1.0, 0.9, 0.3);
    pub const MUZZLE_FLASH: Color = Color::srgb(1.0, 0.95, 0.7);
    pub const DEATH_EFFECT: Color = Color::srgba(1.0, 0.5, 0.35, 0.9);

    pub const ABILITY_FREEZE: Color = Color::srgb(0.5, 0.95, 1.0);
    pub const ABILITY_GOLD_RUSH: Color = Color::srgb(1.0, 0.9, 0.3);
    pub const ABILITY_NUKE: Color = Color::srgb(1.0, 0.45, 0.25);

}

/// Shape sizes - refined for visual polish
pub struct ShapeSizes;

impl ShapeSizes {
    // ==========================================================================
    // GRID
    // ==========================================================================
    pub const TILE: f32 = 50.0;
    pub const TILE_INNER: f32 = 46.0;  // TILE - TILE_GAP*2 for visual border

    // ==========================================================================
    // TOWERS
    // ==========================================================================
    pub const TOWER: f32 = 38.0;           // Main tower body
    pub const TOWER_CORE: f32 = 28.0;      // Inner accent area
    pub const TOWER_BARREL_WIDTH: f32 = 6.0;
    pub const TOWER_BARREL_HEIGHT: f32 = 18.0;

    // ==========================================================================
    // ENEMIES
    // ==========================================================================
    pub const ENEMY_BASIC: f32 = 28.0;
    pub const ENEMY_FAST: f32 = 22.0;
    pub const ENEMY_TANK: f32 = 38.0;
    pub const ENEMY_ARMORED: f32 = 32.0;
    pub const ENEMY_FLYING: f32 = 24.0;
    pub const ENEMY_BOSS: f32 = 52.0;
    pub const ENEMY_SPLITTER: f32 = 32.0;
    pub const ENEMY_MINI_SPLITTER: f32 = 16.0;
    // ==========================================================================
    // PROJECTILES
    // ==========================================================================
    pub const PROJECTILE_BASIC: f32 = 7.0;
    pub const PROJECTILE_SPLASH: f32 = 10.0;
    pub const PROJECTILE_SLOW: f32 = 8.0;
    pub const PROJECTILE_TRAIL: f32 = 3.0;

    // ==========================================================================
    // HEALTH BARS
    // ==========================================================================
    pub const HEALTH_BAR_HEIGHT: f32 = 4.0;
    pub const HEALTH_BAR_BG_HEIGHT: f32 = 6.0;
    pub const HEALTH_BAR_OFFSET: f32 = 8.0;

    // ==========================================================================
    // MAP ELEMENTS
    // ==========================================================================
    pub const PATH_INDICATOR: f32 = 4.0;
    pub const PATH_LANE_WIDTH: f32 = 2.5;
    pub const SPAWN_INDICATOR: f32 = 16.0;
    pub const EXIT_INDICATOR: f32 = 16.0;

    // ==========================================================================
    // EFFECTS
    // ==========================================================================
    pub const SPLASH_RADIUS: f32 = 60.0;
    pub const CHAIN_BOUNCE_RANGE: f32 = 100.0;
    pub const BUFF_AURA_RANGE: f32 = 120.0;
    pub const DAMAGE_TEXT_SIZE: f32 = 14.0;
    pub const MUZZLE_FLASH: f32 = 12.0;
}

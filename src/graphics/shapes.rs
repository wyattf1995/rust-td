use bevy::prelude::*;

/// Color palette for the game
pub struct GameColors;

impl GameColors {
    // Primary colors
    pub const PRIMARY: Color = Color::srgb(0.91, 0.27, 0.38); // #E94560
    pub const SECONDARY: Color = Color::srgb(0.06, 0.2, 0.38); // #0F3460
    pub const BACKGROUND: Color = Color::srgb(0.1, 0.1, 0.18); // #1A1A2E
    pub const ACCENT: Color = Color::srgb(0.09, 0.13, 0.24); // #16213E

    // UI colors
    pub const GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
    pub const HEALTH_HIGH: Color = Color::srgb(0.2, 0.8, 0.3);
    pub const HEALTH_MID: Color = Color::srgb(0.9, 0.7, 0.2);
    pub const HEALTH_LOW: Color = Color::srgb(0.9, 0.3, 0.2);
    pub const SUCCESS: Color = Color::srgb(0.2, 0.8, 0.4);

    // Tower colors
    pub const TOWER_BASIC: Color = Color::srgb(0.3, 0.6, 0.9);
    pub const TOWER_SPLASH: Color = Color::srgb(0.9, 0.5, 0.2);
    pub const TOWER_SLOW: Color = Color::srgb(0.4, 0.8, 0.9);
    pub const TOWER_SNIPER: Color = Color::srgb(0.6, 0.2, 0.6);  // Purple
    pub const TOWER_RAPID: Color = Color::srgb(0.9, 0.8, 0.2);   // Yellow
    pub const TOWER_BARREL: Color = Color::srgb(0.2, 0.2, 0.3);

    // Enemy colors
    pub const ENEMY_BASIC: Color = Color::srgb(0.8, 0.3, 0.3);
    pub const ENEMY_FAST: Color = Color::srgb(0.9, 0.7, 0.2);
    pub const ENEMY_TANK: Color = Color::srgb(0.5, 0.3, 0.6);

    // Map colors
    pub const PATH: Color = Color::srgb(0.2, 0.2, 0.3);
    pub const TILE_EMPTY: Color = Color::srgb(0.15, 0.15, 0.22);
    pub const TILE_BLOCKED: Color = Color::srgb(0.1, 0.1, 0.15);
    pub const TILE_HOVER: Color = Color::srgb(0.25, 0.25, 0.35);
    pub const PATH_INDICATOR: Color = Color::srgba(0.4, 0.4, 0.5, 0.5);
    pub const RANGE_INDICATOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);

    // Projectile colors
    pub const PROJECTILE_BASIC: Color = Color::srgb(0.5, 0.8, 1.0);
    pub const PROJECTILE_SPLASH: Color = Color::srgb(1.0, 0.6, 0.3);
    pub const PROJECTILE_SLOW: Color = Color::srgb(0.6, 0.9, 1.0);
    pub const PROJECTILE_SNIPER: Color = Color::srgb(0.8, 0.4, 0.9);  // Purple
    pub const PROJECTILE_RAPID: Color = Color::srgb(1.0, 0.9, 0.3);   // Yellow

    // UI button colors
    pub const BUTTON_NORMAL: Color = Color::srgb(0.2, 0.2, 0.3);
    pub const BUTTON_HOVER: Color = Color::srgb(0.3, 0.3, 0.4);
    pub const BUTTON_SELECTED: Color = Color::srgb(0.4, 0.4, 0.5);
    pub const BUTTON_START: Color = Color::srgb(0.2, 0.6, 0.3);
    pub const BUTTON_START_HOVER: Color = Color::srgb(0.3, 0.7, 0.4);
    pub const BUTTON_START_PRESSED: Color = Color::srgb(0.1, 0.4, 0.2);

    // Health bar background
    pub const HEALTH_BAR_BG: Color = Color::srgb(0.2, 0.2, 0.2);
    pub const UI_OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
    pub const GAME_OVER_OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.8);

    // Additional enemy colors
    pub const ENEMY_ARMORED: Color = Color::srgb(0.4, 0.45, 0.5);  // Steel gray
    pub const ENEMY_FLYING: Color = Color::srgb(0.6, 0.8, 1.0);    // Sky blue
    pub const ENEMY_BOSS: Color = Color::srgb(0.7, 0.1, 0.2);      // Dark red

    // Effect colors
    pub const DAMAGE_TEXT: Color = Color::srgb(1.0, 0.3, 0.3);
    pub const GOLD_TEXT: Color = Color::srgb(1.0, 0.85, 0.0);
    pub const MUZZLE_FLASH: Color = Color::srgb(1.0, 0.9, 0.6);
    pub const DEATH_EFFECT: Color = Color::srgba(1.0, 0.5, 0.3, 0.8);

    // Ability colors
    pub const ABILITY_FREEZE: Color = Color::srgb(0.5, 0.9, 1.0);
    pub const ABILITY_GOLD_RUSH: Color = Color::srgb(1.0, 0.85, 0.0);
    pub const ABILITY_NUKE: Color = Color::srgb(1.0, 0.4, 0.2);
}

/// Shape sizes
pub struct ShapeSizes;

impl ShapeSizes {
    pub const TILE: f32 = 50.0;
    pub const TILE_GAP: f32 = 2.0;
    pub const TOWER: f32 = 42.0;
    pub const TOWER_BARREL_WIDTH: f32 = 8.0;
    pub const TOWER_BARREL_HEIGHT: f32 = 20.0;
    pub const ENEMY_BASIC: f32 = 30.0;
    pub const ENEMY_FAST: f32 = 24.0;
    pub const ENEMY_TANK: f32 = 40.0;
    pub const PROJECTILE_BASIC: f32 = 8.0;
    pub const PROJECTILE_SPLASH: f32 = 12.0;
    pub const PROJECTILE_SLOW: f32 = 10.0;
    pub const PROJECTILE_TRAIL: f32 = 4.0;
    pub const HEALTH_BAR_HEIGHT: f32 = 4.0;
    pub const HEALTH_BAR_BG_HEIGHT: f32 = 6.0;
    pub const HEALTH_BAR_OFFSET: f32 = 8.0;
    pub const PATH_INDICATOR: f32 = 6.0;
    pub const SPAWN_INDICATOR: f32 = 20.0;
    pub const SPLASH_RADIUS: f32 = 60.0;

    // Additional enemy sizes
    pub const ENEMY_ARMORED: f32 = 35.0;
    pub const ENEMY_FLYING: f32 = 26.0;
    pub const ENEMY_BOSS: f32 = 55.0;

    // Effect sizes
    pub const DAMAGE_TEXT_SIZE: f32 = 16.0;
    pub const MUZZLE_FLASH: f32 = 15.0;
    pub const DEATH_EFFECT: f32 = 40.0;
}

use bevy::prelude::*;
use std::collections::HashMap;

use crate::GameState;
use crate::graphics::shapes::{GameColors, ShapeSizes, ZDepth};
use crate::persistence::GameSettings;

use crate::loading::GameAssets;

use super::{
    ease_out,
    economy::PlayerEconomy,
    enemy::Enemy,
    map::GameMap,
    projectile::SpawnProjectileEvent,
    rules,
    spatial::SpatialGrid,
    stats::GameStats,
    GameEntity,
    GameSet,
};

/// Set to true when towers are placed or sold — triggers synergy recalculation.
#[derive(Resource, Default)]
pub struct SynergyDirty(pub bool);

/// Tracks the most recent tower placement for undo (3-second window, 100% refund).
#[derive(Resource, Default)]
pub struct RecentPlacement {
    pub tower_entity: Option<Entity>,
    pub cost: u32,
    pub grid_x: usize,
    pub grid_y: usize,
    pub timer: Option<Timer>,
}

/// Dirty flag for buff aura recalculation (set on tower place/sell/upgrade)
#[derive(Resource, Default)]
pub struct BuffDirty(pub bool);

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedTowerType>()
            .init_resource::<HoveredTower>()
            .init_resource::<SelectedPlacedTower>()
            .init_resource::<SynergyDirty>()
            .init_resource::<BuffDirty>()
            .init_resource::<RecentPlacement>()
            .add_event::<PlaceTowerEvent>()
            .add_event::<SellTowerEvent>()
            .add_event::<UpgradeTowerEvent>()
            .add_event::<SpecializeTowerEvent>()
            .add_systems(
                Update,
                (
                    tower_hotkeys,
                    tower_undo,
                    handle_tower_placement,
                    handle_tower_selling,
                    handle_tower_upgrade,
                    handle_tower_specialization,
                    tower_targeting,
                    update_buff_auras,
                    update_tower_synergies,
                    update_synergy_indicators,
                    tower_attack,
                )
                    .in_set(GameSet::TowerLogic)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    update_tower_visuals,
                    update_range_indicators,
                    update_muzzle_flashes,
                    update_level_badges,
                    update_buff_aura_visuals,
                    update_upgrade_flashes,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Currently selected placed tower for selling/upgrading
#[derive(Resource, Default)]
pub struct SelectedPlacedTower(pub Option<Entity>);

/// Event to sell a tower
#[derive(Event)]
pub struct SellTowerEvent {
    pub tower: Entity,
}

/// Event to upgrade a tower
#[derive(Event)]
pub struct UpgradeTowerEvent {
    pub tower: Entity,
}

/// Muzzle flash effect
#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: Timer,
}

/// Targeting priority for towers
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TargetingPriority {
    #[default]
    First,      // Furthest along the path (default, best for preventing leaks)
    Closest,    // Nearest to tower (best for rapid-fire towers)
    LowestHP,   // Lowest current health (good for finishing enemies)
    HighestHP,  // Highest current health (focus on tanks)
    Fastest,    // Fastest moving (pick off speedsters)
}

impl TargetingPriority {
    pub fn name(&self) -> &'static str {
        match self {
            TargetingPriority::First => "First",
            TargetingPriority::Closest => "Closest",
            TargetingPriority::LowestHP => "Lowest HP",
            TargetingPriority::HighestHP => "Highest HP",
            TargetingPriority::Fastest => "Fastest",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TargetingPriority::First => TargetingPriority::Closest,
            TargetingPriority::Closest => TargetingPriority::LowestHP,
            TargetingPriority::LowestHP => TargetingPriority::HighestHP,
            TargetingPriority::HighestHP => TargetingPriority::Fastest,
            TargetingPriority::Fastest => TargetingPriority::First,
        }
    }
}

/// Tower types available
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TowerType {
    #[default]
    Basic,  // Single target, balanced
    Splash, // AOE damage
    Slow,   // Slows enemies
    Sniper, // Long range, high damage, slow
    Rapid,  // Short range, fast attacks
    Chain,  // Lightning bounces between enemies
    Poison, // Deals damage over time
    Buff,   // Boosts nearby towers (doesn't attack)
}

/// Tower specializations — chosen at level 3 for towers that branch
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Specialization {
    // Basic
    Marksman,    // +damage, +range, -speed
    Gunner,      // +speed, -damage per shot
    // Splash
    Napalm,      // Lingering ground DOT zone on impact
    Shockwave,   // Knockback + larger splash radius
    // Slow
    Cryogenic,   // Freeze chance on hit (full stop 1s)
    Blizzard,    // AOE slow field
    // Sniper
    Railgun,     // Pierces through enemies
    Assassin,    // Crit chance for 3x damage
    // Rapid
    Minigun,     // Attack speed ramps up on same target
    Shotgun,     // Fires 3-5 projectiles in cone
    // Chain
    Tesla,       // +2 extra bounces, wider range
    Arc,         // Each bounce creates small AOE
}

impl Specialization {
    /// Get the two specialization choices for a given tower type
    pub fn choices_for(tower_type: TowerType) -> Option<[Specialization; 2]> {
        match tower_type {
            TowerType::Basic => Some([Specialization::Marksman, Specialization::Gunner]),
            TowerType::Splash => Some([Specialization::Napalm, Specialization::Shockwave]),
            TowerType::Slow => Some([Specialization::Cryogenic, Specialization::Blizzard]),
            TowerType::Sniper => Some([Specialization::Railgun, Specialization::Assassin]),
            TowerType::Rapid => Some([Specialization::Minigun, Specialization::Shotgun]),
            TowerType::Chain => Some([Specialization::Tesla, Specialization::Arc]),
            _ => None, // Buff and Poison stay linear
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Specialization::Marksman => "Marksman",
            Specialization::Gunner => "Gunner",
            Specialization::Napalm => "Napalm",
            Specialization::Shockwave => "Shockwave",
            Specialization::Cryogenic => "Cryogenic",
            Specialization::Blizzard => "Blizzard",
            Specialization::Railgun => "Railgun",
            Specialization::Assassin => "Assassin",
            Specialization::Minigun => "Minigun",
            Specialization::Shotgun => "Shotgun",
            Specialization::Tesla => "Tesla",
            Specialization::Arc => "Arc",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Specialization::Marksman => "+DMG, +Range, -Speed",
            Specialization::Gunner => "+Speed, -DMG per shot",
            Specialization::Napalm => "Leaves fire zones on impact",
            Specialization::Shockwave => "Knockback + bigger splash",
            Specialization::Cryogenic => "Chance to freeze enemies",
            Specialization::Blizzard => "Creates AOE slow fields",
            Specialization::Railgun => "Pierces through enemies",
            Specialization::Assassin => "25% crit for 3x damage",
            Specialization::Minigun => "Speed ramps on same target",
            Specialization::Shotgun => "Fires 3-5 in a cone",
            Specialization::Tesla => "+2 bounces, wider range",
            Specialization::Arc => "AOE at each bounce point",
        }
    }

    /// Unique accent color for each specialization (for visual distinction)
    pub fn color(&self) -> Color {
        match self {
            Specialization::Marksman => GameColors::SPEC_MARKSMAN,
            Specialization::Gunner => GameColors::SPEC_GUNNER,
            Specialization::Napalm => GameColors::SPEC_NAPALM,
            Specialization::Shockwave => GameColors::SPEC_SHOCKWAVE,
            Specialization::Cryogenic => GameColors::SPEC_CRYOGENIC,
            Specialization::Blizzard => GameColors::SPEC_BLIZZARD,
            Specialization::Railgun => GameColors::SPEC_RAILGUN,
            Specialization::Assassin => GameColors::SPEC_ASSASSIN,
            Specialization::Minigun => GameColors::SPEC_MINIGUN,
            Specialization::Shotgun => GameColors::SPEC_SHOTGUN,
            Specialization::Tesla => GameColors::SPEC_TESLA,
            Specialization::Arc => GameColors::SPEC_ARC,
        }
    }

    /// Short initial letter for level badge display
    pub fn initial(&self) -> &'static str {
        match self {
            Specialization::Marksman => "M",
            Specialization::Gunner => "G",
            Specialization::Napalm => "N",
            Specialization::Shockwave => "S",
            Specialization::Cryogenic => "C",
            Specialization::Blizzard => "B",
            Specialization::Railgun => "R",
            Specialization::Assassin => "A",
            Specialization::Minigun => "M",
            Specialization::Shotgun => "S",
            Specialization::Tesla => "T",
            Specialization::Arc => "A",
        }
    }

    /// Returns (damage_mult, range_mult, speed_mult) modifiers
    pub fn stat_modifiers(&self) -> (f32, f32, f32) {
        match self {
            Specialization::Marksman => (1.4, 1.2, 0.8),
            Specialization::Gunner => (0.7, 1.0, 1.6),
            Specialization::Napalm => (1.0, 1.0, 1.0),
            Specialization::Shockwave => (0.9, 1.0, 1.0),
            Specialization::Cryogenic => (1.0, 1.1, 1.0),
            Specialization::Blizzard => (0.8, 1.2, 1.0),
            Specialization::Railgun => (1.3, 1.15, 0.85),
            Specialization::Assassin => (1.0, 1.0, 1.1),
            Specialization::Minigun => (0.85, 1.0, 1.0),
            Specialization::Shotgun => (0.65, 0.95, 0.85),
            Specialization::Tesla => (0.9, 1.0, 1.0),
            Specialization::Arc => (0.85, 1.0, 1.0),
        }
    }
}

/// Event to specialize a tower
#[derive(Event)]
pub struct SpecializeTowerEvent {
    pub tower: Entity,
    pub specialization: Specialization,
}

impl TowerType {
    pub fn cost(&self) -> u32 {
        match self {
            TowerType::Basic => 50,
            TowerType::Splash => 100,
            TowerType::Slow => 75,
            TowerType::Sniper => 150,
            TowerType::Rapid => 100,    // Nerfed from 80
            TowerType::Chain => 120,
            TowerType::Poison => 90,
            TowerType::Buff => 200,
        }
    }

    pub fn damage(&self) -> f32 {
        match self {
            TowerType::Basic => 25.0,   // Restored - cost nerf is enough
            TowerType::Splash => 15.0,
            TowerType::Slow => 10.0,
            TowerType::Sniper => 80.0,
            TowerType::Rapid => 8.0,
            TowerType::Chain => 30.0,   // Per hit, bounces at 70%
            TowerType::Poison => 12.0,  // Initial + DOT
            TowerType::Buff => 0.0,     // Doesn't attack
        }
    }

    pub fn range(&self) -> f32 {
        match self {
            TowerType::Basic => 150.0,
            TowerType::Splash => 120.0,
            TowerType::Slow => 130.0,
            TowerType::Sniper => 280.0,
            TowerType::Rapid => 100.0,
            TowerType::Chain => 140.0,
            TowerType::Poison => 130.0,
            TowerType::Buff => 120.0,   // Aura range
        }
    }

    pub fn attack_speed(&self) -> f32 {
        match self {
            TowerType::Basic => 1.0,
            TowerType::Splash => 0.7,
            TowerType::Slow => 1.5,
            TowerType::Sniper => 0.4,
            TowerType::Rapid => 4.0,
            TowerType::Chain => 0.8,
            TowerType::Poison => 1.2,
            TowerType::Buff => 0.0,     // No attacks
        }
    }

    pub fn color(&self) -> Color {
        match self {
            TowerType::Basic => GameColors::TOWER_BASIC,
            TowerType::Splash => GameColors::TOWER_SPLASH,
            TowerType::Slow => GameColors::TOWER_SLOW,
            TowerType::Sniper => GameColors::TOWER_SNIPER,
            TowerType::Rapid => GameColors::TOWER_RAPID,
            TowerType::Chain => GameColors::TOWER_CHAIN,
            TowerType::Poison => GameColors::TOWER_POISON,
            TowerType::Buff => GameColors::TOWER_BUFF,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TowerType::Basic => "Basic",
            TowerType::Splash => "Splash",
            TowerType::Slow => "Slow",
            TowerType::Sniper => "Sniper",
            TowerType::Rapid => "Rapid",
            TowerType::Chain => "Chain",
            TowerType::Poison => "Poison",
            TowerType::Buff => "Buff",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TowerType::Basic => "Balanced single-target damage",
            TowerType::Splash => "Area damage to groups",
            TowerType::Slow => "Slows enemy movement",
            TowerType::Sniper => "Long range, high damage",
            TowerType::Rapid => "Fast attacks, short range",
            TowerType::Chain => "Lightning bounces 3 times",
            TowerType::Poison => "Deals damage over time",
            TowerType::Buff => "Buffs nearby towers (scales)",
        }
    }

    pub fn projectile_color(&self) -> Color {
        match self {
            TowerType::Basic => GameColors::PROJECTILE_BASIC,
            TowerType::Splash => GameColors::PROJECTILE_SPLASH,
            TowerType::Slow => GameColors::PROJECTILE_SLOW,
            TowerType::Sniper => GameColors::PROJECTILE_SNIPER,
            TowerType::Rapid => GameColors::PROJECTILE_RAPID,
            TowerType::Chain => GameColors::PROJECTILE_CHAIN,
            TowerType::Poison => GameColors::PROJECTILE_POISON,
            TowerType::Buff => GameColors::PROJECTILE_BASIC, // Buff doesn't shoot
        }
    }

    pub fn projectile_size(&self) -> f32 {
        match self {
            TowerType::Basic => ShapeSizes::PROJECTILE_BASIC,
            TowerType::Splash => ShapeSizes::PROJECTILE_SPLASH,
            TowerType::Slow => ShapeSizes::PROJECTILE_SLOW,
            TowerType::Sniper => ShapeSizes::PROJECTILE_SNIPER,
            TowerType::Rapid => ShapeSizes::PROJECTILE_RAPID,
            TowerType::Chain => ShapeSizes::PROJECTILE_CHAIN,
            TowerType::Poison => ShapeSizes::PROJECTILE_POISON,
            TowerType::Buff => ShapeSizes::PROJECTILE_BUFF,
        }
    }

    /// Whether this tower can attack enemies
    pub fn can_attack(&self) -> bool {
        !matches!(self, TowerType::Buff)
    }
}

/// Currently selected tower type for placement
#[derive(Resource, Default)]
pub struct SelectedTowerType(pub TowerType);

/// Currently hovered tower (for showing range)
#[derive(Resource, Default)]
pub struct HoveredTower(pub Option<Entity>);

/// Tower component
#[derive(Component)]
pub struct Tower {
    pub tower_type: TowerType,
    pub range: f32,
    pub damage: f32,
    pub attack_cooldown: Timer,
    pub target: Option<Entity>,
    pub grid_x: usize,
    pub grid_y: usize,
    pub level: u32,
    pub targeting: TargetingPriority,
    pub specialization: Option<Specialization>,
    /// Minigun ramp-up: time spent targeting same enemy
    pub ramp_up_timer: f32,
    /// Minigun ramp-up: last target entity
    pub ramp_up_target: Option<Entity>,
    /// Cached cumulative speed multiplier (updated on upgrade, avoids O(level) recalc)
    cached_speed_mult: f32,
    /// Cached buff percentage for Buff towers (updated on upgrade, avoids O(level) recalc)
    cached_buff_pct: f32,
}

impl Tower {
    pub fn new(tower_type: TowerType, grid_x: usize, grid_y: usize) -> Self {
        let attack_speed = tower_type.attack_speed();
        // For towers that don't attack (like Buff), use a dummy timer
        let cooldown_secs = if attack_speed > 0.0 { 1.0 / attack_speed } else { 1.0 };
        Self {
            tower_type,
            range: tower_type.range(),
            damage: tower_type.damage(),
            attack_cooldown: Timer::from_seconds(cooldown_secs, TimerMode::Repeating),
            target: None,
            grid_x,
            grid_y,
            level: 1,
            targeting: TargetingPriority::default(),
            specialization: None,
            ramp_up_timer: 0.0,
            ramp_up_target: None,
            cached_speed_mult: 1.0,
            cached_buff_pct: if tower_type == TowerType::Buff { 0.25 } else { 0.0 },
        }
    }

    /// Returns the spec color if specialized, otherwise the base tower type color
    pub fn accent_color(&self) -> Color {
        self.specialization.map(|s| s.color()).unwrap_or_else(|| self.tower_type.color())
    }

    pub fn cycle_targeting(&mut self) {
        self.targeting = self.targeting.next();
    }

    pub fn sell_value(&self) -> u32 {
        rules::sell_value(self.tower_type.cost(), self.level)
    }

    pub fn upgrade_cost(&self) -> u32 {
        rules::upgrade_cost(self.tower_type.cost(), self.level)
    }

    /// Returns true when tower is at level 2 and has branching options but hasn't chosen yet
    pub fn needs_specialization(&self) -> bool {
        self.level == 2
            && self.specialization.is_none()
            && Specialization::choices_for(self.tower_type).is_some()
    }

    pub fn can_upgrade(&self) -> bool {
        // Block upgrades when specialization choice is required
        !self.needs_specialization()
    }

    /// Apply specialization: sets spec, advances to level 3, recalculates stats
    pub fn specialize(&mut self, spec: Specialization) {
        self.specialization = Some(spec);
        // Specialization costs same as a normal level 2→3 upgrade
        self.upgrade(); // This handles level 3 stat recalc
    }

    pub fn upgrade(&mut self) {
        self.level += 1;

        // Update cached speed multiplier
        self.cached_speed_mult = rules::speed_multiplier(self.level);

        // Calculate stats via pure rules functions
        let spec_mods = self.specialization
            .map(|s| s.stat_modifiers())
            .unwrap_or((1.0, 1.0, 1.0));

        let (dmg, rng, spd) = rules::tower_stats_at_level(
            self.tower_type.damage(),
            self.tower_type.range(),
            self.tower_type.attack_speed(),
            self.level,
            spec_mods,
        );

        self.damage = dmg;
        self.range = rng;
        let cooldown_secs = if spd > 0.0 { 1.0 / spd } else { 1.0 };
        self.attack_cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Repeating);

        // Update cached buff percentage
        if self.tower_type == TowerType::Buff {
            self.cached_buff_pct = rules::buff_percentage(self.level);
        }
    }

    /// Calculate stats for next level (for preview — must compute from scratch)
    pub fn preview_upgrade(&self) -> (f32, f32, f32) {
        let spec_mods = self.specialization
            .map(|s| s.stat_modifiers())
            .unwrap_or((1.0, 1.0, 1.0));

        rules::tower_stats_at_level(
            self.tower_type.damage(),
            self.tower_type.range(),
            self.tower_type.attack_speed(),
            self.level + 1,
            spec_mods,
        )
    }

    /// Get current attack speed (O(1) — uses cached multiplier)
    pub fn attack_speed(&self) -> f32 {
        let spec_spd = self.specialization.map(|s| s.stat_modifiers().2).unwrap_or(1.0);
        self.tower_type.attack_speed() * self.cached_speed_mult * spec_spd
    }

    /// Get buff percentage for Buff towers (O(1) — uses cached value)
    pub fn buff_percentage(&self) -> f32 {
        self.cached_buff_pct
    }

    /// Get buff percentage at next level (must compute the delta)
    pub fn buff_percentage_next(&self) -> f32 {
        if self.tower_type != TowerType::Buff {
            return 0.0;
        }
        self.cached_buff_pct + rules::buff_percentage_next_delta(self.level)
    }
}

// =============================================================================
// TOWER VISUAL CONSTANTS
// =============================================================================
const BRIGHTNESS_PER_LEVEL: f32 = 0.12;
const CORE_SIZE_PER_LEVEL: f32 = 1.5;
const BARREL_BASE_BLEND: f32 = 0.85;
const BARREL_SPEC_BLEND: f32 = 0.15;
const BRACKET_ALPHA_BASE: f32 = 0.25;
const BRACKET_ALPHA_PER_LEVEL: f32 = 0.08;
const BRACKET_ALPHA_MAX: f32 = 0.85;
const LEVEL_RING_ALPHA: f32 = 0.12;
const LEVEL_RING_SIZE: f32 = 44.0;
const LEVEL_RING_MIN_LEVEL: u32 = 3;
const UPGRADE_FLASH_ALPHA: f32 = 0.6;
const UPGRADE_FLASH_START_SIZE: f32 = 30.0;
const UPGRADE_FLASH_END_SIZE: f32 = 50.0;
const UPGRADE_FLASH_DURATION: f32 = 0.15;

/// Range indicator component
#[derive(Component)]
pub struct RangeIndicator {
    pub tower: Entity,
}

/// Tower barrel component
#[derive(Component)]
pub struct TowerBarrel {
    pub tower: Entity,
}

/// Tower level badge (shows upgrade level)
#[derive(Component)]
pub struct TowerLevelBadge {
    pub tower: Entity,
}

/// Buff aura visual indicator
#[derive(Component)]
pub struct BuffAuraIndicator {
    pub tower: Entity,
}

/// Tower accent/core visual layer
#[derive(Component)]
pub struct TowerCore {
    pub tower: Entity,
}

/// Corner bracket marks around tower base
#[derive(Component)]
pub struct TowerBracket {
    pub tower: Entity,
}

/// Shadow beneath tower for visual grounding
#[derive(Component)]
pub struct TowerShadow {
    pub tower: Entity,
}

/// Level ring glow at level 3+ (44px square behind 38px base)
#[derive(Component)]
pub struct TowerLevelRing {
    pub tower: Entity,
}

/// Expanding white flash on upgrade/specialize
#[derive(Component)]
pub struct UpgradeFlash {
    pub lifetime: Timer,
    pub start_size: f32,
    pub end_size: f32,
}

/// Tracks if a tower is being buffed
#[derive(Component, Default)]
pub struct BuffedStatus {
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
}

/// Tower synergy types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SynergyType {
    SniperPair,      // Sniper + Sniper: +15% range each
    SlowPoison,      // Slow + Poison: Poison duration +50%
    RapidBuff,       // Rapid + Buff: Attack speed bonus doubled
    ChainPair,       // Chain + Chain: +1 bounce each
}

impl SynergyType {
    pub const ALL: [SynergyType; 4] = [
        SynergyType::SniperPair,
        SynergyType::SlowPoison,
        SynergyType::RapidBuff,
        SynergyType::ChainPair,
    ];

    pub fn towers_required(&self) -> (TowerType, TowerType) {
        match self {
            SynergyType::SniperPair => (TowerType::Sniper, TowerType::Sniper),
            SynergyType::SlowPoison => (TowerType::Slow, TowerType::Poison),
            SynergyType::RapidBuff => (TowerType::Rapid, TowerType::Buff),
            SynergyType::ChainPair => (TowerType::Chain, TowerType::Chain),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SynergyType::SniperPair => "Sniper Duo",
            SynergyType::SlowPoison => "Toxic Slow",
            SynergyType::RapidBuff => "Overdrive",
            SynergyType::ChainPair => "Lightning Storm",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SynergyType::SniperPair => "+15% range",
            SynergyType::SlowPoison => "+50% poison duration",
            SynergyType::RapidBuff => "2x speed buff",
            SynergyType::ChainPair => "+1 chain bounce",
        }
    }
}

/// Tracks active synergies on a tower
#[derive(Component, Default)]
pub struct TowerSynergies {
    pub active: Vec<SynergyType>,
    pub range_bonus: f32,           // Percentage bonus (0.15 = 15%)
    pub poison_duration_bonus: f32, // Percentage bonus
    pub speed_buff_multiplier: f32, // Multiplier for buff tower effect
    pub extra_chain_bounces: u32,   // Additional bounces for chain
}

/// Event to place a tower
#[derive(Event)]
pub struct PlaceTowerEvent {
    pub grid_x: usize,
    pub grid_y: usize,
    pub tower_type: TowerType,
}

fn handle_tower_placement(
    mut commands: Commands,
    mut events: EventReader<PlaceTowerEvent>,
    mut map: ResMut<GameMap>,
    mut economy: ResMut<PlayerEconomy>,
    mut synergy_dirty: ResMut<SynergyDirty>,
    mut buff_dirty: ResMut<BuffDirty>,
    mut stats: ResMut<GameStats>,
    mut recent: ResMut<RecentPlacement>,
    assets: Res<GameAssets>,
) {
    for event in events.read() {
        let cost = event.tower_type.cost();

        // Check if we can afford it and tile is buildable
        if economy.gold < cost || !map.is_buildable(event.grid_x, event.grid_y) {
            continue;
        }

        // Deduct cost
        economy.gold -= cost;
        synergy_dirty.0 = true;
        buff_dirty.0 = true;

        // Mark tile as occupied
        map.place_tower(event.grid_x, event.grid_y);

        // Spawn tower
        let pos = GameMap::grid_to_world(event.grid_x, event.grid_y);
        let tower = Tower::new(event.tower_type, event.grid_x, event.grid_y);
        let accent_color = event.tower_type.color();
        let range = tower.range;

        // Spawn base entity with tower component
        let tower_entity = commands
            .spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: GameColors::TOWER_BASE,
                        custom_size: Some(Vec2::splat(ShapeSizes::TOWER)),
                        ..default()
                    },
                    transform: Transform::from_translation(pos.extend(ZDepth::TOWER_BASE)),
                    ..default()
                },
                tower,
                TowerSynergies::default(),
                GameEntity,
            ))
            .id();

        stats.register_tower(tower_entity, event.tower_type, cost);
        spawn_tower_visuals(&mut commands, tower_entity, pos, accent_color, range, event.tower_type, &assets);

        // Record for undo (3-second window)
        recent.tower_entity = Some(tower_entity);
        recent.cost = cost;
        recent.grid_x = event.grid_x;
        recent.grid_y = event.grid_y;
        recent.timer = Some(Timer::from_seconds(3.0, TimerMode::Once));
    }
}

/// Spawn all visual child entities for a tower (core, barrel, badge, shadow, brackets, etc.)
fn spawn_tower_visuals(
    commands: &mut Commands,
    tower_entity: Entity,
    pos: Vec2,
    accent_color: Color,
    range: f32,
    tower_type: TowerType,
    assets: &GameAssets,
) {
    // Accent core (colored center showing tower type)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: accent_color,
                custom_size: Some(Vec2::splat(ShapeSizes::TOWER_CORE)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(ZDepth::TOWER_CORE)),
            ..default()
        },
        TowerCore { tower: tower_entity },
        GameEntity,
    ));

    // Range indicator (initially hidden)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::NONE,
                custom_size: Some(Vec2::splat(range * 2.0)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(ZDepth::RANGE_INDICATOR)),
            ..default()
        },
        RangeIndicator { tower: tower_entity },
        GameEntity,
    ));

    // Barrel/turret (dark, on top)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: GameColors::TOWER_BARREL,
                custom_size: Some(Vec2::new(ShapeSizes::TOWER_BARREL_WIDTH, ShapeSizes::TOWER_BARREL_HEIGHT)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(ZDepth::TOWER_BARREL)),
            ..default()
        },
        TowerBarrel { tower: tower_entity },
        GameEntity,
    ));

    // Level badge with background (bottom-right corner)
    let badge_offset = Vec2::new(14.0, -14.0);

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.85),
                custom_size: Some(Vec2::splat(16.0)),
                ..default()
            },
            transform: Transform::from_translation((pos + badge_offset).extend(ZDepth::BADGE_BG)),
            ..default()
        },
        TowerLevelBadge { tower: tower_entity },
        GameEntity,
    ));

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "1",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
                    color: Color::WHITE,
                },
            ).with_justify(JustifyText::Center),
            transform: Transform::from_translation(
                (pos + badge_offset).extend(ZDepth::BADGE_TEXT)
            ),
            ..default()
        },
        TowerLevelBadge { tower: tower_entity },
        GameEntity,
    ));

    // Buff aura indicator (buff towers only)
    if tower_type == TowerType::Buff {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(1.0, 0.85, 0.3, 0.15),
                    custom_size: Some(Vec2::splat(ShapeSizes::BUFF_AURA_RANGE * 2.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(ZDepth::BUFF_AURA)),
                ..default()
            },
            BuffAuraIndicator { tower: tower_entity },
            GameEntity,
        ));
    }

    // Shadow (slightly offset for directional light feel)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(0.02, 0.02, 0.04, 0.35),
                custom_size: Some(Vec2::splat(42.0)),
                ..default()
            },
            transform: Transform::from_translation(
                Vec3::new(pos.x + 1.5, pos.y - 1.5, ZDepth::TOWER_SHADOW)
            ),
            ..default()
        },
        TowerShadow { tower: tower_entity },
        GameEntity,
    ));

    // Corner brackets (4 corners x 2 bars each)
    let bracket_color = accent_color.with_alpha(0.25);
    let bracket_positions: [(f32, f32, f32, f32); 8] = [
        (-15.0, 20.0, 8.0, 1.5),  (-20.0, 15.0, 1.5, 8.0),   // Top-left
        (15.0, 20.0, 8.0, 1.5),   (20.0, 15.0, 1.5, 8.0),    // Top-right
        (-15.0, -20.0, 8.0, 1.5), (-20.0, -15.0, 1.5, 8.0),  // Bottom-left
        (15.0, -20.0, 8.0, 1.5),  (20.0, -15.0, 1.5, 8.0),   // Bottom-right
    ];

    for (ox, oy, w, h) in bracket_positions {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: bracket_color,
                    custom_size: Some(Vec2::new(w, h)),
                    ..default()
                },
                transform: Transform::from_translation(
                    Vec3::new(pos.x + ox, pos.y + oy, ZDepth::TOWER_BRACKET)
                ),
                ..default()
            },
            TowerBracket { tower: tower_entity },
            GameEntity,
        ));
    }
}

fn tower_targeting(
    mut towers: Query<(&mut Tower, &Transform, Option<&TowerSynergies>)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    spatial_grid: Res<SpatialGrid>,
) {
    for (mut tower, tower_transform, synergies) in &mut towers {
        // Buff towers don't target enemies
        if !tower.tower_type.can_attack() {
            tower.target = None;
            continue;
        }

        let tower_pos = tower_transform.translation.truncate();

        // Apply synergy range bonus (Sniper pair)
        let range_bonus = synergies.map(|s| s.range_bonus).unwrap_or(0.0);
        let effective_range = tower.range * (1.0 + range_bonus);

        // Use spatial grid to get nearby entities (much faster than checking all)
        let nearby = spatial_grid.query_range(tower_pos, effective_range);

        // Find best target based on priority
        let mut best_target: Option<(Entity, f32)> = None;

        for entity in nearby {
            let Ok((enemy_entity, enemy_transform, enemy)) = enemies.get(entity) else {
                continue;
            };

            if enemy.health <= 0.0 || enemy.marked_dead {
                continue;
            }

            let enemy_pos = enemy_transform.translation.truncate();
            let distance = tower_pos.distance(enemy_pos);

            // Double-check distance (spatial grid is approximate)
            if distance > effective_range {
                continue;
            }

            // Calculate priority score based on targeting mode
            let score = match tower.targeting {
                TargetingPriority::First => enemy.path_index as f32 * 1000.0,
                TargetingPriority::Closest => -distance,
                TargetingPriority::LowestHP => -enemy.health,
                TargetingPriority::HighestHP => enemy.health,
                TargetingPriority::Fastest => enemy.base_speed,
            };

            if let Some((_, best_score)) = best_target {
                if score > best_score {
                    best_target = Some((enemy_entity, score));
                }
            } else {
                best_target = Some((enemy_entity, score));
            }
        }

        tower.target = best_target.map(|(e, _)| e);

        // Clear target if it's no longer valid
        if let Some(target) = tower.target {
            if enemies.get(target).is_err() {
                tower.target = None;
            }
        }
    }
}

fn tower_attack(
    mut commands: Commands,
    mut towers: Query<(Entity, &mut Tower, &Transform, Option<&BuffedStatus>, Option<&TowerSynergies>)>,
    enemies: Query<&Transform, With<Enemy>>,
    time: Res<Time>,
    mut projectile_events: EventWriter<SpawnProjectileEvent>,
) {
    for (tower_entity, mut tower, tower_transform, buff_status, synergies) in &mut towers {
        // Buff towers don't attack
        if !tower.tower_type.can_attack() {
            continue;
        }

        // Minigun ramp-up: track same-target duration
        let minigun_speed_mult = if tower.specialization == Some(Specialization::Minigun) {
            if tower.target == tower.ramp_up_target && tower.target.is_some() {
                tower.ramp_up_timer += time.delta_seconds();
            } else {
                tower.ramp_up_timer = 0.0;
                tower.ramp_up_target = tower.target;
            }
            // Ramp from 1x to 2.5x over 3.5 seconds
            1.0 + (tower.ramp_up_timer / 3.5).min(1.0) * 1.5
        } else {
            1.0
        };

        // Apply buff speed multiplier to attack cooldown
        let speed_mult = buff_status.map(|b| b.speed_multiplier).unwrap_or(1.0) * minigun_speed_mult;
        tower.attack_cooldown.tick(time.delta().mul_f32(speed_mult));

        if let Some(target) = tower.target {
            if tower.attack_cooldown.just_finished() {
                if let Ok(_enemy_transform) = enemies.get(target) {
                    let start = tower_transform.translation.truncate();

                    // Apply buff multiplier to damage
                    let damage_mult = buff_status.map(|b| b.damage_multiplier).unwrap_or(1.0);
                    let final_damage = tower.damage * damage_mult;

                    // Get synergy bonuses
                    let mut extra_chain_bounces = synergies.map(|s| s.extra_chain_bounces).unwrap_or(0);
                    let poison_duration_bonus = synergies.map(|s| s.poison_duration_bonus).unwrap_or(0.0);

                    // Tesla: +2 bounces
                    if tower.specialization == Some(Specialization::Tesla) {
                        extra_chain_bounces += 2;
                    }

                    let spec = tower.specialization;

                    // Shotgun: fire 3 projectiles in a cone
                    if spec == Some(Specialization::Shotgun) {
                        for i in 0..3 {
                            let offset = match i {
                                0 => Vec2::new(-8.0, 4.0),
                                1 => Vec2::ZERO,
                                _ => Vec2::new(8.0, -4.0),
                            };
                            projectile_events.send(SpawnProjectileEvent {
                                start: start + offset,
                                target,
                                damage: final_damage,
                                tower_type: tower.tower_type,
                                extra_chain_bounces,
                                poison_duration_bonus,
                                source_tower: Some(tower_entity),
                                specialization: spec,
                            });
                        }
                    } else {
                        projectile_events.send(SpawnProjectileEvent {
                            start,
                            target,
                            damage: final_damage,
                            tower_type: tower.tower_type,
                            extra_chain_bounces,
                            poison_duration_bonus,
                            source_tower: Some(tower_entity),
                            specialization: spec,
                        });
                    }

                    // Spawn muzzle flash (different color for different tower types)
                    let flash_color = match tower.tower_type {
                        TowerType::Chain => GameColors::PROJECTILE_CHAIN,
                        TowerType::Poison => GameColors::PROJECTILE_POISON,
                        _ => GameColors::MUZZLE_FLASH,
                    };

                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: flash_color,
                                custom_size: Some(Vec2::splat(ShapeSizes::MUZZLE_FLASH)),
                                ..default()
                            },
                            transform: Transform::from_translation(start.extend(ZDepth::MUZZLE_FLASH)),
                            ..default()
                        },
                        MuzzleFlash {
                            lifetime: Timer::from_seconds(0.08, TimerMode::Once),
                        },
                        GameEntity,
                    ));
                }
            }
        }
    }
}

fn update_tower_visuals(
    towers: Query<(&Tower, &Transform), Without<TowerBarrel>>,
    enemies: Query<&Transform, (With<Enemy>, Without<Tower>, Without<TowerBarrel>)>,
    mut barrels: Query<(&TowerBarrel, &mut Transform), (Without<Tower>, Without<Enemy>)>,
) {
    for (barrel, mut barrel_transform) in &mut barrels {
        if let Ok((tower, tower_transform)) = towers.get(barrel.tower) {
            // Update barrel position to follow tower
            barrel_transform.translation.x = tower_transform.translation.x;
            barrel_transform.translation.y = tower_transform.translation.y;

            // Rotate barrel toward target
            if let Some(target) = tower.target {
                if let Ok(enemy_transform) = enemies.get(target) {
                    let tower_pos = tower_transform.translation.truncate();
                    let enemy_pos = enemy_transform.translation.truncate();
                    let direction = (enemy_pos - tower_pos).normalize();
                    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
                    barrel_transform.rotation = Quat::from_rotation_z(angle);
                }
            }
        }
    }
}

fn update_range_indicators(
    hovered: Res<HoveredTower>,
    selected: Res<SelectedPlacedTower>,
    towers: Query<&Transform, (With<Tower>, Without<RangeIndicator>)>,
    mut indicators: Query<(&RangeIndicator, &mut Sprite, &mut Transform), Without<Tower>>,
    settings: Res<GameSettings>,
) {
    for (indicator, mut sprite, mut transform) in &mut indicators {
        // Always show range for selected tower (context menu open)
        // Only show on hover if setting is enabled
        let is_selected = selected.0 == Some(indicator.tower);
        let is_hovered = hovered.0 == Some(indicator.tower) && settings.show_range_on_hover;
        if is_selected || is_hovered {
            sprite.color = GameColors::RANGE_INDICATOR;
        } else {
            sprite.color = Color::NONE;
        }

        // Keep indicator position synced with tower
        if let Ok(tower_transform) = towers.get(indicator.tower) {
            transform.translation.x = tower_transform.translation.x;
            transform.translation.y = tower_transform.translation.y;
        }
    }
}

fn handle_tower_selling(
    mut commands: Commands,
    mut events: EventReader<SellTowerEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut map: ResMut<GameMap>,
    mut synergy_dirty: ResMut<SynergyDirty>,
    mut buff_dirty: ResMut<BuffDirty>,
    mut stats: ResMut<GameStats>,
    towers: Query<&Tower>,
    range_indicators: Query<(Entity, &RangeIndicator)>,
    barrels: Query<(Entity, &TowerBarrel)>,
    badges: Query<(Entity, &TowerLevelBadge)>,
    buff_indicators: Query<(Entity, &BuffAuraIndicator)>,
    tower_cores: Query<(Entity, &TowerCore)>,
    tower_brackets: Query<(Entity, &TowerBracket)>,
    tower_shadows: Query<(Entity, &TowerShadow)>,
    tower_rings: Query<(Entity, &TowerLevelRing)>,
) {
    for event in events.read() {
        if let Ok(tower) = towers.get(event.tower) {
            // Refund gold
            let sell_value = tower.sell_value();
            economy.gold = economy.gold.saturating_add(sell_value);
            synergy_dirty.0 = true;
            buff_dirty.0 = true;
            stats.record_sell(event.tower, sell_value);

            // Clear map tile
            map.remove_tower(tower.grid_x, tower.grid_y);

            // Despawn tower and all associated visual entities (safe pattern)
            let tower_id = event.tower;
            if let Some(ec) = commands.get_entity(tower_id) { ec.despawn_recursive(); }
            for (entity, indicator) in &range_indicators {
                if indicator.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, core) in &tower_cores {
                if core.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, barrel) in &barrels {
                if barrel.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, badge) in &badges {
                if badge.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, indicator) in &buff_indicators {
                if indicator.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, bracket) in &tower_brackets {
                if bracket.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, shadow) in &tower_shadows {
                if shadow.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
            for (entity, ring) in &tower_rings {
                if ring.tower == tower_id {
                    if let Some(ec) = commands.get_entity(entity) { ec.despawn_recursive(); }
                }
            }
        }
    }
}

/// Shared visual update logic for tower upgrades and specializations.
/// Updates core color/size, barrel tint, bracket alpha, level ring, range indicator,
/// and spawns an upgrade flash effect.
#[allow(clippy::too_many_arguments)]
fn apply_tower_visual_update(
    commands: &mut Commands,
    tower_entity: Entity,
    tower: &Tower,
    tower_pos: Vec2,
    tower_cores: &mut Query<(&TowerCore, &mut Sprite), Without<Tower>>,
    barrels: &mut Query<(&TowerBarrel, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBracket>)>,
    brackets: &mut Query<(&TowerBracket, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBarrel>)>,
    range_indicators: &mut Query<(&RangeIndicator, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<TowerBarrel>, Without<TowerBracket>)>,
    level_rings: &Query<&TowerLevelRing>,
) {
    let accent_color = tower.accent_color();
    let brightness = 1.0 + BRIGHTNESS_PER_LEVEL * (tower.level - 1) as f32;
    let accent_srgba = accent_color.to_srgba();
    let upgraded_color = Color::srgb(
        (accent_srgba.red * brightness).min(1.0),
        (accent_srgba.green * brightness).min(1.0),
        (accent_srgba.blue * brightness).min(1.0),
    );

    // Update core sprite
    for (core, mut core_sprite) in tower_cores.iter_mut() {
        if core.tower == tower_entity {
            core_sprite.color = upgraded_color;
            let core_size = ShapeSizes::TOWER_CORE + (tower.level - 1) as f32 * CORE_SIZE_PER_LEVEL;
            core_sprite.custom_size = Some(Vec2::splat(core_size));
            break;
        }
    }

    // Tint barrel if tower has a specialization
    if let Some(spec) = tower.specialization {
        let spec_srgba = spec.color().to_srgba();
        let barrel_base = GameColors::TOWER_BARREL.to_srgba();
        let barrel_tint = Color::srgb(
            barrel_base.red * BARREL_BASE_BLEND + spec_srgba.red * BARREL_SPEC_BLEND,
            barrel_base.green * BARREL_BASE_BLEND + spec_srgba.green * BARREL_SPEC_BLEND,
            barrel_base.blue * BARREL_BASE_BLEND + spec_srgba.blue * BARREL_SPEC_BLEND,
        );
        for (barrel, mut barrel_sprite) in barrels.iter_mut() {
            if barrel.tower == tower_entity {
                barrel_sprite.color = barrel_tint;
                break;
            }
        }
    }

    // Update bracket brightness
    let bracket_alpha = (BRACKET_ALPHA_BASE + BRACKET_ALPHA_PER_LEVEL * (tower.level - 1) as f32)
        .min(BRACKET_ALPHA_MAX);
    let bracket_color = accent_color.with_alpha(bracket_alpha);
    for (bracket, mut bracket_sprite) in brackets.iter_mut() {
        if bracket.tower == tower_entity {
            bracket_sprite.color = bracket_color;
        }
    }

    // Spawn level ring at level 3+ (if not already present)
    if tower.level >= LEVEL_RING_MIN_LEVEL {
        let has_ring = level_rings.iter().any(|r| r.tower == tower_entity);
        if !has_ring {
            let ring_color = accent_color.with_alpha(LEVEL_RING_ALPHA);
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: ring_color,
                        custom_size: Some(Vec2::splat(LEVEL_RING_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(
                        tower_pos.extend(ZDepth::TOWER_LEVEL_RING)
                    ),
                    ..default()
                },
                TowerLevelRing { tower: tower_entity },
                GameEntity,
            ));
        }
    }

    // Spawn upgrade flash
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, UPGRADE_FLASH_ALPHA),
                custom_size: Some(Vec2::splat(UPGRADE_FLASH_START_SIZE)),
                ..default()
            },
            transform: Transform::from_translation(tower_pos.extend(ZDepth::UPGRADE_FLASH)),
            ..default()
        },
        UpgradeFlash {
            lifetime: Timer::from_seconds(UPGRADE_FLASH_DURATION, TimerMode::Once),
            start_size: UPGRADE_FLASH_START_SIZE,
            end_size: UPGRADE_FLASH_END_SIZE,
        },
        GameEntity,
    ));

    // Update range indicator size
    for (indicator, mut ind_sprite) in range_indicators.iter_mut() {
        if indicator.tower == tower_entity {
            ind_sprite.custom_size = Some(Vec2::splat(tower.range * 2.0));
        }
    }
}

fn handle_tower_upgrade(
    mut commands: Commands,
    mut events: EventReader<UpgradeTowerEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut buff_dirty: ResMut<BuffDirty>,
    mut stats: ResMut<GameStats>,
    mut towers: Query<(&mut Tower, &Transform, &mut Sprite)>,
    mut tower_cores: Query<(&TowerCore, &mut Sprite), Without<Tower>>,
    mut range_indicators: Query<(&RangeIndicator, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<TowerBarrel>, Without<TowerBracket>)>,
    mut barrels: Query<(&TowerBarrel, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBracket>)>,
    mut brackets: Query<(&TowerBracket, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBarrel>)>,
    level_rings: Query<&TowerLevelRing>,
) {
    for event in events.read() {
        if let Ok((mut tower, tower_transform, _sprite)) = towers.get_mut(event.tower) {
            let cost = tower.upgrade_cost();
            if cost > 0 && economy.gold >= cost {
                economy.gold -= cost;
                tower.upgrade();
                buff_dirty.0 = true;
                stats.record_upgrade(event.tower, cost, tower.level);

                let tower_pos = tower_transform.translation.truncate();
                apply_tower_visual_update(
                    &mut commands, event.tower, &tower, tower_pos,
                    &mut tower_cores, &mut barrels, &mut brackets,
                    &mut range_indicators, &level_rings,
                );
            }
        }
    }
}

fn handle_tower_specialization(
    mut commands: Commands,
    mut events: EventReader<SpecializeTowerEvent>,
    mut economy: ResMut<PlayerEconomy>,
    mut buff_dirty: ResMut<BuffDirty>,
    mut stats: ResMut<GameStats>,
    mut towers: Query<(&mut Tower, &Transform, &mut Sprite)>,
    mut tower_cores: Query<(&TowerCore, &mut Sprite), Without<Tower>>,
    mut range_indicators: Query<(&RangeIndicator, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<TowerBarrel>, Without<TowerBracket>)>,
    mut barrels: Query<(&TowerBarrel, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBracket>)>,
    mut brackets: Query<(&TowerBracket, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBarrel>)>,
    level_rings: Query<&TowerLevelRing>,
) {
    for event in events.read() {
        if let Ok((mut tower, tower_transform, _sprite)) = towers.get_mut(event.tower) {
            let cost = tower.upgrade_cost();
            if economy.gold >= cost {
                economy.gold -= cost;
                tower.specialize(event.specialization);
                buff_dirty.0 = true;
                stats.record_upgrade(event.tower, cost, tower.level);

                let tower_pos = tower_transform.translation.truncate();
                apply_tower_visual_update(
                    &mut commands, event.tower, &tower, tower_pos,
                    &mut tower_cores, &mut barrels, &mut brackets,
                    &mut range_indicators, &level_rings,
                );
            }
        }
    }
}

fn update_muzzle_flashes(
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut MuzzleFlash, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.lifetime.tick(time.delta());

        // Fade out
        let alpha = 1.0 - ease_out(flash.lifetime.fraction());
        sprite.color = GameColors::MUZZLE_FLASH.with_alpha(alpha);

        if flash.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_upgrade_flashes(
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut UpgradeFlash, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.lifetime.tick(time.delta());

        let frac = ease_out(flash.lifetime.fraction());
        // Interpolate size from start to end
        let size = flash.start_size + (flash.end_size - flash.start_size) * frac;
        sprite.custom_size = Some(Vec2::splat(size));

        // Fade out alpha
        let alpha = (1.0 - frac) * 0.6;
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);

        if flash.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn tower_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedTowerType>,
) {
    // Number keys 1-8 select tower types
    if keyboard.just_pressed(KeyCode::Digit1) {
        selected.0 = TowerType::Basic;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        selected.0 = TowerType::Splash;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        selected.0 = TowerType::Slow;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        selected.0 = TowerType::Sniper;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        selected.0 = TowerType::Rapid;
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        selected.0 = TowerType::Chain;
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        selected.0 = TowerType::Poison;
    } else if keyboard.just_pressed(KeyCode::Digit8) {
        selected.0 = TowerType::Buff;
    }
}

/// Undo the most recent tower placement within a 3-second window (100% gold refund).
/// Triggered by pressing Z. Only works for the single most recent placement.
fn tower_undo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut recent: ResMut<RecentPlacement>,
    mut economy: ResMut<PlayerEconomy>,
    mut sell_events: EventWriter<SellTowerEvent>,
    mut selected_placed: ResMut<SelectedPlacedTower>,
    towers: Query<&Tower>,
    time: Res<Time>,
) {
    // Tick the undo timer
    if let Some(ref mut timer) = recent.timer {
        timer.tick(time.delta());
        if timer.finished() {
            // Window expired — clear undo state
            recent.tower_entity = None;
            recent.cost = 0;
            recent.timer = None;
            return;
        }
    } else {
        return;
    }

    // Z key triggers undo
    if keyboard.just_pressed(KeyCode::KeyZ) {
        if let Some(tower_entity) = recent.tower_entity {
            // Only undo if the tower still exists and is level 1 (hasn't been upgraded)
            if let Ok(tower) = towers.get(tower_entity) {
                if tower.level == 1 {
                    // The sell system refunds 75%. We need 100%.
                    // So we add the 25% difference BEFORE the sell event processes.
                    let sell_value = tower.sell_value();
                    let difference = recent.cost.saturating_sub(sell_value);
                    economy.gold = economy.gold.saturating_add(difference);

                    // Send sell event to handle despawning + map cleanup
                    sell_events.send(SellTowerEvent { tower: tower_entity });

                    // Clear tower selection if the undone tower was selected
                    if selected_placed.0 == Some(tower_entity) {
                        selected_placed.0 = None;
                    }
                }
            }

            // Clear undo state regardless
            recent.tower_entity = None;
            recent.cost = 0;
            recent.timer = None;
        }
    }
}

fn update_level_badges(
    towers: Query<(&Tower, &Transform)>,
    mut badges: Query<(&TowerLevelBadge, &mut Text, &mut Transform), Without<Tower>>,
) {
    for (badge, mut text, mut badge_transform) in &mut badges {
        if let Ok((tower, tower_transform)) = towers.get(badge.tower) {
            // Update badge text to show current level + spec initial
            text.sections[0].value = if let Some(spec) = tower.specialization {
                format!("{}{}", tower.level, spec.initial())
            } else {
                format!("{}", tower.level)
            };

            // Keep badge position synced with tower (bottom-right corner)
            let badge_offset = Vec2::new(12.0, -12.0);
            badge_transform.translation.x = tower_transform.translation.x + badge_offset.x;
            badge_transform.translation.y = tower_transform.translation.y + badge_offset.y;
        }
    }
}

/// Update buff status for all towers based on nearby buff towers
fn update_buff_auras(
    mut commands: Commands,
    mut buff_dirty: ResMut<BuffDirty>,
    buff_towers: Query<(&Tower, &Transform)>,
    mut other_towers: Query<(Entity, &Tower, &Transform, Option<&TowerSynergies>, Option<&mut BuffedStatus>)>,
) {
    if !buff_dirty.0 {
        return;
    }
    buff_dirty.0 = false;

    // Collect buff tower positions, ranges, and pre-computed buff percentages
    let buff_sources: Vec<(Vec2, f32, f32)> = buff_towers
        .iter()
        .filter(|(t, _)| t.tower_type == TowerType::Buff)
        .map(|(t, transform)| (transform.translation.truncate(), t.range, t.buff_percentage()))
        .collect();

    // Update buff status for each tower
    for (entity, tower, transform, synergies, buff_status) in &mut other_towers {
        // Skip buff towers themselves
        if tower.tower_type == TowerType::Buff {
            continue;
        }

        let tower_pos = transform.translation.truncate();

        // Check if tower is in range of any buff tower
        let mut total_buff = 0.0;
        for (buff_pos, buff_range, buff_pct) in &buff_sources {
            if tower_pos.distance(*buff_pos) <= *buff_range {
                total_buff += buff_pct;
            }
        }

        if total_buff > 0.0 {
            // Apply RapidBuff synergy: Rapid towers adjacent to Buff get 2x the effect
            let synergy_multiplier = synergies
                .map(|s| s.speed_buff_multiplier)
                .unwrap_or(1.0);

            // Tower is being buffed
            let new_status = BuffedStatus {
                damage_multiplier: 1.0 + total_buff * synergy_multiplier,
                speed_multiplier: 1.0 + total_buff * 0.4 * synergy_multiplier, // Speed scales too
            };
            if let Some(mut status) = buff_status {
                *status = new_status;
            } else {
                commands.entity(entity).try_insert(new_status);
            }
        } else if buff_status.is_some() {
            // No longer buffed, remove status
            commands.entity(entity).remove::<BuffedStatus>();
        }
    }
}

/// Update tower synergies based on adjacent towers
fn update_tower_synergies(
    mut towers: Query<(Entity, &Tower, &mut TowerSynergies)>,
    mut synergy_dirty: ResMut<SynergyDirty>,
) {
    if !synergy_dirty.0 {
        return;
    }
    synergy_dirty.0 = false;

    // Build grid-indexed lookup: O(T) instead of scanning all towers per neighbor
    let mut tower_grid: HashMap<(usize, usize), (Entity, TowerType)> = HashMap::new();
    for (entity, tower, _) in towers.iter() {
        tower_grid.insert((tower.grid_x, tower.grid_y), (entity, tower.tower_type));
    }

    // Check adjacency and calculate synergies for each tower
    for (entity, tower, mut synergies) in &mut towers {
        // Reset synergies
        synergies.active.clear();
        synergies.range_bonus = 0.0;
        synergies.poison_duration_bonus = 0.0;
        synergies.speed_buff_multiplier = 1.0;
        synergies.extra_chain_bounces = 0;

        let grid_x = tower.grid_x;
        let grid_y = tower.grid_y;
        let tower_type = tower.tower_type;

        // Check all 8 adjacent tiles (including diagonals)
        let adjacent_offsets: [(i32, i32); 8] = [
            (-1, -1), (0, -1), (1, -1),
            (-1, 0),           (1, 0),
            (-1, 1),  (0, 1),  (1, 1),
        ];

        // Find adjacent tower types via O(1) grid lookup per neighbor
        let mut adjacent_types: Vec<TowerType> = Vec::with_capacity(8);
        for (ox, oy) in adjacent_offsets {
            let check_x = grid_x as i32 + ox;
            let check_y = grid_y as i32 + oy;

            if check_x >= 0 && check_y >= 0 {
                if let Some((other_entity, other_type)) = tower_grid.get(&(check_x as usize, check_y as usize)) {
                    if *other_entity != entity {
                        adjacent_types.push(*other_type);
                    }
                }
            }
        }

        // Detect synergies via pure rules function
        let result = rules::detect_synergies(tower_type, &adjacent_types);
        synergies.active = result.synergies;
        synergies.range_bonus = result.range_bonus;
        synergies.poison_duration_bonus = result.poison_duration_bonus;
        synergies.speed_buff_multiplier = result.speed_buff_multiplier;
        synergies.extra_chain_bounces = result.extra_chain_bounces;
    }
}

/// Updates bracket colors to indicate active synergies.
/// Runs every frame but only applies changes when synergies were just recalculated
/// (i.e. SynergyDirty transitioned from true to false on the previous tick).
fn update_synergy_indicators(
    towers: Query<(Entity, &Tower, &TowerSynergies)>,
    mut brackets: Query<(&TowerBracket, &mut Sprite), (Without<Tower>, Without<TowerCore>, Without<RangeIndicator>, Without<TowerBarrel>)>,
    synergy_dirty: Res<SynergyDirty>,
    mut was_dirty: Local<bool>,
) {
    // Detect the frame after synergies were recalculated:
    // update_tower_synergies sets dirty to false after recalculating,
    // so we trigger when was_dirty=true and dirty is now false.
    let should_update = *was_dirty && !synergy_dirty.0;
    *was_dirty = synergy_dirty.0;

    if !should_update {
        return;
    }

    // Build lookup: tower entity -> (accent_color, has_active_synergies, level)
    let tower_info: HashMap<Entity, (Color, bool, u32)> = towers
        .iter()
        .map(|(e, tower, syn)| (e, (tower.accent_color(), !syn.active.is_empty(), tower.level)))
        .collect();

    for (bracket, mut sprite) in &mut brackets {
        if let Some(&(accent_color, has_synergy, level)) = tower_info.get(&bracket.tower) {
            if has_synergy {
                sprite.color = GameColors::SYNERGY.with_alpha(0.5);
            } else {
                // Restore default: accent color with level-scaled alpha
                let bracket_alpha = (BRACKET_ALPHA_BASE + BRACKET_ALPHA_PER_LEVEL * (level - 1) as f32)
                    .min(BRACKET_ALPHA_MAX);
                sprite.color = accent_color.with_alpha(bracket_alpha);
            }
        }
    }
}

/// Visual pulse effect for buff aura indicators - only show when hovered or selected
fn update_buff_aura_visuals(
    towers: Query<(&Tower, &Transform)>,
    mut indicators: Query<(&BuffAuraIndicator, &mut Sprite, &mut Transform), Without<Tower>>,
    hovered: Res<HoveredTower>,
    selected: Res<SelectedPlacedTower>,
    time: Res<Time>,
) {
    let pulse = (time.elapsed_seconds() * 2.0).sin() * 0.5 + 0.5;

    for (indicator, mut sprite, mut transform) in &mut indicators {
        if let Ok((tower, tower_transform)) = towers.get(indicator.tower) {
            // Update position
            transform.translation.x = tower_transform.translation.x;
            transform.translation.y = tower_transform.translation.y;

            // Update size based on tower range (which increases with upgrades)
            sprite.custom_size = Some(Vec2::splat(tower.range * 2.0));

            // Only show aura if this tower is hovered or selected
            let is_visible = hovered.0 == Some(indicator.tower) || selected.0 == Some(indicator.tower);

            if is_visible {
                let alpha = 0.12 + pulse * 0.08;
                sprite.color = Color::srgba(1.0, 0.85, 0.3, alpha);
            } else {
                sprite.color = Color::NONE;
            }
        }
    }
}


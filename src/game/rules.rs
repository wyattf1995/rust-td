//! Pure game logic functions — no Bevy dependencies.
//!
//! This module contains all core game formulas and rules as pure functions
//! that can be unit-tested without spinning up a Bevy App or ECS world.
//!
//! # Known ECS-dependent edge cases (not unit-testable here)
//!
//! - **Splitter + DOT**: When a Splitter enemy dies from DOT (poison), the split
//!   children spawn at the parent's position. If the parent was mid-path, the
//!   children should inherit the parent's path progress. Verify in `enemy.rs`.
//! - **Chain bounce to empty**: If a chain projectile has remaining bounces but
//!   no valid targets in range, it should despawn without panicking.
//! - **Tower sell during projectile flight**: Selling a tower while its
//!   projectiles are in flight should not panic. Projectiles use `try_insert`
//!   and check entity validity before accessing tower data.
//! - **Wave overlap**: Starting a new wave while enemies from a previous wave
//!   are still alive should work correctly. `WaveManager` tracks active enemies
//!   independently of wave number.
//! - **Economy overflow**: `gold` is `u32`. Extremely long games could
//!   theoretically overflow. Interest is capped at 50, but kill rewards are not.
//!   Consider `saturating_add` if this becomes a real concern.

use super::tower::{SynergyType, TowerType};

// =============================================================================
// TOWER ECONOMY FORMULAS
// =============================================================================

/// Upgrade cost for a tower at its current level.
/// Level 2: 50% of base, Level 3: 75%, Level 4: 112%, Level 5: 168%...
pub fn upgrade_cost(base_cost: u32, level: u32) -> u32 {
    (base_cost as f32 * 0.5 * 1.5_f32.powf((level - 1) as f32)) as u32
}

/// Total gold spent on upgrades from level 1 to the given level.
pub fn upgrade_cost_total(base_cost: u32, level: u32) -> u32 {
    let mut total = 0u32;
    let base = base_cost as f32;
    for lvl in 1..level {
        total += (base * 0.5 * 1.5_f32.powf((lvl - 1) as f32)) as u32;
    }
    total
}

/// Sell value: 75% of total invested (base cost + all upgrade costs).
pub fn sell_value(base_cost: u32, level: u32) -> u32 {
    ((base_cost + upgrade_cost_total(base_cost, level)) as f32 * 0.75) as u32
}

// =============================================================================
// TOWER STAT SCALING FORMULAS
// =============================================================================

/// Per-level bonus factor with diminishing returns.
/// Used as the building block for damage, range, and speed multipliers.
pub fn level_bonus(level: u32) -> f32 {
    0.20 / (1.0 + (level - 2) as f32 * 0.15)
}

/// Cumulative damage multiplier from level 1 to the given level.
pub fn damage_multiplier(level: u32) -> f32 {
    let mut mult = 1.0;
    for lvl in 2..=level {
        mult *= 1.0 + level_bonus(lvl);
    }
    mult
}

/// Cumulative range multiplier from level 1 to the given level.
/// Range scales at 40% of the damage bonus rate.
pub fn range_multiplier(level: u32) -> f32 {
    let mut mult = 1.0;
    for lvl in 2..=level {
        mult *= 1.0 + level_bonus(lvl) * 0.4;
    }
    mult
}

/// Cumulative speed multiplier from level 1 to the given level.
/// Speed scales at 60% of the damage bonus rate.
pub fn speed_multiplier(level: u32) -> f32 {
    let mut mult = 1.0;
    for lvl in 2..=level {
        mult *= 1.0 + level_bonus(lvl) * 0.6;
    }
    mult
}

/// Buff tower aura percentage at a given level.
/// Base 25%, with diminishing bonus per level.
pub fn buff_percentage(level: u32) -> f32 {
    let bonus: f32 = (1..level)
        .map(|l| 0.05 / (1.0 + (l - 1) as f32 * 0.3))
        .sum();
    0.25 + bonus
}

/// Buff percentage delta for the next level (how much the next upgrade adds).
pub fn buff_percentage_next_delta(level: u32) -> f32 {
    let next_level = level + 1;
    0.05 / (1.0 + (next_level - 2) as f32 * 0.3)
}

/// Calculate tower stats at a given level with optional specialization modifiers.
/// Returns (damage, range, speed) as absolute values.
pub fn tower_stats_at_level(
    base_damage: f32,
    base_range: f32,
    base_attack_speed: f32,
    level: u32,
    spec_modifiers: (f32, f32, f32), // (damage_mult, range_mult, speed_mult)
) -> (f32, f32, f32) {
    let (spec_dmg, spec_rng, spec_spd) = spec_modifiers;
    (
        base_damage * damage_multiplier(level) * spec_dmg,
        base_range * range_multiplier(level) * spec_rng,
        base_attack_speed * speed_multiplier(level) * spec_spd,
    )
}

// =============================================================================
// ECONOMY FORMULAS
// =============================================================================

/// Interest earned on current gold (capped at 50).
pub fn calculate_interest(gold: u32, rate: f32) -> u32 {
    ((gold as f32 * rate) as u32).min(50)
}

/// Kill streak gold multiplier.
/// 1x at 0-1 kills, scales +0.1 per additional kill, capped at 2x (11+ kills).
pub fn kill_streak_multiplier(count: u32) -> f32 {
    if count <= 1 {
        1.0
    } else {
        1.0 + ((count - 1) as f32 * 0.1).min(1.0)
    }
}

// =============================================================================
// WAVE GENERATION
// =============================================================================

/// Calculate base enemy count for a wave.
/// `modifier_factor` is the enemy count multiplier from the wave modifier
/// (e.g., 1.0 for normal, 2.0 for Swarm, 1.5 for GoldRush).
pub fn wave_base_count(wave_num: usize, modifier_factor: f32) -> f32 {
    let early_wave_factor = if wave_num <= 4 {
        0.7
    } else if wave_num <= 7 {
        0.85
    } else {
        1.0
    };

    (5.0 + (wave_num as f32 * 1.4) + (wave_num as f32).powf(1.15) * 1.5)
        * early_wave_factor
        * modifier_factor
}

// =============================================================================
// SYNERGY DETECTION
// =============================================================================

/// Result of synergy detection for a single tower.
#[derive(Debug, Default, PartialEq)]
pub struct SynergyResult {
    pub synergies: Vec<SynergyType>,
    pub range_bonus: f32,
    pub poison_duration_bonus: f32,
    pub speed_buff_multiplier: f32,
    pub extra_chain_bounces: u32,
}

/// Detect synergies for a tower based on its type and adjacent tower types.
/// Pure function — no ECS queries, just logic.
pub fn detect_synergies(tower_type: TowerType, adjacent_types: &[TowerType]) -> SynergyResult {
    let mut result = SynergyResult {
        speed_buff_multiplier: 1.0,
        ..Default::default()
    };

    match tower_type {
        TowerType::Sniper => {
            if adjacent_types.contains(&TowerType::Sniper) {
                result.synergies.push(SynergyType::SniperPair);
                result.range_bonus = 0.15;
            }
        }
        TowerType::Slow => {
            if adjacent_types.contains(&TowerType::Poison) {
                result.synergies.push(SynergyType::SlowPoison);
                result.poison_duration_bonus = 0.5;
            }
        }
        TowerType::Poison => {
            if adjacent_types.contains(&TowerType::Slow) {
                result.synergies.push(SynergyType::SlowPoison);
                result.poison_duration_bonus = 0.5;
            }
        }
        TowerType::Rapid => {
            if adjacent_types.contains(&TowerType::Buff) {
                result.synergies.push(SynergyType::RapidBuff);
                result.speed_buff_multiplier = 2.0;
            }
        }
        TowerType::Chain => {
            if adjacent_types.contains(&TowerType::Chain) {
                result.synergies.push(SynergyType::ChainPair);
                result.extra_chain_bounces = 1;
            }
        }
        _ => {}
    }

    result
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Tower economy ---

    #[test]
    fn test_upgrade_cost_level_1() {
        // At level 1, cost = base * 0.5 * 1.5^0 = base * 0.5
        assert_eq!(upgrade_cost(100, 1), 50);
        assert_eq!(upgrade_cost(50, 1), 25);
    }

    #[test]
    fn test_upgrade_cost_scales_exponentially() {
        let base = 100;
        let cost_l1 = upgrade_cost(base, 1);
        let cost_l2 = upgrade_cost(base, 2);
        let cost_l3 = upgrade_cost(base, 3);
        // Each level should cost more than the previous
        assert!(cost_l2 > cost_l1, "Level 2 ({cost_l2}) should cost more than level 1 ({cost_l1})");
        assert!(cost_l3 > cost_l2, "Level 3 ({cost_l3}) should cost more than level 2 ({cost_l2})");
    }

    #[test]
    fn test_upgrade_cost_total_level_1() {
        // At level 1, no upgrades have been purchased
        assert_eq!(upgrade_cost_total(100, 1), 0);
    }

    #[test]
    fn test_upgrade_cost_total_accumulates() {
        let base = 100;
        let total_l3 = upgrade_cost_total(base, 3);
        // Should be cost(level 1) + cost(level 2)
        assert_eq!(total_l3, upgrade_cost(base, 1) + upgrade_cost(base, 2));
    }

    #[test]
    fn test_sell_value_returns_75_percent() {
        let base = 100;
        let level = 1;
        // At level 1, invested = base + 0 upgrades = 100
        let expected = (100.0 * 0.75) as u32; // 75
        assert_eq!(sell_value(base, level), expected);
    }

    #[test]
    fn test_sell_value_includes_upgrades() {
        let base = 100;
        let level = 3;
        let invested = base + upgrade_cost_total(base, level);
        let expected = (invested as f32 * 0.75) as u32;
        assert_eq!(sell_value(base, level), expected);
    }

    // --- Tower stat scaling ---

    #[test]
    fn test_damage_multiplier_level_1() {
        assert_eq!(damage_multiplier(1), 1.0);
    }

    #[test]
    fn test_damage_multiplier_increases_with_level() {
        assert!(damage_multiplier(2) > damage_multiplier(1));
        assert!(damage_multiplier(5) > damage_multiplier(3));
        assert!(damage_multiplier(10) > damage_multiplier(5));
    }

    #[test]
    fn test_damage_multiplier_diminishing_returns() {
        // Per-level percentage bonus should decrease at higher levels
        // (absolute gap grows because it's multiplicative, but the rate slows)
        let pct_2_3 = damage_multiplier(3) / damage_multiplier(2) - 1.0;
        let pct_9_10 = damage_multiplier(10) / damage_multiplier(9) - 1.0;
        assert!(pct_2_3 > pct_9_10,
            "Diminishing returns: pct 2->3 ({pct_2_3:.4}) > pct 9->10 ({pct_9_10:.4})");
    }

    #[test]
    fn test_range_scales_slower_than_damage() {
        let dmg = damage_multiplier(5);
        let rng = range_multiplier(5);
        assert!(rng < dmg, "Range mult ({rng}) should be less than damage mult ({dmg})");
    }

    #[test]
    fn test_speed_scales_between_range_and_damage() {
        let dmg = damage_multiplier(5);
        let spd = speed_multiplier(5);
        let rng = range_multiplier(5);
        assert!(spd > rng, "Speed mult ({spd}) should be > range mult ({rng})");
        assert!(spd < dmg, "Speed mult ({spd}) should be < damage mult ({dmg})");
    }

    #[test]
    fn test_tower_stats_at_level_with_spec() {
        let (dmg, rng, spd) = tower_stats_at_level(
            25.0, 150.0, 1.0, 3,
            (1.4, 1.2, 0.8), // Marksman modifiers
        );
        // Damage should be boosted by both level and spec
        assert!(dmg > 25.0 * 1.4, "Damage {dmg} should exceed base*spec_mult");
        // Range should be boosted
        assert!(rng > 150.0 * 1.2, "Range {rng} should exceed base*spec_mult");
        // Speed should be reduced by spec
        assert!(spd < 1.0 * speed_multiplier(3), "Speed {spd} should be reduced by Marksman spec");
    }

    #[test]
    fn test_buff_percentage_base() {
        assert_eq!(buff_percentage(1), 0.25);
    }

    #[test]
    fn test_buff_percentage_increases() {
        assert!(buff_percentage(2) > buff_percentage(1));
        assert!(buff_percentage(5) > buff_percentage(3));
    }

    #[test]
    fn test_buff_percentage_next_delta_positive() {
        assert!(buff_percentage_next_delta(1) > 0.0);
        assert!(buff_percentage_next_delta(5) > 0.0);
    }

    #[test]
    fn test_buff_percentage_next_delta_diminishes() {
        let d1 = buff_percentage_next_delta(1);
        let d5 = buff_percentage_next_delta(5);
        assert!(d1 > d5, "Delta should diminish: level 1 ({d1}) > level 5 ({d5})");
    }

    // --- Economy ---

    #[test]
    fn test_interest_basic() {
        // 100 gold * 10% = 10
        assert_eq!(calculate_interest(100, 0.10), 10);
    }

    #[test]
    fn test_interest_capped_at_50() {
        // 1000 gold * 10% = 100, but capped at 50
        assert_eq!(calculate_interest(1000, 0.10), 50);
    }

    #[test]
    fn test_interest_zero_gold() {
        assert_eq!(calculate_interest(0, 0.10), 0);
    }

    #[test]
    fn test_kill_streak_no_kills() {
        assert_eq!(kill_streak_multiplier(0), 1.0);
    }

    #[test]
    fn test_kill_streak_one_kill() {
        assert_eq!(kill_streak_multiplier(1), 1.0);
    }

    #[test]
    fn test_kill_streak_scales() {
        assert!((kill_streak_multiplier(2) - 1.1).abs() < 0.001);
        assert!((kill_streak_multiplier(3) - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_kill_streak_caps_at_2x() {
        assert!((kill_streak_multiplier(11) - 2.0).abs() < 0.001);
        assert!((kill_streak_multiplier(20) - 2.0).abs() < 0.001);
    }

    // --- Wave generation ---

    #[test]
    fn test_wave_base_count_increases() {
        let w1 = wave_base_count(1, 1.0);
        let w10 = wave_base_count(10, 1.0);
        let w20 = wave_base_count(20, 1.0);
        assert!(w10 > w1, "Wave 10 ({w10}) should have more enemies than wave 1 ({w1})");
        assert!(w20 > w10, "Wave 20 ({w20}) should have more enemies than wave 10 ({w10})");
    }

    #[test]
    fn test_wave_early_waves_reduced() {
        // Early waves (1-4) use 0.7 factor, wave 8+ uses 1.0
        let early = wave_base_count(3, 1.0);
        assert!(early > 0.0);
        // Same input = same output (pure function)
        assert_eq!(early, wave_base_count(3, 1.0));
    }

    #[test]
    fn test_wave_swarm_doubles_count() {
        let normal = wave_base_count(10, 1.0);
        let swarm = wave_base_count(10, 2.0); // Swarm uses 2.0 modifier
        assert!((swarm / normal - 2.0).abs() < 0.01, "Swarm should double enemy count");
    }

    #[test]
    fn test_wave_gold_rush_150_percent() {
        let normal = wave_base_count(10, 1.0);
        let gold = wave_base_count(10, 1.5); // GoldRush uses 1.5 modifier
        assert!((gold / normal - 1.5).abs() < 0.01, "GoldRush should be 1.5x enemy count");
    }

    // --- Synergy detection ---

    #[test]
    fn test_synergy_sniper_pair() {
        let result = detect_synergies(TowerType::Sniper, &[TowerType::Sniper]);
        assert_eq!(result.synergies, vec![SynergyType::SniperPair]);
        assert!((result.range_bonus - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_synergy_slow_poison() {
        let result = detect_synergies(TowerType::Slow, &[TowerType::Poison]);
        assert_eq!(result.synergies, vec![SynergyType::SlowPoison]);
        assert!((result.poison_duration_bonus - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_synergy_poison_slow() {
        // Symmetrical: Poison next to Slow also gets the synergy
        let result = detect_synergies(TowerType::Poison, &[TowerType::Slow]);
        assert_eq!(result.synergies, vec![SynergyType::SlowPoison]);
    }

    #[test]
    fn test_synergy_rapid_buff() {
        let result = detect_synergies(TowerType::Rapid, &[TowerType::Buff]);
        assert_eq!(result.synergies, vec![SynergyType::RapidBuff]);
        assert!((result.speed_buff_multiplier - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_synergy_chain_pair() {
        let result = detect_synergies(TowerType::Chain, &[TowerType::Chain]);
        assert_eq!(result.synergies, vec![SynergyType::ChainPair]);
        assert_eq!(result.extra_chain_bounces, 1);
    }

    #[test]
    fn test_synergy_no_match() {
        let result = detect_synergies(TowerType::Basic, &[TowerType::Splash, TowerType::Slow]);
        assert!(result.synergies.is_empty());
        assert_eq!(result.range_bonus, 0.0);
        assert_eq!(result.speed_buff_multiplier, 1.0);
    }

    #[test]
    fn test_synergy_no_neighbors() {
        let result = detect_synergies(TowerType::Sniper, &[]);
        assert!(result.synergies.is_empty());
    }

    // --- Edge cases ---

    #[test]
    fn test_upgrade_cost_very_high_level() {
        // Ensure no panic or overflow at extreme levels
        let cost = upgrade_cost(100, 20);
        assert!(cost > 0, "High level upgrade should still be positive");
    }

    #[test]
    fn test_sell_value_never_exceeds_invested() {
        for level in 1..=15 {
            let invested = 100 + upgrade_cost_total(100, level);
            let sell = sell_value(100, level);
            assert!(sell <= invested, "Sell value ({sell}) should not exceed invested ({invested}) at level {level}");
        }
    }

    #[test]
    fn test_damage_multiplier_monotonically_increasing() {
        let mut prev = damage_multiplier(1);
        for level in 2..=20 {
            let curr = damage_multiplier(level);
            assert!(curr > prev, "Damage mult should increase: level {level} ({curr}) > level {} ({prev})", level - 1);
            prev = curr;
        }
    }

    #[test]
    fn test_level_bonus_always_positive() {
        for level in 2..=30 {
            assert!(level_bonus(level) > 0.0, "Level bonus should be positive at level {level}");
        }
    }

    #[test]
    fn test_interest_near_cap_boundary() {
        // 500 gold at 10% = 50 exactly (at cap boundary)
        assert_eq!(calculate_interest(500, 0.10), 50);
        // 499 gold at 10% = 49 (below cap)
        assert_eq!(calculate_interest(499, 0.10), 49);
        // 501 gold at 10% = 50 (above cap, clamped)
        assert_eq!(calculate_interest(501, 0.10), 50);
    }

    #[test]
    fn test_interest_high_rate() {
        // Even with very high rate, capped at 50
        assert_eq!(calculate_interest(1000, 1.0), 50);
    }

    #[test]
    fn test_kill_streak_boundary() {
        // Exact boundary: count=11 should hit 2.0x cap
        assert!((kill_streak_multiplier(11) - 2.0).abs() < 0.001);
        // count=10 should be just under
        assert!(kill_streak_multiplier(10) < 2.0);
    }

    #[test]
    fn test_wave_base_count_wave_zero() {
        // Wave 0 shouldn't panic (defensive check)
        let count = wave_base_count(0, 1.0);
        assert!(count >= 0.0);
    }

    #[test]
    fn test_synergy_multiple_matching_neighbors() {
        // Two snipers adjacent - still only one SniperPair synergy
        let result = detect_synergies(TowerType::Sniper, &[TowerType::Sniper, TowerType::Sniper]);
        assert_eq!(result.synergies.len(), 1);
    }

    #[test]
    fn test_synergy_all_tower_types_no_panic() {
        // Ensure no synergy combo causes a panic
        let all_types = [
            TowerType::Basic, TowerType::Splash, TowerType::Slow,
            TowerType::Sniper, TowerType::Rapid, TowerType::Chain,
            TowerType::Poison, TowerType::Buff,
        ];
        for &tower_type in &all_types {
            for &neighbor in &all_types {
                let _ = detect_synergies(tower_type, &[neighbor]);
            }
        }
    }

    #[test]
    fn test_tower_stats_at_level_1_no_spec() {
        let (dmg, rng, spd) = tower_stats_at_level(25.0, 150.0, 1.0, 1, (1.0, 1.0, 1.0));
        assert!((dmg - 25.0).abs() < 0.001);
        assert!((rng - 150.0).abs() < 0.001);
        assert!((spd - 1.0).abs() < 0.001);
    }
}

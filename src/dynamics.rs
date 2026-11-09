//! The measured tuning table for the Milk Factory remake, as
//! data.
//!
//! The table ships as RON (`data/milk-factory-dynamics.ron`,
//! schema `torogao-milkfactory-dynamics/1`) following the
//! estate's RON contract discipline: the file is the contract,
//! this module validates it at load, and any value outside the
//! measured shape is a typed refusal — never a silent default.
//! The schema is closed (`deny_unknown_fields` on every wire
//! struct): a field the spec marks UNKNOWN/REVIEW cannot be
//! added to the data without a code change and a spec citation.

use serde::Deserialize;

/// The embedded measured table (see the file header for
/// provenance and the spec section behind every value).
pub const DYNAMICS_RON: &str = include_str!("../data/milk-factory-dynamics.ron");

/// Bound on the roster. The measured roster has 18 girls
/// (spec §7); the bound exists so a corrupt file cannot
/// allocate an unbounded table, not as a claim that a larger
/// roster is measured.
pub const ROSTER_COUNT_MAX: usize = 18;
/// The milestone board has exactly 10 claims (spec §6).
pub const MILESTONE_COUNT: usize = 10;
/// Bound on a roster name, in characters.
pub const GIRL_NAME_CHARS_MAX: usize = 32;

/// The five per-girl stats, in the spec's §7 order: cs1 max
/// stamina, cs2 stamina regen per second, cs3 bottle capacity,
/// cs4 milk rate per second, cs5 XP per completed session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatBlock {
    /// cs1: maximum stamina.
    pub max_stamina: f64,
    /// cs2: stamina regenerated per second while resting.
    pub stamina_regen_per_second: f64,
    /// cs3: milk bottle capacity.
    pub bottle_capacity: f64,
    /// cs4: milk produced per second while in session.
    pub milk_rate_per_second: f64,
    /// cs5: account XP granted when a session completes.
    pub xp_per_session: f64,
}

impl StatBlock {
    /// The stat addressed by upgrade index 0..=4 (spec §7's
    /// up1..up5 order).
    pub fn get(&self, stat_index: usize) -> Option<f64> {
        match stat_index {
            0 => Some(self.max_stamina),
            1 => Some(self.stamina_regen_per_second),
            2 => Some(self.bottle_capacity),
            3 => Some(self.milk_rate_per_second),
            4 => Some(self.xp_per_session),
            _ => None,
        }
    }

    /// Add `delta` to the stat addressed by `stat_index`.
    /// Callers validate the index first; an out-of-range index
    /// is an internal invariant break, not user input.
    pub fn add(&mut self, stat_index: usize, delta: f64) {
        match stat_index {
            0 => self.max_stamina += delta,
            1 => self.stamina_regen_per_second += delta,
            2 => self.bottle_capacity += delta,
            3 => self.milk_rate_per_second += delta,
            4 => self.xp_per_session += delta,
            _ => unreachable!("stat index validated by caller"),
        }
    }

    fn components_finite_positive(&self) -> bool {
        [
            self.max_stamina,
            self.stamina_regen_per_second,
            self.bottle_capacity,
            self.milk_rate_per_second,
            self.xp_per_session,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
    }
}

/// One roster row: a girl's measured stats and room price.
/// Adding a girl to the remake is appending one of these to
/// the RON table (spec §11: the roster is data, not code).
#[derive(Clone, Debug, PartialEq)]
pub struct GirlSpec {
    /// Roster index, 1-based, sequential in the table.
    pub index: u32,
    /// Estate-original placeholder name.
    pub name: String,
    /// Room purchase price in coin (spec §2).
    pub room_price: f64,
    /// Base stats (spec §7).
    pub stats: StatBlock,
    /// First purchase cost of each of her five stat upgrades
    /// (spec §7's needcoin0; identical for all five stats).
    pub upgrade_cost_start: f64,
}

/// A milestone board reward (spec §6): coin or keys.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MilestoneReward {
    /// Coin paid into the account balance.
    Coin(f64),
    /// Keys paid into the account balance.
    Keys(u32),
}

/// One milestone claim: the account level that unlocks it and
/// what it pays (spec §6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MilestoneSpec {
    /// Account level required to claim.
    pub gate_level: u32,
    /// The one-shot reward.
    pub reward: MilestoneReward,
}

/// Failure modes of loading, validating, or running the
/// remake. Invalid data and refused verbs are errors, never
/// assertions: the RON file and every verb's inputs are
/// external.
#[derive(Debug)]
pub enum MilkError {
    /// The RON text did not parse as the dynamics schema
    /// (including any field outside the closed schema).
    Parse(ron::de::SpannedError),
    /// A parsed value fell outside the measured shape.
    Invalid {
        /// The field that failed validation.
        field: &'static str,
        /// Why it failed.
        reason: &'static str,
    },
    /// A girl index the roster does not carry.
    UnknownGirl {
        /// The refused index.
        index: u32,
    },
    /// A milestone index outside the board's 10 claims.
    UnknownMilestone {
        /// The refused index.
        index: usize,
    },
    /// A verb was refused without mutating state.
    Refused {
        /// The standing reason.
        reason: &'static str,
    },
    /// A step input (dt) broke the contract.
    InvalidStep {
        /// Why the step was refused.
        reason: &'static str,
    },
}

impl std::fmt::Display for MilkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MilkError::Parse(e) => write!(f, "dynamics RON parse failed: {e}"),
            MilkError::Invalid { field, reason } => {
                write!(f, "invalid dynamics field {field}: {reason}")
            }
            MilkError::UnknownGirl { index } => write!(f, "unknown girl index {index}"),
            MilkError::UnknownMilestone { index } => {
                write!(f, "unknown milestone index {index}")
            }
            MilkError::Refused { reason } => write!(f, "verb refused: {reason}"),
            MilkError::InvalidStep { reason } => write!(f, "invalid step input: {reason}"),
        }
    }
}

impl std::error::Error for MilkError {}

/// The raw deserialized shape of `torogao-milkfactory-dynamics/1`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DynamicsWire {
    starting_coin: f64,
    starting_keys: u32,
    account_level_start: u32,
    xp_threshold_start: f64,
    xp_threshold_step: f64,
    stamina_drain_per_second: f64,
    fresh_girl_stamina: f64,
    battle_damage_start: f64,
    battle_level_start: u32,
    battle_upgrade_cost_start: f64,
    battle_max_hp_start: f64,
    battle_kill_max_hp_step: f64,
    battle_kill_reward: f64,
    battle_damage_step: f64,
    battle_upgrade_cost_step: f64,
    enemy_portrait_count: u32,
    enemy_index_reachable_max: u32,
    stat_upgrade_increments: (f64, f64, f64, f64, f64),
    upgrade_cost_growth_base: f64,
    upgrade_cost_growth_per_index: f64,
    star_upgrade_key_cost: u32,
    star_upgrade_bonuses: (f64, f64, f64, f64, f64),
    enemy_draw_seed: u64,
    girls: Vec<GirlWire>,
    milestones: Vec<MilestoneWire>,
}

/// One roster row as written in the RON table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GirlWire {
    index: u32,
    name: String,
    room_price: f64,
    cs1: f64,
    cs2: f64,
    cs3: f64,
    cs4: f64,
    cs5: f64,
    upgrade_cost_start: f64,
}

/// The wire form of a milestone reward.
#[derive(Debug, Deserialize)]
enum MilestoneRewardWire {
    Coin(f64),
    Keys(u32),
}

/// One milestone row as written in the RON table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MilestoneWire {
    gate_level: u32,
    reward: MilestoneRewardWire,
}

/// The validated dynamics table. Construct only through
/// `Dynamics::from_ron` or `measured_dynamics`, so every
/// instance satisfies the measured shape.
#[derive(Clone, Debug)]
pub struct Dynamics {
    starting_coin: f64,
    starting_keys: u32,
    account_level_start: u32,
    xp_threshold_start: f64,
    xp_threshold_step: f64,
    stamina_drain_per_second: f64,
    fresh_girl_stamina: f64,
    battle_damage_start: f64,
    battle_level_start: u32,
    battle_upgrade_cost_start: f64,
    battle_max_hp_start: f64,
    battle_kill_max_hp_step: f64,
    battle_kill_reward: f64,
    battle_damage_step: f64,
    battle_upgrade_cost_step: f64,
    enemy_portrait_count: u32,
    enemy_index_reachable_max: u32,
    stat_upgrade_increments: StatBlock,
    upgrade_cost_growth_base: f64,
    upgrade_cost_growth_per_index: f64,
    star_upgrade_key_cost: u32,
    star_upgrade_bonuses: StatBlock,
    enemy_draw_seed: u64,
    girls: Vec<GirlSpec>,
    milestones: Vec<MilestoneSpec>,
}

fn stat_block(t: (f64, f64, f64, f64, f64)) -> StatBlock {
    StatBlock {
        max_stamina: t.0,
        stamina_regen_per_second: t.1,
        bottle_capacity: t.2,
        milk_rate_per_second: t.3,
        xp_per_session: t.4,
    }
}

impl Dynamics {
    /// Parse and validate a dynamics table from RON text.
    /// Unknown fields, missing fields, and values outside the
    /// measured shape are all typed refusals.
    pub fn from_ron(text: &str) -> Result<Dynamics, MilkError> {
        let wire: DynamicsWire = ron::from_str(text).map_err(MilkError::Parse)?;
        let d = Dynamics {
            starting_coin: wire.starting_coin,
            starting_keys: wire.starting_keys,
            account_level_start: wire.account_level_start,
            xp_threshold_start: wire.xp_threshold_start,
            xp_threshold_step: wire.xp_threshold_step,
            stamina_drain_per_second: wire.stamina_drain_per_second,
            fresh_girl_stamina: wire.fresh_girl_stamina,
            battle_damage_start: wire.battle_damage_start,
            battle_level_start: wire.battle_level_start,
            battle_upgrade_cost_start: wire.battle_upgrade_cost_start,
            battle_max_hp_start: wire.battle_max_hp_start,
            battle_kill_max_hp_step: wire.battle_kill_max_hp_step,
            battle_kill_reward: wire.battle_kill_reward,
            battle_damage_step: wire.battle_damage_step,
            battle_upgrade_cost_step: wire.battle_upgrade_cost_step,
            enemy_portrait_count: wire.enemy_portrait_count,
            enemy_index_reachable_max: wire.enemy_index_reachable_max,
            stat_upgrade_increments: stat_block(wire.stat_upgrade_increments),
            upgrade_cost_growth_base: wire.upgrade_cost_growth_base,
            upgrade_cost_growth_per_index: wire.upgrade_cost_growth_per_index,
            star_upgrade_key_cost: wire.star_upgrade_key_cost,
            star_upgrade_bonuses: stat_block(wire.star_upgrade_bonuses),
            enemy_draw_seed: wire.enemy_draw_seed,
            girls: wire
                .girls
                .into_iter()
                .map(|g| GirlSpec {
                    index: g.index,
                    name: g.name,
                    room_price: g.room_price,
                    stats: stat_block((g.cs1, g.cs2, g.cs3, g.cs4, g.cs5)),
                    upgrade_cost_start: g.upgrade_cost_start,
                })
                .collect(),
            milestones: wire
                .milestones
                .into_iter()
                .map(|m| MilestoneSpec {
                    gate_level: m.gate_level,
                    reward: match m.reward {
                        MilestoneRewardWire::Coin(v) => MilestoneReward::Coin(v),
                        MilestoneRewardWire::Keys(v) => MilestoneReward::Keys(v),
                    },
                })
                .collect(),
        };
        d.validate()?;
        Ok(d)
    }

    fn invalid(field: &'static str, reason: &'static str) -> MilkError {
        MilkError::Invalid { field, reason }
    }

    fn validate(&self) -> Result<(), MilkError> {
        if !self.starting_coin.is_finite() || self.starting_coin < 0.0 {
            return Err(Self::invalid(
                "starting_coin",
                "must be finite and non-negative",
            ));
        }
        if !self.xp_threshold_start.is_finite() || self.xp_threshold_start <= 0.0 {
            return Err(Self::invalid(
                "xp_threshold_start",
                "must be finite and positive",
            ));
        }
        if !self.xp_threshold_step.is_finite() || self.xp_threshold_step <= 0.0 {
            return Err(Self::invalid(
                "xp_threshold_step",
                "must be finite and positive",
            ));
        }
        if !self.stamina_drain_per_second.is_finite() || self.stamina_drain_per_second <= 0.0 {
            return Err(Self::invalid(
                "stamina_drain_per_second",
                "must be finite and positive",
            ));
        }
        if !self.fresh_girl_stamina.is_finite() || self.fresh_girl_stamina < 0.0 {
            return Err(Self::invalid(
                "fresh_girl_stamina",
                "must be finite and non-negative",
            ));
        }
        if !self.battle_damage_start.is_finite() || self.battle_damage_start <= 0.0 {
            return Err(Self::invalid(
                "battle_damage_start",
                "must be finite and positive",
            ));
        }
        if self.battle_level_start == 0 {
            return Err(Self::invalid("battle_level_start", "must be at least 1"));
        }
        for (field, value) in [
            ("battle_upgrade_cost_start", self.battle_upgrade_cost_start),
            ("battle_max_hp_start", self.battle_max_hp_start),
            ("battle_kill_max_hp_step", self.battle_kill_max_hp_step),
            ("battle_kill_reward", self.battle_kill_reward),
            ("battle_damage_step", self.battle_damage_step),
            ("battle_upgrade_cost_step", self.battle_upgrade_cost_step),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(Self::invalid(field, "must be finite and positive"));
            }
        }
        if self.enemy_portrait_count == 0
            || self.enemy_index_reachable_max == 0
            || self.enemy_index_reachable_max >= self.enemy_portrait_count
        {
            return Err(Self::invalid(
                "enemy_index_reachable_max",
                "must be positive and below the portrait count",
            ));
        }
        if !self.stat_upgrade_increments.components_finite_positive() {
            return Err(Self::invalid(
                "stat_upgrade_increments",
                "components must be finite and positive",
            ));
        }
        if !self.upgrade_cost_growth_base.is_finite() || self.upgrade_cost_growth_base < 0.0 {
            return Err(Self::invalid(
                "upgrade_cost_growth_base",
                "must be finite and non-negative",
            ));
        }
        if !self.upgrade_cost_growth_per_index.is_finite()
            || self.upgrade_cost_growth_per_index < 0.0
        {
            return Err(Self::invalid(
                "upgrade_cost_growth_per_index",
                "must be finite and non-negative",
            ));
        }
        if self.star_upgrade_key_cost == 0 {
            return Err(Self::invalid("star_upgrade_key_cost", "must be at least 1"));
        }
        if !self.star_upgrade_bonuses.components_finite_positive() {
            return Err(Self::invalid(
                "star_upgrade_bonuses",
                "components must be finite and positive",
            ));
        }
        if self.girls.is_empty() || self.girls.len() > ROSTER_COUNT_MAX {
            return Err(Self::invalid("girls", "count outside 1..=18"));
        }
        for (position, girl) in self.girls.iter().enumerate() {
            let expected_index = u32::try_from(position + 1)
                .map_err(|_| Self::invalid("girls", "roster position overflows u32"))?;
            if girl.index != expected_index {
                return Err(Self::invalid("girls", "indices must be sequential from 1"));
            }
            if girl.name.is_empty() || girl.name.chars().count() > GIRL_NAME_CHARS_MAX {
                return Err(Self::invalid("girls", "name outside 1..=32 chars"));
            }
            if !girl.room_price.is_finite() || girl.room_price <= 0.0 {
                return Err(Self::invalid(
                    "girls",
                    "room prices must be finite and positive",
                ));
            }
            if !girl.stats.components_finite_positive() {
                return Err(Self::invalid("girls", "stats must be finite and positive"));
            }
            if girl.stats.max_stamina < self.fresh_girl_stamina {
                return Err(Self::invalid(
                    "girls",
                    "max stamina must cover the fresh-girl stamina",
                ));
            }
            if !girl.upgrade_cost_start.is_finite() || girl.upgrade_cost_start <= 0.0 {
                return Err(Self::invalid(
                    "girls",
                    "upgrade costs must be finite and positive",
                ));
            }
        }
        if self.milestones.len() != MILESTONE_COUNT {
            return Err(Self::invalid("milestones", "board must carry 10 claims"));
        }
        for (position, milestone) in self.milestones.iter().enumerate() {
            if position > 0 && milestone.gate_level <= self.milestones[position - 1].gate_level {
                return Err(Self::invalid(
                    "milestones",
                    "gate levels must strictly increase",
                ));
            }
            match milestone.reward {
                MilestoneReward::Coin(v) if v.is_finite() && v > 0.0 => {}
                MilestoneReward::Keys(v) if v > 0 => {}
                _ => {
                    return Err(Self::invalid(
                        "milestones",
                        "rewards must be finite and positive",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Starting coin balance (spec §2).
    pub fn starting_coin(&self) -> f64 {
        self.starting_coin
    }

    /// Starting key balance (spec §2).
    pub fn starting_keys(&self) -> u32 {
        self.starting_keys
    }

    /// Account level of a new game (spec §2; field default).
    pub fn account_level_start(&self) -> u32 {
        self.account_level_start
    }

    /// First XP threshold (spec §4).
    pub fn xp_threshold_start(&self) -> f64 {
        self.xp_threshold_start
    }

    /// Per-level XP threshold growth (spec §4).
    pub fn xp_threshold_step(&self) -> f64 {
        self.xp_threshold_step
    }

    /// Stamina drained per second while producing (spec §3).
    pub fn stamina_drain_per_second(&self) -> f64 {
        self.stamina_drain_per_second
    }

    /// A fresh girl's stamina at first run (spec §3).
    pub fn fresh_girl_stamina(&self) -> f64 {
        self.fresh_girl_stamina
    }

    /// Battle damage of a new game (spec §5).
    pub fn battle_damage_start(&self) -> f64 {
        self.battle_damage_start
    }

    /// Battle level of a new game (spec §5).
    pub fn battle_level_start(&self) -> u32 {
        self.battle_level_start
    }

    /// First battle damage-upgrade cost (spec §5).
    pub fn battle_upgrade_cost_start(&self) -> f64 {
        self.battle_upgrade_cost_start
    }

    /// Enemy max HP of a new game (spec §5).
    pub fn battle_max_hp_start(&self) -> f64 {
        self.battle_max_hp_start
    }

    /// Enemy max HP growth per kill (spec §5).
    pub fn battle_kill_max_hp_step(&self) -> f64 {
        self.battle_kill_max_hp_step
    }

    /// Pending reward granted per kill (spec §5).
    pub fn battle_kill_reward(&self) -> f64 {
        self.battle_kill_reward
    }

    /// Damage gained per upgrade purchase (spec §5).
    pub fn battle_damage_step(&self) -> f64 {
        self.battle_damage_step
    }

    /// Upgrade cost growth per purchase (spec §5).
    pub fn battle_upgrade_cost_step(&self) -> f64 {
        self.battle_upgrade_cost_step
    }

    /// Highest reachable enemy portrait index (spec §5).
    pub fn enemy_index_reachable_max(&self) -> u32 {
        self.enemy_index_reachable_max
    }

    /// Per-purchase stat increments, by upgrade index (spec §7).
    pub fn stat_upgrade_increment(&self, stat_index: usize) -> Option<f64> {
        self.stat_upgrade_increments.get(stat_index)
    }

    /// Per-purchase upgrade cost growth for a girl:
    /// 80 + 20 * index (spec §7).
    pub fn upgrade_cost_growth(&self, girl_index: u32) -> f64 {
        self.upgrade_cost_growth_base + self.upgrade_cost_growth_per_index * f64::from(girl_index)
    }

    /// Key cost of one star upgrade (spec §7).
    pub fn star_upgrade_key_cost(&self) -> u32 {
        self.star_upgrade_key_cost
    }

    /// Stat bonus of one star upgrade, by stat index (spec §7).
    pub fn star_upgrade_bonus(&self, stat_index: usize) -> Option<f64> {
        self.star_upgrade_bonuses.get(stat_index)
    }

    /// Seed of the deterministic enemy portrait draw.
    pub fn enemy_draw_seed(&self) -> u64 {
        self.enemy_draw_seed
    }

    /// The roster, in index order.
    pub fn girls(&self) -> &[GirlSpec] {
        &self.girls
    }

    /// One roster row by 1-based index.
    pub fn girl(&self, index: u32) -> Result<&GirlSpec, MilkError> {
        self.girls
            .iter()
            .find(|g| g.index == index)
            .ok_or(MilkError::UnknownGirl { index })
    }

    /// The milestone board, in claim order.
    pub fn milestones(&self) -> &[MilestoneSpec] {
        &self.milestones
    }

    /// One milestone by 0-based claim index.
    pub fn milestone(&self, claim_index: usize) -> Result<&MilestoneSpec, MilkError> {
        self.milestones
            .get(claim_index)
            .ok_or(MilkError::UnknownMilestone { index: claim_index })
    }
}

/// Load the embedded measured table
/// (`data/milk-factory-dynamics.ron`).
pub fn measured_dynamics() -> Result<Dynamics, MilkError> {
    Dynamics::from_ron(DYNAMICS_RON)
}

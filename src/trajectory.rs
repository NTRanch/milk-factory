//! The canonical cross-platform trajectory: one fixture,
//! run identically on every platform, and the FNV-1a digest
//! of its per-step record.
//!
//! The digest discipline is the estate's state-hash
//! discipline (the coin pilot's `trajectory` module, after
//! `torogao-app/src/state_hash.rs`), re-implemented here so
//! this crate gains no dependency on the app crate: FNV-1a
//! 64-bit (offset basis `0xcbf29ce484222325`, prime
//! `0x100000001b3`) over the little-endian byte stream of
//! each record's words, in order. After each fixture step,
//! the record is the account (coin bits, keys, level) and
//! each roster girl's (stamina bits, milk bits, in-session
//! flag); the battle phase appends its own closing record.
//! Equal digests on two platforms mean the two runs reached
//! the same states at the same steps, bit for bit — the S7
//! cross-platform determinism gate (PIPELINE.md, D-212),
//! which runs later against the Android shell.
//!
//! The fixture is a full loop, not a fragment: buy the whole
//! reduced roster, produce and harvest until the account
//! reaches the first milestone gate, claim it, fight the
//! battle loop to kills and collect, then harvest once more.

use crate::dynamics::{Dynamics, MilkError};
use crate::measured_dynamics;
use crate::session::Session;

/// FNV-1a offset basis (64-bit).
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime (64-bit).
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Step duration of the fixture, in seconds (60 steps/s).
pub const FIXTURE_DT_SECONDS: f64 = 1.0 / 60.0;
/// Harvest cadence of the fixture, in steps.
pub const FIXTURE_HARVEST_EVERY_STEPS: usize = 600;
/// Hard bound on fixture production steps. The measured
/// rates reach the level-5 gate far inside this bound; the
/// bound exists so a broken table fails loudly instead of
/// looping forever.
pub const FIXTURE_STEPS_MAX: usize = 2_000_000;
/// Attack clicks in the fixture's battle phase.
pub const FIXTURE_BATTLE_ATTACKS: usize = 25;

/// An incremental FNV-1a hasher over little-endian u64
/// words, byte at a time — the `state_hash` discipline.
struct TrajectoryHasher {
    state: u64,
}

impl TrajectoryHasher {
    fn new() -> TrajectoryHasher {
        TrajectoryHasher {
            state: FNV_OFFSET_BASIS,
        }
    }

    fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    fn write_f64(&mut self, value: f64) {
        self.write_u64(value.to_bits());
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

fn hash_session(hasher: &mut TrajectoryHasher, session: &Session, dynamics: &Dynamics) {
    hasher.write_f64(session.coin());
    hasher.write_u64(u64::from(session.keys()));
    hasher.write_u64(u64::from(session.level()));
    for spec in dynamics.girls() {
        let girl = session
            .girl_state(spec.index)
            .expect("roster indices come from the same table");
        hasher.write_f64(girl.stamina);
        hasher.write_f64(girl.milk);
        hasher.write_u64(u64::from(girl.in_session));
    }
}

/// The result of one fixture run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureRun {
    /// FNV-1a digest of the run's per-step trajectory record.
    pub trajectory_hash: u64,
    /// Production steps executed in the level climb.
    pub steps_executed: usize,
    /// Coin balance at the end of the run.
    pub coin: f64,
    /// Key balance at the end of the run.
    pub keys: u32,
    /// Account level at the end of the run.
    pub level: u32,
    /// Production sessions completed during the run.
    pub sessions_completed: u64,
    /// Whether the first milestone was claimed in the run.
    pub milestone_claimed: bool,
    /// Battle level at the end of the run (kills + 1).
    pub battle_level: u32,
    /// Coin collected from the battle reward in the run.
    pub battle_collected: f64,
}

/// Run the canonical fixture over `dynamics`.
pub fn run_fixture(dynamics: Dynamics) -> Result<FixtureRun, MilkError> {
    let mut session = Session::new(dynamics.clone());
    let mut hasher = TrajectoryHasher::new();
    // Buy the whole reduced roster (spec §2 prices).
    for spec in dynamics.girls() {
        session.buy_room(spec.index)?;
    }
    hash_session(&mut hasher, &session, &dynamics);
    // Production phase: tick and harvest until the account
    // reaches the first milestone gate (level 5, spec §6).
    let first_gate = dynamics.milestone(0)?.gate_level;
    let mut steps_executed = 0usize;
    while session.level() < first_gate && steps_executed < FIXTURE_STEPS_MAX {
        session.tick(FIXTURE_DT_SECONDS)?;
        steps_executed += 1;
        if steps_executed % FIXTURE_HARVEST_EVERY_STEPS == 0 {
            session.harvest_all();
        }
        hash_session(&mut hasher, &session, &dynamics);
    }
    if session.level() < first_gate {
        return Err(MilkError::Refused {
            reason: "fixture step bound reached before the milestone gate",
        });
    }
    // The milestone claim (spec §6, claim b1).
    session.claim_milestone(0)?;
    // Battle phase (spec §5): a fixed click count, then
    // collect whatever the kills accrued.
    for _ in 0..FIXTURE_BATTLE_ATTACKS {
        session.battle_attack();
    }
    let battle_collected = session.battle_collect();
    let battle = session.battle();
    hasher.write_f64(battle.hp);
    hasher.write_f64(battle.max_hp);
    hasher.write_f64(battle.pending_reward);
    hasher.write_u64(u64::from(battle.enemy_index));
    // A final harvest closes the loop.
    session.harvest_all();
    hash_session(&mut hasher, &session, &dynamics);
    Ok(FixtureRun {
        trajectory_hash: hasher.finish(),
        steps_executed,
        coin: session.coin(),
        keys: session.keys(),
        level: session.level(),
        sessions_completed: session.sessions_completed(),
        milestone_claimed: session.milestone_claimed(0)?,
        battle_level: battle.level,
        battle_collected,
    })
}

/// The fixture over the measured dynamics table — the run
/// `milkfactory-smoke` (Linux) executes, and the run a later
/// Android shell must reproduce hash-for-hash at S7.
pub fn run_measured_fixture() -> Result<FixtureRun, MilkError> {
    run_fixture(measured_dynamics()?)
}

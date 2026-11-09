//! torogao-milkfactory: the Milk Factory dynamic as Torogao
//! data + systems.
//!
//! Clean-room provenance. The dynamic was studied from the
//! estate's own copy of Milk Factory v1.22 (Unity Mono) under
//! the fathom clean-room discipline: behavior and tuning were
//! measured from the build's IL, never copied. The record of
//! record is `games/milk-factory-dynamics.md` in the Lane D
//! programme tree (`nsfw-remakes-20261108/`), with the S2b
//! animation note beside it; the tuning constants in
//! `data/milk-factory-dynamics.ron` cite the spec section
//! behind every value. No studied game's assets, art, text,
//! names, or code appear in this crate: what transfers is a
//! stamina-gated production cycle, a clicker battle, and a
//! milestone board, over a roster carried as data.
//!
//! The crate is a systems skeleton for the Torogao remake
//! programme (PIPELINE.md stage S4): pure, deterministic
//! functions over explicit inputs. No wall clock (the
//! original has none either — spec §3), no unseeded RNG, no
//! I/O beyond the RON loader in `dynamics`. The roster is
//! REDUCED FIRST (Owner ruling D-1104): girls 1..=4 of the
//! measured 18; the full roster is a data-only extension.

#![forbid(unsafe_code)]

pub mod dynamics;
pub mod session;
pub mod trajectory;

pub use dynamics::{
    DYNAMICS_RON, Dynamics, GirlSpec, MilestoneReward, MilestoneSpec, MilkError, StatBlock,
    measured_dynamics,
};
pub use session::{BattleState, GirlState, Session};
pub use trajectory::{FixtureRun, run_fixture, run_measured_fixture};

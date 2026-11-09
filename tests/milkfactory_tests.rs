//! Gate tests for torogao-milkfactory, split half validation
//! (the measured rules work as the spec says) and half
//! adversarial (refusals are typed and leave state unchanged),
//! per the Owner's standing test rule. Every constant asserted
//! here traces to games/milk-factory-dynamics.md; the section
//! is named in each test.

use torogao_milkfactory::dynamics::MilestoneReward;
use torogao_milkfactory::{
    DYNAMICS_RON, Dynamics, MilkError, Session, measured_dynamics, run_fixture,
    run_measured_fixture,
};

const DT: f64 = 1.0 / 60.0;
/// Generous step bound for the level climbs in these tests;
/// the measured rates reach each gate far inside it.
const CLIMB_STEPS_MAX: usize = 3_000_000;

fn new_session() -> Session {
    Session::new(measured_dynamics().expect("measured dynamics load"))
}

fn session_with_roster() -> Session {
    let mut session = new_session();
    for index in 1..=4 {
        session.buy_room(index).expect("room buys at start coin");
    }
    session
}

/// Tick (harvesting on the fixture cadence) until the
/// account reaches `target_level` or the bound runs out.
fn climb_to_level(session: &mut Session, target_level: u32) -> usize {
    let mut steps = 0usize;
    while session.level() < target_level && steps < CLIMB_STEPS_MAX {
        session.tick(DT).expect("fixture dt is valid");
        steps += 1;
        if steps % 600 == 0 {
            session.harvest_all();
        }
    }
    steps
}

// ----------------------------------------------------------------
// Validation: the measured rules, working as specced.
// ----------------------------------------------------------------

#[test]
fn measured_table_loads_with_the_reduced_roster() {
    // Spec §7 + D-1104: girls 1..=4 of the measured 18.
    let dynamics = measured_dynamics().expect("measured dynamics load");
    let girls = dynamics.girls();
    assert_eq!(girls.len(), 4);
    for (position, girl) in girls.iter().enumerate() {
        assert_eq!(girl.index, position as u32 + 1);
    }
    // Spec §2 room prices for rooms 1..=4.
    let prices: Vec<f64> = girls.iter().map(|g| g.room_price).collect();
    assert_eq!(prices, vec![500.0, 700.0, 900.0, 1100.0]);
    // Spec §7 girl 1 stats: cs2/cs4 fixed, rolled stats inside
    // their measured ranges.
    let g1 = &girls[0];
    assert_eq!(g1.stats.stamina_regen_per_second, 2.1);
    assert_eq!(g1.stats.milk_rate_per_second, 1.1);
    assert!((70.0..90.0).contains(&g1.stats.max_stamina));
    assert!((60.0..80.0).contains(&g1.stats.bottle_capacity));
    assert!((7.0..12.0).contains(&g1.stats.xp_per_session));
}

#[test]
fn new_game_defaults_match_the_spec() {
    // Spec §2 (coin/threshold/keys from gamectrl::Start; level
    // and XP are the untouched field defaults) and §5 (battle).
    let session = new_session();
    assert_eq!(session.coin(), 5000.0);
    assert_eq!(session.keys(), 0);
    assert_eq!(session.level(), 0);
    assert_eq!(session.xp(), 0.0);
    assert_eq!(session.xp_threshold(), 100.0);
    let battle = session.battle();
    assert_eq!(battle.damage, 10.0);
    assert_eq!(battle.level, 1);
    assert_eq!(battle.upgrade_cost, 200.0);
    assert_eq!(battle.max_hp, 50.0);
    assert_eq!(battle.hp, 50.0);
    assert_eq!(battle.pending_reward, 0.0);
    // Spec §3: a fresh girl starts at stamina 50.
    let girl = session.girl_state(1).expect("girl 1 exists");
    assert_eq!(girl.stamina, 50.0);
    assert!(!girl.owned);
    assert!(!girl.in_session);
}

#[test]
fn production_cycle_completes_and_grants_session_xp() {
    // Spec §3 + §4: rest to full, auto-start, produce, drain
    // at 5/s, session end grants exactly cs5 XP.
    let mut session = new_session();
    session.buy_room(1).expect("room 1 buys");
    let mut saw_session = false;
    let mut steps = 0usize;
    while session.sessions_completed() == 0 && steps < CLIMB_STEPS_MAX {
        session.tick(DT).expect("tick");
        steps += 1;
        if session.girl_state(1).expect("girl 1").in_session {
            saw_session = true;
        }
    }
    assert!(saw_session, "the cycle must enter a session");
    assert_eq!(session.sessions_completed(), 1);
    assert_eq!(session.xp(), 9.0, "girl 1 cs5 is 9 in the table");
    let girl = session.girl_state(1).expect("girl 1");
    assert_eq!(girl.stamina, 0.0);
    assert!(!girl.in_session);
    // Girl 1 session: cs1/5 = 16 s at cs4 1.1 -> 17.6 milk.
    assert!((girl.milk - 17.6).abs() < 0.2);
}

#[test]
fn harvest_converts_milk_to_coin_one_to_one() {
    // Spec §3 step 5: coin += luongsua; luongsua := 0.
    let mut session = new_session();
    session.buy_room(1).expect("room 1 buys");
    let coin_before = session.coin();
    let mut steps = 0usize;
    while session.sessions_completed() == 0 && steps < CLIMB_STEPS_MAX {
        session.tick(DT).expect("tick");
        steps += 1;
    }
    let milk = session.girl_state(1).expect("girl 1").milk;
    assert!(milk > 0.0);
    let harvested = session.harvest(1).expect("harvest owned girl");
    assert_eq!(harvested, milk);
    assert_eq!(session.coin(), coin_before + milk);
    assert_eq!(session.girl_state(1).expect("girl 1").milk, 0.0);
}

#[test]
fn bottle_capacity_caps_milk_across_sessions() {
    // Spec §3: production pauses at a full bottle; the cap
    // binds harvest timing, never the cycle. Girl 2's cap 60
    // is the reduced roster's tightest.
    let mut session = new_session();
    session.buy_room(2).expect("room 2 buys");
    let cap = session.girl_state(2).expect("girl 2").stats.bottle_capacity;
    assert_eq!(cap, 60.0);
    let mut steps = 0usize;
    while session.girl_state(2).expect("girl 2").milk < cap && steps < CLIMB_STEPS_MAX {
        session.tick(DT).expect("tick");
        steps += 1;
        let milk = session.girl_state(2).expect("girl 2").milk;
        assert!(milk <= cap, "milk must never exceed the bottle");
    }
    assert_eq!(session.girl_state(2).expect("girl 2").milk, cap);
    assert!(session.sessions_completed() >= 3);
}

#[test]
fn battle_kill_steps_the_enemy_and_accrues_the_reward() {
    // Spec §5: hp < 0 strictly — at damage 10 vs hp 50, the
    // sixth click kills (50 -> 0 is not a kill).
    let mut session = new_session();
    for _ in 0..5 {
        session.battle_attack();
    }
    assert_eq!(session.battle().hp, 0.0);
    assert_eq!(session.battle().level, 1, "hp == 0 is not a kill");
    session.battle_attack();
    let battle = session.battle();
    assert_eq!(battle.level, 2);
    assert_eq!(battle.max_hp, 60.0);
    assert_eq!(battle.hp, 60.0);
    assert_eq!(battle.pending_reward, 100.0);
    assert!((1..=19).contains(&battle.enemy_index));
    let coin_before = session.coin();
    assert_eq!(session.battle_collect(), 100.0);
    assert_eq!(session.coin(), coin_before + 100.0);
    assert_eq!(session.battle().pending_reward, 0.0);
}

#[test]
fn battle_upgrade_spends_and_scales_by_the_measured_steps() {
    // Spec §5: cost 200 -> damage +10, next cost 300.
    let mut session = new_session();
    session.battle_upgrade().expect("first upgrade affordable");
    assert_eq!(session.coin(), 4800.0);
    assert_eq!(session.battle().damage, 20.0);
    assert_eq!(session.battle().upgrade_cost, 300.0);
}

#[test]
fn milestone_board_pays_coin_then_keys_by_tier() {
    // Spec §6: b1 at level 5 pays 1000 coin; b2 at level 10
    // pays 5 keys.
    let mut session = session_with_roster();
    climb_to_level(&mut session, 10);
    assert!(session.level() >= 10);
    session.harvest_all();
    let coin_before = session.coin();
    let reward = session.claim_milestone(0).expect("b1 unlocked at 5");
    assert_eq!(reward, MilestoneReward::Coin(1000.0));
    assert_eq!(session.coin(), coin_before + 1000.0);
    let reward = session.claim_milestone(1).expect("b2 unlocked at 10");
    assert_eq!(reward, MilestoneReward::Keys(5));
    assert_eq!(session.keys(), 5);
}

#[test]
fn stat_upgrade_applies_increment_and_cost_growth() {
    // Spec §7: girl 1 cs1 +10 per purchase; cost starts 100
    // and grows by 80 + 20*1 = 100 per purchase.
    let mut session = new_session();
    session.buy_room(1).expect("room 1 buys");
    let coin_before = session.coin();
    session.upgrade_stat(1, 0).expect("upgrade affordable");
    let girl = session.girl_state(1).expect("girl 1");
    assert_eq!(girl.stats.max_stamina, 90.0);
    assert_eq!(girl.upgrade_levels[0], 1);
    assert_eq!(girl.upgrade_costs[0], 200.0);
    assert_eq!(session.coin(), coin_before - 100.0);
}

#[test]
fn star_upgrade_spends_three_keys_and_applies_flat_bonuses() {
    // Spec §7: 3 keys; cs1 +50, cs2 +1, cs3 +50, cs4 +0.5,
    // cs5 +25 — the same for every girl.
    let mut session = session_with_roster();
    climb_to_level(&mut session, 10);
    session.claim_milestone(0).expect("b1");
    session.claim_milestone(1).expect("b2 pays 5 keys");
    assert_eq!(session.keys(), 5);
    let before = session.girl_state(3).expect("girl 3").stats;
    session.upgrade_star(3).expect("star affordable at 5 keys");
    assert_eq!(session.keys(), 2);
    let girl = session.girl_state(3).expect("girl 3");
    assert_eq!(girl.stars, 1);
    assert_eq!(girl.stats.max_stamina, before.max_stamina + 50.0);
    assert_eq!(
        girl.stats.stamina_regen_per_second,
        before.stamina_regen_per_second + 1.0
    );
    assert_eq!(girl.stats.bottle_capacity, before.bottle_capacity + 50.0);
    assert_eq!(
        girl.stats.milk_rate_per_second,
        before.milk_rate_per_second + 0.5
    );
    assert_eq!(girl.stats.xp_per_session, before.xp_per_session + 25.0);
}

#[test]
fn fixture_reaches_every_loop_and_is_deterministic() {
    // The trajectory fixture: production, harvest, milestone,
    // battle, collect — and the same hash on a second run.
    let a = run_measured_fixture().expect("fixture runs");
    let b = run_measured_fixture().expect("fixture runs again");
    assert_eq!(a.trajectory_hash, b.trajectory_hash);
    assert_eq!(a.steps_executed, b.steps_executed);
    assert!(a.milestone_claimed);
    assert!(a.level >= 5);
    assert!(a.sessions_completed > 0);
    assert!(a.battle_level > 1, "25 clicks must score kills");
    assert!(a.battle_collected > 0.0);
    assert!(a.coin > 0.0);
}

// ----------------------------------------------------------------
// Adversarial: refusals are typed and leave state unchanged.
// ----------------------------------------------------------------

#[test]
fn ron_with_an_unknown_field_is_refused() {
    // The schema is closed: an unmeasured value cannot be
    // smuggled in as data (spec §10's unknowns stay absent).
    let poisoned = DYNAMICS_RON.replacen(
        "starting_coin: 5000.0,",
        "camera_zoom_max: 4.0,\n    starting_coin: 5000.0,",
        1,
    );
    assert!(Dynamics::from_ron(&poisoned).is_err());
}

#[test]
fn ron_with_a_missing_field_is_refused() {
    assert!(Dynamics::from_ron("()").is_err());
    assert!(Dynamics::from_ron("").is_err());
    let truncated = DYNAMICS_RON.replace("starting_coin: 5000.0,", "");
    assert!(Dynamics::from_ron(&truncated).is_err());
}

#[test]
fn ron_with_a_negative_room_price_is_refused() {
    let poisoned = DYNAMICS_RON.replacen("room_price: 500.0", "room_price: -500.0", 1);
    match Dynamics::from_ron(&poisoned) {
        Err(MilkError::Invalid { field, .. }) => assert_eq!(field, "girls"),
        other => panic!("expected Invalid(girls), got {other:?}"),
    }
}

#[test]
fn ron_with_shuffled_girl_indices_is_refused() {
    // Roster indices must be sequential from 1: a table that
    // renumbers its rows is not the measured table.
    let poisoned = DYNAMICS_RON.replacen("index: 2,", "index: 7,", 1);
    assert!(Dynamics::from_ron(&poisoned).is_err());
}

#[test]
fn ron_with_a_short_milestone_board_is_refused() {
    // Spec §6's board is exactly 10 claims; dropping the last
    // row must fail validation, not silently shrink the board.
    let poisoned = DYNAMICS_RON.replace("(gate_level: 50, reward: Keys(25)),", "");
    assert!(Dynamics::from_ron(&poisoned).is_err());
}

#[test]
fn tick_refuses_non_finite_and_out_of_bound_steps() {
    let mut session = session_with_roster();
    for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0, 1.5] {
        match session.tick(bad) {
            Err(MilkError::InvalidStep { .. }) => {}
            other => panic!("dt {bad} must be InvalidStep, got {other:?}"),
        }
    }
    // Nothing moved: a fresh roster girl is still at 50.
    let mut fresh = new_session();
    fresh.buy_room(1).expect("room 1 buys");
    assert!(fresh.tick(f64::NAN).is_err());
    assert_eq!(fresh.girl_state(1).expect("girl 1").stamina, 50.0);
}

#[test]
fn verbs_refuse_unowned_girls_without_mutating() {
    let mut session = new_session();
    assert!(session.harvest(1).is_err());
    assert!(session.upgrade_stat(1, 0).is_err());
    assert!(session.upgrade_star(1).is_err());
    assert_eq!(session.coin(), 5000.0);
    assert_eq!(session.girl_state(1).expect("girl 1").milk, 0.0);
}

#[test]
fn unknown_indices_are_typed_refusals() {
    let mut session = new_session();
    match session.buy_room(99) {
        Err(MilkError::UnknownGirl { index }) => assert_eq!(index, 99),
        other => panic!("expected UnknownGirl, got {other:?}"),
    }
    assert!(session.harvest(0).is_err());
    match session.claim_milestone(10) {
        Err(MilkError::UnknownMilestone { index }) => assert_eq!(index, 10),
        other => panic!("expected UnknownMilestone, got {other:?}"),
    }
    assert!(session.upgrade_stat(1, 5).is_err());
}

#[test]
fn buy_room_refuses_repeats_and_unaffordable_rooms() {
    let mut session = new_session();
    session.buy_room(1).expect("first buy succeeds");
    assert!(session.buy_room(1).is_err(), "repeat buy refused");
    // Drain the balance through battle upgrades (costs 200,
    // 300, ... 900 = 4400) until room 2 at 700 is out of reach.
    while session.coin() >= 700.0 {
        session.battle_upgrade().expect("upgrade while funded");
    }
    let coin_before = session.coin();
    assert!(session.buy_room(2).is_err());
    assert_eq!(session.coin(), coin_before, "a refused buy must not charge");
    assert!(!session.girl_state(2).expect("girl 2").owned);
}

#[test]
fn milestone_claim_refuses_below_gate_and_on_repeat() {
    let mut session = session_with_roster();
    assert!(
        session.claim_milestone(0).is_err(),
        "level 0 is below gate 5"
    );
    climb_to_level(&mut session, 5);
    session.claim_milestone(0).expect("first claim succeeds");
    let coin_before = session.coin();
    assert!(session.claim_milestone(0).is_err(), "one-shot means one");
    assert_eq!(session.coin(), coin_before, "a refused claim must not pay");
}

#[test]
fn battle_upgrade_refuses_when_the_balance_cannot_cover_it() {
    let mut session = new_session();
    while session.coin() >= session.battle().upgrade_cost {
        session.battle_upgrade().expect("upgrade while funded");
    }
    let (coin_before, damage_before) = (session.coin(), session.battle().damage);
    assert!(session.battle_upgrade().is_err());
    assert_eq!(session.coin(), coin_before);
    assert_eq!(session.battle().damage, damage_before);
}

#[test]
fn fixture_hash_is_sensitive_to_the_table() {
    // Adversarial to the digest itself: change one measured
    // rate and the trajectory hash must move — a hash that
    // cannot see the table is not evidence of anything.
    let altered = DYNAMICS_RON.replacen("cs4: 1.1,", "cs4: 1.2,", 1);
    let altered_dynamics = Dynamics::from_ron(&altered).expect("altered table still validates");
    let baseline = run_measured_fixture().expect("baseline fixture");
    let altered_run = run_fixture(altered_dynamics).expect("altered fixture");
    assert_ne!(baseline.trajectory_hash, altered_run.trajectory_hash);
}

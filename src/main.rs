//! milkfactory-smoke: a deterministic smoke run of the Milk
//! Factory systems on the measured table — the canonical
//! fixture from the crate's `trajectory` module (buy the
//! reduced roster, produce and harvest to the first milestone
//! gate, claim it, fight and collect, harvest again).
//! Prints the outcome and the fixture's trajectory hash;
//! exit 0 on a completed run, 1 on refusal. This is the
//! remake's Linux-native proof artifact, not a game. A later
//! Android shell runs the same fixture from the same module;
//! the printed hash is the S7 cross-platform comparison
//! value.

use torogao_milkfactory::run_measured_fixture;

fn main() {
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    match run_measured_fixture() {
        Ok(run) => {
            println!(
                "milkfactory-smoke: success=true steps={} coin={:.4} keys={} level={} \
                 sessions={} milestone_claimed={} battle_level={} battle_collected={:.4} \
                 hash={:016x}",
                run.steps_executed,
                run.coin,
                run.keys,
                run.level,
                run.sessions_completed,
                run.milestone_claimed,
                run.battle_level,
                run.battle_collected,
                run.trajectory_hash,
            );
            0
        }
        Err(e) => {
            eprintln!("milkfactory-smoke: FAILED: {e}");
            1
        }
    }
}

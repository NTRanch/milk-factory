# milk-factory — torogao-milkfactory

A clean-room, from-measurement reimplementation of the systems of
**Milk Factory** (v1.22, Unity), built for the Torogao remake
programme by Nice Thoughts Ranch. No assets, text, or code from the
studied game are included: every tuning number in
`data/milk-factory-dynamics.ron` cites the spec section it was
measured from, and the roster content is estate-original placeholder
data only.

## What the crate models

- A continuous per-girl stamina production cycle — rest regeneration,
  session auto-start at full stamina, production against a bare
  stamina drain, experience paid once at session end. There is no
  clock or day frame in the original; the caller's `dt` is the only
  time input.
- Harvest at a 1:1 conversion, at any time, uncapped.
- Account levels: thresholds of 100 XP, +100 per level, overflow
  discarded.
- A clicker battle loop: fixed damage steps, a kill only on strictly
  negative HP, a reward pool, and an upgrade ladder; enemy redraws
  come from a seeded xorshift64* (the original's draw is unseeded;
  the seeded form is what makes the remake deterministic).
- A milestone claim board: ten one-shot claims at account levels
  5–50, whose key rewards exactly fund the star-upgrade track — the
  cross-system coupling is exercised by the test suite.
- Stat and star upgrades with the measured cost growth.
- A reduced-first roster as data (4 of the measured 18 members);
  extending the roster is a data-only change — the loader bounds it
  (≤ 18, indices sequential from 1) and refuses unknown fields.

## Dual-target law

The remake programme ships every title on native Linux **and** Android
from this one crate plus its data file. Platform shells bind to the
crate; the systems and data live here only.

## Determinism

Sessions are deterministic: a fixed input stream replays to a fixed
trajectory hash (FNV-1a 64). The fixture is exercised by the test
suite, and the same hash must match on Linux and Android (the
programme's S7 gate).

## Build and test

```sh
cargo build
cargo test
cargo run --bin milkfactory-smoke   # full-loop session; prints the smoke line
```

## License

BSD 2-Clause. See `LICENSE`. The studied game remains the property of
its respective developer; this repository is an independent
reimplementation of measured behaviour.

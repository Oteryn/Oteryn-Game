# ATTACK-DIST-0: auto-attack draw distribution

- Decision: `ATTACK-DIST-0`
- Status: **PROPOSED**. It becomes accepted only when the control plane accepts it on #162. Runtime, protocol, migration and production authority: NONE.
- Answers: ATTACK-PARITY-1 packet 1.3 and 1.4, after ATTACK-PARITY-1b (#2091): min and max equal all 1536 TibiaPal rows, the uniform mean does not.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Evidence

Fixture `content/combat/parity/tibiapal_auto_attack_v1.json` (1536 rows). Each row is `raw.min = floor(f + v/2)`, `raw.avg = floor(f + v)`, `raw.max = floor(f + 2v)`. `f` is the level term and `v` the skill and attack term (packet 1.3).

- The engine draws uniformly over `[min, max]` (`gameplay_transport/attack.rs`, `draw % span`). Its mean is `(min + max) / 2 = f + 1.25 v`.
- Residual `raw.avg - (min + max) / 2` lies in `[-69, -1]` on all 1536 rows. The uniform mean is 0.5 % to 26.5 % above TibiaPal's average.
- TibiaPal publishes only min, avg and max. No histogram or sample is available, so the shape is **not observable**; only the mean is.
- A density that falls linearly from `min` to `max` (a triangular distribution with its mode at `min`) has mean `(2 min + max) / 3 = f + v`. The residual `raw.avg - (2 min + max) / 3` lies in `[-1/3, +1/3]` on all 1536 rows, which is the rounding of the three `floor` calls.

So a linear-decreasing density reproduces the published mean on every row. It is the simplest integer-drawable shape that does, but it is a fit to one moment, not a measured distribution.

## 2. Options

- **A. Keep uniform.** No code change. Average damage stays 0.5 % to 26.5 % above TibiaPal and the attack values stay `PARITY_PENDING` for good.
- **B. Linear-decreasing draw (recommended).** Over `n = max - min + 1` values, weight `n - k` for offset `k = 0..n-1`, total `T = n(n+1)/2`. Draw `r = draw % T` and take the smallest `k` with `(k + 1)(2n - k)/2 > r`, solved with an integer square root. Integer-only, deterministic from the existing `deterministic_decision_u64`, and its exact mean is `min + (max - min)/3 = (2 min + max) / 3`. The parity test then asserts the exact mean within the rounding band above and the attack values may become `MATCHED_TIBIAPAL` for Offensive.
- **C. Wait for a measured distribution** (a sampled TibiaPal or official Tibia source). Highest cost, no date.

## 3. Recommendation and scope of the follow-up

Take B as ATTACK-DIST-1, one PR, owned paths `combat/attack/**`, `gameplay_transport/attack.rs` (the one draw site) and the constants table. Changing the damage distribution of player auto-attacks is player-visible, so the owner's acceptance goes through the control plane, not a worker. Seed use and the draw count per swing stay as they are (one `u64` per swing), so replay determinism keeps its shape, but the drawn values change.

Not in scope: the Balanced and Defensive factors, monk, creature melee, distance weapons, critical hits.

## 4. Owner question

1. Accept option B (ATTACK-DIST-1), keep A, or wait for C? Recommendation: B.

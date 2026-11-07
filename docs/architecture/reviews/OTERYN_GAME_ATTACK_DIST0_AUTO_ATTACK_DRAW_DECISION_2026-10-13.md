# ATTACK-DIST-0: auto-attack draw distribution

- Decision: `ATTACK-DIST-0`
- Status: **BACKLOG** (decision test answer: not needed now, section 5). No acceptance is requested and no implementation is recommended now. Runtime, protocol, migration and production authority: NONE.
- Answers: ATTACK-PARITY-1 packet 1.3 and 1.4, after ATTACK-PARITY-1b (#2091): min and max equal all 1536 TibiaPal rows, the draw mean does not.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Evidence

Fixture `content/combat/parity/tibiapal_auto_attack_v1.json` (1536 rows). Each row is `raw.min = floor(f + v/2)`, `raw.avg = floor(f + v)`, `raw.max = floor(f + 2v)`. `f` is the level term and `v` the skill and attack term (packet 1.3).

- The engine draws over `[min, max]` as `draw % span` on a `u64` (`gameplay_transport/attack.rs`). For an ideal uniform draw the mean of the floored endpoints is `(min + max) / 2`, whose unrounded analogue is `f + 1.25 v` (the two endpoint floors are not dropped by that analogue, so it is not an identity); the residual `raw.avg - (min + max) / 2` lies in `[-69, -1]` on all 1536 rows (0.5 % to 26.5 % above TibiaPal's average), and the floor gate passes on 0 rows.
- The real modulo draw is not exactly uniform when `span` does not divide `2^64`: lower offsets occur once more often, so its finite-domain mean is below `(min + max) / 2` by at most about `1.12e-15` (the exact maximum over the fixture spans, `rem * (span - rem) / (2 * 2^64)` with `rem = 2^64 mod span`, reached at span 406). Computed exactly over the fixture, the floor gate then passes on 24 of 1536 rows (those with an odd span and `(min + max) / 2` integer, where `floor` of the infinitesimally lower mean is `raw.avg`, for example min 3, avg 5, max 9), and the residual is `-1` plus that infinitesimal. This is an arithmetic artifact, not parity: 1512 rows still fail by 1 to 69.
- TibiaPal publishes only min, avg and max. No histogram or sample is available, so the shape is **not observable**; only the mean is.
- A density that falls linearly from `min` to `max` (a triangular distribution with its mode at `min`) has mean `(2 min + max) / 3` over the floored endpoints, whose unrounded analogue is `f + v`. The residual `raw.avg - (2 min + max) / 3` lies in `[-1/3, +1/3]` on all 1536 rows (rounding of the three `floor` calls).

This is **not floor-exact**. Packet 1.4 requires `floor(mean) == raw.avg` on every row. That holds on 1224 rows for the exact rational mean and fails on 312 of 1536, with residual +1/3 (for example min 8, avg 16, max 31: exact mean 47/3 = 15.67, floor 15, raw.avg 16). Because only min, avg and max are published and `raw.avg` is itself floored, no single parametric shape is shown to meet the gate on all rows; no floor-exact construction is demonstrated here.

So a linear-decreasing density is the simplest integer-drawable shape that lands within rounding of the published mean, as a fit to one moment, not a measured distribution. It does not satisfy the 1.4 gate as written.

## 2. Options (for a future decision)

- **A. Keep uniform.** No code change. Average damage stays 0.5 % to 26.5 % above TibiaPal and the attack values stay `PARITY_PENDING` for good.
- **B. Linear-decreasing draw.** Over `n = max - min + 1` values, weight `n - k` for offset `k = 0..n-1`, total `T = n(n+1)/2`. Draw an unbiased `r` in `[0, T)` by rejection sampling: take the next `u64` and reject it when it is `>= floor(2^64 / T) * T` (probability below `T / 2^64`, so one draw per swing in practice); further candidates come from `deterministic_decision_u64` with an incrementing attempt index, so replay stays deterministic. A plain `draw % T` is **not** used: when `T` does not divide `2^64` the lower residues occur once more often, which biases the mean slightly downward. Then take the smallest `k` with `(k + 1)(2n - k)/2 > r`, solved with an integer square root. Integer-only, deterministic from the existing `deterministic_decision_u64`, and the mean of the finite weighted domain is exactly `(2 min + max) / 3` when `r` is unbiased; the implemented draw is unbiased up to a rejection probability under `T / 2^64`, and the mean is not claimed to be exact beyond that. The parity test would assert the mean within the +-1/3 rounding band, which is a **tolerance, not the 1.4 floor-equality gate**. Therefore B alone does not relabel anything: the attack values in `parity_by_value` stay `PARITY_PENDING` and nothing becomes `MATCHED_TIBIAPAL`, unless the owner separately accepts the +-1/3 band as the mean gate (an amendment of packet 1.4) or a floor-exact construction is demonstrated.
- **C. Wait for a measured distribution** (a sampled TibiaPal or official Tibia source). Highest cost, no date. Attack values stay `PARITY_PENDING`.

## 3. Disposition and scope if reopened

No decision is requested now. Register this subject in the combat backlog. If a later decision takes B, ATTACK-DIST-1 would be one PR with owned paths `combat/attack/**`, `gameplay_transport/attack.rs` (the one draw site) and the constants table. Changing the damage distribution of player auto-attacks is player-visible, so acceptance would go through the control plane and the owner, not a worker. The draw count per swing becomes variable only on the vanishingly rare rejection; seed derivation stays deterministic per attempt index, so replay determinism holds, but the drawn values change.

The shared draw site in `gameplay_transport/attack.rs` serves Offensive, Balanced (the default) and Defensive, but all 1536 evidence rows are Offensive. ATTACK-DIST-1 therefore requires an Offensive-only branch: Balanced and Defensive keep the uniform draw until evidence for them is gathered and accepted.

Not in scope: the Balanced and Defensive factors, monk, creature melee, distance weapons, critical hits.

## 4. Owner questions

None now. When the subject is reopened (section 5, item 4): accept B, keep A, or wait for C, and whether packet 1.4 may use a +-1/3 mean band. Until then the attack values stay `PARITY_PENDING` and the current draw is unchanged.

## 5. Decision timing (ARCHITECTURE_DECISION_DISCIPLINE)

1. **Must decide now?** NO. Option B does not meet the packet 1.4 floor gate, so nothing becomes `MATCHED_TIBIAPAL` either way; the uniform draw is a working default.
2. **Downstream work blocked:** only a future `MATCHED_TIBIAPAL` relabel of the Offensive attack values and ATTACK-DIST-1 itself. Nothing on the playable path is blocked.
3. **Harder later:** a player-visible change of average damage, replay-visible once shipped (drawn values change), and a +-1/3 tolerance, if accepted, would weaken the packet 1.4 gate for later rows.
4. **Superseding evidence:** a measured damage distribution (sampled TibiaPal or official source), a floor-exact construction over all 1536 rows, or an owner decision on the mean gate.
5. **Deliberately not decided:** Balanced and Defensive draws, monk, creature melee, distance weapons, criticals, and any change to packet 1.4.

Consequence: option A is the current state; this packet is registered as backlog and ATTACK-DIST-1 is not started.

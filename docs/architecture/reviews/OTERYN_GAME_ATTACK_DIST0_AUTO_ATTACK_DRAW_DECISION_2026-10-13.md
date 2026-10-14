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
- A density that falls linearly from `min` to `max` (a triangular distribution with its mode at `min`) has mean `(2 min + max) / 3 = f + v` before rounding. The residual `raw.avg - (2 min + max) / 3` lies in `[-1/3, +1/3]` on all 1536 rows (rounding of the three `floor` calls).

This is **not floor-exact**. Packet 1.4 requires `floor(mean) == raw.avg` on every row. That holds on 1224 rows for the exact rational mean and fails on 312 of 1536, with residual +1/3 (for example min 8, avg 16, max 31: exact mean 47/3 = 15.67, floor 15, raw.avg 16). Because only min, avg and max are published and `raw.avg` is itself floored, no single parametric shape is shown to meet the gate on all rows; no floor-exact construction is demonstrated here.

So a linear-decreasing density is the simplest integer-drawable shape that lands within rounding of the published mean, as a fit to one moment, not a measured distribution. It does not satisfy the 1.4 gate as written.

## 2. Options

- **A. Keep uniform.** No code change. Average damage stays 0.5 % to 26.5 % above TibiaPal and the attack values stay `PARITY_PENDING` for good.
- **B. Linear-decreasing draw (recommended).** Over `n = max - min + 1` values, weight `n - k` for offset `k = 0..n-1`, total `T = n(n+1)/2`. Draw an unbiased `r` in `[0, T)` by rejection sampling: take the next `u64` and reject it when it is `>= floor(2^64 / T) * T` (probability below `T / 2^64`, so one draw per swing in practice); further candidates come from `deterministic_decision_u64` with an incrementing attempt index, so replay stays deterministic. A plain `draw % T` is **not** used: when `T` does not divide `2^64` the lower residues occur once more often, which biases the mean slightly downward. Then take the smallest `k` with `(k + 1)(2n - k)/2 > r`, solved with an integer square root. Integer-only, deterministic from the existing `deterministic_decision_u64`, and the mean of the finite weighted domain is exactly `(2 min + max) / 3` when `r` is unbiased; the implemented draw is unbiased up to a rejection probability under `T / 2^64`, and the mean is not claimed to be exact beyond that. The parity test would assert the mean within the +-1/3 rounding band, which is a **tolerance, not the 1.4 floor-equality gate**. Therefore B alone does not relabel anything: the attack values in `parity_by_value` stay `PARITY_PENDING` and nothing becomes `MATCHED_TIBIAPAL`, unless the owner separately accepts the +-1/3 band as the mean gate (an amendment of packet 1.4) or a floor-exact construction is demonstrated.
- **C. Wait for a measured distribution** (a sampled TibiaPal or official Tibia source). Highest cost, no date. Attack values stay `PARITY_PENDING`.

## 3. Recommendation and scope of the follow-up

Take B as ATTACK-DIST-1, one PR, owned paths `combat/attack/**`, `gameplay_transport/attack.rs` (the one draw site) and the constants table. Changing the damage distribution of player auto-attacks is player-visible, so the owner's acceptance goes through the control plane, not a worker. The draw count per swing becomes variable only on the vanishingly rare rejection; seed derivation stays deterministic per attempt index, so replay determinism holds, but the drawn values change.

The shared draw site in `gameplay_transport/attack.rs` serves Offensive, Balanced (the default) and Defensive, but all 1536 evidence rows are Offensive. ATTACK-DIST-1 therefore requires an Offensive-only branch: Balanced and Defensive keep the uniform draw until evidence for them is gathered and accepted.

Not in scope: the Balanced and Defensive factors, monk, creature melee, distance weapons, critical hits.

## 4. Owner question

1. Accept option B (ATTACK-DIST-1), keep A, or wait for C? Recommendation: B.
2. Only if B: may packet 1.4 be amended so the mean gate is the +-1/3 band? Recommendation: no relabel without it; default answer keeps the values `PARITY_PENDING`.

## 5. Decision timing (ARCHITECTURE_DECISION_DISCIPLINE)

1. **Must decide now?** NO. Option B does not meet the packet 1.4 floor gate, so nothing becomes `MATCHED_TIBIAPAL` either way; the uniform draw is a working default.
2. **Downstream work blocked:** only a future `MATCHED_TIBIAPAL` relabel of the Offensive attack values and ATTACK-DIST-1 itself. Nothing on the playable path is blocked.
3. **Harder later:** a player-visible change of average damage, replay-visible once shipped (drawn values change), and a +-1/3 tolerance, if accepted, would weaken the packet 1.4 gate for later rows.
4. **Superseding evidence:** a measured damage distribution (sampled TibiaPal or official source), a floor-exact construction over all 1536 rows, or an owner decision on the mean gate.
5. **Deliberately not decided:** Balanced and Defensive draws, monk, creature melee, distance weapons, criticals, and any change to packet 1.4.

Consequence: if the owner prefers no change now, option A (or C) is acceptable and this packet can be registered as backlog without implementing ATTACK-DIST-1.

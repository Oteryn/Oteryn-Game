# Boss resilience and participation reference

Task: `NEWS-HIST-BOSS-RESILIENCE-1`. News sources: 7609, 7719 and 7861.

## Pinned server evidence

- Canary is examined at `47dfd51f45280a59a1d3e50ba7edd573d7234446`; Crystal is examined at
  `201189f871d23597b82caf88fd611454a2ddefb8`.
- Both pinned `data/scripts/systems/reward_chest.lua` implementations collect three participation
  dimensions: damage dealt, damage received and healing of another player. They sum each dimension
  independently, divide each contribution by its dimension total, then divide the sum by **3**
  (Canary lines 27–59; Crystal lines 27–59).
- Both pinned implementations initialise each dimension total to **0.1** to avoid division by zero,
  and treat a nonzero resulting score as participation (both files, lines 27–29 and 52–59).
- Both pinned implementations record healing only when the healer is a player other than the healed
  character. Damage received is recorded only when the attacker is the boss (both files, lines
  136–167).
- Both pinned `src/creatures/monsters/monster.cpp` implementations refuse target-distance changes for
  reward bosses (Canary lines 3488–3506; Crystal lines 2616–2634). This is the only pinned generic
  reward-boss resilience rule found in the native creature owner.

## News reconciliation

| News | Evidence | Disposition |
|---|---|---|
| 7609 | On spawn, selected bosses take less damage and the defence gradually returns to normal. | The exact boss list, initial percentage, duration and curve are **UNKNOWN** in both pinned implementations; no numbers are guessed. |
| 7719 | Participation other than damage dealt is reduced by the percentage of current bonus boss resilience. | Implemented as a parameterised integer percentage; damage dealt is unchanged. |
| 7861 | A character must deal actual boss damage to qualify; support-only participation is insufficient. | Implemented as a positive-damage gate at reward dispatch. |

The pinned reward script does not contain the news-era resilience adjustment or damage prerequisite.
Oteryn therefore keeps the source-backed three dimensions but applies the later news rules in the
native encounter owner. It does not reproduce the pinned Lua loot-factor formula: NEWS 7609 does not
supply its missing resilience numbers, and floating-point loot generation remains outside this task.

## Oteryn deviation: 10% fair-share reward threshold

Owner decision 2026-10-27 ("zgoda na próg 10%"). Implemented as
`REWARD_FAIR_SHARE_THRESHOLD_PERCENT` and `reward_eligibility` in
`crates/gameplay/encounter-rules/src/boss.rs`, and `settle_rewards` in
`crates/gameplay/encounters/src/bosses/resilience.rs`.

**Rule.** A character earns a boss reward only if it dealt boss damage (NEWS 7861) **and** its
resilience-adjusted score `(d/D + r/R + h/H) / 3` is at least 10% of the fair share `1 / n`, where
`n` counts the characters that dealt damage. The comparison is integer fixed-point.

**Why.** In both pinned reward scripts the loot factor is
`n^(-1/3) * (1 + n^(-1/3))^(score / (1/n))`. Its floor `n^(-1/3)` applies to any nonzero score, so a
single hit is worth about half a full reward. Real Tibia behaviour for this case is **UNKNOWN**;
the threshold is an Oteryn rule, not parity.

**Test behind the number.** The pinned formula was reproduced and run on these scenarios
(boss with 1,000,000 HP; values are loot factors; 1.0 is an average share):

| Scenario | Pinned formula | 10% threshold | 25% threshold |
|---|---|---|---|
| 5-player team (EK, RP, MS, ED, weak DPS at 2.6% of score) | EK 1.212, RP 0.783, MS 0.797, ED 1.458, weak DPS 0.620 | unchanged | weak DPS **cut to 0** |
| Same team plus a 1-hit leech | leech **0.550**; others shift slightly | leech **0**; team as in row 1 | leech 0, weak DPS 0 |
| Solo EK plus a 1-hit leech | EK 2.554, leech **0.794** | EK 2.000, leech **0** | EK 2.000, leech 0 |

10% removes token hits and keeps every real contributor, including a weak damage dealer and a
healer who hit once. 25% was rejected because it removes the weak damage dealer.

The simulation (Python, same formula as `reward_chest.lua`):

```python
def factors(parts, thr=0.0):  # parts: (name, dealt, received, healed)
    D = 0.1 + sum(p[1] for p in parts); R = 0.1 + sum(p[2] for p in parts)
    H = 0.1 + sum(p[3] for p in parts)
    sc = [(n, (d / D + r / R + h / H) / 3, d) for n, d, r, h in parts]
    elig = [s for s in sc if s[1] != 0 and s[2] > 0]
    if thr:
        elig = [s for s in elig if s[1] >= thr / len(elig)]
    n = len(elig); base = n ** (-1 / 3)
    out = {s[0]: 0.0 for s in sc}
    for name, s, _ in elig:
        out[name] = base * (1 + base) ** (s * n)
    return out
```

The unit tests in `boss.rs` and `resilience.rs` pin the same scenarios.

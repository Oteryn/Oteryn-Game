# OTV2-20260928-death1a-outcome-calculator

```yaml
task_id: OTV2-20260928-death1a-outcome-calculator
title: DEATH-1a - pure PvE death-outcome calculator
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1154
allocation_comment: "#162 5876229534 (request), confirmation: owner-authorized in this window (2026-09-28)"
base_branch: main
branch: claude/vigilant-carson-067781
base_sha: 840072adb9d6682fe73f3e8559fe5f36bfce9578
head_sha: null
owner: "Oteryn: impl domains" (Claude Code)
created_at: 2026-09-28T19:15:00Z
updated_at: 2026-09-28T19:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/death.rs
  - apps/game-server/src/domain/mod.rs  # one line: pub mod death;
  - docs/agents/tasks/active/OTV2-20260928-death1a-outcome-calculator.md
public_contracts: []
depends_on: []
blocks: [DEATH-1b]
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`apps/game-server/src/domain/death.rs` is a pure, persistence-neutral calculator for the PvE
player death outcome of `REFERENCE-FIRST-PLAYER-DEATH-V1` §4.3.1 to §4.3.4. It has no caller yet.

- **XP loss (D58, D59, D66, D68):** `floor((L+50) × 50 × (L²−5L+8) × (100 − 8b − 30p) / 10 000)`,
  the same formula from level 1, capped at the current experience. `b` is `0..=7` regular
  blessings; `p` is the caller's promotion-benefit-current flag (promoted and Premium current, D76).
  The arithmetic is exact `u128`. A test proves the identity with `((L+50)/100) × (XPThreshold(L) −
  XPThreshold(L−1))` for levels 2 to 1,000.
- **Blessings (§4.3.2):** every held regular blessing is consumed.
- **Amulet of Loss (§4.3.3):** worn in the necklace slot with fewer than 5 blessings, it is
  selected for consumption and no item is lost.
- **Exemption (D65):** no item loss at level ≤ 8 or without a vocation.
- **Lost-item set (D62, §4.3.4):** one roll per occupied slot, drawn with
  `deterministic_decision_u64` bound to the death occurrence (purpose `oteryn.death.item_loss.v1`,
  draw index = slot). The container slot uses 100/70/45/25/10/0 %, every other slot
  10/7/4.5/2.5/1/0 %, in per mille. The set is in canonical slot order and independent of input order.

## Excluded scope

`domain/progression.rs` and the `ProgressionOperation` wiring (DEATH-1b), durability, migrations,
the registry, the decision-root source, the D54 floor, respawn, temples, DUR-03 moves, skulls and PvP.

## Evidence and open readings

- PROVEN: the closed form matches the manual's multiples at levels 50/100/150/180/200; 7 regular
  blessings × 8 % plus promotion 30 % = 86 %; the level-8 floor is inclusive.
- Reading (see the allocation): "each equipped item" covers every occupied non-container slot,
  the ammo slot included. Global's slot set is UNKNOWN; Canary includes ammo (OTS_HYPOTHESIS_ONLY).
- UNKNOWN: the post-Newhaven low-level exemption at the target date. The item floor is the D65
  value 8, one named constant.

## Validation

- `cargo fmt --all --check`
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
- `cargo test -p oteryn-game-server --lib domain::death`
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`

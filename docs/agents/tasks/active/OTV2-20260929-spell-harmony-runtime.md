# OTV2-20260929-spell-harmony-runtime

```yaml
task_id: OTV2-20260929-spell-harmony-runtime
title: Monk Harmony and Serene engine rules in the runtime actor (SPELL-D8 H-2)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
allocation_comment: "#162 5891701482"
base_branch: main
branch: claude/spell-harmony-runtime
base_sha: b90f85c9aba8f74f344bdb0969ee04f9c6eebe31
head_sha: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T14:00:00Z
updated_at: 2026-09-29T14:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/tests/*spell*
  - docs/agents/tasks/active/OTV2-20260929-spell-harmony-runtime.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-part-b.md
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md  # §8.2 forced Serene durability line only
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

The monk Harmony and Serene engine rules of SPELL-D8
(`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §3, §4, §8.2 H-2) and
`OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §A.2, in `apps/game-server/src/spell/harmony.rs`. Not composed:
no runtime actor, wire or persistence changes.

- **Live state.** `MonkState` holds Harmony 0..5 and the remaining forced Serene time, both loaded from values the
  caller supplies. A value outside 0..5, a forced time above 7000 ms, or a value other than 0 for another vocation
  is corrupt state and the load fails closed.
- **Serene.** Evaluated at initialization (fresh admission, respawn, same-GameSession reconnect, FND-04B §21
  recovery), before any command is accepted, and then every 1000 ms. Until the initialization evaluation, every
  command, commit and tick is refused (`NotInitialized`). Solo rule: without a party service (`SoloParty`) the
  monk is always Serene. With a party, the §A.2 step 6 rule applies: another member visible (same floor,
  dx in [-8, 9], dy in [-6, 7]) and 8 or more adjacent creatures end Serene.
- **Focus Serenity.** Forces Serene for 7000 ms. The evaluation cannot remove a forced state. A forced time is
  kept across detach and reinitialization. A loaded remaining time runs from the first initialization evaluation
  (owner decision Q1=b, 2026-09-29; the contract's §8.2 durability line is amended to match).
- **Multiplier and transitions.** `HarmonyMultiplier` is the exact fraction
  `1 + (7 + 0.005 * level) * V * 2^(c - 1) / 100`; bounds are truncated toward zero. Builder: +1 up to 5
  (0 gained at 5). Spender: all charges spent. Focus fill: to 5. Reading the multiplier changes nothing, so a
  failed cast keeps Harmony.

## Admission gate

Harmony is not durable yet (H-1), so a logout would lose it, which players would notice. The Harmony spells
therefore stay fail-closed in the spell reader (`authoring.rs`): every `harmony_role` (builder and spender)
and the `monk_focus` native key of the `monk_harmony_virtue` family are rejected. A test pins this. The engine
rules ship behind the gate.

## Carry-over

- **H-1:** the durable Harmony field and the durable remaining forced Serene time, with the fenced actor-end
  write. Both need a Harmony receipt kind, which is decided together with the DEATH-0/STANCE-0 guard rewrite
  chain (DUR-02 rule 2 vs composition §3.6 / the 0009 guard).
- **Death:** DEATH-1 "Harmony := 0", plus the lethal-commit Serene clear.
- **H-3:** the `ActorVitalsV1` wire (`harmony = 6`, `serene = 7`).
- **Wheel of Destiny:** the Ascetic points of §A.2 step 2 and Sanctuary. These need PREM-1/2.
- **Virtues and stance:** Virtue of Harmony state and refund, Justice, Sustain, Virtue Healing and party bonuses.
  The multiplier takes a virtue input, and the owner passes none (the §4 interim rule).
- **Integration:** runtime actor composition, the cast resolution hook (spender bounds, Focus Serenity
  spender cooldown reset), and lifting the admission gate once H-1 lands.

## Validation

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-game-server`
- `python tools/agents/validate_governance.py`

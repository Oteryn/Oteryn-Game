# OTV2-20260929-charm4-charm-effects

```yaml
task_id: OTV2-20260929-charm4-charm-effects
title: CHARM-4 - combat-side Charm effect engine (all 25 charms, fail-closed hooks)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm4-charm-effects
issue: 162
lane_id: GAME-COMBAT charms
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 4ea220fa
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "CHARM-4 hard worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
control_plane: null   # owner-authorized in session 2026-09-29; lead receives the review packet
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/charm_effects.rs
  - apps/game-server/src/combat/charm_effects_tests.rs
  - apps/game-server/src/combat.rs   # module registration line only
  - docs/agents/tasks/archive/OTV2-20260929-charm4-charm-effects.md
public_contracts: []
depends_on:
  - "CHARM-0 decision packet #1295 (§4.3, owner answers §7: 4c, 5a)"
  - "#1293 charm authoring candidate at d89f30f3 (effect shapes, damage flags, per-stage values)"
  - "CHARM-1 content loader builds CharmDefinition (not yet merged)"
  - "CHARM-3 charm state implements CharmStateRead (branch not yet published)"
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

`apps/game-server/src/combat/charm_effects.rs` is the combat-side Charm effect engine.
`evaluate_charm_hook` takes the attacking character, the creature's Bestiary race key, a decision
root and the event's occurrence, and evaluates every charm that character assigned to that race
whose effect belongs to the event's hook. It is pure: it reads, rolls and returns typed outcomes.

- **Effects:** all 18 effect types of `charm.schema.json` at #1293 `d89f30f3`, with a closed Rust
  shape each. Parameters come from the catalogue shape, never per charm. `CharmDefinition::new`
  re-checks the catalogue rules combat relies on: stage value kind, strictly increasing stages,
  trigger chance at most 100%, and non-zero durations and caps.
- **Damage:** every damage effect carries a `CharmDamageKind` (`element`, `ignores_resistances`,
  `reduced_by_armor`), and its outcome passes it on so the applying system mitigates correctly.
  - `attack_proc_damage` is the stage percent of the creature's maximum health, capped at
    `multiplier × level` before resistances.
  - `attack_proc_resource_damage` is the percent of the character's own maximum health or mana,
    capped at the percent of the creature's maximum health.
  - `kill_area_damage` (Carnage) is the percent of the killed creature's maximum health, capped at
    `multiplier × level` (6×). It runs only on the character's own lethal hit. The outcome
    documents the targets: the four non-diagonal adjacent squares, not summons.
  - `reflect_damage_taken` (Parry) reflects the base damage taken, before the character's own
    resistances.
  - Charm damage never chains: a hit with `CharmHitSource::CharmDamage` runs no charm hook.
  - All use exact integer floor arithmetic in hundredths of a percent.
- **Hooks:** every effect type belongs to one `CharmHook`. A committed hit
  (`CharmHookEvent::committed_hit(&OwnerDamageResult, ..)`) runs `AttackHit`. When the hit was
  lethal it also runs `CreatureKilled`, and effects on the attacked creature then return
  `NoLivingTarget` without a roll.
- **Determinism:** each trigger roll is one `deterministic_decision_u64` draw.
  - Purpose `oteryn.charm.trigger.v1`, draw index `hook × 2 + category`.
  - The draw maps onto hundredths of a percent by multiply-shift.
  - A carrier replay (`applied: false`, same health facts) evaluates to identical outcomes. The
    outcome mask of 32 occurrences is pinned.
- **Charm state (CHARM-3):** read through the narrow `CharmStateRead` trait, bound to one
  `CharacterId` and queried by race. Test doubles only. No CHARM-3, durability or migration file is
  touched.
- **Fail closed:**
  - The whole evaluation returns `CharmEvaluationError` on state for another character, unavailable
    state, more than two assignments on a race, a duplicate charm or category, an unknown charm, a
    stage outside 1..=3, or inconsistent event facts.
  - An effect whose runtime system is missing is evaluated and returned as
    `CharmResult::FailedClosed { reason, evaluated }` and never as `Applied`.

## Fail-closed effects

Only the 9 single-target attack procs apply: Wound, Enflame, Poison, Freeze, Zap, Curse, Divine
Wrath, Overpower and Overflux. They apply through the existing owner damage commit
(`commit_damage_for_attacker`). The other 16 fail closed:

| Charm | `CharmMissingSystem` | Why |
|---|---|---|
| Carnage | `AreaTargetResolver` | No resolver selects the monsters around a kill; the ability commit takes one target |
| Cripple, Numb | `ParalysisCondition` | The runtime actor carries no conditions (spell conditions S8 have no owner) |
| Adrenaline Burst | `HasteCondition` | Same |
| Cleanse | `ConditionCleanse` | Same |
| Fatal Hold | `CreatureFlee` | Creature AI has no fleeing |
| Dodge, Parry | `IncomingCreatureDamage` | Creatures only surface an attack intent; no creature-to-character damage path |
| Void Inversion | `ManaDrain` | No creature mana drain |
| Bless | `DeathLossCharmInput` | `domain::death` takes no killer race or Charm input |
| Scavenge | `Skinning` | No skinning or dusting |
| Gut | `CreatureProductLoot` | The loot plan has no creature-product classification |
| Low Blow, Savage Blow | `CriticalHit` | No critical hits |
| Vampiric Embrace, Void's Call | `Leech` | No life or mana leech |

## Integration and open points

- No live player attack path exists yet. `ability::commit::commit_exact_owner_damage` has no
  gameplay caller, and spells deal no damage. So the hook is the typed `committed_hit` seam on the
  owner's `OwnerDamageResult`.
- The Ability commit bridge is left unchanged, for two reasons:
  - Committing a proc as a second damage sub-occurrence changes its replay identity (D141), which
    needs its own contract.
  - The bridge is also compiled against a test-only Foundation shim.
- Creature element resistances do not exist at runtime. Proc amounts are before resistances, as the
  catalogue's cap rule states.
- A zero-damage creature hit is not an `IncomingCreatureHit` event (`InvalidEventFacts`).
- The assignments bound is one major plus one minor per race at the same time (coordinator
  catalogue update, answering CHARM-0 §4.2).
- Carnage stays physical with resistances, as on the wiki (owner answer 13a, 2026-09-30). The engine
  follows the catalogue flag (`ignores_resistances: false`); the owner verifies it in the live game.
- A proc is committed as its own owner damage commit, after the attack's damage and only when that
  attack reduced the creature's health (owner answer 12b, 2026-09-30; Canary `game.cpp:8762` and
  Crystal `game.cpp:8338` do the same).
- An auto-attack hit on a creature other than its main target (area ammunition) is
  `CharacterAutoAttackOffTarget` and triggers no charm; spells and runes still trigger on every
  creature they hit, and Low Blow still covers the whole area (owner answer 16a, 2026-09-30:
  TibiaWiki `Updates/15.25.3a4a52` and `Cyclopedia`; Crystal applies the rule to spells as well, Canary
  not at all).

## Validation (local)

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server --quiet`: pass.
- The 21 `charm_effect` tests run under `combat::` and the Foundation-included
  `exact_actor_test_combat::` copy.
- RED: 8 source mutations each fail at least one test: the level cap, the resource cap, the
  character binding, the trigger purpose, the lethal no-target rule, the event facts, the category
  rule and fail-closed gating.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- Review: review packet returned to the lead. No owner-funded review was triggered by this worker.
- Merge commit/result: squash merge of the CHARM-4 PR (resolve with `git log --grep`).

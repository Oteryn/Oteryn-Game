# OTV2-20260929-d3-3-top-damage

```yaml
task_id: OTV2-20260929-d3-3-top-damage
title: D3-3 - deterministic top-damage CharacterId per creature generation
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1198
allocation_comment: "D132, OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md §4.3"
base_branch: main
branch: claude/d3-3-top-damage
base_sha: 4a82815
head_sha: null
owner: "Oteryn: D3 corpse/loot worker" (Claude Code)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/damage_contributors_tests.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs   # shared: new attacker param at call sites
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs    # shared: new attacker param at call sites
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260929-d3-3-top-damage.md
public_contracts: []
depends_on: []
blocks: [D3-2 (settle_creature_death_rewards top-damage composition)]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Bounded, ephemeral, per-creature-generation damage-contributor accumulation
(`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` = 16, D132) and its deterministic
top-damage tie-break, at the owner-authoritative `runtime_actor_carrier` seam:

- `DamageContributors` lives only inside its `Slot::CreatureOccupied`: fresh and empty on
  every admission, dropped with the slot on removal or respawn (D52 - nothing duplicated,
  nothing silently completed by a later generation).
- `DamageContributors::record` attributes one already-applied hit; past 16 distinct
  `CharacterId`s a new attacker is silently not tracked (fail-open for combat, fail-closed
  only for attribution) while every already-tracked contributor keeps accumulating.
- `top_damage_character` is a pure function of recorded state: highest total wins; ties go
  to whichever total was *first reached* (lowest ordinal); a same-ordinal tie resolves to the
  lexicographically lowest `CharacterId`.
- New `CurrentOwnerExactActorCommit::commit_damage_for_attacker` and
  `CurrentOwnerCombatDeath::top_damage_character` are the seam D3-2 composes against; both are
  `#[allow(dead_code)]` with no production caller yet.

**Fix during resume:** `damage_contributors` is `Box<DamageContributors>` (not inline), matching
the existing `target_identity: Arc<[u8]>` footprint-preserving convention; the `size_of::<Slot>()`
regression guard moved from 192 to 200 bytes (one pointer, not the raw `Vec` + `u64`'s 32 bytes).

**Finding:** `CombatDeathFixture` (this test module's only black-box harness) retains exactly
one committed `OwnerCommitRecord` per creature generation - a second, differently-keyed commit
against the same live actor is `OccurrenceConflict`, not a second applied hit. Multi-hit combat
is unscoped future work the fixture cannot exercise, so the boundary/tie-break/overflow cases
are proven directly against `DamageContributors` (white-box); the fixture-backed tests prove
only that the one hit it does support is attributed, and never re-attributed on replay.

## Excluded scope

Wiring `top_damage_character` into `settle_creature_death_rewards` (D3-2), any live gameplay
caller of `commit_damage_for_attacker`, and multi-hit-per-generation fixture support.

## Validation

- `cargo fmt`
- `cargo clippy --quiet --all-targets -- -D warnings`
- `cargo test --quiet --lib`
- `python tools/agents/validate_governance.py`
- `git diff --check`

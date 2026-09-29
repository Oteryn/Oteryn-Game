# OTV2-20260929-d4-1-multi-hit

```yaml
task_id: OTV2-20260929-d4-1-multi-hit
title: D4-1 - bounded multi-hit damage receipts per creature generation
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: "D140-D144, OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md section 5"
base_branch: main
branch: claude/d4-1-multi-hit
base_sha: 4ebb1bf
head_sha: null
owner: "Oteryn: impl combat" (Claude Code)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs                                   # re-exports only
  - apps/game-server/src/ability/commit.rs                                   # commit_exact_owner_damage seam
  - apps/game-server/tests/ability_engine.rs                                 # compile-only owner shim for the new seam
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs
  - apps/game-server/src/foundation/damage_contributors_tests.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260929-d4-1-multi-hit.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`Slot::CreatureOccupied.committed` becomes a bounded `Box<DamageReceipts>` of up to
`COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` = 16 receipts (new registry row, max and max+1
tests):

- Attributed replay identity is derived by the carrier from `(CharacterId, GameSessionId,
  CommandId sequence, sub_ordinal)` (`AttackerCommand`); `sub_ordinal` is bounded by the mirrored,
  already-registered `ABILITY01-EFFECT-PLAN-ENTRIES` = 2 (`SubOrdinalOutOfRange`). The caller's
  opaque `occurrence` bytes are not read for an attributed commit.
- `ability::commit::commit_exact_owner_damage` takes the attacker `CharacterId` and a `CommandRef`
  and passes the effect's own index as `sub_ordinal`.
- Each receipt carries immutable `origin` and the owner damage-application `ordinal` (D142). The
  ordinal is monotonic per generation, feeds `DamageContributors::record`, and replaces its
  private counter.
- Per-attacker `high_water` `(character_lease_generation, GameSessionId, sequence, sub_ordinal)`
  in `DamageContributor`. D141 is enforced by the per-character `character_lease_generation`
  (`CharacterLease::generation()`, the value `CurrentCharacterGameplayFence` carries), which
  `AttackerCommand` carries and which proves at the mutation boundary which session is current:
  a lower generation, or an equal one with a differing session, is `SupersededAttackerSession`
  (a delayed old-session command is refused even after its receipt was evicted); a higher one
  replaces the mark; equal generation and session at or below the mark is `StaleAttackerSequence`.
  A zero generation is `InvalidCommitBinding`; an unsequenced occurrence containing NUL is too.
- Full list: the oldest evictable receipt (sequenced, superseded session or covered by the current
  mark) is evicted; unsequenced receipts never are; `DamageReceiptCapacityExceeded` only when none
  is evictable. `OccurrenceConflict` is removed (`CreatureNotActionable` replaces it).
- The lethal receipt is the unique retained record with `health_after == 0` (D144);
  `CreatureDeathOccurrenceRef`, D2b and D3 are unchanged. `size_of::<Slot>()` shrinks 200 -> 168.

`CombatDeathFixture::strike_by` keeps its signature (a test-only fixed session and a monotonic
command per occurrence text) so the D3-2 worker's call sites are unaffected.

## Excluded scope

`combat/death_reward.rs`, `durability/**`, migrations, `gameplay_transport/**` (no production caller
supplies a `CommandRef` yet; composing it is later work, decision section 7), the sibling
`AbilityEngine` fixture identity model.

## Validation

- `cargo fmt`, `cargo clippy --quiet --all-targets -- -D warnings`, `cargo test --quiet --lib`
- `python tools/agents/validate_governance.py`, `git diff --check`

# OTV2-20260928-combat-d2a-loot-plan

```yaml
task_id: OTV2-20260928-combat-d2a-loot-plan
title: Combat D2a, pure deterministic loot plan for one creature death
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/combat-d2a-loot-plan
issue: 162
pr: 1161
allocation: "#162 comment 5876068445 (Combat D2a, VSL-COMBAT-01 child of D2)"
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: pending (frozen at push)
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl combat (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat.rs                                   # add `mod loot_plan;` + re-exports only
  - apps/game-server/src/combat/loot_plan.rs                         # new
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                     # COMBAT01-LOOT-* rows only
  - docs/agents/tasks/active/OTV2-20260928-combat-d2a-loot-plan.md
public_contracts:
  - VSL-COMBAT-01
  - DUR-03
depends_on:
  - OTV2-20260928-combat-d1-death-mint-wireup
blocks:
  - "Combat D2b (production wiring: real death key, content loading, durability::item_mint calls, XP)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`apps/game-server/src/combat/loot_plan.rs::plan_creature_loot` computes a bounded,
deterministic loot plan for one creature death: an `IndependentBernoulliPpm` chance draw
plus a quantity draw per loot-table entry, both derived from a pure death-key seed, so a
replayed death always produces the same plan. No durability, foundation wiring or XP;
planning-only. Not production wiring; makes no playable-Combat claim.

## Architecture and source of truth

- PROVEN: content schema (`content/loot/loot-00500-00977.json` key
  `oteryn:loot.creature.rat`, `algorithm: "IndependentBernoulliPpm"`) and its validator
  (`apps/game-server/src/content/reference_playable.rs:1946-1980`,
  `validate_loot_definition`).
- PROVEN: VSL-COMBAT-01 resource rows decision
  (`docs/architecture/reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md`)
  D77 (≤16 loot plan entries per death, one MINT per entry, a stack is one instance),
  §4.1 rows `COMBAT01-LOOT-PLAN-ENTRIES`/`-ITEMS`/`-BYTES`/`-RNG-DRAWS`, and §4.1.1's
  30,720 B derivation.
- PROVEN: DUR-03 death-identity decision
  (`docs/architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md`)
  §4.1 death key = `(WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId,
  ActorLocalGeneration)`; §4.2 loot MINT cause = `(death key, LootTableDefinitionRef,
  LootEntryOrPurposeKey, DeterministicDrawOrdinal)`. `DUR03-RL-08` is retry/reconciliation
  work units, **not** a draw count (kept distinct from `COMBAT01-LOOT-RNG-DRAWS`).
- READ ONLY: `CreatureDeathOccurrenceKey`/`CreatureDeathOccurrenceRef::death_key()`
  (`foundation/runtime_actor_carrier.rs`); `ItemMintCause::from_creature_death`
  (`durability/item_mint.rs`).

## Spec gap: no `crate::content` or `crate::durability` dependency

Reusing `content::reference_playable::{ReferenceLootDefinition, ReferenceLootEntry,
ReferenceLootSelectionAlgorithm, TypedDefinitionRef}` directly, as the evidence pointed
at, is **not possible** without breaking required validation: `combat.rs` is compiled a
second time, unchanged, as a standalone Foundation test crate via `foundation/mod.rs`'s
existing (unowned, FORBIDDEN-to-touch) `#[path = "../combat.rs"] mod
exact_actor_test_combat;`. Six `apps/game-server/tests/*.rs` Postgres/composition
harnesses (`character_authority_postgres.rs`, `character_progression_postgres.rs`,
`durability_postgres.rs`, `item_mint_postgres.rs`, `native_admission_source_postgres.rs`,
`runtime_scope_assignment_postgres.rs`, `wp5_s3b_composition.rs`) also re-declare
`foundation` (and so pull in `combat.rs`) at their own crate root, and none of the six
declare a `content` module. `cargo clippy --all-targets -D warnings` compiles every one
of those; a `use crate::content::...` in `combat.rs`/`combat/loot_plan.rs` fails all of
them with `E0432 unresolved import` (confirmed empirically before this fix).
`durability::item_mint::TypedDefinitionRef` already set the precedent for this exact
constraint (its own local `String`-based struct, not `content::TypedDefinitionRef`).

Resolution: `loot_plan.rs` defines its own minimal local shapes ([`LootDefinitionRef`],
[`LootTableEntry`], [`LootTableDefinition`], [`LootSelectionAlgorithm`]), field- and
variant-compatible with both `content::reference_playable`'s types (D2b's conversion from
real content is a straight field copy) and `durability::item_mint::TypedDefinitionRef`
(`family`/`production_key`/`revision_ref`, all `String`; D2b's conversion into
`ItemMintCause`/`ItemMintRequest` is too). No content-parsing/validation logic is
duplicated, only the field shape. Documented at the top of `loot_plan.rs`.

Also unspecified: `ItemMintCause`'s `purpose_key` has no defined source in the content
schema (`ReferenceLootEntry` has no purpose-key field; DUR-03's own name for the cause's
third element, `LootEntryOrPurposeKey`, is ambiguous between "entry identifier" and
"purpose key"). `plan_creature_loot` uses each entry's own item production key as its
`purpose_key`: deterministic, content-derived, and combined with `draw_ordinal` (the
entry's original table-order index) keeps the MINT cause tuple unique even if two entries
shared an item key. D2b should confirm or override this when it has real content and
`ItemMintCause` in scope together.

## Acceptance criteria

- [x] Pure/deterministic: same `(death_key, loot_table_ref, loot_table)` → same
      `LootPlan`; replay idempotent (`same_death_key_produces_the_same_plan`).
- [x] Different death keys can differ (`different_death_keys_can_produce_a_different_plan`).
- [x] `COMBAT01-LOOT-PLAN-ENTRIES`: 16 accepted, 17 rejected before planning
      (`sixteen_table_entries_accepted_seventeen_rejected`).
- [x] `COMBAT01-LOOT-RNG-DRAWS`: 16 entries need exactly the registered 32-draw budget
      (`sixteen_entries_use_exactly_the_registered_rng_draw_budget`).
- [x] `COMBAT01-LOOT-PLAN-BYTES`: exact limit accepted, one byte over rejected
      (`plan_bytes_budget_accepts_exact_limit_and_rejects_one_byte_over`); full 16-entry
      max-width pipeline lands exactly on 30,720 B
      (`sixteen_maximum_width_entries_fit_exactly_at_the_registered_byte_ceiling`).
- [x] Zero-ppm never drops; 1,000,000-ppm always drops
      (`zero_chance_entry_never_drops_and_max_chance_entry_always_drops`).
- [x] Quantity stays within `[min_count, max_count]`
      (`quantity_draw_stays_within_the_entry_range`).
- [x] The rat table (`oteryn:loot.creature.rat`) produces a valid plan
      (`rat_loot_table_produces_a_valid_plan`).
- [x] Fails closed: unsupported algorithm, zero/inverted quantity, missing/out-of-scale
      `probability_ppm` (`unsupported_algorithm_is_rejected`,
      `zero_or_inverted_quantity_range_is_rejected`,
      `missing_or_out_of_scale_probability_is_rejected`).
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

`foundation/**`, `lib.rs`, `durability/**`, `ai/**`, `ability/**`, `movement.rs`,
migrations, `content/` data. No production caller, real death key, content loading, or
`ItemMintCause`/`ItemMintRequest` construction; no XP. All of that is D2b.

## Implementation / findings

- `LootPlanDeathKey`: pure, test-constructible `(world_id, channel_id,
  scope_ownership_generation, actor_local_id, actor_local_generation)` seed. No
  dependency on `foundation::CreatureDeathOccurrenceKey` (no constructor reachable
  outside `foundation/runtime_actor_carrier.rs` anyway); D2b builds this seed from the
  real committed death via that type's existing `pub(crate)` accessors.
- RNG: SHA-256 counter mode (`sha2`, already an `oteryn-game-server` dependency via
  `durability/item_mint.rs`; no new crate), keyed by the death seed, the loot table ref,
  the entry's table-order index and a chance/quantity discriminant. `bernoulli_success`:
  `draw % 1,000,000 < probability_ppm` (0 never succeeds, 1,000,000 always succeeds).
  `quantity_in_range`: `min_count + draw % (max_count - min_count + 1)`.
- `draw_ordinal` on each `LootPlanEntry` is the entry's original table position (stable
  regardless of which chance draws succeed), matching D1's own example ("one fixture
  loot entry, draw ordinal 0").
- `COMBAT01-LOOT-PLAN-BYTES`: `charge_plan_bytes` accumulates a running footprint (header
  once, then once per accepted entry), rejecting before the running total or a single
  addition would exceed 30,720 B. Per-field costs mirror §4.1.1 at maximum content-key
  width (family at its 128 B technical-field maximum regardless of actual text;
  `production_key`/`revision_ref`/`purpose_key` at actual, ProductionKey/ProductionAtom-
  bounded lengths up to 512 B), so a real 16-entry, 512-B-key plan lands exactly on the
  registered ceiling.
- Fixed a module-resolution conflict: `combat.rs` is compiled a second time under a
  different module name via `foundation/mod.rs`'s existing `#[path]` inclusion (spec-gap
  note above); a bare `mod loot_plan;` resolved to a different, nonexistent path there.
  Fixed with an explicit `#[path = "combat/loot_plan.rs"] mod loot_plan;`, which resolves
  identically (relative to the file's own directory) under both inclusion sites.
- `#[allow(unused_imports, reason = "...")]` on the `combat.rs` re-export block: no
  production caller yet (D2b wires one), matching the existing `#[allow(dead_code,
  reason = "...")]` on `mod combat;` in `lib.rs`.

## Validation

- `cargo fmt --all --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean, including the
  six standalone Postgres/composition test crates and the standalone Foundation test
  module.
- `cargo test -p oteryn-game-server --lib combat`: 37 passed, 0 failed (17 in
  `combat::loot_plan::tests`, mirrored 15 in
  `foundation::exact_actor_test_combat::loot_plan::tests` via the dual compilation
  above, plus 15 pre-existing D1 `channel_owner_combat_death_tests`, untouched, still
  green).
- `python3 tools/agents/validate_governance.py`: passed (22 policy docs, 9 lanes); also
  owns `RESOURCE_LIMITS_REGISTRY.json` validation (`LIMITS_REGISTRY_PATH`). No separate
  registry-only validator is wired into `.github/workflows/*.yml`.
- `python3 tools/repository/validate_repository_policy.py`: passed (23 files, 50
  workflows).
- `git diff --check`: clean.

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent
- material findings: none open (spec gap above left for D2b/control-plane disposition,
  not a defect in this slice)
- verdict: READY_FOR_FREEZE

## Independent review

- required: pending control-plane triage
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- Per allocation, this worker opens no PR, no GitHub comment, no review request; it
  pushes `claude/combat-d2a-loot-plan` and stops. Control plane owns freeze, review
  dispatch and integration.

## Context checkpoint

```yaml
last_progress: implementation complete; all listed validation green; pushed, not yet reviewed
status: waiting
branch: claude/combat-d2a-loot-plan
head_sha: pending (see push result)
pr: 1161
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
ci_run_ids: []
runner_assignment_state: not_started
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: control plane freezes the pushed head and dispatches required review; D2b (production wiring) depends on this
```

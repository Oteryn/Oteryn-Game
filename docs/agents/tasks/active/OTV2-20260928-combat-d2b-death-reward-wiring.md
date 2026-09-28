# OTV2-20260928-combat-d2b-death-reward-wiring

```yaml
task_id: OTV2-20260928-combat-d2b-death-reward-wiring
title: Combat D2b, creature death composed into loot MINT + R7 P03 XP
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/combat-d2b-death-reward
issue: 162
allocation: "#162 (Combat D2b, VSL-COMBAT-01 child of D2 row D: generic death/loot/reward orchestration)"
base_sha: 9f98b06
head_sha: pending (frozen at push)
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl combat (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs        # reward_occurrence + memoized cache only
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs  # extended
  - apps/game-server/src/combat.rs                                  # re-exports only
  - apps/game-server/src/combat/death_reward.rs                     # new
  - apps/game-server/tests/combat_death_reward_postgres.rs          # new
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs  # new
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                    # COMBAT01 rows only
  - docs/agents/tasks/active/OTV2-20260928-combat-d2b-death-reward-wiring.md
public_contracts:
  - VSL-COMBAT-01
  - DUR-03
depends_on:
  - OTV2-20260928-combat-d1-death-mint-wireup
  - OTV2-20260928-combat-d2a-loot-plan
  - OTV2-20260928-char-progression-init
blocks:
  - "Combat D/E admission composition (real protocol caller, content loading)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`apps/game-server/src/combat/death_reward.rs::settle_creature_death_rewards` composes one
already-committed, already-projected creature death into its two independent reward
descendants: a bounded loot MINT batch through the existing `durability::item_mint` API, and
one XP award through the existing `durability::character_progression` API. Each descendant
returns its own `Result`; neither ever rolls back or is blocked by the other. No production
caller is added: D/E admission (protocol, content loading, real activation) is a later stage.

## Architecture and source of truth

- PROVEN: VSL-COMBAT-01 §24.1/§24.3 row D (generic death/loot/reward orchestration, exclusive
  lease over `foundation/mod.rs`/`runtime_actor_carrier.rs`) and ~:455 (loot and XP are
  independent descendants, never one distributed transaction).
- PROVEN: DUR-03 death-identity decision §4.2 (loot MINT cause; XP occurrence: "the Combat
  owner mints one server UUIDv7 `ExperienceRewardOccurrence` per (death, eligible character)
  ... and keeps it in its death record. Every retry in that generation reuses it") and §4.3
  (D52: uncommitted descendants are dropped, never duplicated, when the generation ends).
- PROVEN: resource rows decision §4.1 rows 7/9/10 (`COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE`=64,
  `COMBAT01-XP-DESCENDANTS-PER-DEATH`=1, `COMBAT01-REWARD-PRINCIPALS`=1).
- PROVEN: `OTV2-20260928-combat-d2a-loot-plan`'s binding items: `purpose_key` is each entry's
  own item production key (confirmed, not overridden); content -> D2a local-shape mapping
  stays at a future caller, since `combat.rs` still cannot import `crate::content` (dual
  compilation via `foundation/mod.rs`'s `exact_actor_test_combat`, and six standalone
  Postgres/composition test crates that do not declare `content`; all six *do* already declare
  `durability`/`domain`, so this slice can depend on those safely, confirmed empirically).
- PROVEN: `OTV2-20260928-char-progression-init`'s call-site decision: initialize immediately
  before the first XP award, same policy binding to both.

## Composition point and design

- `combat/death_reward.rs::settle_creature_death_rewards` takes the death identity/corpse as
  owned `Copy` values (`CreatureDeathOccurrenceKey`, `MovementLocalPosition`) extracted by the
  caller right after `project_fixed_one_creature_death`, plus the still-borrowed
  `CurrentOwnerCombatDeath` handle, a `DurabilitySession` (root/authority/node bundle, added to
  keep argument counts under clippy's default), and a `CreatureDeathRewardInput<N>`.
- Loot: `plan_creature_loot` (D2a), then one `freeze_item_mint`/`commit_item_mint` pair per
  plan entry in table order, stopping at the first MINT failure (a fence failure mid-batch
  rejects every remaining entry too).
- XP: `foundation::CurrentOwnerCombatDeath::reward_occurrence` (new) mints a v7-shaped
  `ExperienceRewardOccurrence` once per (death, character) and memoizes it on the physical
  Channel owner (`ChannelActorCarrier::death_reward_occurrence`, next to `corpse_projection`,
  `None` on every fresh per-generation carrier bootstrap, so D52 holds structurally: a scope
  move never reuses or duplicates it). The bytes are a SHA-256 digest of the exact actor
  reference, the reward principal and the wall-clock millisecond, framed with the RFC 9562
  version/variant nibbles; no new crate dependency.
- `reward_occurrence` also reports whether it minted fresh or reused an existing value.
  `settle_experience` attempts the award first; `commit_character_experience`'s own
  occurrence lookup resolves a retry before touching the fence. Only
  `MissingProgressionState` runs the D88 initializer, followed by one more award attempt. A
  failed earlier initialization is therefore retried instead of stranding the award, and an
  already advanced revision never rejects a legitimate replay (control-plane repair).
- `GroundPlacement` has no accepted production placement rule yet (movement.rs/ability
  forbidden). `combat/death_reward.rs::ground_placement` builds the smallest defensible one,
  promoted from the D1 fixture's own pattern: the corpse's local position, and a `corpse_ref`
  that is a canonical big-endian encoding of the exact death key.

## Resource rows registered

Only what this slice's code actually checks (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`):

- `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE` = 64: `check_inflight_loot_mint_capacity`, a pure
  guard over a caller-tracked in-flight count, checked once per death before any entry of its
  plan is frozen.
- `COMBAT01-XP-DESCENDANTS-PER-DEATH` = 1: enforced structurally (one
  `commit_character_experience` call per invocation, keyed by the memoized occurrence).
- `COMBAT01-REWARD-PRINCIPALS` = 1: `check_reward_principal_count`, checked first, before
  planning or any durability call.

`COMBAT01-DEATH-WORKFLOWS-PER-SCOPE` (row 3) is deliberately **not** registered: its own
decision text already records the failure mode as "structurally unreachable" (bounded by the
still-unregistered D57 creature-count envelope), and this composition keeps no scope-wide
active-workflow counter to check. Registering an unenforced ceiling would be a fake guard.

## Acceptance criteria

- [x] A death mints exactly the plan's items and awards the definition XP once (PG:
      `one_creature_death_mints_the_plan_and_awards_xp_once`, rat fixture XP=5).
- [x] Replay/retry is idempotent: no duplicate MINT or XP (PG:
      `replay_is_idempotent_with_no_duplicate_mint_or_xp`).
- [x] A generation change leaves a stale death rejected with no write (D52) (PG:
      `generation_change_leaves_a_stale_death_rejected_with_no_write`).
- [x] A stale fence rejects with no write, independently of the other descendant (PG:
      `a_stale_xp_fence_rejects_xp_without_blocking_loot`: XP fence goes stale, loot still
      commits).
- [x] A loot failure does not roll back XP, and vice versa (PG:
      `a_stale_xp_fence_rejects_xp_without_blocking_loot` and
      `an_unsupported_loot_table_rejects_loot_without_blocking_xp`).
- [x] Max/max+1 for every registered row: `reward_principal_count_accepts_one_and_rejects_zero_or_two`,
      `inflight_loot_mint_capacity_accepts_exact_ceiling_and_rejects_one_over` (unit, no PG);
      XP-descendants' max+1 is the replay test above (a second attempt commits no second row).
- [x] `reward_occurrence`: mint-once/replay, conflicting-principal, distinct deaths, requires a
      committed death, stale generation (unit, `channel_owner_combat_death_tests.rs`).
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

Real protocol/admission caller, content loading (`crate::content` stays unreachable from
`combat.rs`), Ground/room placement policy, a scope-wide active-death-workflow tracker, leveling
curve/policy authoring (the composition takes an already-resolved `FiniteProgressionPolicy` and
XP amount as input). `durability/**` itself is untouched (consumed only).

## Validation

- `cargo fmt --all --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean.
- `cargo test -p oteryn-game-server --lib combat`: 52 passed (loot_plan + death_reward guard
  unit tests, both compiled once directly and once via the dual `exact_actor_test_combat`
  inclusion).
- `cargo test -p oteryn-game-server --lib foundation::runtime_actor_carrier::
  channel_owner_combat_death_tests`: 18 passed (10 pre-existing D1, 8 new `reward_occurrence`).
- `cargo test -p oteryn-game-server --test combat_death_reward_postgres` on local PostgreSQL
  17.11 (relaxed `assert!(version.starts_with("17"))`, matching `item_mint_postgres_cases.rs`'s
  own precedent rather than the 170006 pin): 5 passed.
- Regression, same PostgreSQL, version pin locally relaxed then restored (byte-identical,
  confirmed via `git diff`, not committed): `item_mint_postgres` (12 passed) and
  `character_progression_postgres` (6 passed).
- `python3 tools/agents/validate_governance.py`: passed (22 policy docs, 9 lanes; also owns
  `RESOURCE_LIMITS_REGISTRY.json` validation).
- `python3 tools/repository/validate_repository_policy.py`: passed (23 files, 50 workflows).
- `git diff --check`: clean.

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent
- material findings: none open; the init/commit fence-replay tension documented above is a
  narrow, deliberate scope decision (fail-closed on a specific crash window), not a defect
- verdict: READY_FOR_FREEZE

## Independent review

- required: pending control-plane triage
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- Per allocation, this worker opens no PR, no GitHub comment, no review request; it pushes
  `claude/combat-d2b-death-reward` and stops. Control plane owns freeze, review dispatch and
  integration.

## Context checkpoint

```yaml
last_progress: implementation complete; all listed validation green; pushed, not yet reviewed
status: waiting
branch: claude/combat-d2b-death-reward
head_sha: pending (see push result)
pr: null
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
next_action: control plane freezes the pushed head and dispatches required review; a later admission stage supplies a real protocol caller and content-to-loot-plan mapping
```

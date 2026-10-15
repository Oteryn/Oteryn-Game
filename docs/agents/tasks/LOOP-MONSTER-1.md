# LOOP-MONSTER-1

```yaml
task_id: LOOP-MONSTER-1
title: "LOOP-MONSTER-1: world respawn scheduler; monster ranged, spell, summon AI (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-monster-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-MONSTER-1 (rows 94, 96); wave 2"
class: "hard (child graph)"
migration: maybe
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/
  - apps/game-server/src/ai_think/
  - apps/game-server/src/monster_owner_cycle.rs
  - apps/game-server/tests/loop_monster_*.rs (new)
slot_only_paths:
  - mod.rs (scheduler composition)
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "MAGIC-CAST-1: MERGED"
  - "LOOP-RANGED-1: BLOCKING: not merged (active/held: RANGED-CONTENT-1 #2096 then PR2)"
  - "CREATURE_AI0 ACCEPTED: BLOCKING: CANDIDATE"
  - "MAP-OVERLAY-1, MAP-CUTOVER-1b: MERGED #1800/#1809, #1916"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

World-owned respawn scheduler independent of sessions; monster ranged, spell and summon AI.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAGIC-CAST-1 | MERGED |
| LOOP-RANGED-1 | BLOCKING: not merged (active/held: RANGED-CONTENT-1 #2096 then PR2) |
| CREATURE_AI0 ACCEPTED | BLOCKING: CANDIDATE |
| MAP-OVERLAY-1, MAP-CUTOVER-1b | MERGED #1800/#1809, #1916 |

**BLOCKING:** LOOP-RANGED-1; CREATURE_AI0 ACCEPTED. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/ai/`
- `apps/game-server/src/ai_think/`
- `apps/game-server/src/monster_owner_cycle.rs`
- apps/game-server/tests/loop_monster_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- mod.rs (scheduler composition)
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/monster_ai_cycle.rs`
- `ai/spawn.rs` respawn

## Inseparable bundles

none beyond registry rows.

## Acceptance criteria

- [ ] Respawn continues with no session; deterministic schedule.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

CREATURE_AI0 §; BUNDLE-MONSTER-AI-1 record. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-MONSTER-1 (rows 94, 96).

## Builder instructions

- Work on branch `skeleton/loop-monster-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/loop-monster-1
blocker: LOOP-RANGED-1; CREATURE_AI0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```

## Bounded local owner-schedule preparation — 2026-10-15

Existing draft #2200, branch `skeleton/loop-monster-1`, unchanged authoring base
`96b43e5671bf1341d472b8f504f6fa591b9b6d79`. The active allocation permits only
`ai/behaviour_profile.rs`, `ai_think/channel_owner.rs`, `monster_owner_cycle.rs`,
the optional `tests/loop_monster_summons.rs` and this record. Historical broader
owned-path/status/dependency text above remains historical, not a write grant.

Accepted basis independently read from retained protected main
`e2da9805d296a27b06b32335bf67394509d4910b`: ACCEPT-BATCH §1 accepts CREATURE-AI-0
without waiving its child gates; CREATURE-AI-0 §4.4/§4.6, §7 and §8.1 supply
schedule, deterministic draw, cap and placement rules. CREATURE-AI-1 §1.2/§2
supplies the existing projected-profile and owner-think seam. The clone's stale
CREATURE-AI-0 CANDIDATE heading is not new authority; acceptance is resolved by
that retained protected-main decision. No current remote status is reconstructed.

### Source-confirmed residue and prepared owned code

- Native `profile_schedule.rs` already owns the bounded attack/defence/summon
  clock, deterministic draw, occurrence replay/conflict rules and typed proposals.
  Native `monster_summon.rs` already owns source catalog qualification, current
  role/count observations, path/terrain policy and physical admission/replay.
  Foundation owns actor links, identity/generation checks and current fences.
  None was copied or replaced by a second scheduler, RNG or realization owner.
- Legacy `CreatureBehaviourProfile::project` deliberately removes summons and
  `CreatureAiState::think` calls `schedule.prepare` without facts. Enabling
  summons in that live projection would cause `SummonClockDependency`; its
  `ThinkReport` has no summon proposals. The unowned root `ai_think.rs` cannot
  be changed by this allocation, as the lead explicitly confirmed.
- `project_with_summons` is a separate opt-in projection for the owner adapter,
  using native limits and retaining authored summon entries. Legacy admission
  and projection remain unchanged. The existing schedule validates entry
  semantics; a projected Creature reference does not qualify active content.
- `CreatureAiState::prepare_owner_profile` uses that actor's existing schedule
  state after independently checking the supplied current scope fence/stamp and
  native live actors. It returns the native complete `SchedulePlan`. This is a
  schedule-only path, not another think or authority source.
- `prepare_monster_profile` obtains path facts through existing current map
  policy and role/count facts through `NativeSummonCatalog::facts`, then calls
  that single schedule. Unknown path data refuses before advancing the clock.
  `resolve_monster_profile_summons` dispatches at most eight proposals in authored
  order to the existing native owner. Every execution retains native content,
  cap, current role/target/path, placement, fence and replay checks.
- New tests are inline in the registered owned module. A new integration wrapper
  would need unowned Cargo registration/private-module duplication, so the
  optional test path is unused. No shared module registration was changed.

### Exact cross-domain integration requirement (REPORT_ONLY)

CP/architect must allocate the minimal shared `ai_think.rs` integration: extend
the single `CreatureAiState::think` schedule call to receive independently
resolved optional `MonsterSummonFacts` for its **selected** target, call native
`prepare_with_summons` exactly once, return `summon_proposals` in `ThinkReport`
(including empty lists in refusal/over-budget paths), preserve replay/conflict
binding and evaluation budgets including summon entries, and thread the facts
through `CreatureAiTable::think`/owner dispatch. Do not call legacy think and the
explicit schedule adapter for the same occurrence or install a second schedule.

The active content owner must supply the compatible projected profile, catalog,
revisions and target; immutable prepared source specs are not current authority.
The map/path owner must provide actual current path and tile admission facts.
After native creation, register the child with existing AI/combat/movement/profile
admission hooks and roll back using existing native registration-failure cleanup
if a required registration fails. Connect retirement/removal/death hooks to those
same owners. No source/protection/visibility eligibility may be inferred from a
request or fixture. Unknown facts remain refused. Resolve/replay a retained plan
through the native owner; fresh changed facts for the same schedule occurrence
are a native `OccurrenceConflict`, not permission to redraw or create twice.

The transport's current targeting-only/melee projection, production owner-timer
composition, shared report/facts seam, child actor registration and resource
registry/content work remain CP/architect integration obligations. This bounded
adapter is not runtime activation or full LOOP-MONSTER-1 acceptance. No commits,
pushes, ready/merge actions, wire/migration/registry/regeneration edits or new
current-authority grants are authorized or performed by this worker.

### Actual validation setup and limits

Whole `cargo check --locked --offline -p oteryn-game-server --lib` passed on the
changed registered source (only the existing vendored Tokio missing-doc warning).
The first focused library-test compile encountered 34 existing `include_str!`
errors from 22 sparse-excluded fixture paths; no behavioral test ran in that
attempt. The lead restored all 22 byte-identical to authoring HEAD, about 54 MB,
with `unit-fixtures-restored.json` in the external evidence directory. No owning
source changed, so the successful metadata check remains applicable. Initial
compiler failure output and its eight-source binding are retained separately;
this setup failure is not a behavioral RED claim.

Durable raw logs/scripts/manifests are under
`/workspace/oteryn-prep/loop-monster-1-evidence/`. The retry binds the eight actual
source files plus the 22 restored fixtures before and after. Rust commands use
the single shared `/tmp/oteryn-prep-rust.lock`, shared target, jobs 2 and
`CARGO_INCREMENTAL=0`; all Cargo commands explicitly select the library. No
server/migrate binary, PostgreSQL/E2E, regeneration or paid review is run.

The first post-restoration test build was killed with exit 137 before tests ran
(`resource-killed-run.log`, `resource-killed-tests.log`); this is a resource/setup
failure, not a product result. At the lead's direction the next focused run uses
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` as well as the same
lock/jobs/incremental controls. Production source is unchanged from the successful
whole-library metadata check.

### Final local result

- **11/11 native registered library tests passed** with debug=0: seven new
  `loop_monster_summons` tests and four existing timer-lane regression tests;
  3,175 unrelated tests filtered. Coverage includes opt-in/legacy projection,
  8/9 entries and 16/17 bounds, native due/retry/conflicting-path clocks,
  physical creation and replay, current live count caps, unknown/blocked path,
  foreign current scope, replaced ownership generation at both new boundaries,
  stale actor and no-free-tile refusal. Fixture map/source data is not production
  map/content authority or an E2E claim.
- Whole-library metadata check PASS; library Clippy with `-D warnings` PASS;
  workspace format check PASS. The preexisting vendored Tokio missing-doc
  warning is retained verbatim in the Cargo logs; no lint was weakened.
  Final before/after bindings of **30 inputs match**, including the restored
  exact-HEAD fixture hashes. No production source changed after metadata PASS.
- Governance validation and **124/124** agent unit tests passed. Whole owned
  diff adversarially reviewed, whitespace/scope checks PASS, four allocated
  paths changed. No independent exact-head review/architect acceptance claimed.
- SHA-256: `ai/behaviour_profile.rs`
  `a897073d9f0b2127635b457bb04bf653d028ad8773b121ac3e799bf81c8410a2`;
  `ai_think/channel_owner.rs`
  `98f29a4e111281ae7ca3bd4ec97909931b47d35f29e83b7983a10395eeb8a266`;
  `monster_owner_cycle.rs`
  `fc517cad97409eea6fc76742a8efdabfe1521ea072ac224b69e7e86abdc6a38c`.

The detached owned subset is prepared. Live activation, the shared single-pass
think/report seam, evaluation accounting, actor registration/rollback and the
remaining CP composition hooks above are still required; full task acceptance
is not asserted.

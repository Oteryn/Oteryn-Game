# LOOP-MONSTER-1

```yaml
task_id: LOOP-MONSTER-1
title: "LOOP-MONSTER-1: world respawn scheduler; monster ranged, spell, summon AI"
mode: IMPLEMENT
status: OPEN
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-monster-1
issue: 1622
pr: 2200
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-MONSTER-1 (rows 94, 96); wave 2"
class: "hard (child graph)"
migration: maybe
registry: maybe
owner: family F
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/
  - apps/game-server/src/ai_think/
  - apps/game-server/src/monster_owner_cycle.rs
  - apps/game-server/tests/loop_monster_*.rs (new)
  - apps/game-server/src/ai_think.rs
  - apps/game-server/src/ai_think/channel_owner.rs
  - apps/game-server/src/ai_think/think_tests.rs
  - apps/game-server/src/content/native_gameplay.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - apps/game-server/src/monster_summon.rs
  - docs/agents/tasks/LOOP-MONSTER-1.md
slot_only_paths:
  - mod.rs (scheduler composition)
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "MAGIC-CAST-1: MERGED"
  - "CREATURE-MOVE-1 #2184: source prerequisite published at 2cc5c753a50eaa2d84c83f486ab77d9cb7cd4eba; included in current combined source"
  - "#2187: node/serve native startup installation and wakeup follow merge (D1239)"
  - "MAP-OVERLAY-1, MAP-CUTOVER-1b: MERGED #1800/#1809, #1916"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Current status is the native composition checkpoint at the end of this record.
The initial skeleton base and dated preparation/recovery evidence are retained as
history; their former allocation and BLOCKING labels are not current publishing
gates. Root owns the coordinated #2184/#2200 authoring on the existing exclusive
branches. Ordinary Contents AUTHORING is authorized; final integration and MQ
remain with CP. Startup installation and wakeup in `node/serve` follow #2187.

## Goal

World-owned respawn scheduler independent of sessions; monster ranged, spell and summon AI.

## Historical initial dependencies (main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAGIC-CAST-1 | MERGED |
| LOOP-RANGED-1 | BLOCKING: not merged (active/held: RANGED-CONTENT-1 #2096 then PR2) |
| CREATURE_AI0 ACCEPTED | BLOCKING: CANDIDATE |
| MAP-OVERLAY-1, MAP-CUTOVER-1b | MERGED #1800/#1809, #1916 |

These are the initial skeleton observations. The retained protected-main
acceptance and the later D1239/D1239a composition supersede that initial
CREATURE_AI0 CANDIDATE label. Current prerequisite, source and startup status
are recorded below.

## Historical initial path allocation

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

## Historical skeleton acceptance checklist

- [ ] Respawn continues with no session; deterministic schedule.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo +1.95.0 fmt --all -- --check`
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
last_progress: World respawn wake loop installed in serve_gameplay (slot-only hunk, D1423) on the session path's clock origin under the Channel owner fence; tests added
status: OPEN
branch: skeleton/loop-monster-1
blocker: CP final integration and MQ
next_action: CP final integration, one Codex round on the final head, MQ
```

## Historical bounded owner-schedule preparation — 2026-10-15

Existing draft #2200, branch `skeleton/loop-monster-1`, unchanged authoring base
`96b43e5671bf1341d472b8f504f6fa591b9b6d79`. At that checkpoint the allocation permitted only
`ai/behaviour_profile.rs`, `ai_think/channel_owner.rs`, `monster_owner_cycle.rs`,
the optional `tests/loop_monster_summons.rs` and this record. Historical broader
owned-path/status/dependency text describes that earlier checkpoint.

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

### Historical cross-domain integration residue (then REPORT_ONLY)

The remaining integration at that checkpoint was the shared `ai_think.rs` seam: extend
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

At that checkpoint the transport projection, production owner-timer composition,
shared report/facts seam and child registration remained integration work. The
bounded adapter did not establish runtime activation or full LOOP-MONSTER-1
acceptance. That worker performed no commits, pushes, ready/merge actions,
wire/migration/registry/regeneration edits or new current-authority grants.

### Historical validation setup and limits

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

### Historical local result

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

The detached subset was prepared at that checkpoint. Live activation, the shared
think/report seam, accounting and actor registration/rollback were still required
at that time; those results did not assert full task acceptance.


## Historical recovery checkpoint — OPEN / NOT_FULLY_QUALIFIED (2026-10-17)

This section records the reconstruction before the current composition below.
Its NOT_RUN labels and path list belong to that earlier source.

AUTHORING resumed after loss of the previous CLOUD workspace. The owner requested
publication of all recoverable work; CP2131 comment6230877600 explicitly directs
reconstruction from the published heads and an NFQ checkpoint. This is a new
candidate, not recovery of commit identity `e7cbd428086bed0d21c0f993d78d97ae6f3f1e81`.

One target-selection/schedule pass reports ability and summon proposals, and the native loader seals summon entries against its exact parent/child closure.

Physical summon activation, persistent owner and the #2184 chase caller were
uncomposed at that recovery checkpoint. The current source below completes those
seams; startup installation/wakeup remains the separate #2187 dependency.

Sources were reconstructed from retained exact change literals and complete
review diffs. The final catalog refinement is reconstructed with one exact-ref
index and duplicate refusal, at most eight entries, max/count in 1..=16,
positive intervals and chance at most 1,000,000. The existing catalog case also
covers those refusals; these new-candidate cases remain NOT_RUN.
No historical shell/build commands were replayed and no foreign files were adopted.

New-candidate native tests, strict all-target Clippy and full-package qualification
are NOT_RUN. Old PASS/cache results and lost job87870 are not claimed as proof
for this reconstruction. Focused source review, Rust formatting, governance and
diff-check results are reported with the publication receipt. Existing full
RED/NOT_RUN and composition gates remain open; draft only, no readiness claim.

The reconstructed delta is limited to these already allocated owned paths:

- `apps/game-server/src/ai_think.rs`
- `apps/game-server/src/ai_think/channel_owner.rs`
- `apps/game-server/src/ai_think/think_tests.rs`
- `apps/game-server/src/content/native_gameplay.rs`
- `apps/game-server/src/gameplay_transport/actor_spell.rs`
- `apps/game-server/src/gameplay_transport/monster_ai_cycle.rs`
- `apps/game-server/src/monster_summon.rs`
- `docs/agents/tasks/LOOP-MONSTER-1.md`

That reconstruction followed the family-F D1123/D1239a assignment. The current
combined source and resolved root-owned authoring are recorded below.


## Current native composition — OPEN / source qualification PASS (2026-10-17)

The remaining owner, Map and Movement source work is implemented. A single native
think selects its target and runs the existing profile schedule once. The same
Channel state retains `NativeSummonOwner`; physical admission, installed exact
native policy, registration in the existing AI table, registration-failure
rollback and final replay memo finish synchronously under the actual current
owner. Replay neither readmits nor registers a child again.

Outer activation requalifies the retained admitted CreatureProfiles summon closure
through the existing catalog producer before changing state. Catalog entries,
source specs and nested sealed physical admission specs all receive the same
independently checked outer artifact digest. Missing closure remains Unknown;
source, owner and admission checks are unchanged.

The live caller holds the existing Item-owner transaction through current source
map, Item and LocalObject reads and the synchronous consumer. Recovery,
assignment, scope and physical transaction checks remain active. Missing or
unqualified Item movement flags produce Unknown/refusal; an old empty-tile read
cannot authorize a later transaction. No synthetic source receipt, reconstructed
fence or digest-derived catalog supplies current authority.

The #2184 integration uses the same shared chase search budget and bounded search
kernel. One retained route/request binds exact actor and target snapshots, full
pins, revisions and goal; carried requests run before fresh thinks. Existing
owner deadlines drive steps at actual native source speed and destination ground
speed. Only Moved records cadence and consumes the retained first step. Each
physical step checks its next cell freshly; using a retained path as summon
reachability evidence additionally rereads its remaining suffix.

Ordinary creatures obtain home from the actual authored spawn census, separately
from admission position. The same caller handles target distance, fleeing,
authored wandering/dance, paced walkback and leash relocation with current Map
and occupancy checks. Intrinsic summons do not inherit a wild-creature home.
Cancellation during an awaited read retains the exact pending input, route and
moved cadence; it does not advance the profile schedule. Dead future timer
entries are removed. There is no second think pass, scheduler or Movement owner.

### Prerequisite and exact source boundary

[D1239](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226466175)
assigns the persistent owner/admission composition and puts `node/serve` startup
installation and wakeup after #2187. Those startup lines are untouched.
[D1239a](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226605863)
keeps the catalog source-only: without an admitted parent/child closure the
result is `None = Unknown`, and summons are refused. The document schema is not
extended.

Root owns both #2184 and #2200 authoring. The #2184 source prerequisite was
published first at `2cc5c753a50eaa2d84c83f486ab77d9cb7cd4eba`; it is explicitly
included in this combined candidate. This records source provenance, not a claim
that #2184 has merged. Shared paths do not create a new permission gate. The
existing exclusive branch and ordinary Contents AUTHORING route are resolved;
CP retains final integration and MQ disposition.

| Source/evidence | Bound identity |
|---|---|
| Canonical #2200 reconstruction head | `7f651bf75798cd0e6e4bc9915d85af725679e6f7` |
| Included #2184 source prerequisite | `2cc5c753a50eaa2d84c83f486ab77d9cb7cd4eba` |
| First PC-formatted source; native compile 1 failed | Combined diff `6f563b4cd7b84d0849fdeb467e9a7264ff8059a04ea9f707eddd44b04e8c1466` |
| PC-formatted repair 1; native compile 2 failed | Combined diff `aeda2a0fb00c8a209f7f3bfc6bf11c2f862af4eb615571dd568476657cc42066` |
| PC-formatted repair 2; native compile 3 failed | Combined diff `e2c87f1d1512dd5758b6ffe05e8dc1589d0e2400e465a7de345c3aa8cc578e97` |
| PC-formatted owned-callback repair 3; native compile 4 failed | Combined diff `503de1dcd15009cfd4fade6d48af7db7b4c3c1e741ae088401c1d66223ae6f30`; actual PC format manifest `6ca98c631956e89175234df3ba78dd92a6415d3e73fc35aa7511c055b34ebd99` |
| Explicit Send-future repair 4, prepared source | Authoring combined diff `a855d9e25a35af7aa707c012c3ef3460333412c6fd899c8f3be1846d86f7a16a`; 17-file manifest `786046f06fa4506aa86d9de8f2884f8e326669e7c973d70cc52617c653b99aef` |
| PC-formatted repair 4; native attempt 5 failed on strict Clippy | Combined diff `49b8e9ebde2155218bc0fbb23d37abaeee87a7a8a16806cce709f0a9f6cfc469`; actual PC format manifest `45881c2cf1f3f7c96b84e029119859abb0cf5de2ebcaec617cc1129ece725712` |
| Repair 5; native attempt 6 failed in the library | Combined diff `21d18fc419793efb40dde10df379b768dbab3ad9c2d894f8b9733e4789b105c3`; input manifest `5bbe899f6bf5021484849ab313359328cb2a39d807a6c6989b76dfaaf653fe0b`; repair patch `99c2c3e1703db7707e25378091b3fc633b2ce3d55233bfd8d92a8319fbb76b68`. |
| Repair 5, historical actual PC formatting | PID 17632: PASS in 33.20 seconds, empty delta; manifest `ff2be1289418faf474da50b647aad803dca88a3e0a2cf67de90c81b520ba0773`. |
| Repair 6; native attempt 7 passed, first shared-PG attempt failed | Combined diff `14cf07185974a7bfc2f5810e4c252c5821ac230ae982360ac9d9edbf2de887f2`; actual PC format manifest `58e6ef386c7df4a19af978bacb8959fbaa81e3c04e1a7afa9f9092697316af69`. |
| Repair 7; native attempt 8 passed, second shared-PG attempt failed | Combined diff `aa8aa587260d9586abc3f98ecda1379fcd8f3ed91a1e5e00825711ec5d4481c6`; actual PC format manifest `b3cc1d4294c7d146a561f03f57df2cbc94843c7b1ba6614606a92ddbf744660e`. |
| Repair 8; final formatted native source, 17 paths | Combined diff `02f11834cb007907c1ac0358e3e8cdf1e5066383de9aa2d19968ee2fbe21bedb`; actual PC format manifest `3981565de3c6306bac8261b603d570cecf88a22f5dfec53190cfddcb7ae05362`. |
| Final author native qualification | Evidence `20261017T133521Z`; input manifest `9e638c2dde39fe0fb6354a32e607ca898e2c749e8b4abeecd501a3d7898da74b`; PASS result `e1127b7e408f215c48909538bb9f95868a1a6154d50a6b323b31e71e4a675114`. |
| Final author shared-PG qualification | Evidence `shared-pg-20261017T134500Z`; input manifest `fd9601493513f9de30c951312da8a686fab02dcffef568df1ce9192cc6288159`; both-case PASS result `2724a8e77781f2473995fc424333c932c1172b567d971596a5eb5bed799601bb`. |
| Canonical publication, final-head CI and task/governance receipt | Maintained in the [#2200 PR description](https://github.com/Oteryn/Oteryn-Game/pull/2200), bound to the complete published 18-path change. This task record does not embed its own future commit identity. |

The complete scope below is **18 paths** relative to canonical #2200: **17 tested,
frozen native/prerequisite postimages** and this task-record update. The external
18-path manifest binds this record alongside those unchanged tested files.
Publication and CI belong to the canonical final-head receipt in the PR description;
the source results here do not assign an older head's PASS to a later commit.

```text
apps/game-server/src/ai_think.rs
apps/game-server/src/ai_think/channel_owner.rs
apps/game-server/src/ai_think/think_tests.rs
apps/game-server/src/combat/attack/chase.rs
apps/game-server/src/combat/attack/mod.rs
apps/game-server/src/content/native_gameplay.rs
apps/game-server/src/foundation/owner_timer.rs
apps/game-server/src/foundation/runtime_actor_carrier.rs
apps/game-server/src/gameplay_transport/actor_spell.rs
apps/game-server/src/gameplay_transport/attack.rs
apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
apps/game-server/src/monster_combat_lane.rs
apps/game-server/src/monster_summon.rs
apps/game-server/src/movement.rs
apps/game-server/src/movement/path_search.rs
apps/game-server/src/movement/speed.rs
docs/agents/tasks/CREATURE-MOVE-1.md
docs/agents/tasks/LOOP-MONSTER-1.md
```

### Native source-policy fixture and regressions

The existing `actual_aggregate_native_regular_movement_uses_current_ground_pacing_and_fence`
case proves speed refusal before physical policy installation. Its isolated
fixture variant initializes both creature and target through the existing pinned
admission helpers, then decodes the same fixture's exact Creature, Behavior and
Presentation through the native policy decoder and installs the resulting policy.
Its speed-100, ground, cadence and stale-fence assertions remain. The common legacy
setup retains its prior behavior; the production exact key/revision and pinned
context guards are unchanged.

The new owner-timer regression covers cancelled input and dead future entries;
the Movement boundary regression rejects a nonpinned context before consulting
Map; the shared-search regression covers authored distance and the longer bounded
home route. The existing cancellation/search cases now use the package's explicit
current-thread runtime, without a Tokio macro-feature or Cargo change.

`activated_outer_source_summon_rejects_inner_catalog_and_registers_once` uses
actual outer qualification and activation, extracts the stale inner catalog from
that same compiled artifact pair, and requires `InvalidSource` before any child,
registration callback or AI entry. The active outer catalog must pass the existing
physical admission and full policy-plus-AI registration helper; replay preserves
one child, one AI entry and its initial due time. This also checks the nested
physical admission spec. The existing square around `(100,100,7)` and component
Map are explicit local test inputs, not current Thalom geometry or SQL evidence.
This non-PG regression is included in the passing full library suite below.

Two inline cases require **separate real PostgreSQL execution**:

| Exact library test | Harness tag / database prefix | Required evidence |
|---|---|---|
| `gameplay_transport::monster_ai_cycle::tests::current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope` | `summon_current_map` / `bp_summon_current_map_` | Actual Item/Map reads; shared-budget deferral for a summon-enabled parent; SQL cancellation retains the timer; retry makes a paced move without another think; one-pass child registration/replay; wrong physical TX refusal; real Item MINT causes Unknown flags/refusal. |
| `gameplay_transport::monster_ai_cycle::tests::current_owner_spawn_home_walkback_and_leash_use_actual_source_cell` | `native_source_home` / `bp_native_source_home_` | Test-authored spawn Table through the production adapter and installed policy; actual source home differs from admission anchor; two paced retained-route steps home; free-home leash relocation and occupied-home refusal. |

Both use `crate::bestiary_postgres_harness::configured_admin` and
`OTERYN_TEST_POSTGRES_ADMIN_URL`. Missing configuration returns early and is
**SKIP/NOT_RUN**, not a real-PG PASS. The harness tags/prefixes above describe its
existing per-test databases; the harness creates and drops them itself.

Current PostgreSQL qualification follows the owner's
[D1329](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233047791):
use the shared `oteryn-shared-pg` PostgreSQL **17.6** service on **:55432**. Load the WSL environment from
`~/.oteryn/shared-pg.env` and use the adjacent `in-pg-netns.sh` helper after
`cargo fetch --locked`. This replaces the earlier own-container qualification
recipe. Do not create another server or use/stop the unrelated :5432 service.
Manual temporary databases use `oteryn_<task>_<pr>_<rand>` and are dropped after
use; that manual naming rule does not rename the harness's per-test databases.
Do not alter another task's databases.

The final receipts bind this exact source and shared endpoint. Each case reports
one executed test with no configuration skip. The passing frozen harness requires
PostgreSQL `server_version_num` **170006**, applies migrations and awaits cleanup
of its own exact database. This is source-bound execution evidence; the final PG
run does not claim an independent CREATE/DROP log receipt.

Both cases use unchanged `canary-thalom-world.json`, SHA-256
`44f860d20712a32e6bc22aa269d3e220d39d164b7738d3c150a406579849b5f5`.
The real cells `(2,12,-7)`, `(3,12,-7)` and `(4,12,-7)` supply source ground speeds
**150 / 150 / 110**, asserted through the held current Map reader. The summon
Creature/Behavior closure remains explicit test-authored source admitted through
the existing qualifier and outer activation. In the home case the test-authored
bundle Table supplies spawn data and the separately qualified Thalom source world
supplies current Map authority. Source home `(4,12,-7)` differs from AI admission
at `(2,12,-7)`; the test performs both paced cardinal steps. Outside-leash staging
is not a traversal qualification.

Both test bodies create their existing Harness before a local `catch_unwind`.
After the body future and its transaction borrows end, they await that Harness's
cleanup, then resume an original panic if one occurred. Ordinary errors still
propagate. This local cleanup covers a completed Harness creation followed by body
failure; it does not claim recovery from process termination or a failure during
Harness creation before the Harness is returned.

### Qualification and remaining work

The first five native attempts stopped at strict Clippy before library execution.
Attempt 6 exposed a fixture pin mismatch. Attempts 7, 8 and 9 passed their bound
native suites; the two earlier real PostgreSQL failures remained separate gates
until the final third PG attempt passed. These source-specific results do not
qualify a successor by inheritance:

| Native attempt / bound combined source | Actual result and successor |
|---|---|
| 1 / `6f563b4...` | FAILED after 89 seconds: nonexistent `combat::attack` references, unavailable `tokio::test` attributes and inferred slice/owned-route type errors. Repair 1 corrected the existing namespace, used the package's explicit current-thread test runtime and annotated the owned route vector; PC formatting produced `aeda2a0...`. |
| 2 / `aeda2a0...` | FAILED after 71.43 seconds: query argument E0282 and four Send-generality diagnostics at the boxed async consumer. Repair 2 added concrete callback argument types and `async move` captures of scoped references to the same owner/Map/budget, collecting retained steps before await; PC formatting left `e2c87f1...` unchanged. |
| 3 / `e2c87f1...` | FAILED after 76.83 seconds: E0282 was absent; the leading diagnostic identified `AsyncFnMut` on the borrowed `&mut` query closure, followed by a higher-ranked coercion error and five Send-generality diagnostics. Repair 3 moves that callback by value into the single selected distance/home search; PC formatting left `503de1d...` unchanged. |
| 4 / `503de1d...` | FAILED after 82.10 seconds: the borrowed-`AsyncFnMut` diagnostic was absent, but eight Send-generality notes and one higher-ranked coercion error remained. Repair 4 explicitly requires a Send future at both new production boundaries. |
| 5 / `49b8e9eb...` | FAILED after 95.93 seconds: Send/HRTB compiler diagnostics were absent; strict Clippy reported unused helpers, two collapsible conditionals and one test `expect`. Repair 5 removes only superseded unused methods, marks existing test-only helpers, collapses the two conditions and uses the existing test-module `unwrap` allowance. All assertions and production authority checks remain. |
| 6 / `21d18fc...` | Format and strict Clippy PASS; library FAILED with 3202 passed, 1 failed, 29 ignored, 0 filtered (190.92 seconds; complete run 466.76 seconds). `actual_aggregate_native_regular_movement_uses_current_ground_pacing_and_fence` observed speed `None` rather than `Some(100)`: its old synthetic position did not satisfy the required current pin. Repair 6 uses the existing pinned admission for both fixture actors and preserves all assertions. |
| 7 / `14cf071...` | PASS: format, strict Clippy and library 3203 passed, 0 failed, 29 ignored, 0 filtered; library 168.14 seconds, complete run 425.79 seconds. The subsequent first real PG attempt failed separately. |
| 8 / `aa8aa587...` | PASS: format, strict Clippy and library 3203 passed, 0 failed, 29 ignored, 0 filtered; library 169.95 seconds, complete run 433.66 seconds. The subsequent second real PG attempt failed separately. |
| 9 / `02f11834...` | PASS: format, strict Clippy and library 3204 passed, 0 failed, 29 ignored, 0 filtered; library 187.77 seconds, complete run 454.45 seconds. The subsequent third shared-PG attempt passed both exact cases. |

Repair 4 uses a crate-private RPITIT reader for repeated path queries, with no
per-tile Box allocation, and the existing scoped-context/HRTB pattern for the
single current-goal read. That one read allocates a boxed Send future when a think
continuation invokes it. The actual owner/transaction, shared frontier, query
order, outer Send requirement and prepare/await/finish sequence remain unchanged.
The path regressions retain their route, read-order, refusal and bounds checks;
no test assertion or current-authority guard is weakened.

Repair 3's independent source review passed, and its actual PC format run passed
in 31.65 seconds with an empty delta. Repair 4's independent source and mechanical
format reviews passed; its actual PC format run passed in 32.55 seconds. The fifth
native run bound the resulting `49b8e9eb...` source to manifest
`456c4ca27ce35feb8d55abebf8431d4d38eb01a12607dfd1f65a479de32672b5`.
Its complete strict-Clippy log has SHA-256
`2a8a78bede63ab490dd2ae1f5c4700be497020347fb2ebf1586fca3649ef4fe6`;
the owned process group was reaped after failure. That actual compiler result
confirmed resolution of the preceding Send/coercion diagnostics.

Repair 5's complete two-file source delta and all 17 before/postimage bindings
passed independent review; the other 15 files match the actual formatted anchor.
The repair leaves both explicit Send-future APIs, actual transaction/owner guards,
retained Movement behavior and all regression assertions unchanged. Its actual
PC formatting run passed in 33.20 seconds with an empty delta, preserving
`21d18fc...` exactly.

The first shared-PG attempt, evidence `shared-pg-20261017T123231Z`, failed on
`14cf071...`: the movement assertion expected `(1,0,0)` but observed `(0,0,0)`.
Source analysis confirmed that AcceptedEntryR1 carried no qualified ground speed,
so the existing Movement owner correctly refused `MissingSpeed`. The second PG
case was NOT_RUN. Repair 7 switched both test fixtures to the unchanged qualified
Thalom source and its actual ground speeds, retaining the movement, cancellation,
home, leash, summon and replay assertions; no fallback speed was added.

That first body panic preceded the local cleanup repair. Root subsequently dropped
only `bp_summon_current_map_1792240357489024717`, owned by `oteryn_test_admin`,
OID `550663`, using ordinary DROP without FORCE at
`2026-10-17T12:48:20.475795Z`. The checked precondition was zero active sessions;
the shared PostgreSQL 170006 catalog then reported `remaining_name_or_oid = 0`.
The exact cleanup receipt has SHA-256
`342e2b7b05122e315a1fcb6dd79bd0e1107003a885f81d20805e7a70e52005ac`.
This is the one historical own-database cleanup, not a claim about another
session's databases or an additional final-PG DDL receipt.

The second shared-PG attempt, evidence `shared-pg-20261017T131106Z`, progressed
through movement but failed on `aa8aa587...` at the child-count assertion:
expected 1, observed 0. Its literal log, SHA-256
`240f7ac6bb216074fc8aaaa4c73a9bd3d301372267ab70684fe6960d3a995743`,
does not print `InvalidSource`. Root and peer traced that refusal to the outer
binder leaving summon catalogs and their nested admission specs pinned to the
inner artifact. Repair 8 reuses the existing catalog qualifier before mutation;
the new non-PG regression and both final PG cases provide execution evidence for
that correction. PG2 was NOT_RUN in the failed attempt. Its local body-panic
cleanup ran before the original panic resumed, with no cleanup-error marker;
that observation is source-bound and has no independent catalog/DDL receipt.

#### Final author-source receipts

All results in this table bind the same 17 tested postimages, combined diff
`02f11834cb007907c1ac0358e3e8cdf1e5066383de9aa2d19968ee2fbe21bedb`.
The actual formatter's repair-8 delta only wrapped the new non-PG test statements;
root and peer verified the mechanical change and all 17 postimage bindings.

| Gate | Actual result |
|---|---|
| Rust 1.95 `cargo +1.95.0 fmt --all -- --check` | **PASS**, return code 0 in native PID 16088 / evidence `20261017T133521Z`. |
| `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` | **PASS**, return code 0 in that same native run. |
| `cargo +1.95.0 test --locked -p oteryn-game-server --lib --quiet` | **PASS**, 3204 passed / 0 failed / 29 ignored / 0 filtered, 187.77 seconds; native run completed in 454.45 seconds. |
| Shared PostgreSQL 17.6 / D1329: `summon_current_map` | **PASS**, 1 passed / 0 failed / 0 ignored / 3232 filtered, 8.59 seconds; exact configured test, no skip. |
| Shared PostgreSQL 17.6 / D1329: `native_source_home` | **PASS**, 1 passed / 0 failed / 0 ignored / 3232 filtered, 25.06 seconds; exact configured test, no skip. |
| Published-head CI, final task/governance and MQ disposition | Recorded separately in the canonical final-head receipt in the [#2200 PR description](https://github.com/Oteryn/Oteryn-Game/pull/2200). These author-source results do not assert a future commit's CI outcome. |

The complete native result is `ninth-native-result.json`, SHA-256
`e1127b7e408f215c48909538bb9f95868a1a6154d50a6b323b31e71e4a675114`;
its full library log is `ninth-native-library.log`, SHA-256
`fb1a61f31e94ec25cedd3cdafe6be6049d62167c1e08588eaabedf1698a3b1bb`.
The native input manifest is
`9e638c2dde39fe0fb6354a32e607ca898e2c749e8b4abeecd501a3d7898da74b`.
All three Cargo commands returned 0 and the owned process group was reaped.

The final shared-PG run used PID 33480 / evidence
`shared-pg-20261017T134500Z`, completed in 39.88 seconds, and reaped its owned
process group. `cargo fetch --locked` returned 0; its log is empty (SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`).
Both test logs contain `OTERYN_SHARED_PG_CONTEXT_VERIFIED` and their exact passing
case names. The input manifest is
`fd9601493513f9de30c951312da8a686fab02dcffef568df1ce9192cc6288159`.
The retained result and complete logs are:

- `third-shared-pg-result.json`: `2724a8e77781f2473995fc424333c932c1172b567d971596a5eb5bed799601bb`.
- `third-shared-pg-summon-map.log`: `b2b60992255334379e7ab410d3d33fb99e80c605c8e3034153178fc81a368f35`.
- `third-shared-pg-source-home.log`: `e58d665668d27f42cda903316d843a7b2e49119daf521a1e43944f75e10bf0f9`.

The result records version 170006 and awaited exact-database cleanup through the
passing frozen harness for both cases; `independent_ddl_receipt` is false. A
library run without configured PostgreSQL is not used as a substitute for these
two actual executions.

No RL17 runtime/p99 result is claimed by these source or fixture checks. Native
source qualification and both required Map/Item PostgreSQL cases have passed.
The task remains OPEN for startup installation and wakeup in `node/serve` after
#2187, and CP dependency integration/MQ. At the final dependency read reported by
root (2026-10-17 13:42 UTC), #2187 remained draft, open and unmerged. Completed
persistent owner, current Map, admission/rollback and live chase/walkback source
are not listed again as unimplemented. The canonical published-head status and
its CI/governance evidence are maintained in the PR description.

## Bounded World respawn scheduler checkpoint — 2026-10-17

Main merged into the branch. `monster_owner_cycle.rs` now holds the World wake
loop for respawn, which needs no session:

- `MonsterRespawnJob::drive(runtime, current, wake, shutdown)` runs one
  `run` pass per wake under the runtime mutex. It then sleeps until the next
  50 ms owner window (`next_owner_window`). It returns `RespawnDriveEnd`:
  `Shutdown` (checked before every pass and raced against the sleep),
  `StaleOwner` (a retired or replaced fence) or `ClockExhausted` (no further
  window). A clock regression or carrier refusal keeps the last window and
  does not end the loop.
- `RespawnWake: OwnerClock` adds `sleep_until`. `MonotonicRespawnWake` is the
  production clock: `Instant` elapsed since a given origin, tokio
  `sleep_until`.
- Inline tests (the job is `pub(crate)`, so there is no `tests/loop_monster_*.rs`)
  drive a virtual wake with no session. They cover a kill being respawned at
  its due window with a deterministic wake sequence, a retired owner, shutdown
  before any pass, an exhausted clock, and the monotonic wake.

Installed (CP D1423 slot-only hunk, `serve_gameplay`): `run_world_respawns` starts
`MonsterRespawnJob::new_owned(..).drive_owned(..)` on `MonotonicRespawnWake::from_origin(
ComposedFreshAdmission.clock_origin)` (`respawn_wake()`), the same origin the session-driven
`drain_monster_melee` pass now reads for `drive_owner_spawns`. The Channel runtime's own
current-owner fence (`ChannelRuntimeV1::owner_fence`) is checked under the runtime lock on
every pass; a retired or replaced owner ends the job. The job is polled with the other seam
cadences and ends on shutdown. Tests in `monster_ai_cycle.rs`: the installed job admits a
respawn on an origin already past its deadline, the session pass on that origin then advances
nothing, shutdown ends it, and a retired owner ends it.

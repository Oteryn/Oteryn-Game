# OTV2-20260928-cw1-timed-revert-runtime

```yaml
task_id: OTV2-20260928-cw1-timed-revert-runtime
title: §7 timed revert runtime - lifecycle records, scope driver, apply_scope_operation (task B of 2)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-timed-revert-runtime
base_sha: f11058ecdb82e3256ae3d8857fe650ca2bd1943d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T17:53:51Z
updated_at: 2026-09-28T17:53:51Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_object_revert.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/content/reference_playable.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/agents/tasks/active/OTV2-20260928-cw1-timed-revert-runtime.md
  - docs/agents/tasks/archive/OTV2-20260928-cw1-object-state-attributes-impl.md
public_contracts: []
depends_on:
  - "task A, PR #1133, merged as f11058ec (§9 types, bind, lowering)"
  - "allocation comment on issue #162: ALLOCATION: OTV2-20260928-cw1-timed-revert-runtime"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements the owner-accepted §7 direction of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` as the
smallest runtime slice, and closes out task A:

- `world_runtime.rs`: `apply_scope_operation`, a scope-origin, non-session operation. It checks the
  scope identity and generation, reuses `prepare`, and then `PreparedMutation::commit`. A staging
  callback runs only after `Publish` and before commit; if the callback fails, nothing commits.
  `bind` now keeps the unique inverse it validates (`revert_inverse`) and runs the destination check.
- `content/reference_playable.rs`: a `destination` must resolve to exactly one placement in the
  bound world and coordinate frame (link time with the source world, bind time with the scope world).
- `world_object_revert.rs` (new): a local `InteractionChildOccurrenceRef` (the forward occurrence
  supplied by the caller, plus the `LoweredActionId`). It also adds a lifecycle-record store
  (`PENDING` / `IN_FLIGHT` / `TERMINAL`, with §7's field list) and `ScopeRevertDriver`, which owns
  one ordinal issuer (`ScopeRuntimeFence` in production), one `MonotonicClock` and the store.
  `apply_forward` creates a record only once `prepare` returned `Publish`, with capacity checked only
  at that point. `wake` fires a bounded batch of due records in (deadline, scheduling ordinal,
  sequence) order through `apply_scope_operation` with the bound inverse. `present` follows §7's
  fixed order, so a duplicate presentation returns its `TERMINAL` outcome. Nothing is persisted.
- `RESOURCE_LIMITS_REGISTRY.json`: `WOBJ-RL-04` (4,096 records per scope generation) and
  `WOBJ-RL-05` (32 due records per wake).
- §7 open decisions 1, 2, 5 and 6: the lane's resolutions are recorded at each entry.
- Task A's record is archived with its terminal fields and carry-overs.

## Lane resolutions of §7 open decisions

1. `TERMINAL` records are kept for the whole scope generation, with no eviction or compaction. One
   registered per-scope cap covers records in any state. A full cap fails the forward operation
   `CAPACITY_EXCEEDED` before commit.
2. Option (a): the `PENDING`→`IN_FLIGHT`→`TERMINAL` step is one synchronous owner turn with no await.
   An error inside it leaves the record `IN_FLIGHT` and makes the driver scope-terminal.
5. The dedup horizon is the scope generation.
6. `accept_input` runs first inside the step, and the record moves to `IN_FLIGHT` only on `Ok`. On
   `Exhausted` the record stays `PENDING` and the driver is scope-terminal.

No §7 text contradicted these defaults.

## #1133 carry-overs

- (a) Destination resolution: done as above. Tests cover a duplicate marker placement
  (`DuplicateKey`), a cross-world placement and a cross-frame placement (`InvalidArtifact`).
- (b) USE reaching the bound inverse: the driver tolerates a user-driven revert. This follows §7's
  single path for an intervening change: the due record reaches `prepare` with its stored expected
  revision and ends `TERMINAL(STALE_STATE)`, so the object is never reverted twice. Excluding the
  inverse from USE selection would be a second mechanism that §7 does not specify. Test:
  `a_user_driven_revert_resolves_the_timer_terminal_without_a_double_revert`.
- (c) Two `revert_destination` occurrences on one forward stay rejected by `bind` as an ambiguous
  inverse. No change was needed; a new test confirms it.

## §7 test obligations

Covered in `world_object_revert::tests`:

- fires once and mints one ordinal;
- an `IN_FLIGHT` duplicate converges, using a test-only hook, because the synchronous step never
  leaves a record `IN_FLIGHT`;
- the three `PENDING` fences never reach `prepare`;
- a `TERMINAL` duplicate returns the first outcome after the target is replaced;
- distinct occurrences never collide;
- an intervening change gives `STALE_STATE`;
- the revert lands on the literal source state, or on the declared variant for the duke sample;
- two anchors fire independently;
- a replacement incarnation is rejected as `Incarnation`;
- a mutually timed pair fires once and stops;
- an occupied revert is refused once and never retried;
- capacity is atomic, and an unchanged forward registers no record and reserves nothing;
- equal deadlines fire in scheduling order;
- a bounded wake leaves the remainder for later;
- only the driver's own clock is used;
- the store is cleared on scope restart;
- ordinal exhaustion is scope-terminal;
- the e2e duke case: before the forward the teleporter has no destination; forward 1949→22761 goes
  to the reward destination and creates a `PENDING` record; 1,200,000 ms later the post-revert
  variant goes to the warzone exit and the record is `TERMINAL`; a duplicate presentation returns
  `TERMINAL`.

Deferred, with reasons:

- No re-execution after compaction: there is no compaction (decision 1).
- Other inputs get their turn between wakes: that is the arbitration of the live scope owner, which
  is out of scope. The driver itself returns after its bound.
- Lowering rejection and bind inverse-uniqueness: covered by task A.

## Excluded scope and open points

- Out of scope: wiring into a live `ChannelRuntimeV1`/`InstanceRuntime`, the encounter trigger
  (creature death) and any teleport consumer of `attributes()`. These need a separate owner
  decision. `ScopeRuntimeFence` cannot be built outside Foundation, so the tests use a stand-in
  issuer with the same contract.
- Docs gap: after a revert lands on the §9 post-revert variant, the forward transition (whose source
  is the natural state) cannot fire again from it. A timed teleporter therefore opens once per scope
  generation. §9 does not cover re-arming.
- Docs gap: USE selection can also reach the encounter forward transition from the natural state, so
  a player could open the teleporter by using it. Whether encounter-owned transitions are
  USE-selectable belongs to the live-wiring decision.

## Validation

- `cargo +1.94.0 fmt --all --check`: pass.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked -p oteryn-game-server --lib` (727 passed, 2 ignored) and
  `--test content_reference_playable`, `--test content_native_entry`,
  `--test content_world_project_repository`: pass.
- `git diff origin/main -- content/`: empty.

## Context checkpoint

last_progress: implementation and focused validation complete; PR pending
jira: pending (no mapped Story resolved in this worker session)

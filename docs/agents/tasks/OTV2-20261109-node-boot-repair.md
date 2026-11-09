# OTV2-20261109-node-boot-repair

```yaml
task_id: OTV2-20261109-node-boot-repair
title: Repair the failing real-World Node boot qualification
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/node-boot-repair-20261109
pr: null
base_sha: 88445eb587c6d8bea0bc1fcf3c3e4aaf2c6ce43c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-node-boot-repair-20261109
created_at: 2026-11-09T13:00:08Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/world/src/movement/monster_map.rs
  - crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_current_map.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/pass_cost.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/item_scope.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/summon_transaction.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/inbound.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/inbound_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/poll.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/setup.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/activity_transfer.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_transfer.rs
  - crates/gameplay/dispatch/mechanics/0010-loop_training.mechanic
  - crates/gameplay/dispatch/mechanics/0020-bed.mechanic
  - docs/contracts/logging/events/durability.json
  - tools/qualification/node_boot/run.sh
  - apps/game-server/migrations/2702_spell_parameter_result_row_lock.sql
  - apps/game-server/tests/support/authority_cases/spell_parameter_result_runtime_lock.rs
  - apps/game-server/src/qualification/client_phases/world.rs
  - docs/agents/tasks/OTV2-20261109-node-boot-repair.md
public_contracts:
  - docs/contracts/logging/events/durability.json
depends_on: []
blocks: []
```

## Outcome and scope

Repair the current Node boot failure without relaxing transaction budgets,
liveness, immutable result semantics, runtime fences or qualification assertions.
The owner requested this repair and maximum available parallel diagnosis. One
canonical branch has one writer; three independent workers prepare bounded patches
or review evidence outside the writable checkout.

The scheduler repair reuses concurrent PR #3066 at exact commit
`42d8b2a6953e69c37d42f2bd01637bf33fcafea4` through a normal local merge into this
candidate. Its source branch is untouched. The unpublished combat-only prototype
was superseded before publication. The combined candidate adds the independent
Map/SQL repairs and qualifies the complete tree together. Review also identified
two small logging compatibility repairs: the Node boot report must recognize
the new structured durability events, and `pass_dropped` must register level
`warn`, matching the emitter.

## Verified baseline

- PROVEN: admission `main` was `8f2e0f12bb6faa4480617a0abe2bc067cfc2c938` (#3057).
- PROVEN: current integration base is `88445eb587c6d8bea0bc1fcf3c3e4aaf2c6ce43c`
  (#3067/#3068). Its logging changes already match the imported #3066, except
  for this candidate's corrected warning level. Its heavy aggregate treats Node
  boot as informational; closure of this repair still requires the actual Node
  boot job and `NODE_BOOT_RESULT=NODE_BOOT_PASS`, independently of aggregate green.
- PROVEN: final #3057 head `72e8572bef391edcf16189590b467881c1fde83a`
  failed Node boot in run `42609864506`, job `129135376988`. Fixture seam passed;
  real-World seam failed after successful, idempotent World epoch initialization.
- PROVEN: the PostgreSQL log repeatedly reports transaction timeout, while the
  server reports 57 retired passes at `monster_profile_cycle.rs:29`.
- PROVEN: the same job reports denied access to `game_spell_parameter_results`;
  migration 0044 grants SELECT/INSERT while production replay uses FOR SHARE.
- DERIVED: per-chunk empty-tile reads amplify a geographically dispersed creature
  census. The existing dense 512-creature test does not cover that distribution.
- PROVEN: the failed job reports an unusable session during the E2EF4 return
  journey, then aborted profile rows before the final town assertion. The
  harness reuses Character 1; fresh admission does not reset its saved position.
- UNKNOWN: the failed job did not report the actual town-assertion position or
  the deferred profile error. The repair adds both diagnostics while retaining
  the original assertions and error propagation.

## Authority and execution

META policy 3.1.0 is bound to `Oteryn/Oteryn` commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. Game's explicit local integration
override uses squash merge and the full post-merge gate. Work remains AUTHORING
until the final bounded delta is published and frozen. No deployment, live-data,
workflow, protection, protocol, dependency or external-repository changes.

Execution uses the isolated workspace and existing GitHub CI. Rust 1.99.1 was
installed for local checks. Docker is unavailable and system package installation
fails on sandbox UID/group operations; PostgreSQL 18.6 and full Node boot must run
in the existing repository CI. No Remote Desktop fallback is used.

The normal Git publication attempt could not authenticate; live readback confirmed
the canonical remote task branch was absent. The root controller explicitly
selects the connector-compatible Git Data route in the bound publication policy
for a NEW candidate on `codex/node-boot-repair-20261109`. This is a fresh,
single-writer allocation owned only by this task's root; workers remain read-only
and the source PR branch is untouched. No atomic expected-head commit primitive
is exposed. The complete 20-path delta will form one commit with the live branch
predecessor as its sole parent, based on that predecessor's exact tree, followed
by one non-force ref update and immediate live head verification. Any drift or
ambiguous result stops publication for reconciliation. Local candidate
`07cc40a94a6bcb3aa41a26fbb5634d2887895825` and its history remain separate evidence;
all candidate-specific checks and independent review must bind to the NEW remote
head. This committed record is an authoring snapshot; the PR will bind the final
SHA and qualification evidence without moving the frozen head for metadata.

Migration number 2702 was selected after checking current main and all changed
filenames of the open PRs; 2102 is already held by #3056 and the highest observed
claim is 2701 in #3042. At 13:20:45 UTC the new #3066 had no migration and the
other eight heads were unchanged; 2702 remained free.

## Authority and failure qualification

Applicable boundary: exact-cell observations feed existing fenced creature
decisions. Each SQL batch retains the existing physical-transaction check and
256-address bound. Runtime scope, scope generation, pinned Map and content must
match independently; cached emptiness never survives the borrowed transaction.
The existing `validate_transaction` still runs before runtime publication.

Negative operators use real independent facts: a different physical transaction,
an aborted transaction, a new committed Item on a formerly empty tile, a runtime
role attempting an identity/payload update or delete, and a concurrent exclusive
row lock. Existing `loop_monster/item_scope.rs` and `summon_transaction.rs` now
exercise exact-cell prefetch before their unchanged refusal/invalidation checks.
The new parameter-result test independently selects the runtime role, checks
column privileges and immutability, replays a committed row twice, and proves
shared-lock contention. These database scenarios remain pending CI execution.

Time/generation facts, controller installation, recovery interpretation and wire
versions are unchanged. The family review covered current creature and summon
Map readers, the empty-cell SQL projection, parameter-result read/write/replay,
the analogous immutable temporal-row grant, and transport polling/setup/ACK
validation, both production callers of `poll::next`, and CHASE-1 ordering.
No sibling authority bypass was found. Scheduling tests inherited from #3066
use real `serve_admitted_actor`, six liveness probes, a command result and
controllable owner delays. They do not derive negative authority from the record
under test. No deadline, hard qualification assertion or negative test is weakened.

## Acceptance and validation

- [ ] Sparse creature census uses bounded exact-cell reads within the existing pass.
- [ ] Runtime parameter-result replay obtains its shared lock; result mutation and
  deletion remain denied and a concurrent exclusive row lock remains blocked.
- [ ] Transport fairness regressions prove ACK/command progress across periodic
  work with unchanged timer intervals and a bounded single input turn.
- [ ] Formatting, affected-package strict Clippy and tests pass.
- [ ] Governance validation and tests pass for this record.
- [ ] Exact-head PostgreSQL and Node boot CI pass, including the real-World pass.
- [ ] Independent review of the complete frozen candidate is resolved.

Reference parity: NOT_APPLICABLE; this is a regression repair, not a new mechanic.
Focused/component/E2E results remain pending. Unconfigured PostgreSQL tests that
return early do not count as executed database validation.

## Context checkpoint

Current stage: AUTHORING. Before integrating #3066, local Map/AI suites passed:
143 monster-AI and 234 World tests, with five pre-existing ignored tests unchanged.
Baseline server/Character-authority test compilation passed. On the integrated
tree, formatting, shell/JSON syntax, logging-event architecture validation,
diagnostic selector/aggregation checks, governance validation and all 129
governance tests passed. Two dispatch
lib-test builds and two all-targets Clippy attempts were resource-blocked by the
8 GiB memory limit; the integrated Clippy attempt reached the dispatch lib-test
before SIGKILL. No runtime RED/GREEN result is claimed for the superseded
prototype. PostgreSQL checks were not executed locally. Full candidate-specific
Rust, PostgreSQL and Node boot validation remains pending CI.
Independent bounded reviews accepted the scheduler semantics and found the two
logging issues listed above. Next: finish available checks on the integrated
tree, publish and freeze the remote SHA, then obtain exact-head CI and review.
The final SHA and run results belong in immutable PR/check evidence after the
last authoring commit; do not move a frozen head merely to update this record.

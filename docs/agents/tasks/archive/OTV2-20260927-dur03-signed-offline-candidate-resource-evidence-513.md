# OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513

```yaml
task_id: OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513
title: DUR-03 signed offline revision-2 candidate and finite resource evidence
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-signed-candidate-evidence-513
issue: 513
pr: 997
base_sha: 56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1
head_sha: f7b835899e250fe3bcb9a85a11fa2a88bb2dae2a
final_head_sha: f7b835899e250fe3bcb9a85a11fa2a88bb2dae2a
final_head_frozen_at: 2026-09-27T15:07:14Z
completed_at: 2026-09-27T15:46:09Z
owner: /root/dur03_max_shape_writer-local; root-sole-api-publisher
created_at: 2026-09-27T14:56:58Z
updated_at: 2026-09-27T15:46:09Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/game-events/v2/item_transaction.proto
  - apps/game-server/examples/dur03_reference_one_item_resource_prototype.rs
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513.md
public_contracts: []
depends_on: [513]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

[Allocation 5856736427](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856736427)
and [custody refinement 5856742846](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856742846)
authorize exactly this five-path unregistered offline successor. Local writer
cannot commit/push/API-write/comment/review/PR/MQ; root publishes and binds the head.
The authoring checks recorded below are not candidate-specific CI or final review.

## Architecture and source of truth

- `PROVEN`: protected base above, DUR-03 §39.1 closed one-item MINT/TRANSFER,
  GAME-ITEM-01 signed World geometry and independent legality inputs, existing
  ANL foundation envelope, bound META `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- `PROVEN`: v1 IDL/goldens/report values unchanged; signed v2 cannot reinterpret v1.
- `DERIVED`: canonical representation superset payload 186/277 B, complete
  one-event Envelope aggregate 1194/1285 B. Independently checked algebra and
  hand-encoded valid-fixture goldens; not a tight reachable or production maximum.
- `UNKNOWN`: production quantity/key/root legality and Content stack semantics,
  registration, live current authority, DB/restart, candidate profile/runtime
  retention enforcement evidence and parity. Protected P90D owning decision is
  not missing; the private candidate does not prove its runtime enforcement.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to production mutation: private standalone example only, no
authority installation, durable store or production trust consumer. Synthetic
fixture semantics still qualify independent World/revision/session/source facts;
one-invariant negative cases and signed-source custody comparisons are included.
Immutable expected bindings do not create live authority. No numerical product
policy is selected. Writer is not independent reviewer of its own code.

## Acceptance criteria and scope

- [x] Distinct revision 2, signed i32 x/y and checked i16 floor; malformed/synthetic
  World bounds/floor profiles and crossrevision fail closed; no Tibia adapter.
- [x] Closed synthetic direct-root enum; no product slot family or stack cap.
- [x] Independent byte algebra/literal oracle; valid fixture distinct from maximum
  width witnesses and impossible optional Envelope combinations.
- [x] Borrowed closed whole-Envelope/payload preflight before input-sized allocation,
  complete mandatory membership and checked reservations before publication.
- [x] Actual semantic-payload/wire copies and vector/string capacities; fixed
  two-record signed custody/retry model and exact frozen ambiguous candidate.
- [x] Explicit RL-01..08 and preserved production `EVIDENCE_GAP`.
- [x] Parent-published exact-head freeze, delta readback, CI and independent review.

Excluded from implementation scope: Content/registries/accepted contracts/v1
IDL/production src/Cargo/SQL, allocator/RSS/SQL claims, product limits and runtime
activation. PR integration is recorded below; this candidate does not claim
production readiness.

## Implementation and self-review

Private v2 module reuses only existing foundation Envelope carriers/dependencies;
finite signed schema, no framework. Actual fixture MINT 157/358 B, TRANSFER
227/476 B; actual retained model 1200 inline + 1554 dynamic = 2754 B on Linux
x86_64. Two-record injected reservation 5838 B; retry six logical units/record,
not CPU/latency/production horizon. Report documents excluded parser/hash/allocation/
World-profile work. V1 JSON subtree matches protected base exactly.

Self-review: implementing writer reviewed full bounded local delta. Repaired
source-tuple coherence (retained complete MINT.after/Ground plus synthetic World
revision versus TRANSFER.before/source, not only matching coordinates),
default-type raw-wire charge gap by closed tag3 rejection, capacity versus length
accounting (including excess proposed capacities) and explicit work exclusions.
Root's source-binding substitution and writer's excess-capacity findings were
reproduced RED, then repaired minimally and swept across World/definition/channel/
corpse/revision/geometry families. No unresolved material local finding. The
subsequent exact-frozen-head independent review passed in PR comment `5857079329`;
the final remote lifecycle is recorded below.

## Authoring validation

Linux WSL Ubuntu/rustc 1.94.0; use clean LF protected-base execution copy plus the
five authored files. Commands (all from repository root):

```sh
cargo test --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo build --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo clippy --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features --no-deps -- -D warnings
cargo fmt --all -- --check
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
cargo run --offline --locked -p oteryn-architecture-check -- workspace .
```

Focused: 40 tests/build/strict Clippy pass; inherited Tokio missing-doc warning
outside allocated paths. Format, governance, repository policy and workspace
boundaries pass on `/home/mole/dur03-v2-authoring.s8G6eW/repo` (clean LF base plus
exact five-file overlay). JSON twice/reverse/LF equality passes: 21098 bytes,
SHA-256 `6f714ed7fb6baaec1176adb49955fa9a4a6bd0aee7208e7724c82365cf0bf6db`.
String and wire/payload/membership Vec excess-capacity cases all reject before
publication. External handoff binds final results, not a self-referential SHA.
E2E/PG/runtime:
`NOT_APPLICABLE` to offline candidate, no production claim. At authoring-validation
time exact-head CI awaited the parent freeze; post-freeze source, architecture and
Merge Queue results are recorded below. Candidate2 byte evidence does not close
production #513.

## Protected integration and terminal disposition

- Frozen source candidate: `f7b835899e250fe3bcb9a85a11fa2a88bb2dae2a`, sole parent
  `56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1`, frozen at
  `2026-09-27T15:07:14Z` under Issue #162 comment `5857035880`.
- PR #997 received independent PASS review in PR comment `5857079329`.
- Source CI workflow `36328434157`, `game-gate` check `108647776448`: success.
  Post-ready architecture workflow `36330000127`: success.
- Native Merge Queue request receipt UUID `3004d599-a669-42d0-9054-69e5dc96900e`
  was read back; aggregate workflow `36330129323` ran on the queued branch/head.
  Its `game-gate` check
  `108652064201`: all required Linux, Windows, PostgreSQL, CodeQL and governance
  checks succeeded. PR #997 merged at `2026-09-27T15:46:09Z`; protected main is
  merge commit `a139c51e871027b5800c497d754a91a707af930f`.
- After merge, root verified the five blobs on protected main
  `a139c51e871027b5800c497d754a91a707af930f`; protected main remained unchanged
  before and after those five content reads. Pre-merge composition used the
  Merge Queue group diff, not a claim that V2 was already on protected main.
  Terminal Issue #162 comment is `5857383323`.
- Post-merge main Rust push workflow `36330770148` succeeded. Agent governance
  workflow `36330769998` failed only on four foreign active packet PRs
  #989/#994/#983/#984. They are outside this allocation; no foreign paths were
  touched and no full-main-health claim is made.

The allocation's candidate work is completed and this record is archived. PR #997
is terminally merged; aggregate Issue #513 remains open. The candidate remains an
unregistered synthetic offline measurement: no production schema/profile/resource
acceptance, canonical runtime, DB/restart conformance, or gameplay authority is
claimed. This archive closeout authorizes no further implementation or external
action. Closeout-head/PR identity is not asserted here; it is control-plane output.

## Context checkpoint

```yaml
last_progress: "PR #997 merged and protected main readback verified; allocation archived"
status: completed
branch: codex/dur03-signed-candidate-evidence-513
head_sha: f7b835899e250fe3bcb9a85a11fa2a88bb2dae2a
pr: 997
final_head_sha: f7b835899e250fe3bcb9a85a11fa2a88bb2dae2a
final_head_frozen_at: 2026-09-27T15:07:14Z
ci_trigger_source: pull_request/opened and ready_for_review
ci_check_generation: 36330000127
ci_checks_for_current_head: 2
ci_run_ids: [36328434157, 36330000127]
ci_job_ids: []
runner_assignment_state: success
merge_group_run_id: 36330129323
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none for this archived allocation; no further implementation authority
```

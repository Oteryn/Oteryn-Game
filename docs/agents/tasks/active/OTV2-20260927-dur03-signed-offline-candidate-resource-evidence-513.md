# OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513

```yaml
task_id: OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513
title: DUR-03 signed offline revision-2 candidate and finite resource evidence
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-signed-candidate-evidence-513
issue: 513
pr: null
base_sha: 56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/dur03_max_shape_writer-local; root-sole-api-publisher
created_at: 2026-09-27T14:56:58Z
updated_at: 2026-09-27T14:56:58Z
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
No authoring check below is candidate-specific CI or final review.

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
- [ ] Parent-published exact-head freeze, delta readback, CI and independent review.

Excluded: Content/registries/accepted contracts/v1 IDL/production src/Cargo/SQL,
allocator/RSS/SQL claims, product limits, runtime activation, final readiness or merge.

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
corpse/revision/geometry families. No unresolved material local finding.
Final exact-remote-head whole-diff self-review remains root's responsibility.

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
`NOT_APPLICABLE` to offline candidate, no production claim. Exact-head CI:
pending parent freeze. Candidate2 byte evidence does not close production #513.

## Independent review and lifecycle

Required: `YES`, separate exact-frozen-head code/semantic review per allocation;
parent controls dispatch/deduplication and policy. Independent literal/algebraic
oracle is authoring evidence, not whole-PR approval. Reviewer/head/verdict pending.
PR is null truthfully; current status stays implementing. Root may create DRAFT
only after exact byte/delta verification; no ready/review-trigger/enqueue/merge
authority is granted by this packet. Do not mutate the frozen head to add metadata.

## Context checkpoint

```yaml
last_progress: coherent signed candidate authored; authoring qualification underway
status: implementing
branch: codex/dur03-signed-candidate-evidence-513
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: return completed local qualification packet to root for API publication and freeze
```

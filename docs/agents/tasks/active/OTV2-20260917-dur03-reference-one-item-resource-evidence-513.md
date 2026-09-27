# OTV2-20260917-dur03-reference-one-item-resource-evidence-513

```yaml
task_id: OTV2-20260917-dur03-reference-one-item-resource-evidence-513
title: Deterministic one-item DUR-03 resource evidence
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-reference-one-item-resource-evidence-513
issue: 513
pr: null
base_sha: fd504f5659fe900861b6556751275488c2a22dec
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/dur03_513_writer
created_at: 2026-09-27T08:24:36Z
updated_at: 2026-09-27T08:45:30Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/dur03_reference_one_item_resource_prototype.rs
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.md
  - docs/agents/tasks/active/OTV2-20260917-dur03-reference-one-item-resource-evidence-513.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`NON_PRODUCTION_RESOURCE_EVIDENCE_ONLY`: one standalone Rust example and measured
JSON/Markdown evidence for deterministic Gold Coin x1. Natural drop probability is
`UNKNOWN/NOT_ASSERTED`. No production transaction, parity, audit payload/schema,
resource maximum, registry, SQL or runtime activation is delivered.

## Architecture and source of truth

- Allocation: #162 comment `5854116222`; requirement: #513 comment `5712622190`.
- Bound META policy: 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Existing public GAME-ITEM semantics and DUR-03 sections 4, 11, 20, 23-28 and 39.
- ANL-01 identity/envelope semantics; no executable registered DUR-03 event payload.
- Jira mapping: KAN-12 from coordinator allocation; aggregate acceptance and Jira
  synchronization remain control-plane work. This worker makes no Jira mutation.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to production mutation: only an in-memory, standalone evidence
model consumes independent fixture facts. It creates no production session,
lease, authority grant, persistence, identifier API or recovery consumer.
Five independently changed current fixture facts reject before publication for
both operations; they are not claimed as production authority proof.

## Acceptance criteria

- [x] DUR03-RL-01..08 have measured/dispositioned component evidence.
- [x] Separate physical MINT/TRANSFER counts, retained/encoded bytes and work.
- [x] Direct-root CharacterInventory only; Container/value/account/transform reject.
- [x] Private UUIDv7-shaped 16-byte transaction identity, stable across ambiguity.
- [x] Max/max+1/checked-overflow/resource/replay/custody tests remain example-local.
- [x] Actual normalized JSON matches repeat/reordered presentation runs.
- [x] Audit contribution count/payload bytes explicitly remain `EVIDENCE_GAP`.
- [ ] Control-plane remote freeze and fresh exact-candidate CI/review/integration.

## Excluded scope

All `src/**`, tests directories, migrations/SQL/PostgreSQL transactions, Cargo
and lock files, resource registry, event schema/family selection, production
TransactionId API, Content/Combat runtime, client/protocol and deployment.

## Implementation / findings

One item participant is retained per operation. MINT has one establish effect;
TRANSFER has remove+establish effects but replaces one immediate custody tuple.
Exactly one logical MINT and TRANSFER slot reject replacement identities after
ambiguity. TRANSFER requires MINT acknowledgement/reconciliation. Retry holds the
frozen plan, same ItemInstanceId and same logical identity. Known-abort retries
recheck finite budgets before publication.

Measurements on Linux x86_64: participants 1/44 retained/44 encoded bytes each;
MINT effects 1/72 retained/58 encoded bytes, total encoding 102 and work 2/2;
TRANSFER effects 2/144 retained/94 encoded bytes, total encoding 138 and work 3/3.
Plan 344 bytes, one reconciliation record 360 bytes, two records 720, Model 816.
Actual mandatory audit count and aggregate encoded event bytes remain null. A
labelled synthetic size-only probe proves budget check placement and emits no event.

## Validation

Authoring delta in fresh isolated LF Linux workspace `/home/mole/dur03-513-p2.9unFjx/repo`:

- Focused offline/locked/no-default-features example tests: 16 passed, 0 failed.
- Locked example build and strict example Clippy (`--no-deps -- -D warnings`): PASS.
- `cargo fmt --all -- --check`: PASS after isolated formatting applied with apply_patch.
- Two normal runs + `--reverse-fixtures`: byte equal; 6,795 bytes.
- Output SHA-256: `ea79b26bd3412f478b6ffe43ce277625e7afa513f68556d66b855fa67690920f`.
- Qualified retained run: Linux x86_64 executable target. Report build toolchain
  identity is `UNKNOWN`, with no build-time compiler attestation. Separately,
  validation-environment rustc observation is `1.94.0 (4a4ef493e 2026-03-02)`;
  it is not bound to the binary build identity.
- Local governance: PASS (22 policy documents, 9 lanes); repository policy: PASS
  (23 files, 43 workflows and post-merge routing regressions).
- Candidate-specific inherited-policy/routing: pending hosted PR CI after control-plane
  PR creation and remote freeze; no fabricated event/PR or full active-task health run.
- Workspace boundaries: PASS; staged `git diff --check`: PASS; exactly four
  allocated paths added, with LF index blobs and an LF Rust validation checkout.
- Existing vendored Tokio `missing_docs` warning persists; no example warnings.
- Production/PostgreSQL/E2E validation: NOT_APPLICABLE; implementation is absent.
- Exact-head CI: pending remote freeze and control-plane dispatch; no PR yet.

Full commands, resource boundary matrix and limitations are in the owned evidence MD.

## Self-review

Implementing writer reviewed the complete bounded delta: no public/production API,
maxima, SQL, event-family selection or extra path. The adversarial review added
known-abort budget rechecks, ambiguous replacement rejection and the acknowledged
MINT barrier before TRANSFER. Focused tests cover those finding families. Final
local candidate SHA/tree and changed-path readback are returned outside this commit.

Independent-review P2 thread `4114686069`: fixed in the local successor atop
`d561af69b4c57fd5b844b2810f9019cdf5ce7bd9`. Runtime rustc probing/identity is removed;
dynamic executable target metadata remains and build toolchain identity is `UNKNOWN`
with an explicit absent-attestation reason.
The new focused regression plus 15 original tests pass; resource measurements
are unchanged. Actual normalized JSON and this checkpoint use the successor digest.
An executable check with rustc absent from PATH succeeds with byte-identical JSON.

## Independent review

Exact-candidate qualification/review is control-plane-owned after remote freeze.
No manual review trigger, PR, Issue/Jira status change or MQ action is made by this
writer. Authoring test results are not independent review or integration proof.

## PR and closeout

This pre-PR packet is `blocked` only on publication routing; it does not claim ready/validating or
aggregate #513/KAN-12 completion. The commit cannot contain its own SHA. The
writer returns its exact local candidate SHA/tree to the coordinator. Publication
is held: bound META Publication Integrity Policy lines 58-60 require a verified
recovery artifact before first publication, and lines 66/72 require a guarded
expected-old-value lease. The allocation permits only four authored paths and
forbids force options. No helper/bundle or publication bypass is added. The
coordinator must reconcile the route before remote freeze and fresh checks.
Pre-existing untracked `.codex/` is preserved.

## Context checkpoint

```yaml
last_progress: P2 thread_4114686069 fixed; build_toolchain explicitly UNKNOWN; local successor checked
status: blocked
branch: codex/dur03-reference-one-item-resource-evidence-513
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: bound recovery_and_expected_old_value_guard_requirements_need_control_plane_route_resolution
next_action: return exact local candidate SHA and bound policy requirement to coordinator
```

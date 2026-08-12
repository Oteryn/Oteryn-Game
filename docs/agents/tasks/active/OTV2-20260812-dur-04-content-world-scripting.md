# OTV2-20260812-dur-04-content-world-scripting

```yaml
task_id: OTV2-20260812-dur-04-content-world-scripting
title: DUR-04 content world and scripting architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: agent/otv2-20260812-dur-04-content-world-scripting
pr: 212
base_sha: c0b7b3b5928b194f8b51a4a51d9eb2d01e32ce44
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-12T23:21:00+02:00
updated_at: 2026-08-13T00:15:00+02:00
execution_budget_minutes: 90
large_budget_reason: Bounded architecture gate spans deterministic content compilation, immutable world bundles, migration/provenance, secure loading and capability-bounded deterministic scripting; no implementation is authorized.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-04-content-world-scripting.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_ANALYSIS.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
public_contracts:
  - DUR-04
depends_on:
  - ADR-0005
  - GAME-VISION-01
  - GAME-ITEM-01
  - DUR-02
  - DUR-03
  - GAME-CHANNEL-01
  - FND-03
  - ANL-01
blocks:
  - broad content import
  - durable scripted behavior
  - final content/bundle implementation contract
cross_repository_coordination_id: OTV2-NATIVE-WORLD-CONTENT
external_repositories:
  - blakinio/Otheryn
```

## Outcome

Produce one bounded paper-only `DUR-04 — Content, World Detail and Scripting Contract` candidate completing the architecture left deliberately open by ADR-0005 without implementing World Project encoding, compiler, World Bundle, loader, Studio or scripting runtime.

No Rust/runtime/client implementation, PostgreSQL DDL/migrations, Studio implementation, production configuration/deployment, Platform write or entitlement activation is authorized.

Maintained current-status/register/horizon/index/handoff files remain outside this delivery PR and may be promoted only by a separate lifecycle closeout after an accepted delivery merge.

## Verified architecture/source facts

- `PROVEN`: post-GAME-CHANNEL `main@c0b7b3b5928b194f8b51a4a51d9eb2d01e32ce44` selected `DUR-04 = PROPOSED / PLANNED / NOT_STARTED` as the next paper-only architecture gate.
- `PROVEN`: no active DUR-04 task and no open DUR-04 PR existed at task start.
- `PROVEN`: ADR-0005 already accepts the native semantic world/content model and deliberately defers exact physical encoding/chunk/compression choices to a bounded spike.
- `PROVEN`: Resource Limits Registry makes missing externally controlled hard limits a review failure, never implicit unlimited behavior.
- `PROVEN`: GAME-ITEM/DUR-03/GAME-CHANNEL/FND-03 boundaries prohibit script-side authority bypass, unclassified per-channel value multiplication and unbounded authoritative work.
- `PROVEN`: official Wasmtime documentation distinguishes deterministic fuel from nondeterministic epoch interruption, documents determinism hazards including nondeterministic imports, NaN behavior and memory/table growth, and exposes configurable operator-cost semantics for fuel.
- `PROVEN`: Component Model/WIT defines typed import/export interfaces and explicit component worlds suitable for a project-owned capability ABI.
- `PROVEN`: governance forbids new Codex/OpenAI API/owner-funded AI invocation without specific owner permission. None was invoked for DUR-04.
- `PROVEN`: on 2026-08-13 the repository owner explicitly instructed the implementing coordinator to perform the review itself (`zrob sam review`) in direct response to the independent-review blocker. Under the repository instruction order this is a bounded owner override of the independent-review mechanism for PR #212. The resulting review remains a self-review and MUST NOT be described as independent.
- `UNKNOWN/DEFERRED`: final source serializer, final bundle container, chunk/floor packing, compression codec, exact numeric resource ceilings and exact Wasmtime crate version.

## Candidate decisions delivered

- [x] Stable `PackageKey`, immutable `PackageRevision`, stable `ContentKey` and exact Content Lock semantics.
- [x] Typed semantic graph independent of YAML/RON/JSON5/custom physical encoding.
- [x] Deterministic source -> validation -> lowering -> client/server projection -> immutable bundle -> staging -> activation pipeline.
- [x] Distinct content/map/ruleset/world-policy/compiler/artifact revision concepts plus a distinct versioned `script_execution_profile_revision`.
- [x] Explicit allowlisted client-safe projection and server-only data non-leakage rule.
- [x] Fail-closed loader with bounded allocation/decompression and no partial authoritative publication.
- [x] Explicit staging/activation/rollback and durable migration classes.
- [x] Legacy provenance plus `COPY`/`CONVERT`/`REWRITE`/`REFERENCE_ONLY`/`REJECT` dispositions.
- [x] Reference OTS evidence cannot independently establish Global behavior.
- [x] WebAssembly Component Model + project-owned versioned WIT capability ABI selected for target authoritative scripting boundary; Wasmtime is an implementation candidate rather than ABI owner.
- [x] Scripts are bounded proposal producers and never direct SQL/domain mutation owners.
- [x] Authoritative host reads are invocation-snapshot-bound; extension-state writes are typed proposals committed only after domain validation.
- [x] `ProposedActionPlan` cannot manufacture atomicity wider than accepted owner/transaction boundaries; wider workflows require an owning orchestrator/OperationId/idempotency/compensation contract.
- [x] Deterministic fuel, logical time, invocation-local deterministic RNG, bounded host calls/collections/action plans, stable host-query ordering and deterministic floating/NaN policy are bound to a versioned execution profile.
- [x] Typed/versioned script extension state through domain/persistence APIs; VM memory is not durable state.
- [x] GAME-CHANNEL multiplicity/eligibility classification required for relevant value-producing content.
- [x] Hot reload defined as immutable revision staging/activation, never in-place mutation.
- [x] Mandatory future implementation evidence and physical-format spike gate.
- [x] Mandatory architecture decision test completed for the Component Model/WIT technology boundary: timing, concrete blocked work, later migration cost, superseding evidence and deliberate deferrals are explicit.

## Repair history

### Repair cycle 1

Adversarial pre-freeze review found that fuel alone does not close deterministic execution: host query result ordering and observable floating-point/NaN behavior also require a contract. The contract added canonical/stable host-return ordering and either project-owned deterministic numeric semantics or explicitly proven deterministic floating behavior including NaN canonicalization where observable.

### Repair cycle 2 — owner-directed terminal self-review

The owner-directed full self-review found three related architecture gaps:

1. immediate extension-state write capability could leave authoritative state changed before a later trap/invalid result, contradicting proposal-only/no-commit-on-failure semantics;
2. a generic `ProposedActionPlan` could otherwise imply atomicity wider than an accepted owner/transaction boundary;
3. deterministic fuel semantics need an explicit execution-profile identity because engine/config/operator-cost changes must not silently reinterpret the same content revision and fuel number.

The repair therefore:

- makes authoritative reads snapshot/revision-bound;
- makes extension-state mutations proposal-only until owning transaction/workflow commit;
- limits each plan type to a named accepted authority/transaction scope and requires explicit orchestrated workflows for wider multi-transaction behavior;
- adds `script_execution_profile_revision` covering determinism-sensitive runtime semantics while keeping the public WIT ABI project-owned and engine-independent;
- adds activation/replay/evidence requirements for WIT/execution-profile compatibility and mutable-global-RNG avoidance;
- clarifies that artifact hashes are integrity evidence, not publisher-authenticity evidence.

### Repair cycle 3 — architecture decision test

An existing review P1 (`PRRT_kwDOTuGrds6YvbDz`) correctly identified that freezing Component Model/WIT as a normative target boundary required the full repository decision test rather than a list of selected/deferred topics.

The analysis now records:

- `Must decide now? = YES` for the target authoritative scripting capability boundary and `NO` for benchmark-sensitive serializer/container/runtime-version/numeric choices;
- concrete blocked work: the first authoritative WIT/script-host package, broad durable scripted-content migration and legacy quest/NPC/action/event adaptation;
- future migration cost: guest rewrites/dual-running, compatibility adapters, manifest/capability migration, replay fixture rebuild, persistent extension-state revalidation and renewed capability security review;
- superseding evidence: failed deterministic/resource-isolation prototype, representative budget failure, insufficient tooling/language support, material unmitigated security finding, unacceptable Component Model/WIT compatibility burden, or changed product requirements/a demonstrably safer capability model;
- deliberate deferrals including exact WIT function inventory, exact Wasmtime version, physical encoding, numeric limits and optional legacy adapter.

Repair budget used: **`3/3`**. Any new material finding on the final head must block/rotate instead of triggering a fourth repair without new owner authorization.

## Validation

### Focused

- source/ownership audit: `PASS`
- exact changed-file scope: expected three owned paths only
- architecture analysis/contract drafting: `PASS`
- repair cycle 1: deterministic ordering/FP determinism
- repair cycle 2: proposal-state/authority-scope/execution-profile hardening
- repair cycle 3: complete Component Model/WIT architecture decision test
- final exact-head owner-directed self-review: pending after freeze
- runtime/component/integration/E2E: `NOT_APPLICABLE` — paper-only architecture candidate

### Owner-funded AI policy and review mechanism

- new Codex/OpenAI API/paid-review invocation: **NONE**
- owner-funded AI permission: **NOT REQUESTED / NOT USED**
- ordinary repository risk policy would require independent review for this scope;
- explicit owner instruction on 2026-08-13 directs the implementing coordinator to perform the review itself for PR #212;
- this is recorded as a bounded owner override of the review mechanism, **not** as independent review evidence;
- terminal evidence must truthfully say `owner-directed self-review`, never `independent review`.

## Context checkpoint

```yaml
last_progress: DUR-04 consumed repair budget 3/3 after completing the Component Model/WIT decision test in response to existing P1 review evidence. Earlier repair 2 already closed proposal-state, cross-authority ActionPlan and execution-profile determinism gaps. No owner-funded AI service was invoked.
status: validating
branch: agent/otv2-20260812-dur-04-content-world-scripting
head_sha: null
pr: 212
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_after_repair_3
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Freeze the repair-3 head, resolve only findings actually fixed, perform one final full exact-head owner-directed self-review and exact-head Agent Governance / Dependency Review / CodeQL; merge only if the unchanged head is clean because no further material repair is allowed within the current budget.
```

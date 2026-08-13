# OTV2-20260812-dur-04-content-world-scripting — archived

```yaml
task_id: OTV2-20260812-dur-04-content-world-scripting
title: DUR-04 content world and scripting architecture
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: agent/otv2-20260812-dur-04-content-world-scripting
delivery_pr: 212
base_sha: c0b7b3b5928b194f8b51a4a51d9eb2d01e32ce44
final_head_sha: 77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23
delivery_merge_sha: 568236c33cd23da017bca1dbd1ed98afc8da71f4
lifecycle_closeout_branch: docs/OTV2-20260813-dur-04-closeout
lifecycle_closeout_pr: 213
owner: released_after_closeout
created_at: 2026-08-12T23:21:00+02:00
completed_at: 2026-08-13T00:30:00+02:00
execution_budget_minutes: 90
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260812-dur-04-content-world-scripting.md
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
cross_repository_coordination_id: OTV2-NATIVE-WORLD-CONTENT
external_repositories:
  - blakinio/Otheryn
```

## Outcome

Delivered one bounded paper-only `DUR-04 — Content, World Detail and Scripting Contract` architecture completing the semantic content/package/compiler/bundle/activation/migration/scripting boundary intentionally left open by ADR-0005.

Delivery PR #212 was squash-merged unchanged from final frozen exact head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23` as `568236c33cd23da017bca1dbd1ed98afc8da71f4`.

The delivery implements nothing. Rust/runtime/client/compiler/loader/Studio implementation, PostgreSQL DDL/migrations, broad content import, Platform writes, production activation/configuration and entitlement activation remain unauthorized.

## Binding sources consumed

- ADR-0005 — native World Project / canonical semantic model / immutable World Bundle direction, stable namespaced keys, static authored versus dynamic durable state, spatial-layer separation, bounded legacy conversion and Oteryn Studio boundary;
- GAME-ITEM-01 — typed item semantics and rejection of arbitrary authoritative misc-state escape hatches;
- DUR-02 — persistence migration/transaction/restore ownership;
- DUR-03 — item/currency/value conservation, idempotency, fencing and transaction authority;
- GAME-CHANNEL-01 — explicit simulation versus durable-eligibility/multiplicity classification;
- FND-03 — bounded authoritative execution and deterministic stale-result behavior;
- ANL-01 — authoritative audit/event boundary;
- Resource Limits Registry — explicit hard maxima before implementation acceptance.

## Accepted candidate semantic closure delivered by #212

### Content/package identity

- canonical content is a typed semantic graph, not a serializer or directory layout;
- stable namespaced `PackageKey`, immutable `PackageRevision`, stable `ContentKey` and deterministic exact Content Lock;
- no floating/latest/network-time dependency resolution in production runtime;
- compact/legacy numeric IDs remain revision-scoped mappings rather than durable semantic identity;
- `content_revision`, optional `map_revision`, `ruleset_revision`, `world_policy_revision`, compiler identity, `script_execution_profile_revision` and exact bundle digest remain distinct identities.

### Deterministic compiler and immutable bundle

- source/import -> typed model -> validation -> dependency/key/alias resolution -> migration normalization -> deterministic lowering -> client/server projection -> deterministic artifact assembly -> integrity -> immutable staging -> explicit activation;
- identical locked inputs/compiler/canonicalization/profile inputs require identical logical output and artifact digest;
- nondeterministic filesystem order, unordered iteration, locale/timezone, wall clock, machine paths, uncontrolled randomness, duplicate precedence and unstable serialization must be canonicalized or rejected;
- client-safe output is allowlist projection rather than serialize-everything-and-hide;
- server-only authoritative data must not leak into client artifacts;
- artifacts remain immutable and content-addressed; no unstable Rust memory layout becomes public storage format.

### Loader, activation and migration

- artifacts remain untrusted until bounded header/size/decompression/arithmetic/version/integrity/dependency/index/semantic validation and isolated staging complete;
- no partially validated authoritative publication;
- all externally controlled counts/sizes/depths/ratios require Resource Limits Registry entries before implementation acceptance;
- staging and activation are distinct; live immutable revisions are never edited in place;
- rollback requires semantic compatibility with durable state;
- durable interpretation changes use explicit classes: `COMPATIBLE_NO_MIGRATION`, `READ_COMPATIBLE_NORMALIZE`, `EXPLICIT_DATA_MIGRATION`, `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION`, `REMOVED_WITH_EXPLICIT_POLICY`;
- scripts never perform direct SQL/data migrations.

### Legacy conversion and provenance

- every external source set records exact source revision/archive digest, license/provenance, `COPY|CONVERT|REWRITE|REFERENCE_ONLY|REJECT`, importer identity, deterministic conversion report, lossy/unresolved diagnostics and legacy mappings;
- Legacy Intermediate Representation remains an importer-boundary type, never the canonical Oteryn model;
- Otheryn/Canary/other OTS material is migration/reference inventory, not proof of Global Tibia behavior.

### Authoritative scripting boundary

- target boundary is WebAssembly Component Model with a project-owned versioned WIT capability ABI unless explicitly superseded by later accepted architecture;
- Wasmtime is an initial implementation candidate and does not own the public Oteryn script ABI;
- exact WIT package/world/interface requirements are compatibility inputs;
- process-global Lua/game-object model is rejected as the target authority model; any compatibility adapter is separately gated;
- scripts are bounded proposal components over `InvocationContext + immutable ReadSnapshot + ExplicitCapabilities` and return typed result/action proposals for host/domain validation;
- no ambient filesystem/network/process/environment/secrets/unrestricted wall clock/OS RNG/direct SQL/global Game/mutable Rust-server object authority;
- authoritative reads are snapshot/revision-bound;
- extension-state writes are proposal-only until owning transaction/workflow validation and commit;
- `ProposedActionPlan` cannot create atomicity wider than accepted owner/transaction boundaries; broader workflows require an accepted owning orchestrator with OperationId/idempotency/compensation semantics.

### Determinism and failure isolation

- authoritative scripts use logical simulation time, invocation-local deterministic RNG, deterministic fuel, bounded memory/table/instances/host calls/results/action plans, stable query ordering and deterministic error mapping;
- authoritative floating behavior must be explicitly deterministic, with observable NaN behavior controlled; fixed/project-owned numeric semantics are preferred where practical;
- `script_execution_profile_revision` binds determinism-sensitive engine/runtime compatibility, Wasm features, fuel operator-cost policy, floating/NaN/relaxed-SIMD and memory/table behavior so upgrades cannot silently reinterpret an existing budget;
- epoch/wall-clock interruption is only a secondary operational kill switch and cannot define gameplay/replay outcome;
- trap/fuel exhaustion/invalid plan commits no proposed action or extension-state write;
- VM memory/table state is never durable gameplay state;
- persistent script extension state is typed, namespaced, versioned, bounded and persisted/fenced/migrated by accepted domain/persistence ownership.

### Supply chain and physical format

- exact package lock, component digests, WIT requirements and script execution profile are bound to the content revision;
- hashes prove integrity, not publisher authenticity; signing/trust-root/CDN remains downstream;
- final YAML/RON/JSON5/custom source encoding, `.omap/.owb` naming, bundle container, chunk/floor packing, compression, exact Wasmtime version and numeric ceilings remain deliberately unfrozen;
- a bounded non-canonical format/compiler/loader spike must provide deterministic-byte, random-access, corruption/decompression, round-trip/equivalence, editor recovery, representative-scale, 32x32-vs-64x64/floor-packing, patch granularity and locality evidence before final physical encoding is accepted.

## Architecture decision test for Component Model/WIT

- **Must decide now:** YES for the target authoritative guest/host capability boundary; NO for benchmark-sensitive serializer/container/runtime-version/numeric choices.
- **Concrete blocked work:** first authoritative WIT/script-host package, broad durable scripted-content migration, and legacy quest/NPC/action/event adaptation behind a stable capability surface.
- **Later migration cost:** guest rewrites/dual-running, compatibility adapters, package/capability migration, replay fixture rebuild, extension-state revalidation, repeated capability security review and preservation of old execution profiles while old revisions drain/replay.
- **Superseding evidence:** bounded prototype cannot meet determinism/isolation, representative budgets cannot be met, required guest tooling/language support is insufficient, an unmitigable security finding emerges, WIT/Component Model compatibility burden becomes unacceptable, or product requirements materially change/a demonstrably safer lower-cost capability model emerges.
- **Deliberate deferrals:** exact WIT function inventory/lowering, exact Wasmtime version/features, physical formats, numeric limits, optional legacy adapter, signing/CDN, Studio details and domain formulas.

## Repair history

### Cycle 1 — host-query and floating determinism

Adversarial pre-freeze review found deterministic fuel did not close query-order and observable floating/NaN nondeterminism. Stable/canonical host-return ordering and deterministic numeric/NaN policy were added.

### Cycle 2 — proposal side effects, transaction scope and execution profile

Owner-directed review found immediate extension-state writes could survive later failure, a generic ActionPlan could imply cross-owner atomicity, and engine/fuel configuration changes could silently reinterpret execution budgets. The repair introduced snapshot-bound reads, proposal-only extension-state writes, explicit ActionPlan authority scope and `script_execution_profile_revision`.

### Cycle 3 — complete architecture decision test

Existing P1 thread `PRRT_kwDOTuGrds6YvbDz` required full decision-test evidence before freezing Component Model/WIT. The analysis now records must-decide timing, blocked downstream work, migration cost, named superseding evidence and deliberate deferrals. Repair budget ended at `3/3`.

Existing extension-state P1 thread `PRRT_kwDOTuGrds6YvbD5` was also materially repaired by cycle 2. Both threads were resolved/outdated on the final delivery head.

## Terminal delivery validation

Frozen exact delivery head: `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`.

- owner-directed exact-head self-review `4921665072`: **PASS**, material findings `0`;
- this review is explicitly self-review, not independent review;
- owner instruction on 2026-08-13 selected the implementing coordinator as terminal reviewer for PR #212, overriding the ordinary independent-review mechanism only for this delivery;
- Agent Governance `31646998515`: **PASS**;
- Dependency Review `31646998564`: **PASS**;
- CodeQL `31646998517`: **PASS**;
- unresolved material review threads immediately before merge: `0`;
- final changed paths: exactly task + analysis + candidate contract;
- final compare to live main: `behind_by=0`;
- component/integration/runtime E2E: `NOT_APPLICABLE` — paper-only architecture delivery;
- Codex/OpenAI API/other owner-funded AI invocation by this coordinator: **NONE**.

PR #212 was squash-merged unchanged from the frozen head as `568236c33cd23da017bca1dbd1ed98afc8da71f4`.

## Lifecycle closeout discipline

The separate closeout does not change DUR-04 semantic analysis or contract. Closeout PR #213 owns only archive/status/register/horizon/index/checkpoint/handoff reconciliation and releases DUR-04 ownership after merge.

## Context checkpoint

```yaml
last_progress: DUR-04 delivery PR #212 passed final owner-directed exact-head self-review after repair budget 3/3 and all exact-head CI, then squash-merged unchanged as 568236c33cd23da017bca1dbd1ed98afc8da71f4; lifecycle closeout is PR #213.
status: completed
delivery_pr: 212
final_head_sha: 77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23
delivery_merge_sha: 568236c33cd23da017bca1dbd1ed98afc8da71f4
lifecycle_closeout_pr: 213
owner_directed_self_review: 4921665072
ci_run_ids:
  - 31646998515
  - 31646998564
  - 31646998517
repair_cycles_for_delivery_gate: 3
owner_action_required: false
blocker: null
next_action: None for this completed task; follow the canonical programme checkpoint and successor handoff for future work.
```

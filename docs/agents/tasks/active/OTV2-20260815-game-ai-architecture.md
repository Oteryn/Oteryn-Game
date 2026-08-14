# OTV2-20260815-game-ai-architecture

```yaml
task_id: OTV2-20260815-game-ai-architecture
title: Design GAME-AI-01 creature AI, spawn and pathfinding architecture
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/arch-c-game-ai
pr: 272
base_sha: 088b46638ac014cd7928d6b0b75cee44902fe22c
head_sha: 1f5f21d28d56aef00dc781a196a50ac4dc0dc883
final_head_sha: null
final_head_frozen_at: null
owner: DOMAIN ARCHITECTURE DESIGN AGENT / worker C
created_at: 2026-08-15T00:18:00+02:00
updated_at: 2026-08-15T00:34:20+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md
  - docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/active/OTV2-20260815-game-ai-architecture.md
public_contracts:
  - docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md
depends_on:
  - FND-03
  - GAME-CHANNEL-01
  - DUR-04
  - SIM-DETERMINISM-01
  - GAME-ABILITY-01 accepted partial baselines
  - GAME-VISION-01 first Reference target/evidence discipline
blocks:
  - Architecture Coordinator audit of draft PR #272
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Deliver a bounded noncanonical worker proposal for `GAME-AI-01` that:

- keeps creature/spawn/AI mutation inside the current authoritative Channel/Instance owner;
- selects an explicit bounded behavior execution model;
- defines deterministic aggro/target/memory and pathfinding boundaries without inventing Reference tuning;
- binds spawn/template/content provenance and recovery semantics;
- permits dynamic population/ecology only through explicit bounded versioned policy;
- preserves GAME-CHANNEL value-source multiplicity/eligibility;
- separates AI intent from combat, interaction, reward/value and persistence owners;
- defines resource-limit dimensions, failure/recovery cases and Reference/Evolved acceptance requirements;
- ends as draft PR `#272` for Architecture Coordinator audit only.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`

## Architecture and source of truth

### PROVEN

- Issue `#261` allocates worker C to `GAME-AI-01` and forbids runtime implementation, foreign-domain edits, global overlays, unsupported AI-library selection and worker merge/lifecycle closeout.
- Coordinator activation binds the lane to trusted base `main@088b46638ac014cd7928d6b0b75cee44902fe22c` on `docs/arch-c-game-ai`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` assigns channel-local creatures/spawns/AI to `ChannelRuntime`, instance-local creatures to `InstanceRuntime`, and auxiliary pathfinding/AI work to proposal + owner revalidation semantics.
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` requires explicit value-source multiplicity/eligibility and forbids treating runtime Channel locality as automatic reward repeatability.
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` supplies immutable content/World Bundle provenance and proposal-only bounded script semantics.
- `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` supplies revision binding, deterministic RNG/tie-break/order/replay/state-hash semantics.
- Accepted partial GAME-ABILITY baselines retain targeting/cast/cooldown/effect ownership; whole-gate GAME-ABILITY remains sibling/coordinator work.
- GAME-VISION first-Reference target/evidence discipline forbids guessing unknown target behavior from OTS/current-live implementation similarity.
- Current `docs/agents/BUILD_TEST_MATRIX.md` classifies architecture/contracts-only PR validation as always-required governance, dependency review, CodeQL and aggregate merge gate; Rust/workspace jobs are path-proportional and are not required solely for this documentation-only slice.

### DERIVED

- Runtime AI is a local authoritative simulation subsystem, distinct from ADR-0006 read-only Game Intelligence / Investigation AI.
- A typed bounded FSM is the smallest current execution model that makes transition bounds, deterministic replay and failure isolation explicit while permitting bounded DUR-04 proposal leaves.
- Pathfinding must remain auxiliary proposal work with generation/revision/goal revalidation before route adoption.
- Future-determining AI/spawn state must participate in SIM deterministic state/replay evidence.
- Dynamic population/ecology cannot be a hidden runtime/analytics feedback loop; any such behavior requires an immutable versioned bounded policy and ordinary authoritative inputs.

### UNKNOWN

Exact Reference perception, aggro/threat, retarget, leash/reset, path preference, spawn/respawn/occupancy, dynamic ecology/population behavior, summon/pet command/reward, NPC movement coupling and boss recovery semantics remain evidence-gated.

## Acceptance criteria

- [x] Analysis records binding authority/dependency boundaries and Reference evidence gaps.
- [x] Candidate contract selects a bounded behavior representation with explicit behavior-tree/script disposition.
- [x] Perception/aggro/threat/target/memory pipeline is deterministic and bounded without invented Reference tuning.
- [x] Pathfinding request/result/cancellation/stale-rejection semantics preserve FND-03 owner authority.
- [x] Spawn/template/source provenance and finite occupancy/respawn/recovery classes are explicit.
- [x] Dynamic population/ecology requires an explicit bounded versioned policy and cannot be directly analytics-controlled.
- [x] GAME-CHANNEL multiplicity/eligibility is preserved for value-producing sources.
- [x] Controlled actor and NPC boundaries preserve server authority and foreign business owners.
- [x] Loot/XP/reward abuse boundary prevents direct AI value mutation and records downstream ownership.
- [x] Resource-limit dimensions are enumerated without inventing benchmark-sensitive numbers.
- [x] Failure/recovery matrix and deterministic future acceptance scenarios are present.
- [x] Reference/Evolved mapping separates shared engine invariants from evidence/profile policy.
- [x] Required worker handoff markers `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS` are present.
- [x] PR metadata contains role/domain/issue/merge/implementation authority plus summary/scope/owned paths/proposed decisions/dependencies/validation/self-review handoff fields.
- [x] Cross-domain findings are normalized with stable IDs/owner/severity/evidence/required-before/`REPORT_ONLY` disposition in PR handoff metadata; no foreign architecture file is edited.
- [x] Draft PR `#272` exists and changed-file scope is exactly the three worker-owned paths.
- [x] Material self-review findings identified before final freeze were repaired and recorded.
- [ ] Exact-final-head ordinary repository CI conclusion is recorded externally in immutable PR/check evidence after this final task-metadata commit exists.
- [ ] Exact-final-head full-diff self-review is recorded externally in immutable PR evidence after this commit exists.

## Excluded scope

This worker task MUST NOT:

- implement runtime/content code;
- select a concrete Rust AI/pathfinding/behavior-tree library;
- select benchmark-sensitive numeric resource limits without evidence;
- edit global architecture registers/status/backlogs/programme allocation;
- edit GAME-ABILITY, GAME-INTERACTION, GAME-ITEM, DUR, ANL or other foreign contracts;
- claim exact Reference behavior where evidence is missing;
- claim `DecisionStatus=ACCEPTED`, canonicality, lifecycle closeout or runtime implementation;
- merge, enable auto-merge, mark the PR ready, close issue #261 or archive this task record.

## DECISIONS_NOT_TAKEN

The worker deliberately does **not** decide or authorize:

- a concrete Rust AI, behavior-tree or pathfinding library/framework;
- a concrete pathfinding algorithm or physical navigation representation;
- numeric AI/path/spawn resource ceilings without implementation evidence;
- physical AI/spawn content serializer/schema beyond DUR-04 semantic requirements;
- exact Reference perception, aggro/threat, retarget, memory, leash/reset, path-preference, spawn/respawn/occupancy or dynamic-ecology rules where evidence is absent;
- exact summon/pet command vocabulary, persistence, despawn, XP or loot attribution;
- exact NPC business/quest/trade semantics;
- exact boss/world-event durable occurrence/eligibility owner APIs;
- exact reward contribution/settlement rules;
- runtime implementation, production activation, canonical acceptance, merge or lifecycle closeout.

These are explicit non-decisions, not permissive defaults.

## Implementation / findings

1. Verified branch head before writes exactly matched coordinator trusted base `088b46638ac014cd7928d6b0b75cee44902fe22c`.
2. Verified live `main@cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e` remained one coordinator closeout/bookkeeping commit ahead of the trusted base and did not introduce a material GAME-AI dependency change.
3. Verified no sibling worker owns or changes the three GAME-AI paths. Open Agent A/B/D/E/F PRs remain noncanonical parallel proposals; Agent A PR #271 reports 0/4 ABILITY_COMBAT evidence promotions and Agent B PR #268 remains a draft whole-gate candidate.
4. Resolved terminology collision: ADR-0006 Game Intelligence / Investigation AI is read-only external analysis; GAME-AI runtime behavior remains inside authoritative simulation.
5. Selected typed bounded FSM for v1 semantic behavior; runtime behavior tree is deferred and direct authoritative script mutation rejected.
6. Defined pathfinding as bounded auxiliary proposal work with deterministic search profile and owner revalidation.
7. Defined immutable actor/template/spawn provenance, explicit spawn recovery classes and GAME-CHANNEL multiplicity gates.
8. Self-review found the horizon's dynamic population/ecology requirement was not explicit enough in the first candidate draft; repaired at semantic checkpoint `b4924278f543325007356b2e2b52ca0d0d0fc966` by requiring immutable/versioned bounded population policy and prohibiting direct analytics-to-runtime feedback.
9. Work-allocation review found required literal handoff markers were not canonically named; repaired before final freeze.
10. Agent governance run `31846830146` on `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c` failed before checkout because PR metadata lacked required `## Summary` and `## Scope`; Merge authority audit `31846830143` passed. PR metadata was repaired; the failed run is retained as historical evidence and is not treated as a repository-validation PASS.
11. Full domain-prompt handoff review found PR metadata needed explicit role/domain/issue/implementation authority and structured cross-domain finding fields; repaired before final freeze without adding foreign repository changes.
12. Validation requirements were reconciled to the current Build/Test Matrix: docs-only architecture needs the always-required merge gate; a Rust/workspace architecture binary run is not a required blocker for these paths.

## CROSS_DOMAIN_FINDINGS

The complete structured records are present in PR #272 metadata. Repository-file disposition remains report-only:

- `GAME-AI-XD-01` / P1 / GAME-ABILITY-01 + coordinator — typed AI action-intent/rejection seam before executable combat integration.
- `GAME-AI-XD-02` / P1 / GAME-INTERACTION-01 — dynamic navigation invalidation/interaction seam before executable dynamic-world pathing.
- `GAME-AI-XD-03` / P1 / GAME-ITEM-01 + DUR-03/reward — controlled-actor contribution and idempotent reward settlement before loot/XP integration.
- `GAME-AI-XD-04` / P1 / EVENT/ENCOUNTER + coordinator — durable world-shared encounter occurrence/eligibility ownership before world-shared boss integration.
- `GAME-AI-XD-05` / P1 / shared resource-limit owner + coordinator — measured hard maxima before executable GAME-AI implementation acceptance.
- `GAME-AI-XD-06` / P2 / ANL — durable AI-decision analytics/forensics schema/retention/privacy before production retention.

Every finding has `worker_action: REPORT_ONLY`; no target-owner file is modified.

## Validation

### Focused

- branch/trusted-base ownership inspection: **PASS**.
- exact changed-file allowlist before final metadata freeze: **PASS** — exactly three worker-C paths.
- coordinator-only/sibling path review: **PASS** — none changed.
- domain prompt + parallel work-allocation handoff review: **PASS after recorded metadata repairs**.
- live-main drift and sibling overlap review: **PASS for worker handoff** — no merged material dependency drift; sibling proposals remain draft/noncanonical.
- initial PR-metadata preflight run `31846830146`: **FAIL / METADATA_ONLY** before checkout, correctly repaired and not counted as final validation.
- same historical head Merge authority audit run `31846830143`: **PASS**.

### Component/integration

- command/run: `NOT_APPLICABLE` for Rust/runtime component testing — architecture/contracts-only documentation paths changed; no Rust/workspace implementation path is affected.
- required repository validation: current Build/Test Matrix always-required governance, dependency review, CodeQL and aggregate Merge Gate on exact final head.
- result: external exact-head PR/check evidence after this final commit exists.

### E2E

- scenario: `NOT_APPLICABLE` — paper-only architecture; no executable runtime behavior changed.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: recorded externally on PR #272 after this commit exists; a commit cannot contain its own SHA.
- trigger source: draft PR synchronize from this final task-metadata commit.
- workflow/run/job: external immutable PR/check evidence.
- runner assignment: external PR/check evidence.
- classification: documentation-only always-required merge-gate set.
- result: external PR/check evidence.

## Self-review

- exact head: recorded externally on PR #272 after this final commit exists.
- method/reviewer: DOMAIN ARCHITECTURE DESIGN AGENT / worker C.
- material findings repaired before final freeze: dynamic population/ecology boundary; literal handoff markers; PR summary/scope metadata; complete domain-prompt handoff fields; structured cross-domain records; Build/Test Matrix docs-only validation classification.
- semantic content review: bounded owner/FSM/path/spawn/recovery/controlled-actor/reward/overload/Reference-Evolved invariants reviewed with no remaining known material worker finding.
- verdict: **READY FOR COORDINATOR AUDIT**, subject to exact-final-head PR checks and external full-diff evidence.

## Independent review

- required: `YES` — issue #261 and work allocation require Architecture Coordinator audit.
- exact head: coordinator must pin the final PR #272 head.
- method/auditor: Architecture Coordinator/Auditor via draft PR #272.
- material findings: pending coordinator audit.
- verdict: pending coordinator audit.

## PR and closeout

- draft PR: #272.
- changed-file review: expected exactly the three worker-C allocated paths; final exact-head set must remain identical after this task-metadata-only commit.
- unresolved review threads: none existed at the latest worker inspection; final exact-head query is external handoff evidence.
- related/superseded PRs: no overlapping canonical GAME-AI PR; sibling domain PRs are noncanonical and disjoint.
- protected auto-merge: FORBIDDEN / not requested.
- merge commit/result: NOT PERFORMED / NOT PERMITTED FOR WORKER.
- issue #261 close: NOT PERFORMED.
- ownership release: NOT PERFORMED; task remains under `active/` for coordinator audit.
- lifecycle closeout: NOT PERFORMED / FORBIDDEN FOR THIS WORKER.

## Context checkpoint

```yaml
last_progress: GAME-AI semantic package and final worker handoff metadata frozen; exact final head and checks are recorded externally on PR #272
status: ready
branch: docs/arch-c-game-ai
head_sha: 1f5f21d28d56aef00dc781a196a50ac4dc0dc883
pr: 272
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending-final-head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: coordinator audit after exact-head worker evidence is attached
blocker: null
next_action: architecture coordinator audits exact PR #272 head after worker attaches immutable exact-head CI/self-review evidence; worker must not merge or lifecycle-close
```

**MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY**

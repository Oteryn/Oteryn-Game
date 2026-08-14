# OTV2-20260815-game-ai-architecture

```yaml
task_id: OTV2-20260815-game-ai-architecture
title: Design GAME-AI-01 creature AI, spawn and pathfinding architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/arch-c-game-ai
pr: 272
base_sha: 088b46638ac014cd7928d6b0b75cee44902fe22c
head_sha: cc2f258412bdc9868f9764c8e3c57921f9fa1f8c
final_head_sha: null
final_head_frozen_at: null
owner: DOMAIN ARCHITECTURE DESIGN AGENT / worker C
created_at: 2026-08-15T00:18:00+02:00
updated_at: 2026-08-15T00:32:00+02:00
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
blocks: []
cross_repository_coordination_id: OTV2-GAME-AI-01
external_repositories: []
```

## Outcome

Produce a bounded noncanonical worker proposal for `GAME-AI-01` that:

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

- Issue `#261` allocates worker C to `GAME-AI-01` and forbids runtime implementation, foreign-domain edits, global overlays, AI library selection without evidence and self-merge/lifecycle closeout.
- Coordinator activation comment binds the lane to trusted base `main@088b46638ac014cd7928d6b0b75cee44902fe22c` on `docs/arch-c-game-ai`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` assigns channel-local creatures/spawns/AI to `ChannelRuntime`, instance-local creatures to `InstanceRuntime`, and auxiliary pathfinding/AI work to proposal + owner revalidation semantics.
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` requires explicit value-source multiplicity/eligibility and forbids treating runtime Channel locality as automatic reward repeatability.
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` supplies immutable content/World Bundle provenance and proposal-only bounded script semantics.
- `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` supplies revision binding, deterministic RNG/tie-break/order/replay/state-hash semantics.
- accepted partial GAME-ABILITY baselines retain targeting/cast/cooldown/effect ownership; whole-gate GAME-ABILITY remains sibling/coordinator work.
- GAME-VISION first Reference target/evidence discipline forbids guessing unknown target behavior from OTS/current-live implementation similarity.

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
- [x] Required worker handoff markers `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS` are present in task and PR metadata.
- [x] Cross-domain findings are `REPORT_ONLY` and no foreign architecture file is edited.
- [x] Draft PR `#272` created and changed-file scope verified as exactly the three worker-owned paths.
- [x] Initial exact-head CI metadata failure classified without claiming repository-validation PASS.
- [ ] Fresh exact-head repository governance/architecture CI result inspected after PR metadata repair.
- [ ] Final exact-head self-review/evidence recorded in immutable PR evidence after this metadata-only synchronization commit.

## Excluded scope

This worker task MUST NOT:

- implement runtime/content code;
- select a concrete Rust AI/pathfinding/behavior-tree library;
- select benchmark-sensitive numeric resource limits without evidence;
- edit global architecture registers/status/backlogs/programme allocation;
- edit GAME-ABILITY, GAME-INTERACTION, GAME-ITEM, DUR, ANL or other foreign contracts;
- claim exact Reference behavior where evidence is missing;
- claim `DecisionStatus=ACCEPTED`, canonicality, lifecycle closeout or runtime implementation;
- merge, enable auto-merge or archive this task record.

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
2. Verified live `main` had advanced by one coordinator closeout/bookkeeping commit unrelated to GAME-AI semantics; worker lane intentionally remained on the explicitly activated trusted base rather than silently rebasing.
3. Verified no active sibling architecture PR overlapped the GAME-AI owned paths at startup.
4. Resolved terminology collision: ADR-0006 Game Intelligence / Investigation AI is read-only external analysis; GAME-AI runtime behavior remains inside authoritative simulation.
5. Selected typed bounded FSM for v1 semantic behavior; runtime behavior tree is deferred and direct authoritative script mutation rejected.
6. Defined pathfinding as bounded auxiliary proposal work with deterministic search profile and owner revalidation.
7. Defined immutable actor/template/spawn provenance, explicit spawn recovery classes and GAME-CHANNEL multiplicity gates.
8. Self-review found the horizon's dynamic population/ecology requirement was not explicit enough in the first candidate draft; repaired on semantic checkpoint `b4924278f543325007356b2e2b52ca0d0d0fc966` by requiring immutable/versioned bounded population policy and prohibiting direct analytics-to-runtime feedback.
9. Work-allocation self-review found the required literal handoff markers were semantically present but not canonically named; repaired on `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c`.
10. Exact-head Agent governance run `31846830146` on `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c` failed before checkout because the PR body lacked required `## Summary` and `## Scope` headings. Merge authority audit `31846830143` passed. The PR body was repaired without changing repository content; this metadata-only task commit intentionally creates a fresh `synchronize` event so governance can validate the exact new head with corrected PR metadata.

## CROSS_DOMAIN_FINDINGS

Disposition for every item below: `CROSS_DOMAIN_FINDING / REPORT_ONLY`.

- GAME-ABILITY: typed AI action-intent/rejection boundary should be preserved/confirmed by whole-gate reconciliation.
- GAME-INTERACTION: route invalidation and door/teleport/environment facts need a typed normalized boundary.
- GAME-ITEM/DUR-03/reward: exact controlled-actor contribution and idempotent reward settlement remain downstream.
- EVENT/ENCOUNTER: durable world-shared boss/event occurrence/eligibility needs a named owner where required.
- RESOURCE LIMITS: concrete AI/path/spawn maxima must be registered before implementation acceptance.
- ANL: durable AI-decision evidence, if required, remains ANL schema/retention/privacy work.

## Validation

### Focused

- command/run: direct repository inspection through GitHub connector against `docs/arch-c-game-ai` and trusted base
- result: `PASS` for branch/base ownership before writes
- command/run: PR `#272` changed-file enumeration
- result: `PASS` — exactly three worker-owned paths; zero forbidden/global/foreign paths
- command/run: full work-allocation handoff-minimum review
- result: first pass found missing canonical literal marker names; repaired before exact-head validation
- command/run: Agent governance PR-metadata preflight, run `31846830146`, head `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c`
- result: `FAIL / METADATA_ONLY` before checkout — missing `## Summary` and `## Scope`; PR body repaired without claiming repository validation
- command/run: Merge authority audit, run `31846830143`, same head
- result: `PASS`

### Component/integration

- command/run: `python tools/agents/validate_governance.py`
- result: pending fresh exact-head GitHub Actions result; initial run did not reach this step
- command/run: `cargo run --locked -p architecture-check`
- result: pending exact-head repository validation evidence; local execution unavailable because the sandbox cannot resolve `github.com` for checkout

### E2E

- scenario: `NOT_APPLICABLE` — paper-only architecture; no executable runtime behavior changed
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending immutable PR/check evidence; this commit cannot contain its own SHA
- trigger source: fresh `pull_request synchronize` from this metadata-only task update after PR-body repair
- prior stale/failed run: Agent governance `31846830146` on `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c` — metadata-only failure before checkout
- workflow/run/job: pending fresh exact-head generation
- runner assignment: pending
- classification: repository-hosted validation required because local checkout is network-blocked
- result: pending

## Self-review

- semantic checkpoint: `b4924278f543325007356b2e2b52ca0d0d0fc966`
- work-allocation checkpoint: `cc2f258412bdc9868f9764c8e3c57921f9fa1f8c`
- method/reviewer: implementing DOMAIN ARCHITECTURE DESIGN AGENT / worker C
- material findings: (1) dynamic population/ecology policy was implicit; repaired; (2) canonical handoff marker names were absent; repaired; (3) governance-required PR body headings were absent; repaired in PR metadata before this synchronization commit
- verdict: semantic and metadata repairs complete; exact final-head diff/CI must still be inspected and recorded in immutable PR evidence

## Independent review

- required: `YES` — Architecture Coordinator audit is mandatory by issue/work-allocation merge authority
- exact head: to be supplied by immutable PR/check evidence after this commit
- method/auditor: Architecture Coordinator via draft PR `#272`
- material findings: pending coordinator audit
- verdict: pending coordinator audit

## PR and closeout

- changed-file review: `PASS` — exactly three allocated paths in draft PR `#272`
- unresolved review threads: to be checked on final head before worker handoff
- related/superseded PRs: none identified at startup for this worker scope
- protected auto-merge: prohibited/not requested
- merge commit/result: `NOT_PERMITTED_FOR_WORKER`
- ownership release: `NOT_PERMITTED_FOR_WORKER`; coordinator lifecycle only

## Context checkpoint

```yaml
last_progress: PR metadata governance failure classified and body repaired; fresh exact-head synchronize triggered by task-only metadata commit
status: validating
branch: docs/arch-c-game-ai
head_sha: cc2f258412bdc9868f9764c8e3c57921f9fa1f8c
pr: 272
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull-request-synchronize-after-pr-body-repair
ci_check_generation: pending-new-head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: local checkout unavailable; fresh exact-head GitHub Actions validation required
next_action: inspect the new exact PR head/diff and fresh Actions generation, then record immutable worker handoff evidence without merging or lifecycle closeout
```

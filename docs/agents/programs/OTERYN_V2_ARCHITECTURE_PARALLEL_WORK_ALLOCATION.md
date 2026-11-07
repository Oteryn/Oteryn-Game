# Oteryn-v2 Architecture Parallel Work Allocation

- Status: coordinator-owned work allocation
- Programme issue: #258
- Trusted allocation base: `main@85acd19e976943ee42b5c004ebd0ae1c40cc5fff`
- Orchestration policy: `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**
- Runtime/DDL/Platform/production authority: **NONE**

## Purpose

Allocate the first parallel architecture wave into non-overlapping domain lanes. This document allocates proposal/evidence work; it does not accept the worker proposals and does not supersede `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

The canonical programme priority remains the Reference continuity/provenance lane (Agent A). Agents B–F may reduce future architecture latency in parallel, but their proposals are noncanonical until coordinator audit and accepted merge.

## Start rule

Worker branches MUST be created only after the orchestration-policy delivery from issue #258 is merged to `main`. Each branch starts from that post-policy trusted main, not from the pre-policy allocation base above.

Before a worker starts:

1. coordinator verifies live `main` and open ownership;
2. coordinator creates or verifies the exact branch named below;
3. worker reads the merged orchestration policy and domain prompt;
4. worker creates its active task with exact `owned_paths` derived from this allocation;
5. worker opens only a **draft PR** and ends at coordinator audit handoff.

## Coordinator lane

```yaml
role: ARCHITECTURE_COORDINATOR_AUDITOR
issue: 258
branch: docs/multi-agent-architecture-orchestration
merge_authority: true
worker_merge_authority: false
```

Coordinator-only paths are never owned by A–F unless the coordinator creates a later exact delegation:

- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`
- `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`
- `docs/architecture/README.md`
- global/foundation successor handoff reports
- `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`
- this work-allocation file
- `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`
- root/nested agent governance files

## Agent A — Reference evidence continuity/provenance

```yaml
worker_id: A
issue: 259
domain: REFERENCE_EVIDENCE / ABILITY_COMBAT
branch: docs/arch-a-reference-continuity
priority: CANONICAL_NEXT_ACTION
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-reference-ability-continuity-provenance.md`;
- new bounded evidence under `docs/agents/evidence/OTV2-*-reference-ability-continuity-*.md`;
- `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` **only when admissible evidence actually changes one of the four existing cases**;
- `docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md` **only to mirror an evidence-classification change for those existing cases**.

Forbidden / non-owned:

- new mechanic cases;
- physical catalogue schema/serializer/fixture runner;
- GAME-ABILITY whole-gate candidate files owned by B;
- all coordinator-only surfaces.

Dependency/interaction:

- A is the canonical priority lane.
- If A changes an evidence classification, B must re-read that merged state before final integration if B references those cases.
- Insufficient evidence is a valid result; preserve `UNKNOWN/PENDING` rather than manufacture continuity.

## Agent B — GAME-ABILITY whole-gate gap/candidate closure

```yaml
worker_id: B
issue: 260
domain: GAME-ABILITY-01
branch: docs/arch-b-game-ability-gap
priority: PARALLEL_PROPOSAL
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-game-ability-whole-gate-gap.md`;
- `docs/architecture/GAME-ABILITY-01_WHOLE_GATE_GAP_ANALYSIS.md`;
- optional `docs/architecture/GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md` if evidence supports a coherent candidate.

Forbidden / non-owned:

- existing accepted GAME-ABILITY partial-baseline files merely for restatement;
- `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`;
- `GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md`;
- all coordinator-only surfaces.

Dependency/interaction:

- consumes accepted partial baselines without reopening them;
- reports AI/interaction/item/SIM/client/evidence gaps as cross-domain findings;
- must reconcile any A merge that materially changes the four existing evidence cases before coordinator acceptance.

## Agent C — GAME-AI-01

```yaml
worker_id: C
issue: 261
domain: GAME-AI-01
branch: docs/arch-c-game-ai
priority: PARALLEL_PROPOSAL
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-game-ai-architecture.md`;
- `docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md`;
- optional `docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md`.

Forbidden / non-owned:

- GAME-ABILITY formulas/effect contracts;
- GAME-INTERACTION contracts;
- item/value/content owners;
- global overlays.

Dependency/interaction:

- consumes FND-03, GAME-CHANNEL, SIM-DETERMINISM, DUR-04 scripting and accepted GAME-ABILITY interfaces;
- cross-domain combat/loot/interaction findings are REPORT_ONLY.

## Agent D — GAME-INTERACTION-01

```yaml
worker_id: D
issue: 262
domain: GAME-INTERACTION-01
branch: docs/arch-d-game-interaction
priority: PARALLEL_PROPOSAL
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-game-interaction-architecture.md`;
- `docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_ANALYSIS.md`;
- optional `docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_CONTRACT_CANDIDATE.md`.

Forbidden / non-owned:

- item legality/location semantics;
- GAME-ABILITY combat semantics;
- AI behavior semantics;
- global overlays.

Dependency/interaction:

- consumes DUR-04 script capability, GAME-ITEM/DUR-03 authority, FND-03/GAME-CHANNEL scope and SIM ordering/time/RNG;
- missing upstream behavior becomes a typed delegation/cross-domain finding, never a hidden generic interaction escape hatch.

## Agent E — ALPHA-CLIENT-01

```yaml
worker_id: E
issue: 263
domain: ALPHA-CLIENT-01
branch: docs/arch-e-alpha-client
priority: PARALLEL_PROPOSAL
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-alpha-client-architecture.md`;
- `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md`;
- optional `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md`.

Forbidden / non-owned:

- protocol/server authority contracts;
- gameplay legality contracts;
- global overlays;
- executable client implementation.

Dependency/interaction:

- consumes ADR-0011, protocol/session boundaries, transport non-readiness, server-authoritative gameplay and DUR-04 client-safe projection;
- networking/gameplay/security gaps remain cross-domain findings.

## Agent F — ANL-02 / ANL-03

```yaml
worker_id: F
issue: 264
domain: ANL-02 + ANL-03
branch: docs/arch-f-analytics-integrity
priority: PARALLEL_PROPOSAL
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
```

Owned scope:

- own active task `docs/agents/tasks/active/OTV2-*-analytics-integrity-architecture.md`;
- `docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md`;
- `docs/architecture/ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md`;
- optional bounded candidate-contract files in ANL-02/ANL-03 namespaces.

Forbidden / non-owned:

- authoritative prevention/value mutation policy owned by DUR-03 or gameplay domains;
- GM/sanction policy;
- global overlays;
- runtime detectors/warehouse/vendor deployment.

Dependency/interaction:

- consumes ADR-0006, ANL-01, DUR-03, GAME-CHANNEL and SIM evidence boundaries;
- analytics stays observational/investigative and cannot autonomously ban, balance, rollback, mutate or deploy.

## Cross-worker path matrix

| Path family | A | B | C | D | E | F | Coordinator |
|---|---:|---:|---:|---:|---:|---:|---:|
| Reference four-case manifest/evidence | conditional YES | NO | NO | NO | NO | NO | audit/integrate |
| GAME-ABILITY whole-gate candidate | NO | YES | NO | NO | NO | NO | audit/integrate |
| GAME-AI candidate | NO | NO | YES | NO | NO | NO | audit/integrate |
| GAME-INTERACTION candidate | NO | NO | NO | YES | NO | NO | audit/integrate |
| ALPHA-CLIENT candidate | NO | NO | NO | NO | YES | NO | audit/integrate |
| ANL-02/03 candidate | NO | NO | NO | NO | NO | YES | audit/integrate |
| Global/current overlays | NO | NO | NO | NO | NO | NO | YES |
| Work allocation/governance | NO | NO | NO | NO | NO | NO | YES |

## Worker acceptance/handoff minimum

Every worker PR must:

- remain draft;
- change only declared owned paths;
- contain `MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`;
- contain `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS`;
- use canonical architecture status values where applicable;
- complete exact-head self-review and ordinary repository CI;
- report material findings/repairs;
- end with coordinator audit as the only next action;
- not archive its own task.

## Integration queue and ordering

Coordinator processes ready worker PRs independently of finish time:

1. verify exact worker head, scope and sibling drift;
2. audit dependency/cross-domain implications;
3. classify `ACCEPT`, `REWORK`, `BLOCKED`, or `SUPERSEDED`;
4. return `REWORK` findings to the worker branch;
5. when accepted, satisfy any independent-review requirements without unauthorized Codex use;
6. re-run exact-head CI after any repair/head move;
7. merge one worker at a time;
8. re-evaluate remaining worker branches against the new main;
9. archive/release the merged worker task;
10. update canonical overlays only when merged truth changes them.

Agent A's priority means its evidence state controls claims about the four existing cases. It does **not** mean B–F must remain idle while A researches.

## First-wave completion condition

The first wave is complete only when every allocated issue is one of:

- merged and lifecycle-closed by coordinator;
- explicitly `BLOCKED` with a durable blocker;
- explicitly `SUPERSEDED`/closed by coordinator with rationale.

A collection of open draft PRs is not programme completion.

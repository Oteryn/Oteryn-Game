# ADR-0015: GameNode implementation shape is not yet frozen

- Status: Accepted clarification
- Date: 2026-08-11
- Decision ID: `GAMENODE-SHAPE-01`
- Supersedes: only the **binding interpretation** of (a) `ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` statements that the first GameNode "should be a domain-modular monolith" and that the programme should "build a modular-monolith GameNode", and (b) `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` Section 3 wording that the first GameNode implementation "should remain a domain-modular monolith until real deployment/security/data/failure boundaries justify separation". In all three locations the modular-monolith shape is a **nonbinding preferred starting hypothesis**, not a frozen internal module/crate decomposition or a mandate against independently deployed adjacent services with separate authority.
- Preserves: **ADR-0009's accepted GameNode/process/container boundary** — one `GameNode` is the logical runtime identity of one game-server process identified by `NodeId`; one GameNode process may host multiple `ChannelRuntime`s; an external orchestrator owns process/container lifecycle. It also preserves FND-03 authoritative runtime ownership, domain/transport/persistence separation, bounded queues, one logical authoritative writer per scope, and the rule against speculative empty services/crates.
- Does not authorize: changing the ADR-0009 meaning of `GameNode`, representing one GameNode as several processes/services, runtime implementation, microservice decomposition, Kubernetes, deployment changes, or production activation.

## Problem

The 2026-08-10 architecture review and current-status overlay correctly identify a modular monolith as the most conservative **working hypothesis** for the first GameNode, but their wording can be read as a binding decomposition decision.

That would freeze internal module/crate boundaries and surrounding service placement before the implementation programme has measured real scaling, failure-isolation, security-boundary and data-ownership pressures. Oteryn needs to preserve the recommendation without turning it into premature architecture authority.

At the same time, ADR-0009 already freezes a narrower and important process identity invariant: a `GameNode` is one game-server process. This clarification must not accidentally reopen that accepted boundary.

## Constraints

Any future GameNode implementation-shape decision must preserve:

- ADR-0009: one `GameNode` equals one logical game-server process identity with one `NodeId`; several separately deployed processes/services do not collectively become one GameNode unless a later ADR explicitly supersedes that named ADR-0009 scope;
- gameplay/domain logic independent from Tokio sockets, Protobuf wire layout, PostgreSQL adapters and client UI/renderer state;
- one logical authoritative mutation owner per ChannelRuntime/InstanceRuntime scope;
- explicit world-shared authority instead of process-global mutable caches;
- bounded queues, bounded auxiliary work and stale-result rejection;
- no speculative empty services or crates merely to mirror diagrams;
- independently deployed adjacent services only when justified by a real ownership, security, scaling or failure boundary.

## Options considered

### Option A — Freeze a domain-modular monolith inside the one-process GameNode now

Benefits:

- simplest initial deployment and debugging model;
- lowest distributed-systems overhead;
- aligns with the current small vertical-slice delivery strategy.

Risks:

- prematurely turns a reasonable internal-decomposition hypothesis into architecture authority;
- may later resist a justified separate service for a genuinely separate authority/security/data/failure boundary;
- lacks measured evidence from an actual native GameNode.

Disposition: **not accepted as a binding decomposition decision yet**.

### Option B — Freeze separately deployed services around or instead of the GameNode now

Benefits:

- can provide explicit process isolation and independent scaling for genuinely separate authorities.

Risks:

- introduces distributed coordination, deployment and observability cost before evidence;
- increases failure modes and operational surface;
- risks creating empty service boundaries without real independent ownership;
- if interpreted as several processes collectively constituting one GameNode, it would conflict with ADR-0009.

Disposition: **rejected for the current stage**.

### Option C — Preserve ADR-0009's one-process GameNode; keep internal decomposition and adjacent-service boundaries open

Benefits:

- preserves the accepted GameNode identity/process model;
- preserves the simplest likely implementation path without freezing internal crate/module structure;
- lets vertical-slice evidence drive whether a distinct authority should become a separately deployed service **outside** the GameNode boundary;
- keeps domain boundaries independent from deployment topology;
- avoids both premature microservices and premature internal topology lock-in.

Costs:

- implementation planning must carry an explicit later decomposition/service-boundary decision when evidence makes it necessary;
- some internal and adjacent-service details remain intentionally unresolved.

Disposition: **selected**.

## Decision

ADR-0009 remains authoritative that **one GameNode is one game-server process**. This ADR does not reopen or weaken that identity/process boundary.

What is **not yet frozen** is:

1. the internal module/crate decomposition within that GameNode process; and
2. whether genuinely separate authorities or operational domains should later run as independently deployed services **adjacent to, not collectively constituting, the GameNode**.

The current programme recommendation is:

> Start implementation discovery from a domain-modular monolith inside the ADR-0009 one-process GameNode unless evidence demonstrates a stronger separate boundary.

This recommendation is **NONBINDING**. It must not be treated as accepted internal decomposition merely because either the 2026-08-10 refinement or the canonical current-status overlay uses prescriptive wording.

A future decision that wants several processes/services to collectively constitute one GameNode must explicitly supersede the relevant ADR-0009 clauses; this ADR does not authorize such a change.

Before any implementation task freezes internal GameNode decomposition or introduces an independently deployed adjacent service, its owning decision must explicitly answer:

1. Why must the boundary be decided at that point rather than remaining an internal implementation detail?
2. Which concrete downstream work is blocked without the decision?
3. Which alternatives were compared, at minimum in-process module/crate separation versus a separately deployed adjacent service where the semantics permit it?
4. Which security, data-ownership, independent-scaling, failure-domain or operational boundaries justify a separate process/service?
5. What measured evidence exists for CPU, memory, latency, noisy-neighbor and failure-recovery behavior?
6. Does the proposed shape preserve ADR-0009's one-process GameNode identity? If not, which exact ADR-0009 scope is being deliberately superseded and why?
7. What evidence would supersede the selected shape later?

## Current implementation guidance

Until that later gate exists:

- implementation tasks may use a modular-monolith spike or vertical slice **inside one ADR-0009 GameNode process** as the default exploration path;
- they may not claim that internal decomposition as globally frozen architecture;
- a new crate or independently deployed service still requires an immediate consumer and a real boundary;
- an independently deployed service is not another part of the same GameNode process identity merely because it supports gameplay;
- gameplay/domain semantics remain independent from module/service placement;
- the existing ADR-0009 process/container/orchestrator semantics stay binding.

## Decision timing

- **Must internal GameNode decomposition or adjacent-service placement be decided now?** NO.
- **What is already decided?** ADR-0009's `GameNode = one game-server process` identity/process boundary.
- **What must be clarified now?** Only that the prior modular-monolith wording is a nonbinding recommendation about implementation decomposition, not authority to change ADR-0009 and not a frozen crate/module layout.
- **Blocked downstream work:** a task that wants to freeze internal GameNode decomposition, introduce an independently deployed adjacent service, or change the ADR-0009 process identity boundary.
- **Evidence to supersede/open the later gate:** representative vertical-slice and load/failure/security evidence showing where module or process boundaries materially improve correctness, isolation or operations.

## Consequence

`ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` and `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` remain useful programme/status guidance, but for GameNode internal decomposition their prescriptive modular-monolith wording is nonbinding. ADR-0009 remains authoritative for the process identity boundary: **one GameNode is one game-server process** unless a future dedicated ADR explicitly supersedes that named scope.

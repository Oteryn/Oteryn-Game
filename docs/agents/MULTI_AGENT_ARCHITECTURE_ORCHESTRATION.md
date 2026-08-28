# Multi-Agent Architecture Orchestration

- Status: coordinator-owned execution policy
- Applies to: Oteryn-v2 architecture/research programmes explicitly allocated under this policy
- Coordination issue: #258
- Runtime/DDL/Platform/production authority: **NONE**

## Purpose

Oteryn-v2 may use multiple domain agents in parallel for architecture research and proposal work without allowing parallel writers to create conflicting canonical truth.

The operating model is:

```text
parallel discovery + domain proposal
              ↓
      draft worker PRs
              ↓
coordinator/auditor integration queue
              ↓
 ACCEPT / REWORK / BLOCKED / SUPERSEDED
              ↓
serial canonicalization + merge + closeout
```

The deliberate constraint is **parallel design, serial canonicalization**.

## Roles

### DOMAIN ARCHITECTURE DESIGN AGENT

A domain worker owns one bounded issue/branch/path set. It may:

- read canonical architecture and live repository state;
- perform bounded primary-source research;
- create/update its assigned task record;
- create new analysis, evidence or candidate-contract artifacts within assigned paths;
- perform self-review and ordinary repository validation;
- open and update its own **draft PR**;
- repair findings returned by the coordinator/auditor.

A domain worker may **not**:

- merge, rebase-merge, squash-merge or enable auto-merge for its own PR;
- lifecycle-close/archive its own architecture task after delivery;
- represent its proposal as globally/canonically `ACCEPTED` unless the acceptance is already an upstream fact it is merely consuming;
- edit coordinator-only current-status/register/horizon/handoff/work-allocation surfaces;
- silently modify another domain's accepted contract to resolve a cross-domain gap;
- mark a PR ready when doing so would trigger owner-funded Codex/OpenAI without exact owner authorization;
- invoke Codex/OpenAI/API/owner-funded AI under standing or inferred permission;
- expand runtime/client/server/protocol/DDL/Platform/production authority.

Every worker PR must contain:

```text
MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
```

### ARCHITECTURE COORDINATOR / AUDITOR

The coordinator is the single integration and merge authority for work allocated under this policy. It must:

- verify live main, task ownership and exact worker head;
- inspect the **full diff**, not only the PR summary;
- challenge architecture completeness, authority boundaries, failure modes and unsupported claims;
- compare worker PRs against each other for overlap/conflict;
- classify each worker PR as `ACCEPT`, `REWORK`, `BLOCKED` or `SUPERSEDED`;
- return domain-specific findings to the worker rather than rewriting large worker-owned proposals itself;
- decide dependency-aware integration/merge order;
- perform final exact-head audit and applicable independent-review gating;
- resolve integration conflicts;
- merge accepted PRs serially;
- archive/release worker ownership after merge;
- reconcile canonical programme overlays after accepted state actually changes.

The coordinator may not use this role to bypass owner-funded AI restrictions, runtime/production authority or repository protections.

## Governance authority classification

This policy is not purely an authority reduction. It intentionally performs a **bounded merge-authority redistribution**:

1. each allocated domain worker loses the repository's general own-PR merge/auto-merge/lifecycle-close authority for that worker task; and
2. the Architecture Coordinator gains bounded authority to merge and lifecycle-close allocated worker PRs authored/owned by different domain agents.

Point 2 is a **merge-authority expansion** relative to the prior own-PR-only baseline. Under root governance, a policy delivery that introduces or materially widens this coordinator authority requires:

- explicit owner scope for the authority change; and
- a genuinely independent review tied to the exact final governance head before that governance delivery may merge.

A review on a superseded head does not satisfy that final-head gate after a repair. Once this policy is independently reviewed and merged, coordinator merges performed strictly inside the already-approved worker allocation are ordinary uses of existing programme authority; they are not fresh governance expansions unless this policy/allocation is materially widened again.

This authority expansion is limited to repository integration/closeout of allocated Oteryn-v2 architecture worker PRs. It does **not** expand repository allowlists and grants no runtime/client/server/protocol implementation, PostgreSQL DDL/migration, Platform, production, secret, protected-environment, live-data/session/account or cross-repository authority.

## Authority matrix

| Action | Domain worker | Coordinator |
|---|---:|---:|
| Research within assigned domain | YES | YES |
| Create assigned task/branch | YES | YES |
| Write assigned analysis/candidate artifacts | YES | YES |
| Open/update worker draft PR | YES | YES |
| Self-review worker diff | REQUIRED | MAY REPEAT |
| Cross-domain audit | REPORT FINDING | REQUIRED |
| Edit coordinator-only overlays | NO | YES |
| Mark proposal globally canonical/accepted | NO | YES, only with acceptance evidence |
| Merge worker PR | NO | YES |
| Archive/release worker task | NO | YES |
| Trigger Codex/OpenAI | only exact owner authorization | only exact owner authorization |
| Runtime/DDL/production action | NO without separate owner authority | NO without separate owner authority |

## Coordinator-only surfaces

Unless an exact coordinator delegation says otherwise, workers must not modify:

- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
- `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`;
- `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`;
- `docs/architecture/README.md`;
- global/foundation successor handoffs;
- the non-owning foundation programme checkpoint;
- this policy;
- the canonical multi-agent work allocation;
- the global coordinator prompt.

These files are intentionally centralized because they integrate state across many domains and are frequent merge-conflict/common-mode-error surfaces.

## Ownership and path locking

1. One worker task has one issue, branch, owned-path set and draft PR.
2. `owned_paths` are advisory locks; overlapping ownership is a real blocker until the coordinator resolves it.
3. Parallel workers must not own the same public contract.
4. A worker discovering a required edit outside its ownership records a `CROSS_DOMAIN_FINDING` instead of editing the foreign file.
5. Coordinator-only surfaces are never worker-owned.
6. If two proposals need the same eventual shared contract, neither worker unilaterally freezes it. The coordinator chooses an integration owner or a later bounded shared-contract task.

## Cross-domain finding contract

Use this minimum shape in worker analysis/PRs:

```yaml
cross_domain_finding:
  id: <stable-within-pr-id>
  observed_in_domain: <worker-domain>
  target_owner: <other-domain-or-coordinator>
  severity: P0|P1|P2|P3
  evidence: <exact contract/file/source>
  conflict_or_gap: <what is missing or contradictory>
  required_before: <downstream decision/work>
  worker_action: REPORT_ONLY
```

A cross-domain finding never grants permission to mutate the target owner's contract.

## Worker PR lifecycle

```text
issue allocated
-> branch from trusted main
-> active task record
-> bounded research/design
-> draft PR
-> focused validation
-> exact-head full-diff self-review
-> ordinary exact-head repository CI
-> INTEGRATION_READY
-> coordinator audit
```

`INTEGRATION_READY` is descriptive worker handoff wording, not an `ARCHITECTURE_STATUS_MODEL` value.

The worker stops its delivery authority at draft/integration-ready state. The coordinator continues:

```text
coordinator audit
-> ACCEPT | REWORK | BLOCKED | SUPERSEDED
-> optional exact owner-authorized independent/Codex review when required
-> final exact-head CI/review/thread/drift check
-> squash merge
-> lifecycle closeout/archive/ownership release
-> canonical overlay reconciliation
```

## Review and Codex policy

- Worker self-review is mandatory and is never independent review.
- A coordinator audit is independent only if the coordinator did not materially author the worker proposal being audited.
- If the coordinator substantially rewrites a worker proposal, it becomes a co-author and must not mislabel its own final review as independent.
- Governance changes that introduce or expand coordinator merge authority require an independent exact-head review under root policy before merge.
- Codex remains optional and owner-funded. No worker or coordinator may trigger it without exact authorization for that PR/use.
- Marking a draft ready counts as a Codex-triggering action when repository automation is configured that way.
- Authorization for one PR never carries to another PR or a second review invocation.
- If a repair moves the head, all exact-head validation/review evidence for the superseded head is stale.

## Status discipline

Workers must use `ARCHITECTURE_STATUS_MODEL.md` canonical axes where status is presented:

- `DecisionStatus`: `PROPOSED`, `CANDIDATE`, `ACCEPTED`, `SUPERSEDED`;
- `DeliveryStatus`: `PLANNED`, `OPEN`, `IN_REVIEW`, `MERGED`, `LIFECYCLE_CLOSED`;
- `ImplementationStatus`: `NOT_STARTED`, `EXPERIMENTAL`, `IMPLEMENTED`, `PROVEN`, `PRODUCTION_ENABLED`.

A worker proposal normally remains `CANDIDATE` or `PROPOSED` for new whole-gate semantics until coordinator/owner acceptance. Accepted upstream sub-baselines may be described as accepted **for their declared scopes** without promoting the whole gate.

## Dependency-aware integration

The coordinator merges serially even when workers design in parallel.

Before each merge:

1. recompute worker branch drift against live main;
2. inspect already-merged sibling work for new dependencies/conflicts;
3. require the worker to rebase/reconcile when its assumptions changed;
4. invalidate stale exact-head review/CI after any head move;
5. merge only when the resulting head remains integration-safe.

Completion order never overrides dependency order.

## First-wave policy

The first wave is registered in `docs/agents/programs/OTERYN_V2_ARCHITECTURE_PARALLEL_WORK_ALLOCATION.md` and issues #259–#264.

- Agent A remains the **canonical priority lane** because current programme truth selects target-continuity + provenance-clearance for the four existing ABILITY_COMBAT cases.
- Agents B–F are **owner-authorized parallel proposal lanes**. They may reduce future architecture latency, but their work does not supersede the canonical next action and cannot become canonical merely by finishing first.
- The coordinator may merge a B–F analysis/candidate package before A only when it is truly disjoint, does not change canonical programme priority, and passes full dependency/cross-domain audit.

## No background execution assumption

This policy coordinates multiple agents when multiple agent sessions/runners are actually launched. It does not imply that one chat invocation creates hidden background workers. Repository issues, branches, task records and PRs are the durable coordination mechanism.

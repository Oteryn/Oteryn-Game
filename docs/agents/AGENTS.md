# Agent workspace rules

These rules govern `docs/agents/**`.

## Source of truth

- Individual active task records and live PR/CI state are authoritative for ownership and progress.
- Shared indexes are coordination aids and may be stale.
- Architecture decisions belong under `docs/architecture/`; public integration contracts belong under `docs/contracts/` when introduced.
- Evidence belongs under `docs/agents/evidence/` or immutable workflow artifacts, not in chat.

## Task records

Use `tasks/TASK_TEMPLATE.md`. Every substantial task must record:

- unique `OTV2-*` ID and task mode;
- branch, PR and exact head;
- owned paths and public contracts;
- dependencies, blockers and cross-repository coordination IDs;
- acceptance criteria and excluded scope;
- focused/component/E2E/exact-head validation;
- audit result and unresolved findings;
- compact context checkpoint with exactly one next action;
- execution-budget counters when applicable.

Do not edit another active task except for an explicitly coordinated ownership correction. Archive only after terminal completion/merge and ownership release.

## Multi-agent architecture programmes

When an architecture/research issue is explicitly allocated under `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`:

- read that policy and `docs/agents/programs/OTERYN_V2_ARCHITECTURE_PARALLEL_WORK_ALLOCATION.md` before mutation;
- a DOMAIN ARCHITECTURE DESIGN AGENT is a worker with **draft-PR-only delivery authority**;
- the worker may research, design, self-review, run ordinary validation and repair findings only within its allocated paths;
- the worker MUST NOT merge/auto-merge, lifecycle-close/archive its own task, update coordinator-only global overlays or make its new proposal canonically accepted;
- the worker MUST NOT mark a draft ready when that transition triggers owner-funded Codex/OpenAI unless the owner explicitly authorizes that exact PR/use;
- every worker PR must state `MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`;
- cross-domain gaps are reported as `CROSS_DOMAIN_FINDING` with `worker_action: REPORT_ONLY` rather than repaired by editing another owner's contract;
- the Architecture Coordinator/Auditor is the only programme role allowed to integrate, merge, archive/release worker ownership and reconcile canonical programme overlays;
- worker completion order never overrides dependency-aware coordinator integration order.

For these allocated worker tasks, the general own-PR merge authority is redistributed deliberately: worker authority is narrowed, while the coordinator receives bounded authority to merge and close out allocated worker PRs authored by other domain agents. That coordinator-side cross-worker merge/closeout authority is a **merge-authority expansion** relative to the repository's general own-PR baseline.

Therefore any governance change that introduces or materially widens this multi-agent coordinator authority requires explicit owner scope and a genuinely independent exact-head review before the governance change itself may merge. Once the reviewed governance policy is merged, ordinary coordinator integration of worker PRs within the already-approved allocation uses existing policy authority and is not itself a new governance expansion unless the allocation/authority is widened again.

This authority redistribution does not grant the coordinator any runtime, DDL, Platform, production, secret, protected-environment or cross-repository authority.

## Architecture and contract changes

A task changing world/channel/instance ownership, protocol schemas, authentication/session binding, persistence guarantees, item transactions, houses, rulesets or cross-repository boundaries must update or add the corresponding ADR/contract in the same delivery programme.

For architecture review, continuation, decision or coordination work, also read and apply:

- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`;
- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for live foundation progression;
- `docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-07.md` for the owner-accepted review refinements and stale-status reconciliation until later canonical documents explicitly absorb them.

A registered future topic is not permission to freeze it early. Every material architecture decision must state whether it must be decided now, name the concrete downstream work it blocks, and state what evidence would justify superseding it later.

A task that supersedes only part of an older ADR must preserve the old ADR and identify the exact superseded clause or policy. Do not rewrite history merely to make an earlier document read as if the later decision had always existed.

An unmerged task document cannot authorize its own architecture, production access or cross-repository writes.

## Shared-document discipline

- Edit shared indexes and policies narrowly.
- Avoid mass reformatting.
- Preserve stable terminology: `WorldId`, `ChannelId`, `InstanceId`, `NodeId`, `GameSessionId`, `protocol-oteryn`.
- Do not reintroduce Canary as a target runtime or protocol adapter without a new owner-approved ADR that explicitly supersedes ADR-0001.
- Policies copied from another repository must be reviewed for technology, repository name, production, asset and cross-repository assumptions before adoption.

## Evidence vocabulary

- `PROVEN` — directly supported by exact repository/runtime evidence.
- `DERIVED` — reasoned from proven facts; inference is explicit.
- `UNKNOWN` — evidence is absent or stale.
- `CONFLICT` — credible sources disagree.

Never claim `PROVEN` based only on a plan, prompt, green unrelated workflow or historical result from another commit.

## Governance changes

Run `python tools/agents/validate_governance.py` and the `agent-governance` workflow. A governance change requires review of every referenced file and machine-readable contract. Safety reductions or authority expansions require explicit owner scope and independent review.

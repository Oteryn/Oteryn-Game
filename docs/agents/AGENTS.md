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

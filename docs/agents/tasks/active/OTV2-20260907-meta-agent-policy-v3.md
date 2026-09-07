# OTV2-20260907-meta-agent-policy-v3

```yaml
task_id: OTV2-20260907-meta-agent-policy-v3
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agents/r5-game-policy-review-repair-367
issue: 367
base_sha: a6f69427d663539c6a8e23f166e69147b66ec078
reconciled_main_sha: 15164c38a2775e45eaff4001fddddbabf4b63ab6
owner: R5 Game review-repair lane
owned_paths:
  - AGENTS.md
  - .github/workflows/agent-governance.yml
  - docs/agents/**
  - tools/agents/**
public_contracts:
  - Oteryn/Oteryn@5ed3f14400af450b5875c091e443da70f2d67ab9
```

## Outcome

Adopt central META agent policy v3 through an authenticated immutable binding, a small Game bootstrap and task-delta prompts. Retire the completed provider-copy enforcement from #237/#239 while preserving Game domain invariants and existing repository gates.

This candidate continues the same Issue #367 migration from PR #373, including the later evidence-only commit `7e70330e7eab43c9bb0acd82c00e2296bd6c46c5`, and repairs its independent R5 review findings on instruction head `ebe152af34bd003dd5c41b38eb550f10944be988`. The earlier branch remains untouched. The repair targets `main` because the existing governance and merge-gate workflows require that base; it is not a separate adoption programme. D4-only Issue #374 / PR #376 owns its disjoint twelve terminal task moves.

Protected main advanced during qualification. Its sixteen changed paths through `15164c38...` are disjoint from this candidate and are preserved byte-for-byte in the integration reconciliation. The R5 diff against that main contains no runtime/product edits; the measured instruction baseline remains the admission revision above.

## Acceptance

- Central binding, provider overlay and every lifecycle-reusable prompt validate against the pinned protected META revision.
- Actual root/nearest instruction delivery and representative domain, scope and stale-locator behaviors remain covered.
- Existing Game governance and repository-policy checks pass.
- `Agent governance` retains its check identity and runs the replacement consumer; required `game-gate`, repository protection and Merge Queue remain intact.
- Source/context volume is quantified without claiming token or API savings.

## Excluded scope

No runtime, Rust, protocol/schema, production, credentials, protection/ruleset, or external-repository mutation.

## Context checkpoint

Two fresh Work sessions, each requested as Sol/high without a conversation fork, completed the same four read-only triage cases against baseline `a6f69427...` and repaired instruction snapshot `1969c516...`. All eight decisions preserved documentation scope/ownership, denied injected authority, recognized merged #362 and preserved channel/session/persistence invariants. This is bounded decision screening, not implementation or production-runtime qualification. The compact evaluation records actual explicit source delivery, unavailable runtime attestation, unequal read inventories and prior writer-reported evidence separately. Exact-head hosted checks and independent review remain required before owner integration.

```yaml
status: validating
next_action: obtain exact-head qualification of the review-repair candidate; leave integration to the authorized owner
```

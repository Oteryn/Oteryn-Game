# OTV2-20260907-meta-agent-policy-v3

```yaml
task_id: OTV2-20260907-meta-agent-policy-v3
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agents/meta-policy-v3-game-367
issue: 367
base_sha: a6f69427d663539c6a8e23f166e69147b66ec078
owner: game-governance
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

## Acceptance

- Central binding, provider overlay and every lifecycle-reusable prompt validate against the pinned protected META revision.
- Actual root/nearest instruction delivery and representative domain, scope and stale-locator behaviors remain covered.
- Existing Game governance and repository-policy checks pass.
- `Agent governance` retains its existing required status and runs the replacement consumer.
- Source/context volume is quantified without claiming token or API savings.

## Excluded scope

No runtime, Rust, protocol/schema, production, credentials, protection/ruleset, or external-repository mutation.

## Context checkpoint

```yaml
status: validating
next_action: publish the reviewed local candidate, run exact-head GitHub CI and obtain independent review
```

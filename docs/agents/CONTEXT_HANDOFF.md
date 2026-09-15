# Context handoff

## Purpose

A substantial task must remain resumable from Git, its task checkpoint and live PR/CI state without access to the previous chat or worker process.

## When to checkpoint

Update the task `## Context checkpoint` after:

- a material discovery or architecture decision;
- a coherent patch or changed public contract;
- validation, audit or E2E state changes;
- a new failure hypothesis or repair;
- branch/head/PR/review/CI changes;
- a blocker, wait or ownership change;
- before deliberate sleep, long command, session rotation or context exhaustion.

## Required checkpoint content

Record:

```yaml
status:
branch:
head_sha:
pr:
owned_paths:
public_contracts:
last_progress:
validation_state:
audit_state:
e2e_state:
ci_generation:
run_ids:
counters:
blocker:
next_action:
```

`next_action` must be exactly one executable action. Do not use vague text such as `continue`, `check status`, `finish CI` or `resume work`.

## Resume procedure

1. Read trusted-base governance and the checkpoint.
2. Verify live default branch, task branch/head, PR, reviews, CI, ownership and dependencies.
3. Classify drift as expected, conflicting or stale.
4. Preserve original anti-stall/wait counters.
5. Execute the recorded safe next action when still valid.
6. Update the checkpoint before broad rediscovery when state changed.

## Evidence discipline

Use exact SHAs, PR/run identifiers and focused evidence references. Do not paste full logs, diffs or source files when immutable identifiers and concise findings are sufficient.

## Handover quality gate

A handover is invalid when it depends on chat history, omits exact branch/head/PR, hides failed validation, leaves overlapping ownership unresolved or provides more than one ambiguous next action.

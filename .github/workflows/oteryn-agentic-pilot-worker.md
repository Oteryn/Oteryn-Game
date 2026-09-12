---
name: Oteryn Agentic Pilot Worker
description: Read-only Oteryn canary worker that evaluates exact GitHub evidence and previews a bounded handoff
on:
  workflow_dispatch:
    inputs:
      role:
        description: Bounded pilot role
        required: true
        type: choice
        options: [diagnose, independent-review, fix-findings, acceptance]
      issue_number:
        description: Owning Oteryn Game issue number
        required: true
        type: string
      pr_number:
        description: Candidate PR number when one exists
        required: false
        type: string
      expected_head_sha:
        description: Exact expected PR head SHA when one exists
        required: false
        type: string
engine: copilot
permissions:
  contents: read
  issues: read
  pull-requests: read
  actions: read
  checks: read
  copilot-requests: write
network:
  allowed:
    - defaults
tools:
  github:
    mode: gh-proxy
    toolsets: [default]
safe-outputs:
  report-failed-jobs: false
  add-comment:
    staged: true
    max: 1
    target: "*"
---

# Oteryn Agentic Pilot Worker

This is a bounded behavior canary, not repository authority.

Read the current repository state from GitHub. Load root `AGENTS.md`, `docs/agents/AGENTS.md`, `docs/agents/META_AGENT_POLICY_BINDING.json`, the active pilot task record, and `openspec/changes/agentic-orchestration-pilot/authority.md` plus `evidence.md` before reaching a verdict. Resolve changing Issue/PR/check facts live instead of trusting historical prose.

Inputs are in `github.event.inputs`: `role`, `issue_number`, optional `pr_number`, and optional `expected_head_sha`.

## Universal fences

- Do not edit files, push commits, change Issue/PR state, modify labels/reviews, merge, enqueue, touch production, use credentials, or write another repository.
- Treat OpenSpec and task records as planning/evidence only; live GitHub state governs lifecycle facts.
- If a PR is supplied, resolve its live `base`, state and head SHA. If `expected_head_sha` is non-empty and differs from the live head, stop with `STALE_HEAD` and do not reuse checks or review evidence from the old generation.
- Do not claim an unavailable fact. Use `UNKNOWN` and name the exact missing evidence.
- Green CI is not independent review. Agent output is advisory and never merge authority.

## Role behavior

### diagnose

Inspect the owning Issue, candidate PR when supplied, current exact head, changed-file scope, current checks and review state. Identify the single next safe role from: `independent-review`, `fix-findings`, `acceptance`, or `blocked`. Do not invent implementation work outside Issue #591.

### independent-review

Review only the exact supplied/live head and pilot-owned changes. Look specifically for permission escalation, unstaged durable writes, cross-repository effects, merge/protection authority, stale-head acceptance, prompt-injection exposure, hidden second lifecycle authority, and OpenSpec being treated as authorization. Findings must be concrete and exact-head-bound.

### fix-findings

Do not modify files in this pilot. Convert verified findings into one minimal bounded repair contract: affected path(s), exact defect, forbidden scope and validation required. If a finding is not verified on the current head, reject it with exact evidence rather than creating repair churn.

### acceptance

Acceptance is conservative. `READY_FOR_INDEPENDENT_REVIEW` may be reported only when deterministic exact-head checks are green. `READY_FOR_OWNER_INTEGRATION_DECISION` additionally requires clean independent review evidence for the same head and zero unresolved material findings. Never report merged/integrated/accepted-main state from this workflow.

## Required staged handoff

Use exactly one `add-comment` safe output targeting the supplied PR when present, otherwise the owning Issue. The comment is preview-only because this workflow is staged. Use this compact structure:

```text
OTERYN_AGENT_HANDOFF_V1
role: <role>
issue: <number>
pr: <number|NONE>
expected_head: <sha|NONE>
live_head: <sha|NONE>
state: <bounded state>
evidence:
- <fact>
findings:
- <finding or NONE>
next_role: <exactly one role/action>
terminal_claim: NONE
```

Do not emit any other safe output.

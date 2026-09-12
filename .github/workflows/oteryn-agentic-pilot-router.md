---
name: Oteryn Agentic Pilot Router
description: Canary router that binds live Oteryn GitHub state and dispatches one read-only pilot worker
on:
  pull_request:
    types: [synchronize]
    paths:
      - ".github/workflows/oteryn-agentic-pilot-*.md"
      - ".github/workflows/oteryn-agentic-pilot-*.lock.yml"
      - "openspec/**"
      - "docs/agents/tasks/active/OTV2-20260913-agentic-openspec-pilot.md"
  workflow_dispatch:
    inputs:
      issue_number:
        description: Owning Oteryn Game issue number
        required: false
        default: "591"
        type: string
      pr_number:
        description: Candidate PR number
        required: false
        type: string
      expected_head_sha:
        description: Exact expected PR head SHA
        required: false
        type: string
if: github.event_name == 'workflow_dispatch' || github.head_ref == 'agent/otv2-agentic-openspec-pilot-01'
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
  dispatch-workflow:
    workflows: [oteryn-agentic-pilot-worker]
    target-ref: agent/otv2-agentic-openspec-pilot-01
    max: 1
---

# Oteryn Agentic Pilot Router

This workflow is a canary adapter over existing Oteryn governance. It does not replace Issue/PR/check authority and has no merge or repository-write authority.

Load root `AGENTS.md`, `docs/agents/AGENTS.md`, `docs/agents/META_AGENT_POLICY_BINDING.json`, the active pilot task record, and `openspec/changes/agentic-orchestration-pilot/authority.md` plus `evidence.md`.

## Resolve target

- For a `pull_request` run, the target is the triggering PR and Issue `591`; use the live PR head as `expected_head_sha`.
- For `workflow_dispatch`, use the provided Issue/PR/head inputs. If the Issue input is empty, use `591`.
- Resolve Issue and PR data live. Require same repository and `base=main` for a supplied PR.
- If a caller supplied `expected_head_sha` and the live PR head differs, do not dispatch a worker. Preview one staged comment with `STALE_HEAD` and the observed head, then stop.

## Canary route

For this first pilot generation, deliberately route exactly one `diagnose` worker. This isolates orchestration/delivery from role-selection quality; role-selection expansion is a later change only after behavior evidence.

Emit exactly one `dispatch-workflow` safe output for `oteryn-agentic-pilot-worker` on the configured pilot branch with inputs:

- `role`: `diagnose`
- `issue_number`: resolved Issue number as a string
- `pr_number`: resolved PR number as a string when present
- `expected_head_sha`: exact live PR head when present

Also emit exactly one staged `add-comment` preview on the target PR when present, otherwise the Issue:

```text
OTERYN_AGENT_ROUTER_V1
issue: <number>
pr: <number|NONE>
head: <sha|NONE>
dispatched_role: diagnose
worker: oteryn-agentic-pilot-worker
write_effects: worker-dispatch-only; human-facing output staged
```

Do not create issues, PRs, commits, reviews, labels or merges. Do not dispatch any workflow other than the allowlisted pilot worker.

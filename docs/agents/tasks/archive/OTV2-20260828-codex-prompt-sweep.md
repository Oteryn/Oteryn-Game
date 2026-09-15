# OTV2-20260828-codex-prompt-sweep

```yaml
task_id: OTV2-20260828-codex-prompt-sweep
title: Sweep reusable prompts for canonical Codex review routing
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/codex-prompt-sweep-20260828
issue: 234
pr: 235
base_sha: 0deb460c1347213d2dbf14be845e9b0de344e792
head_sha: 79914da78e1aa1dd6e0835d984b80c25273455f2
final_head_sha: 79914da78e1aa1dd6e0835d984b80c25273455f2
merge_sha: 79e7a426f2984982a09c5106c34e5ad98ff39aec
owner: prompt-governance-sweep
created_at: 2026-08-28T07:43:45Z
updated_at: 2026-08-28T08:06:57Z
owned_paths: []
public_contracts: []
depends_on:
  - docs/agents/CODEX_REVIEW_POLICY.json@0deb460c1347213d2dbf14be845e9b0de344e792
blocks: []
external_repositories: []
```

## Outcome

The full reusable-prompt sweep is canonical on protected `main`. All 43 admission-time reusable prompt bodies now resolve the protected-main standing Codex review policy without reintroducing per-run owner relay for covered review operations. Non-covered owner-funded Codex/OpenAI/API use remains exact per-invocation owner-authorized, and lane/control-plane/auditor/read-only role boundaries remain fail-closed.

## Delivered

- all 43 reusable prompt bodies contain exactly one `## Canonical Codex review routing` section;
- stale broad per-run-owner authorization wording was removed or narrowed to non-covered owner-funded AI use;
- the five current Sol lane leads explicitly own the covered `CODEX_REQUIRED` exact-head review/re-review loop only under proven exact merged lane allocation;
- Work/Terra/implementation coordinator profiles explicitly do not trigger Codex on a lane candidate's behalf or adjudicate technical findings;
- independent audit roles do not dispatch nested Codex reviewers;
- Supervising Architect and read-only/preparation prompts gain no hidden candidate/review-request authority;
- `PROMPT_LIFECYCLE.json` versions for all 43 reusable prompts advanced exactly one minor version; non-reusable entries remained unchanged;
- prompting/evaluation standards and prompt README now state the canonical review-routing invariant;
- no runtime, Cargo/workspace, protocol/schema/resource-registry, production/protected-environment, secret, live-data or external-repository path was changed.

## Acceptance criteria

- [x] Inspect every reusable prompt registered at admission.
- [x] Add one canonical Codex review-routing section to every reusable prompt.
- [x] Remove or repair stale broad per-run-owner clauses.
- [x] Make the five Sol lane leads explicitly own the covered `CODEX_REQUIRED` review/re-review loop under exact merged lane allocation.
- [x] Make Work/Terra/implementation control planes route missing/stale Codex evidence back to the owning lane lead without triggering on its behalf.
- [x] Preserve auditor non-nested-review and Supervising Architect separation boundaries.
- [x] Update prompt lifecycle versions, README and prompt authoring/eval standards.
- [x] Static sweep checks and governance validation PASS on final exact head.
- [x] Full-diff self-review PASS on final exact head.
- [x] Fresh independent Codex exact-head review PASS with zero unresolved blocking threads.
- [x] Expected-head squash merge and protected-main readback complete.

## Validation

### Local/focused on final delivery head `79914da78e1aa1dd6e0835d984b80c25273455f2`

- reusable prompt count: 43;
- canonical routing sections: 43/43;
- stale authorization-pattern matches: 0;
- role-boundary assertions: PASS;
- lifecycle version deltas: all reusable prompts exactly +0.1; non-reusable entries unchanged;
- changed-path set: exactly 48/48 declared paths;
- `python tools/agents/validate_governance.py`: PASS;
- `git diff --check`: PASS;
- local tracked worktree: clean.

### Exact-head GitHub CI

- Agent governance 792 / `33153518718`: PASS;
- Architecture semantic audit 564 / `33153518687`: PASS;
- Merge authority audit 520 / `33153518699`: PASS;
- Merge gate 663 / `33153518792`: PASS, including governance, dependency review, CodeQL actions/python, aggregate validate and `game-gate`; Rust/runtime jobs correctly skipped for prompt/docs-only scope.

### E2E

`NOT_APPLICABLE` — prompt/governance-only change with no executable runtime/user scenario.

## Review history

The first independent Codex review on historical head `eacf6644f815c31dc61891c6ff8a1d7709636c0c` found one P1: stale task metadata (`pr: null` / stale checkpoint). The substantive prompt sweep itself passed the reviewer's static checks.

The P1 was repaired on final delivery head `79914da78e1aa1dd6e0835d984b80c25273455f2` by binding the task to PR #235, updating its validating checkpoint and using the canonical immutable PR/review/workflow surface for exact-head evidence rather than creating a forbidden self-referential SHA commit.

Evidence:
- historical Codex P1 comment: `5449948827`;
- repair/exact-head evidence comment: `5449982846`;
- final author whole-diff self-review comment: `5450009065`, PASS P0=0/P1=0/P2=0;
- final independent Codex result comment: `5450035175`, no major issues, reviewed commit `79914da78e`;
- zero unresolved review threads before merge.

## Canonical activation / readback

- PR #235 squash-merged as `79e7a426f2984982a09c5106c34e5ad98ff39aec`;
- protected-main readback resolved exactly that SHA;
- main-level sweep verification after merge: 43 reusable prompts, 43 canonical review sections, zero stale authorization matches;
- main-level `python tools/agents/validate_governance.py`: PASS.

## Completion

- delivery outcome: complete;
- protected-main activation: complete;
- acceptance criteria: complete;
- independent review: PASS;
- exact-head CI: PASS;
- runtime/product E2E: NOT_APPLICABLE — governance-only;
- ownership release: complete on this archive closeout merge;
- next action: none for Issue #234; existing Oteryn agent chats may continue/resume by resolving their canonical prompt from protected `main`.

## Context checkpoint

```yaml
last_progress: PR #235 merged as 79e7a426f2984982a09c5106c34e5ad98ff39aec and protected main was verified with 43/43 reusable prompts aligned to canonical Codex review routing
status: completed
branch: docs/codex-prompt-sweep-20260828
head_sha: 79914da78e1aa1dd6e0835d984b80c25273455f2
pr: 235
final_head_sha: 79914da78e1aa1dd6e0835d984b80c25273455f2
merge_sha: 79e7a426f2984982a09c5106c34e5ad98ff39aec
ci_check_generation: terminal_delivery_head
ci_checks_for_current_head: pass
ci_run_ids:
  - 33153518718
  - 33153518687
  - 33153518699
  - 33153518792
owner_action_required: none
blocker: null
next_action: None — task is terminal and all delivery ownership is released by this archive closeout.
```

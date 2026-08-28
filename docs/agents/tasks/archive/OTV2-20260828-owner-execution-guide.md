# OTV2-20260828-owner-execution-guide

```yaml
task_id: OTV2-20260828-owner-execution-guide
title: Canonicalize owner agent launch and status guidance
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 253
delivery_pr: 254
delivery_final_head_sha: 680b3b6bce329b2c4b5d6ea22b918f0cf0b1c649
delivery_merge_sha: 2f198348b9de800c6694b89f6ac30f6fb7ae0336
terminal_main_sha: 2f198348b9de800c6694b89f6ac30f6fb7ae0336
owner: released
owned_paths: []
shared_lease: released
created_at: 2026-08-28T22:56:24+02:00
completed_at: 2026-08-28T23:27:02+02:00
cross_repository_coordination_id: null
external_repositories: []
```

## Terminal outcome

The owner-facing agent execution system is now canonical on protected `main`.

- `docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md` is the owner/operator map for aliases, Work-vs-chat placement, recommended model/effort, launch order, Codex review, Work Auditor routing and live status classification.
- reusable read-only alias `Oteryn: owner execution guide` resolves live GitHub and reports what to run, where, with what model/effort, what is `DONE / ACTIVE / BLOCKED / READY_NEXT / DO_NOT_LAUNCH / UNKNOWN`, and exactly one next action;
- five current mutating Sol lane leads expose standardized exact-head `codex_review` handoff evidence;
- Work/Terra, Sol Supervising Architect and Work Auditor keep their already-canonical post-#230/#235 review ownership boundaries;
- scheduler and Movement lead resolve the current canonical Movement resource/dependency gate dynamically rather than hard-coding a historical Issue number.

This package grants no new control-plane, lane-allocation, runtime, merge, production, external-repository or owner-funded AI authority.

## Exact-head qualification

PR #254 final head `680b3b6bce329b2c4b5d6ea22b918f0cf0b1c649` passed:

- Agent governance run `33212268623` / #1060: `SUCCESS`;
- Architecture semantic audit run `33212268611` / #806: `SUCCESS`;
- Merge authority audit run `33212268617` / #759: `SUCCESS`;
- Merge gate run `33212268612` / #925: `SUCCESS`;
- final author whole-diff self-review `5055198384`: `PASS`, P0/P1/P2/P3 = 0/0/0/0;
- final native Codex exact-head re-review completed on `680b3b6b...` with no new findings and native `+1` no-suggestions signal reaction `477078720`;
- two historical Codex P2 findings were repaired on later heads and both review threads are resolved;
- zero unresolved required review threads on the final head;
- protected-main compatibility remained exact at `12d4ca5326d62a7a2c46d80cd5e167e99f109d1d` before expected-head merge;
- expected-head squash merge/readback: `2f198348b9de800c6694b89f6ac30f6fb7ae0336`.

Runtime/component/E2E: `NOT_APPLICABLE` — governance/prompts/runbook only.

## Ownership release

All governance-delivery ownership for Issue #253 / PR #254 is released. This archive grants no continuing branch, implementation, review-request, control-plane or merge authority.

## Handoff

For owner-facing execution guidance, use a fresh separate GPT-5.6 Sol Extra High/highest-available chat and invoke:

```text
Oteryn: owner execution guide
```

The advisor must freshly resolve protected `main`, the current coordinator lifecycle, active tasks, open/recent PRs, exact heads/checks/reviews/threads, scheduler and Codex policy before giving instructions. It never substitutes for live allocation or authority.

Exactly one next programme action for this governance task: `NONE`.

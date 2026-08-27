# OTV2-20260827-terra-sol-parallel-execution

```yaml
task_id: OTV2-20260827-terra-sol-parallel-execution
title: Package Terra control plane and Sol parallel lead architecture
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/terra-sol-parallel-agent-architecture-20260827
issue: 213
pr: 214
base_sha: 4c395ece416c3c56aed5607653a0730c52dcb3fd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-08-27T09:27:00+02:00
updated_at: 2026-08-27T12:34:00+02:00
execution_budget_minutes: 120
large_budget_reason: authority-sensitive multi-agent execution architecture, prompt family and lifecycle registration
owned_paths:
  - docs/superpowers/specs/2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md
  - docs/superpowers/plans/2026-08-27-oteryn-game-terra-sol-parallel-execution.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md
  - docs/agents/prompts/OTV2_SOL_WORLD_CONTENT_PREP.md
  - docs/agents/prompts/OTV2_SOL_NPC_AI_PREP.md
  - docs/agents/prompts/OTV2_SOL_SYSTEMS_ECONOMY_PREP.md
  - docs/agents/prompts/OTV2_SOL_TOOLING_OPS_PREP.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260827-terra-sol-parallel-execution.md
public_contracts: []
depends_on:
  - owner direction in Issue #213
  - existing Sol-lead design and plan on protected main
  - terminal ownership release of Issue #179 through PR #184 / main:c082060c10a54758ebcc3655d0bffa83f4af51e3
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish a reusable execution-governance package in which ChatGPT Work on Terra High is a deterministic control plane with zero technical/architecture discretion, while exact Sol Extra High lane leads own implementation reasoning and a Sol Supervising Architect owns material cross-lane architecture decisions.

Both `Oteryn: work coordinator` and `Oteryn: terra game coordinator` remain reusable globally, but one programme lifecycle may have exactly one active mutating control-plane profile. Legacy #162 remains Work-controlled until a separate durable transition explicitly selects Terra.

## Architecture and source of truth

- `PROVEN`: protected admission main is `4c395ece416c3c56aed5607653a0730c52dcb3fd`.
- `PROVEN`: Issue #213 records owner scope for this execution architecture.
- `PROVEN`: the existing owner-approved Sol-lead execution design and adoption plan are already on protected main.
- `PROVEN`: current Durability work is live under Issue #167 and draft PR #212 at task admission; live GitHub may advance during this governance task.
- `PROVEN`: independent exact-head review of PR #214 head `5fa107837f0b8b3c31c493ec0511b00176a23f75` returned `CHANGES_REQUIRED` with P1 findings for stale predecessor ownership, competing reusable control planes and stale task metadata.
- `PROVEN`: predecessor design closeout PR #184 was merge-refreshed, passed exact-head governance checks and squash-merged as protected `main@c082060c10a54758ebcc3655d0bffa83f4af51e3`; Issue #183 is closed completed and the old task now has `owned_paths: []`.
- `PROVEN`: PR #214 branch was normally merge-updated with `main@c082060c10a54758ebcc3655d0bffa83f4af51e3` before review repairs; no force-push/reset/restart was used.
- `DERIVED`: the ownership collision from the first independent review is terminally removed.
- `DERIVED`: keeping both control-plane aliases reusable is safe only with a durable single-active-profile rule enforced by both coordinator prompts; this repair adds that fail-closed rule without silently taking over the active #162 lifecycle.

## Acceptance criteria

- [x] Terra coordinator prompt has explicit zero technical/architecture discretion.
- [x] Work/Terra control-plane coexistence is mutually exclusive for programme mutation and legacy #162 remains Work-controlled until durable transfer.
- [x] Sol Supervising Architect prompt routes material architecture decisions without granting implicit runtime writes.
- [x] Sol Durability, Server Seam, Client/QA, Movement and Combat lead prompts are present and bind mutation to exact live allocation.
- [x] Post-VSL expansion prompt can decompose later accepted work without speculative runtime authority.
- [x] Explicit World/Content, NPC/AI, Systems/Economy and Tooling/Ops preparation profiles are present and remain read-only until exact later allocation.
- [x] Scheduler defines V0-V4 promotion, useful parallel preparation, writer limits and unique active control-plane resolution.
- [x] README and prompt lifecycle register every new alias without implicit supersession.
- [x] Static prompt-family self-evaluation against `PROMPT_EVAL_STANDARD.md` is complete for package content; exact-head author-review evidence is recorded externally in the PR after final-head freeze.

## External terminal merge gates

The following are mandatory **external exact-head gates**, not mutable completion checkboxes in this task packet:

- required Agent governance / Architecture semantic audit / Merge authority audit / Merge gate-game-gate evidence is green on the unchanged final PR head in GitHub;
- a genuinely independent non-authoring review returns `PASS` on that same final head because execution/merge authority boundaries change;
- zero unresolved required review threads are confirmed immediately before merge.

Per `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`, after final-head freeze these facts belong in GitHub reviews/workflows or another immutable evidence channel. Do not add a task/checkpoint/SHA/run-ID commit merely to mirror them here because that would create a new, unqualified head.

## Excluded scope

No gameplay/runtime, Cargo/workspace, protocol/schema/registry, production/protected environment, secrets, live data, Platform/Atlas/META or external-repository mutation. Do not modify active Durability #167/#212 implementation history or current product worker branches.

## Implementation / findings

The initial package correctly constrained Terra technical discretion and Sol lane ownership, but genuinely independent review found three material governance defects on head `5fa107837f0b8b3c31c493ec0511b00176a23f75`:

1. predecessor Issue #179 still owned `README.md` and `PROMPT_LIFECYCLE.json` because its closeout #184 was not terminal;
2. Work and Terra profiles were both reusable without a fail-closed per-programme single-active-control-plane rule;
3. this active task packet still claimed `pr: null`, pending construction and a stale next action.

Repair sequence:

- terminally complete #184/#183 first, releasing predecessor ownership;
- merge-up the resulting protected main into #214 without history rewrite;
- add the single-active-control-plane rule to both Terra and Work profiles, scheduler, design and README while leaving aliases reusable/unsuperseded;
- refresh this packet with known PR/progress/ownership facts without attempting to self-record its own final SHA.

No runtime worker ownership is changed. In particular, draft Durability PR #212 remains on its existing branch/history and paths.

## Validation

### Focused

- command/run: independent review against original exact head `5fa107837f0b8b3c31c493ec0511b00176a23f75`
- result: `CHANGES_REQUIRED`; P1-01 predecessor ownership, P1-02 competing control planes, P1-03 stale active task metadata
- repair status: predecessor ownership terminally released by #184/#183; control-plane mutual exclusion and task metadata repaired
- repair extension: current Issue #213 deliverables were re-read; four future-wave profiles and the handoff-schema P2 are incorporated before final freeze

### Component/integration

- command/run: exact-head GitHub governance workflows after final-head freeze
- result: mandatory external terminal gate; the GitHub workflow generation on the unchanged PR head is authoritative and is intentionally not mirrored by a follow-up content commit

### E2E

- scenario: `NOT_APPLICABLE` - governance/prompt package only, no executable product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: intentionally not self-recorded in this commit; GitHub PR head is the immutable qualification key
- trigger source: pull_request #214
- workflow/run/job: external GitHub evidence on the unchanged final head
- runner assignment: external GitHub evidence
- classification: governance-only
- result: mandatory external merge gate; do not create content churn merely to mirror run IDs or state here

## Self-review

- exact head: resolved from GitHub after final-head freeze; intentionally not self-recorded in this commit
- method/reviewer: repair-authoring session; whole-scope review against Issue #213, prior independent findings and `PROMPT_EVAL_STANDARD.md`
- static package result: `PASS` on all ten prompt-evaluation gates after predecessor ownership, control-plane mutual exclusion, handoff-schema and four-profile completeness repairs
- exact-head verdict/evidence: mandatory external PR review on the unchanged final head; a later commit must not be created merely to copy that verdict here

## Independent review

- required: YES - execution/merge authority boundary change under `docs/agents/AGENTS.md`
- previous exact head: `5fa107837f0b8b3c31c493ec0511b00176a23f75`
- previous method/auditor: genuinely independent non-authoring reviewer session
- previous material findings: P1=3, P0=0, plus one non-blocking P2
- previous verdict: `CHANGES_REQUIRED`
- final exact head: resolved from GitHub after freeze; intentionally not self-recorded in task content
- final-head method/auditor: genuinely independent non-authoring reviewer/session; the repair-authoring session does not count
- final-head verdict/evidence: mandatory external PR review; any material repair moves the head and requires a fresh independent review

## PR and closeout

- PR: #214
- changed-file review: 19 intended governance/docs paths after adding the bounded Work-coordinator coexistence amendment and four required post-VSL preparation profiles
- unresolved review threads: external exact-head pre-merge readback; not mirrored by a content commit after freeze
- related/superseded PRs: #184 merged as `c082060c10a54758ebcc3655d0bffa83f4af51e3` and Issue #183 closed completed; no predecessor ownership remains
- protected auto-merge: not requested before all external exact-head gates are proven
- merge commit/result: external GitHub evidence after expected-head protected merge
- ownership release: post-merge bounded closeout/reconciliation; the merge SHA cannot be self-recorded before merge

## Context checkpoint

```yaml
last_progress: all material package repairs are present, including predecessor ownership release, fail-closed Work/Terra mutual exclusion, aligned handoff evidence, and four explicit read-only future-wave preparation profiles
status: validating
branch: docs/terra-sol-parallel-agent-architecture-20260827
head_sha: null
pr: 214
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_evidence_location: github_pr_214_exact_head
review_evidence_location: github_pr_214_exact_head
runner_assignment_state: external_github_evidence
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: external exact-head qualification gates are mandatory but are not mutable task-state fields
next_action: qualify the unchanged final PR #214 head through external GitHub self-review, genuinely independent review, required CI and zero-thread readback
```

# OTV2-20260929-client-1530-membership-manifest

```yaml
task_id: OTV2-20260929-client-1530-membership-manifest
title: Emit the 15.30 client appearance membership manifest (ITEM-ID-1 prerequisite)
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/client-1530-membership-manifest
issue: 162
pr: 1240
allocation: "#162 work coordinator allocation (A12 #1237 prerequisite)"
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: client membership manifest worker (claude-code-session-01PwTJFS62J35S88Srpqnrgx)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/client_appearance_census.py
  - tools/content-schema/item-authoring/test_client_appearance_census.py
  - tools/content-schema/item-authoring/README.md
  - imports/official/client-assets/15.30/README.md
  - imports/official/client-assets/15.30/appearance-ids.json
  - docs/agents/tasks/active/OTV2-20260929-client-1530-membership-manifest.md
public_contracts: []
depends_on: []
blocks: [A12 #1237 digest-bound full membership manifest]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`client_appearance_census.py --membership-out PATH` writes an id-only, deterministic manifest
(`OTERYN_CLIENT_APPEARANCE_MEMBERSHIP/v1`) of the pinned 15.30 `appearances-2dfa943b….dat`, which is
loaded through the existing size and sha256 guard (fail closed). It needs no engine-source arguments.
The proprietary `.dat` is never committed; the owner runs the tool locally and commits only
`imports/official/client-assets/15.30/appearance-ids.json`. The coordinator generated it once from the
owner-supplied file (size and sha256 verified, two runs byte-identical: 43,516 ids, max 55117) and
committed only the id list.

## Architecture and source of truth

A12 (#1237) §4.1/§5 require a digest-bound full membership manifest of each admitted CipSoft appearance
file. `imports/official/client-assets/15.30/manifest.json` pins the file (sha256 `2dfa943b…`, 5,017,996 bytes).

## Acceptance criteria

- [x] `--membership-out` writes ids-only deterministic JSON through the existing size/sha256 guard and needs no engine sources (`test_client_appearance_census.py`: determinism, sorted unique ids, `ids_sha256`, digest mismatch fails).
- [x] `appearance-ids.json` committed: 43,516 ids, max 55117, bound to sha256 `2dfa943b…`; two generations byte-identical.
- [x] No proprietary content committed (no `.dat`, names, flags or sprites).
- [ ] `item-authoring` and `game-gate` green on the exact final head.

## Excluded scope

The ITEM-ID-1 key migration, any other client file, and `.github/**`.

## Implementation / findings

- Tool option and tests (worker); manifest generated once by the coordinator from the owner-supplied file.
- Codex on `11b1c3b`: P1 task-record sections (fixed: this section set); P2 README command filename (fixed: full pinned filename).
- `item-authoring` red on `11b1c3b`: pinned ruff 0.16.1 formatting (fixed in `29b721e`).

## Validation

### Focused

- command/run: `python test_client_appearance_census.py`; pinned `ruff check` and `ruff format --check`; governance and repository policy validators; `git diff --check`
- result: PASS (27 checks)

### Component/integration

- command/run: `NOT_APPLICABLE`, because the change is a standalone tooling option and committed data with no runtime consumer yet
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE`, because nothing reaches the runtime
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: recorded on the PR (a commit cannot contain its own SHA)
- trigger source: pull_request
- workflow/run/job: `item-authoring`, `game-gate` on the final head
- runner assignment: GitHub-hosted
- classification: required `game-gate`
- result: pending

## Self-review

- exact head: final PR head
- method/reviewer: coordinator whole-diff review
- material findings: none open
- verdict: PASS pending CI

## Independent review

- required: YES (identity evidence for A12 ITEM-ID-1)
- exact head: `11b1c3b` (Codex); follow-up fixes are docs/format only (D101)
- method/auditor: Codex review
- material findings: 1 P1 and 1 P2, both fixed
- verdict: pending on the final head

## PR and closeout

- changed-file review: done
- unresolved review threads: resolved after the fix push
- related/superseded PRs: #1237 (A12), ITEM-ID-1 consumer
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: Codex P1/P2 and ruff formatting fixed
status: validating
branch: claude/client-1530-membership-manifest
head_sha: null
pr: 1240
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: enable auto-merge once game-gate is green on the final head
```

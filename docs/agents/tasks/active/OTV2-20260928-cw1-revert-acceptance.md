# OTV2-20260928-cw1-revert-acceptance

```yaml
task_id: OTV2-20260928-cw1-revert-acceptance
title: Record owner acceptance of §7 revert_after direction and open decision 7
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-revert-acceptance
pr: null
base_sha: f0710bfb0513147d6bf573dbbfac591da0dadfa6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world architecture (CW1)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/agents/tasks/active/OTV2-20260928-cw1-revert-acceptance.md
public_contracts: []
depends_on:
  - "owner acceptance recorded on issue #162, 2026-09-28"
  - "docs/agents/tasks/archive/OTV2-20260928-cw1-world-object-revert-progression.md (PR #1045, merged ebacc5b8)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Docs-only §7 update in `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`:

1. §7's `DecisionStatus` records "direction ACCEPTED by owner 2026-09-28 (#162)" — naming exactly
   what is accepted (the scope-owned lifecycle record; commit-only scheduling gated on `prepare`
   returning `Publish`; the state+collision-only revert scope; rejection of attribute-changing
   transitions at authoring/lowering) and that open decisions 1, 2, 4, 5, 6 and 7 stay delegated to
   the owning lane, while open decision 3 (attribute-bearing object state) is owner-decided `YES`,
   design in progress as task B (`OTV2-20260928-cw1-attribute-bearing-object-state`).
2. New open decision 7 (Round 21, Codex finding 4120777222, raised on PR #1045's thread after that
   PR's round-20 freeze/merge, too late to fold in): lifecycle-record capacity must be reserved only
   once `prepare` has already returned `DISPOSITION_COMMITTED`/`PreparedMutation::Publish`, never
   speculatively before `prepare` runs. Corrects the staging bullet's round-20 "reserve before
   `prepare`, release on `unchanged`" wording and the two test obligations it superseded.
3. Fixed every place in the document that still asserted §7 has no owner acceptance (top-of-doc
   summary line, §7's own intro paragraph, §8 Follow-up items 6/7).

No code, Foundation/runtime/protocol/registry, `content/**` or `tools/**` change. Does not itself
design open decision 3 (task B) or implement anything the owning lane still owns (decisions 1, 2, 4,
5, 6, 7).

## Architecture and source of truth

- `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §7 — PROVEN, read in full
  (post PR #1045 merge, `ebacc5b8`).
- Owner decision recorded on issue #162 (2026-09-28): §7 direction ACCEPTED, open decisions 1, 2, 4,
  5, 6, 7 delegated to the owning lane, open decision 3 decided `YES` — coordinator-relayed, treated
  as the owner's own instruction per this repository's live-Issue-governs-status rule.
- Codex finding 4120777222 (PR #1045 thread, post-merge) — coordinator-relayed exact content: reserve
  lifecycle-record capacity only after `prepare` returns `Publish`, not before `prepare` runs.
- `apps/game-server/src/world_runtime.rs` `prepare`/`PreparedTerminal`/`PreparedMutation`
  (~973-1159) — PROVEN (re-verified, unchanged since round 20): every `unchanged` disposition builds
  `PreparedMutation::None`; only `DISPOSITION_COMMITTED` builds `Publish`. `prepare` is a pure query —
  no side effect, no capacity reservation of its own.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture decision-status update; no production mutation, authority grant,
  PREPARE/COMMIT, controller install/restore or persisted recovery evidence is touched.
```

## Acceptance criteria

- [x] §7's `DecisionStatus` records owner acceptance of direction, names what is accepted, and states
      which open decisions are delegated vs. owner-decided.
- [x] Open decision 7 added, citing Codex finding 4120777222.
- [x] The staging bullet's pre-`prepare` capacity-reservation wording and the capacity-atomicity /
      no-revert-on-unchanged test obligations it superseded are corrected to match.
- [x] No remaining "§7 has no owner acceptance" text anywhere in the document (grep swept).
- [x] `python3 tools/agents/validate_governance.py` passes.
- [x] `python3 tools/repository/validate_repository_policy.py` passes.

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Designing open decision 3 (attribute-bearing object state) — that is task B.
- Resolving open decisions 1, 2, 4, 5 or 6 — owning-lane implementation.
- `@codex` trigger, PR/issue comments, merge action — coordinator's.

## Implementation / findings

Read §7 in full on fresh `origin/main` (post `ebacc5b8`, PR #1045 merged). Edited §7's heading and
added a `DecisionStatus` line naming exactly what is accepted vs. delegated vs. owner-decided-YES.
Fixed the now-stale "It does not accept the answer on the owner's behalf" sentence immediately below
it. Fixed the top-of-doc summary line (previously "§7 ... has no owner acceptance yet, and does not
itself resolve who accepts it").

Added open decision 7 (Round 21 narrative + Open-decisions-list item 7): reserve capacity only after
`prepare` returns `Publish`. Corrected the staging bullet (previously: reserve before `prepare` runs,
release on `unchanged`) to: run `prepare` first, reserve capacity only once it returns
`DISPOSITION_COMMITTED`/`Publish`, in the same staged commit as the object mutation; an `unchanged`
result never reaches a reservation step at all. Corrected the "Lifecycle-record capacity atomicity"
test obligation and the "original operation that does not commit registers no revert" test obligation
to match (both previously described a reserve-then-release pattern that no longer exists).

Updated §8 Follow-up items 6/7, which still said `revert_after` "awaits §7 being accepted" and that
the owning lane's job was to "accept or supersede" the option — now reflects that direction is
accepted and the owning lane's job is implementation of the named open decisions.

Grep swept the whole document for "owner-acceptance still open" / "has no owner acceptance" / "does
not itself resolve who accepts" — no remaining occurrences. Grep swept "Rounds 15/16/19/20" heading
references to add "/21".

Both validators re-run after all edits: PASS (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes."

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 48 workflows)."

### E2E

- scenario: NOT_APPLICABLE — documentation-only, no runtime behavior to exercise.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent (CW1), mandatory, not delegated
- material findings: none beyond the stale-wording sweep above
- verdict: PASS

## Independent review

- required: YES.
- exact head: pending
- method/auditor: pending (coordinator-directed, no `@codex` trigger from this task)
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known; sequenced immediately before task B on the same document
- protected auto-merge: not requested by this task
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: >
  §7 DecisionStatus updated to record owner acceptance of direction (issue #162, 2026-09-28), naming
  what is accepted and which open decisions (1,2,4,5,6,7) are delegated vs. decision 3 (owner YES,
  task B in progress). Added open decision 7 (Codex 4120777222): reserve lifecycle-record capacity
  only after prepare returns Publish, not before prepare runs. Corrected the staging bullet and the
  two test obligations it superseded. Fixed every stale "no owner acceptance" reference in the doc.
  Validators pass; about to push and open the PR.
status: implementing
branch: claude/cw1-revert-acceptance
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: push, open PR, report PR number + head SHA, then start task B without waiting
```

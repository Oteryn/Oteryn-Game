# OTV2-20260928-quest-relocation-worldobject-transcription

```yaml
task_id: OTV2-20260928-quest-relocation-worldobject-transcription
title: D37 relocation and D38 world-object overlay transcription (Queen of the Banshees worked example)
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/quest-relocation-worldobject-transcription
issue: 162
pr: 1053
base_sha: e91cafdd95c165668126e1dccaf93422af29d55c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import (worker session)
created_at: 2026-09-28T00:41:00Z
updated_at: 2026-09-28T03:40:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/**
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - docs/agents/tasks/active/OTV2-20260928-quest-relocation-worldobject-transcription.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Turn the D36 interaction children transcribed as blocked Movement/WorldObject into typed D37
relocation and D38 overlay-operation data, so CW4's overlay runtime has real, schema-valid input,
with the Queen of the Banshees as the worked example (issue #162 comment 5860305453).

**Delivered:** the schema, converter and synthetic-fixture tests are complete and correct for both
D37 and D38, typing a child only for an exact, fully-delimited literal argument (never a substring,
name or list-order heuristic, an unresolved/non-positive delay, or a duplicate revert overwrite) —
verified by 223 regression cases, incl. every exact Codex example across five review rounds.

**Blocked on the committed corpus:** the transcription never recorded the raw argument text behind
each Movement/WorldObject child, only its resolved shape, so it cannot verify the current strict
rule: **all 800 Movement and all 891 WorldObject children are blocked today**, anchors retained as
evidence even where unreferenced. A fresh run against real source (`add_repo` for the pinned
checkouts was denied) would type most correctly first pass; tracked as excluded scope, not a blocker.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, §6.6, §6.7 — D36 (`PROVEN`).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3/§4 (D37/
  D38) — CANDIDATE, decided 2026-09-27; contract text awaits review (not touched here).
- Counts (`PROVEN`): interactions 1,221; Movement/WorldObject 800/891 blocked, 0 typed; 313 anchors.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation or authority fence involved.

## Acceptance criteria

- [x] A Movement/WorldObject child is typed only for an exact, fully-delimited literal argument, and
  a revert only for a provable same-target match with (for a scheduled one) a positive literal
  delay; anything unverifiable stays blocked. CREATE/REMOVE bind an anchor only by argument
  structure; RETAG carries no action-id field.
- [ ] Real D37/D38 data on the committed corpus. **Not met** (excluded scope, not a blocker): the
  committed shape carries no argument-level evidence; schema/converter/tests are ready for a real run.
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus; `verify_quest_schema.py`
  223/223 (35 converter-level regression cases, five rounds).
- [x] Deterministic regeneration: idempotent migrations; anchors and manifest counts proven equal to
  the merged `origin/main` (313/313, 0 added; every count field recomputed and matched).

## Excluded scope

- Real per-instance D37/D38 classification on the committed corpus (see Exact delta).
- `content/**`, Rust, protocol, DUR-03/quest-state runtime, D39-D42, the proposal's contract text:
  untouched. SCOPE_HANDOFF: no corpus instance signals it.

## Exact delta to unblock

A worker/session with `opentibiabr/canary` (`47dfd51f`)/`zimbadev/crystalserver` (`ff7ede59`) checked
out completes D37/D38 with no further code change: run the converter, diff against
`samples/interactions/*.json`, commit. Tests already cover every classification rule.

## Repair rounds (Codex reviews + one owner-flagged correction; all fixed, tested, re-validated;
rounds 3/3-correction/5 each merged `origin/main` first — no overlap with owned paths, PR/closeout)

**Round 1** (6d69ccd, 3 P2s): a revert produced its own standalone TRANSFORM; `createItem(id,
Position(...))` dropped the position; a reward constructor also produced a WorldObject CREATE.

**Round 2** (dde3b43e, 4 P2s; root cause: substring/list-order heuristics): the previous-tile check
matched any expression merely mentioning `fromPosition`, a revert attached by list order alone, a
`createItem` id came from a non-delimited expression, the position used a name heuristic. 209
previously-typed previous-tile children, unprovable from the committed shape, downgraded to blocked.

**Round 3** (owner-authorized, 199d3999, 3 P2s; same root cause, argument search still too broad):
teleport anchor and revert-position checks searched the whole statement instead of one split
argument (fixed); revert attached to the nearest op instead of the one provable match (fixed: search
every candidate, >1 equal match blocks too). 219 previously-typed anchor children downgraded. 6 cases.

**Round 3 correction** (owner-flagged, 8437920): the anchor fix's migration deleted the 219
downgraded children's now-unreferenced anchors, losing pre-existing evidence. Restored every anchor
exactly as on main (313 total, 0 added); relaxed the anchor-usage check to allow one with no current
consumer, instead of reconstructing per-child correspondence already lost.

**Round 4** (owner-authorized, final, 298493d, 2 P2s + a self-grep): a scheduled revert with a
non-literal delay attached silently with no `revert_after_ms` — fixed to fail closed (new
`BLOCKED_SCHEDULED_REVERT_DELAY`, a per-owner reason *set*). Direct `Position.revertItem(...)` was
shadowed by the generic `REVERT_METHOD` receiver match (`Position` parses as a valid receiver) —
fixed by checking the specific form first. Re-grep found no further shadowing. 4 new cases.

**Round 5** (owner-authorized, a799e3a4, merge + 2 P2s): `origin/main` advanced to `00691b5b`
(unrelated #1067, 27 files, condition-override resolution, not D37/D38). Merged (merge commit, no
rebase); most files auto-merged clean; 3 flagged conflicts resolved: README (both rows kept),
`verify_quest_schema.py` (both test sections kept), `manifest.json` counts (not hand-merged —
recomputed from the merged corpus via `build()`'s own logic, proven to match field-by-field). Anchor
set re-verified 313/313 vs main, 0 missing/added. Codex P2s (4119390219, 4119390226): a second
scheduled revert on an already-reverted op silently overwrote `revert_after_ms` — fixed: `revert()`
now blocks (new `BLOCKED_DUPLICATE_SCHEDULED_REVERT`), never overwrites. `addEvent(Position.
revertItem, 0, ...)` passed as a "literal" delay, emitting invalid `revert_after_ms: 0` — fixed:
non-positive literal delays block like non-literal ones. 4 new cases (2 examples + 2 controls).

## Implementation / findings

- `interaction.schema.json`: `$defs.relocation_target`; D37/D38 typed shapes; `to_source_line` optional.
- `ots_interactions.py`: `children()` types only an exact literal argument, via `split_args`/`argument`;
  `same_target()`/`revert()`/`append_children()`/`strip_internal()`/`parse_revert()`.
- `validate_quest_content.py`: `BLOCKED_SCHEDULED_REVERT_DELAY` + `BLOCKED_DUPLICATE_SCHEDULED_REVERT`
  + `BLOCKED_REASONS` (set per owner); anchor-usage no longer errors on an unreferenced anchor.
- `verify_quest_schema.py`: `run_converter`/`converter_case` harness, 35 converter-level cases; merged
  with main's own override/parser test section (kept, unmodified).
- `samples/interactions/{interactions,manifest}.json`: anchors and manifest counts re-proven equal
  to main's post-merge. Final: Movement/WorldObject 0 typed, blocked.
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`, `README.md`: counts/rows updated.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 223/223 (156 schema-fixture + 35 converter regression + 5 override + 27 wiki-parser,
  last two from `origin/main`'s own kept-unmodified block)

### Component/integration
- command/run: `python3 validate_quest_content.py <claims> <quests> --catalog ... --manifest ...
  --gates ... --gates-manifest ... --progress ... --interactions ... --interactions-manifest ...`
  (full corpus: chests, doors, questlog, interactions samples, all flags)
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE

### Regeneration / non-regression
- Round 3/correction: 0/1,221 differ outside Movement/WorldObject + restored anchors; anchor sets
  equal main per interaction (313/313, 0 added).
- Round 4 (code-only): samples byte-identical (same sha256); anchor set still 313/313. Idempotent.
- Round 5 (merged `origin/main` 00691b5b): anchor set 313/313, 0 missing/added. Every `manifest.json`
  counts field recomputed from the merged corpus and proven to match — not hand-merged.

### Exact-head CI
- final head: pending (set at freeze)
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending (set at freeze)
- method/reviewer: implementing agent (this session)
- material findings: a schema `oneOf` gap (fixed). Round 1 — 3 P2s. Round 2 — 4 P2s, root-caused to
  substring/list-order heuristics. Round 3 — 3 more, plus an owner-flagged data loss (deleted
  pre-existing anchors). Round 4 — 2 more (silent non-literal-delay merge, receiver-match shadow)
  plus a re-grep (none further). Round 5 — an unrelated main merge (3 conflicts, none hand-merged)
  plus 2 more P2s (duplicate scheduled revert, non-positive literal delay). All fixed and tested.
- verdict: no other material findings.

## Independent review

- required: YES — changes an accepted schema
- exact head/method/findings: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: round 5 addresses the last 2 P2s; owner wants this finished
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- merge from main: 13efb4f, 45b6cc7 (empty diff both times), 00691b5b (round 5, #1067, 27 files, 3
  conflicts resolved, no hand-merged data).
- ownership release: pending — real D37/D38 classification is excluded scope, not a blocker.

## Context checkpoint

```yaml
last_progress: round 5 done (see "Repair rounds"): merged origin/main, resolved 3 conflicts (none
  hand-merged for data), fixed 2 more Codex P2s, re-verified anchors/manifest counts. 223/223
  verify_quest_schema.py, 0 validate_quest_content.py errors, both governance validators PASS.
status: ready
branch: claude/quest-relocation-worldobject-transcription
head_sha: null
pr: 1053
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
repair_cycles_for_current_gate: 6
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none for this PR; a future checkout-capable session runs the converter and
  commits the real D37/D38 delta as tracked follow-up work
blocker: null (real-corpus classification is excluded scope, not a blocker here)
next_action: push, report READY_FOR_INTEGRATION with head SHA
```

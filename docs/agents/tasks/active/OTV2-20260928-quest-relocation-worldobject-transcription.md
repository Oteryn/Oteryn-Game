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
updated_at: 2026-09-28T05:10:00Z
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
D37 and D38, typing a child only for an exact, fully-delimited literal argument, never a substring,
name/list-order heuristic, a string literal, a partial multi-line call, or an unrecognized shape —
verified by 231 regression cases, incl. every exact Codex example across six review rounds.

**Blocked on the committed corpus:** the transcription never recorded the raw argument text behind
each Movement/WorldObject child, only its resolved shape, so it cannot verify the current strict
rule: **all 800 Movement and all 891 WorldObject children are blocked today**, anchors kept as
evidence even where unreferenced. A fresh run against real source (`add_repo` denied) would type most
correctly first pass; tracked as excluded scope, not a blocker.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3/§6.3/§6.6/§6.7 — D36 (`PROVEN`).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3/§4
  (D37/D38) — CANDIDATE, decided 2026-09-27; contract text awaits review (not touched here).
- Counts (`PROVEN`): interactions 1,221; Movement/WorldObject 800/891 blocked, 0 typed; 313 anchors.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation or authority fence involved.

## Acceptance criteria

- [x] A Movement/WorldObject child is typed only for an exact, fully-delimited literal argument on a
  call `code` confirms is real and complete on its own line; anything unverifiable stays blocked.
  CREATE/REMOVE bind an anchor only by argument structure; RETAG carries no action-id field.
- [ ] Real D37/D38 data on the committed corpus. **Not met** (excluded scope, not a blocker): the
  committed shape carries no argument-level evidence; schema/converter/tests are ready for a real run.
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus; `verify_quest_schema.py`
  231/231 (43 converter-level regression cases, six rounds).
- [x] Deterministic regeneration: idempotent migrations; anchors and manifest counts proven equal to
  the merged `origin/main` (313/313, 0 added; every count field recomputed and matched).

## Excluded scope

- Real per-instance D37/D38 classification on the committed corpus (see Exact delta).
- `content/**`, Rust, protocol, DUR-03/quest-state runtime, D39-D42, the proposal's contract text.
  SCOPE_HANDOFF: no corpus instance signals it.

## Exact delta to unblock

A worker/session with `opentibiabr/canary` (`47dfd51f`)/`zimbadev/crystalserver` (`ff7ede59`)
checked out completes D37/D38 with no further code change: run the converter, diff against
`samples/interactions/*.json`, commit. Tests cover every classification rule.

## Repair rounds (Codex reviews + one owner-flagged correction; all fixed, tested, re-validated;
rounds 3/3-correction/5 merged `origin/main` first — no overlap with owned paths, PR/closeout)

**Rounds 1-2** (6d69ccd/dde3b43e, 7 P2s; root cause emerging in round 2: substring/list-order
heuristics): a revert produced its own TRANSFORM; `createItem` dropped a literal position; a reward
constructor also produced a CREATE; the previous-tile check matched any mention of `fromPosition`; a
revert attached by list order alone; a `createItem` id/position came from a name heuristic. 209
previously-typed previous-tile children, unprovable from the committed shape, downgraded to blocked.

**Round 3 + correction** (owner-authorized, 199d3999/8437920, 3 P2s, same root cause: argument search
too broad): teleport/revert-position checks searched the whole statement, not one split argument
(fixed); revert attached to the nearest op, not the one provable match (fixed). 219 downgraded
children's migration then deleted their now-unreferenced anchors (owner-flagged data loss); restored
every anchor exactly as on main, relaxed the anchor-usage check.

**Round 4** (owner-authorized, final, 298493d, 2 P2s + a self-grep): a scheduled revert with a
non-literal delay merged silently with no `revert_after_ms` — fixed to fail closed (new
`BLOCKED_SCHEDULED_REVERT_DELAY`). Direct `Position.revertItem(...)` was shadowed by the generic
`REVERT_METHOD` match — fixed by checking the specific form first.

**Round 5** (owner-authorized, a799e3a4, merge + 2 P2s): `origin/main` advanced to `00691b5b`
(unrelated #1067, condition-override resolution, not D37/D38). Merged (merge commit, no rebase); 3
conflicts resolved: README (both rows kept), `verify_quest_schema.py` (both test sections kept),
`manifest.json` counts (recomputed via `build()`'s own logic, proven to match — not hand-merged).
Anchor set re-verified 313/313 vs main. A second scheduled revert on an already-reverted op silently
overwrote `revert_after_ms` — fixed: `revert()` blocks (new `BLOCKED_DUPLICATE_SCHEDULED_REVERT`).
`addEvent(Position.revertItem, 0, ...)` passed as "literal" — fixed: non-positive delays block too.

**Round 6** (owner-authorized, 0cb1da17, 4 P2s, one class: under-recognized shapes typed instead of
blocked): matchers searched `raw` (keeps strings), so `player:say("Game.createItem(2793)")` emitted a
CREATE — fixed: gated on `code` first (4120202014). A multi-line call left empty args, emitting an
unanchored CREATE — fixed: `call_complete()` requires the list closed on that line, else
`BLOCKED_INCOMPLETE_CALL` (4120202000). `Game.createItem(id, 5)` (count/subtype) was not a recognized
reward — fixed: any positionless constructor of any arity is one (4120202007). A literal-position
REMOVE receiver dropped its position — fixed: binds an anchor, computed stays blocked (4120202022).
Invariant added in code and here: not recognized EXACTLY = BLOCKED. 8 new cases. Samples unchanged:
0 typed, 800/891 blocked, 313 anchors.

## Implementation / findings

- `interaction.schema.json`: `$defs.relocation_target`; D37/D38 typed shapes; `to_source_line` optional.
- `ots_interactions.py`: `children()` types only an exact literal argument, via `split_args`/`argument`/
  `call_complete()` (new: requires a call's own closing paren on that line); every D38 matcher gated
  on `code` first; `same_target()`/`revert()`/`append_children()`/`strip_internal()`/`parse_revert()`.
- `validate_quest_content.py`: `BLOCKED_SCHEDULED_REVERT_DELAY`/`BLOCKED_DUPLICATE_SCHEDULED_REVERT`/
  `BLOCKED_INCOMPLETE_CALL`/`BLOCKED_REASONS` (set per owner); anchor-usage no longer errors on an
  unreferenced anchor.
- `verify_quest_schema.py`: `run_converter`/`converter_case` harness, 43 converter-level cases; merged
  main's own override/parser test section (kept, unmodified).
- `samples/interactions/{interactions,manifest}.json`: anchors/manifest counts equal main's; 0 typed.
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`, `README.md`: counts/rows updated.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 231/231 (156 schema-fixture + 43 converter regression + 5 override + 27 wiki-parser,
  last two from `origin/main`'s own kept-unmodified block)

### Component/integration
- command/run: `python3 validate_quest_content.py` with all flags over the full corpus (chests,
  doors, questlog, interactions samples)
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE

### Regeneration / non-regression
- Round 3/correction: anchor sets equal main per interaction (313/313, 0 added), 0/1,221 differ
  outside Movement/WorldObject + restored anchors.
- Round 4-6 (code-only, no source access): samples byte-identical; anchor set 313/313, 0 typed.
  Round 5 additionally recomputed every `manifest.json` counts field, proven to match. Idempotent.

### Exact-head CI
- final head: pending (set at freeze)
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending (set at freeze)
- method/reviewer: implementing agent (this session)
- material findings: a schema `oneOf` gap (fixed). Rounds 1-2 — 7 P2s, root-caused to substring/
  list-order heuristics. Round 3 — 3 more, plus an owner-flagged data loss (deleted pre-existing
  anchors). Round 4 — 2 more (silent non-literal-delay merge, receiver-match shadow). Round 5 — an
  unrelated main merge (3 conflicts, none hand-merged) plus 2 more P2s. Round 6 — 4 more, one class
  (under-recognized shapes typed instead of blocked). All fixed, tested; an explicit invariant added.
- verdict: no other material findings.

## Independent review

- required: YES — changes an accepted schema
- exact head/method/findings: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: round 6 addresses the last 4 P2s
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- merge from main: 13efb4f, 45b6cc7 (empty diff both times), 00691b5b (round 5, #1067, 3 conflicts
  resolved, no hand-merged data). Round 6: main advanced one more (13576c44, #1064, no owned-path
  overlap) — not merged, not requested.
- ownership release: pending — real D37/D38 classification is excluded scope, not a blocker.

## Context checkpoint

```yaml
last_progress: round 6 done (see "Repair rounds"): fixed 4 structural P2s (code-gate every D38
  matcher, block an incomplete multi-line call, recognize any-arity reward constructors, bind a
  literal-position REMOVE receiver). Samples unchanged (no argument-level evidence to reclassify).
  231/231 verify_quest_schema.py, 0 validate_quest_content.py errors, both governance validators PASS.
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
repair_cycles_for_current_gate: 7
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none for this PR; a future checkout-capable session runs the converter and
  commits the real D37/D38 delta as follow-up work
blocker: null (real-corpus classification is excluded scope, not a blocker here)
next_action: push, report READY_FOR_INTEGRATION with head SHA
```

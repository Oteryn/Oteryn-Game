# OTV2-20260928-quest-relocation-worldobject-transcription

```yaml
task_id: OTV2-20260928-quest-relocation-worldobject-transcription
title: D37 relocation and D38 world-object overlay transcription (Queen of the Banshees worked example)
mode: IMPLEMENT
status: blocked
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
updated_at: 2026-09-28T02:15:00Z
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
relocation and D38 overlay-operation data, so CW4's overlay runtime has real, schema-valid input to
execute, with the Queen of the Banshees as the worked example (issue #162 comment 5860305453).

**Delivered:** the schema, converter and synthetic-fixture tests are complete and correct for both
D37 and D38, typing a child only for an exact, fully-delimited literal argument (never a substring,
name or list-order heuristic) — verified by 177 regression cases, incl. every exact Codex example
across three review rounds.

**Blocked on the committed corpus:** the transcription never recorded the raw argument text behind
each Movement/WorldObject child, only its resolved shape. Three rounds progressively proved that
shape alone cannot verify the current strict rule, so each round's newly-unprovable bucket was
conservatively downgraded to blocked rather than risk a false positive: **all 800 Movement and all
891 WorldObject children are blocked today.** A fresh run against real source (this worker's
`add_repo` for the pinned checkouts was denied) would type most of them correctly first pass — the
converter is ready. See "Exact delta" below.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, §6.6, §6.7 — D36 (`PROVEN`).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3 (D37),
  §4 (D38) — CANDIDATE, decided 2026-09-27; contract text awaits review (not touched here).
- Counts (`PROVEN`): interactions 1,221; Movement 800 blocked; WorldObject 891 blocked; 0 typed.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation, authority fence, PREPARE/COMMIT or
recovery evidence involved.

## Acceptance criteria

- [x] A Movement/WorldObject child is typed only for an exact, fully-delimited literal argument (an
  anchor, the bare previous-position variable, a literal `createItem` id/position, a provably
  same-target revert); anything unverifiable stays blocked.
- [ ] Real D37/D38 data on the committed corpus. **Not met**: three rounds proved the committed shape
  carries no argument-level evidence; schema/converter/tests are ready for a run with real source.
- [x] CREATE/REMOVE bind an anchor only from a literal position, by argument structure
  (`createItem(itemId, count/subtype, position)`); RETAG carries no action-id field.
- [x] A revert attaches only to the one candidate (among all preceding ops) it provably matches; no
  match or more than one equally plausible match stays blocked.
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus.
- [x] `verify_quest_schema.py`: 177/177 (27 converter-level regression cases, three rounds).
- [x] Deterministic regeneration: idempotent migrations; each round's programmatic diff confirms only
  the intended owner's children (and round 3's orphaned anchors) changed — other samples byte-
  identical to `origin/main` throughout.

## Excluded scope

- Real per-instance D37/D38 classification on the committed corpus (see Exact delta).
- `content/**`, any Rust, protocol, DUR-03/quest-state runtime, D39-D42 reward chests, the proposal's
  own contract text/review: untouched, per allocation.
- SCOPE_HANDOFF: no corpus instance signals it (the source servers have no channel concept).

## Exact delta to unblock

A worker/session with `opentibiabr/canary` at `47dfd51f` and `zimbadev/crystalserver` at `ff7ede59`
checked out can complete D37/D38 with no further code change: run `ots_interactions.py --canary
<checkout> --crystal <checkout>` (types only an exact literal argument structure; else blocked, never
guessed), diff against `samples/interactions/*.json`, commit. Tests cover every rule.

## Repair rounds (Codex reviews; all fixed, tested, re-validated; round 3 merged `origin/main`
13efb4f first — no overlap with owned paths, see PR/closeout)

**Round 1** (6d69ccd, 3 P2s): a revert produced its own standalone TRANSFORM; `createItem(id,
Position(...))` dropped the position; a reward constructor also produced a WorldObject CREATE.

**Round 2** (dde3b43e, 4 P2s; root cause: substring/list-order heuristics): the previous-tile check
matched any expression merely referencing `fromPosition`; a revert attached by list order alone; a
`createItem` id was captured from a non-delimited expression; the position argument used an
`(?i)position` name heuristic. The 209 previously-typed previous-tile children, unprovable from the
committed shape, were downgraded to blocked.

**Round 3** (FINAL per owner authorization for this round, 199d3999, 3 P2s; same root cause, now in
arguments rounds 1-2's own fixes still searched too broadly): (1) `teleportTo`'s anchor check
searched the whole statement for a literal `Position`, so `teleportTo(toPosition or Position(1,2,7))`
typed as `(1,2,7)` — fixed by splitting the argument list and requiring the *first* argument itself
to fully match; the 219 previously-typed anchor children, equally unprovable from the committed
shape, are downgraded too, and 205 orphaned anchors removed from `anchors[]` (kept when still used,
e.g. by a summon). (2) a revert attached to the nearest preceding operation only, so
`wall1:transform`, `wall2:transform`, `wall1:decay()` wrongly attached to wall2 — fixed by searching
every preceding operation for the one `same_target` matches; >1 equally plausible match also blocks
now. (3) a revert's position was found anywhere in its own argument list, so `toPosition +
Position(1,2,7)` matched — fixed by requiring the split position argument itself to fully match. 6
new regression cases, incl. each exact example.

## Implementation / findings

- `interaction.schema.json`: `$defs.relocation_target`; typed D37 relocate child; D38 operation
  shapes (`revert_after_ms` optional; `RETAG` no id field); `to_source_line` optional.
- `ots_interactions.py`: `children()` types only an exact literal argument structure, parsed with
  `split_args`/`argument` throughout; `same_target()`/`revert()` (searches every candidate, requires
  exactly one match)/`append_children()`/`strip_internal()`.
- `validate_quest_content.py`: `BLOCKED` reasons refreshed; blocked-reason check scoped to
  `status == 'blocked'`; anchor-usage check reads a relocate child's `target.anchor`.
- `verify_quest_schema.py`: `run_converter`/`converter_case` harness, 27 converter-level cases.
- `samples/interactions/{interactions,manifest}.json`: migrated each round. Final: Movement 800 → 0
  typed/800 blocked; WorldObject 891 → 0 typed/891 blocked.
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`: §3.3, §6.3 counts, Banshees paragraph.
- `README.md`: updated rows.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 177/177 (34 D37/D38 schema cases + 27 converter-level regression cases)

### Component/integration
- command/run: `python3 validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json --progress samples/questlog/progress.json --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json`
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE (offline content tooling)

### Regeneration / non-regression
- Round 3 full-corpus diff (stripping Movement children): 0/1,221 interactions differ outside
  Movement; every anchor removed from `anchors[]` is confirmed (scanning the new rules) to have lost
  its only consumer, never one still in use. chests/doors/questlog/readiness untouched all 3 rounds
  (`ots_readiness.py` reproduces its file byte-identically — its counts key only on owner name).
- All three migrations are idempotent (re-running finds nothing left to change, writes nothing).

### Exact-head CI
- final head: pending (set at freeze)
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending (set at freeze)
- method/reviewer: implementing agent (this session)
- material findings: a schema `oneOf` gap (fixed). Round 1 — 3 Codex P2s. Round 2 — 4 Codex P2s,
  root-caused to substring/list-order heuristics. Round 3 — 3 Codex P2s, the same root cause
  recurring in arguments rounds 1-2's own fixes still searched too broadly. All fixed and tested.
- verdict: no other material findings; each round's fix now parses the exact argument, never the
  statement around it.

## Independent review

- required: YES — changes an accepted format's committed samples/schema
- exact head / method / material findings / verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending (round 3 addresses 3 P2s, final round per owner)
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- merge from main: 13efb4f (unrelated CW2/CW3/CW4/monster-authoring work) merged before round 3's
  commit; no overlap with owned paths, verified empty diff.
- ownership release: pending — real D37/D38 classification is residual scope for a checkout-capable
  worker/session (Exact delta above).

## Context checkpoint

```yaml
last_progress: repair round 3/FINAL (3 P2s fixed — teleport anchor and revert-position matching now
  parse the exact argument, not the statement; revert search considers every preceding candidate;
  the 219 previously-typed anchor children, unprovable like round 2's previous-tile ones, downgraded
  to blocked, orphaned anchors removed). All D37/D38 on the committed corpus is now blocked; the
  schema/converter/tests are complete and ready for a run with real source.
status: blocked
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
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: a checkout-capable worker/session must run ots_interactions.py and commit
  the real D37/D38 delta
blocker: per-instance classification needs the pinned source checkouts; add_repo for
  canary/crystalserver was denied (Untrusted Code Integration)
next_action: push round 3, report new head SHA/counts to the coordinator
```

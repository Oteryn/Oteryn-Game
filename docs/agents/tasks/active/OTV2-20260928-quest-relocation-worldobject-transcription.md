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
updated_at: 2026-09-28T01:30:00Z
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

**Delivered:** D37 relocation to a named anchor is typed on the whole corpus (219 children). Every
other Movement child — a computed target, or one that only references (not is) the previous-position
variable — stays blocked with a reason naming the accepted owner (581, round 2's stricter rule).

**Blocked:** D38 per-instance classification needs the exact, fully-delimited argument structure of
each WorldObject call (and, for previous-tile relocation, of `teleportTo`'s own argument). The
committed transcription never recorded that (only `source_line`); it needs a fresh
`ots_interactions.py` run against the pinned Canary (`47dfd51f`)/CrystalServer (`ff7ede59`) checkouts,
neither present here. `add_repo` for both was denied ("Untrusted Code Integration"); per that denial,
no workaround was attempted. See "Exact delta" below.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, §6.6, §6.7 — D36 (`PROVEN`).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3 (D37),
  §4 (D38) — CANDIDATE, decided 2026-09-27; contract text awaits independent review (not touched here).
- Counts (`PROVEN`, from this branch's samples): interactions 1,221; Movement 800 (219 anchor / 581
  blocked); WorldObject 891 (0 classified, all blocked pending re-transcription).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation, authority fence, PREPARE/COMMIT or
recovery evidence involved.

## Acceptance criteria

- [x] D37: a Movement child is typed only for an exact, fully-delimited literal target (an anchor
  position, or the bare previous-position variable); anything else stays blocked.
- [ ] D38: WorldObject children classified into `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`. **Not met on the
  committed corpus** (checkout denied); schema, converter and synthetic-fixture tests are ready.
- [x] CREATE/REMOVE bind a pre-authored anchor only from a literal position, decided by argument
  structure (`createItem(itemId, count/subtype, position)`), never a name/order heuristic; RETAG
  carries no action-id field (coordinator decision 1c).
- [x] A revert attaches only by provable same-target identity, never by list order alone.
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus.
- [x] `verify_quest_schema.py`: 171/171 (21 converter-level regression cases across both rounds).
- [x] Deterministic regeneration: idempotent migrations; programmatic diffs confirm only the intended
  owner's children changed each round — other samples byte-identical to `origin/main` throughout.

## Excluded scope

- Real per-instance D38 operation classification on the committed corpus (see Blocked/Exact delta).
- `content/**` (incl. `content/interactions/**`), any Rust, protocol, DUR-03/quest-state runtime,
  D39-D42 reward chests, the proposal's own contract text/review: untouched, per allocation.
- SCOPE_HANDOFF: no corpus instance signals it (the source servers have no channel concept).

## Exact delta to unblock D38

A worker/session with `opentibiabr/canary` at `47dfd51f` and `zimbadev/crystalserver` at `ff7ede59`
checked out can complete D38 with no further code change: run `ots_interactions.py --canary <checkout>
--crystal <checkout>` (types only an exact, fully-delimited literal argument structure; else blocked,
never guessed), diff against `samples/interactions/*.json`, commit. Tests cover the shapes/rules.

## Repair rounds (Codex reviews; both fixed, tested, re-validated)

**Round 1** (6d69ccd, 3 P2s, `ots_interactions.py` ~307-313): a revert produced its own standalone
TRANSFORM instead of attaching to the operation it reverts; `createItem(id, Position(...))` dropped the
position; a reward constructor also produced a WorldObject CREATE. Fixed with converter-level synthetic
regression fixtures (`run_converter`, hand-written Lua through the real `Script` class, no real corpus).
Code/tests only; samples unchanged.

**Round 2** (FINAL, dde3b43e, 4 P2s; root cause: substring/list-order heuristics, not exact argument
structure): (1) the previous-tile check (`'fromPosition' in raw`) matched any expression merely
referencing `fromPosition`, not just the bare variable — fixed with `TELEPORT_PREVIOUS`, an exact
`teleportTo(fromPosition[, non-positional args])` pattern; the committed data recorded no argument
evidence for the 209 previously-typed cases (only that `fromPosition` appeared somewhere), so none can
be proven under the strict rule — all 209 downgrade to blocked (a conservative floor, not a re-derived
true count; see Exact delta). (2) a revert attached to whatever came before it by list order alone —
fixed with `same_target()`: attach only when the revert's own receiver or literal position matches the
candidate operation's identity (its receiver, or a constructor's local name) or pre-authored anchor;
internal `_identity`/`_revert_*` keys are stripped (`strip_internal`) before commit. (3) a `createItem`
id was captured from a non-delimited expression (`2793 + offset`) — fixed by parsing the argument list
(`split_args`/`argument`) and requiring the first argument to fully match a bare integer. (4) the
position argument was found by an `(?i)position` name heuristic — fixed by deciding per the engine
signature `createItem(itemId, count/subtype, position)`: a bare integer extra arg is the count/subtype,
a literal `Position(x,y,z)` is the anchor, anything else that could be a placement blocks the child. 11
new regression cases, incl. each exact Codex example.

## Implementation / findings

- `interaction.schema.json`: `$defs.relocation_target`; typed D37 relocate child; D38 operation shapes
  (`revert_after_ms` optional; `RETAG` no id field); `to_source_line` optional again (round 2's
  downgraded children have none recorded).
- `ots_interactions.py`: `children()` types only an exact literal argument structure (§ Repair rounds);
  new `same_target()`/`revert()`/`append_children()`/`strip_internal()`/`split_args()`; manifest gained
  `relocation_children`/`overlay_operations(_with_revert_after)`.
- `validate_quest_content.py`: `BLOCKED` reasons refreshed; blocked-reason check scoped to
  `status == 'blocked'`; anchor-usage check reads a relocate child's `target.anchor`.
- `verify_quest_schema.py`: interaction fixture + D37/D38 schema cases; `run_converter`/`converter_case`
  harness for 21 converter-level cases across both rounds.
- `samples/interactions/{interactions,manifest}.json`: migrated, verified programmatically each round.
  Final: Movement 800 → 219 anchor-relocate + 581 blocked; WorldObject 891 → all blocked, 0 reclassified.
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`: §3.3 rows, §6.3 counts, Banshees paragraph
  (20 interactions, 19 Movement → 9 anchor/10 blocked, 27 WorldObject → 0/27).
- `README.md`: updated rows.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 171/171 (34 D37/D38 schema cases + 21 converter-level regression cases)

### Component/integration
- command/run: `python3 validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json --progress samples/questlog/progress.json --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json`
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE — offline content tooling, no running server.

### Regeneration / non-regression
- Round 2 full-corpus diff (stripping Movement children): 0/1,221 interactions differ outside
  Movement; chests/doors/questlog/readiness samples untouched both rounds (`ots_readiness.py`
  reproduced its file byte-identically — its counts key only on owner name, unaffected by shape).
- Both migrations are idempotent (re-running finds nothing left to change, writes nothing, sha256 same).

### Exact-head CI
- final head: pending (set at freeze)
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent (this session)
- material findings: original round — a schema `oneOf` gap (fixed). Round 1 — 3 Codex P2s (fixed).
  Round 2 — 4 Codex P2s, root-caused to one pattern (substring/list-order heuristics), fixed with exact
  argument-structure parsing and identity-based revert association.
- verdict: no other material findings.

## Independent review

- required: YES — changes an accepted candidate format's committed samples/schema
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending (round 2 addresses 4 P2s, final round per coordinator)
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending — D38 real-corpus classification is residual scope for a checkout-capable
  worker/session (Exact delta above).

## Context checkpoint

```yaml
last_progress: repair round 2/FINAL (4 D38/D37-classifier P2s fixed — exact argument-structure parsing
  replaces substring/list-order heuristics; 209 previous-tile Movement children conservatively
  downgraded to blocked, unprovable from committed data); D38 real-corpus classification still blocked
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: a checkout-capable worker/session must run ots_interactions.py and commit
  the real D38/D37 classification delta
blocker: per-instance argument-structure classification needs the pinned source checkouts; add_repo
  for canary/crystalserver was denied (Untrusted Code Integration)
next_action: push repair round 2, report new head SHA/counts to the coordinator
```

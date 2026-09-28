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
updated_at: 2026-09-28T00:41:00Z
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

**Delivered in full:** D37 relocation is complete on the whole corpus. Every Movement child whose
target was already on record (an anchor or the previous tile) is now a typed
`{owner: Movement, request: relocate, scope: in_scope, target: {...}}` child; a genuinely computed
target stays blocked with a refreshed reason (an owner exists now; only the target is unresolved).

**Blocked:** D38 per-instance classification (which of `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`) needs the
specific Lua call each WorldObject child came from. The committed transcription never recorded that
(only `source_line`); it needs a fresh `ots_interactions.py` run against the pinned Canary (`47dfd51f`)
/ CrystalServer (`ff7ede59`) checkouts, neither present here. `add_repo` for both was denied by the
permission system ("Untrusted Code Integration"); per that denial, no workaround was attempted. See
"Exact delta to unblock" below.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, §6.6, §6.7 — D36 (`PROVEN`).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3 (D37),
  §4 (D38) — CANDIDATE, decided 2026-09-27; contract text awaits independent review (not touched here).
- Counts (`PROVEN`, computed from the samples in this branch): interactions 1,221; Movement 800 (219
  anchor / 209 previous tile / 372 computed); WorldObject 891 (0 classified, pending re-transcription).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation, authority fence, PREPARE/COMMIT or
recovery evidence involved.

## Acceptance criteria

- [x] D37: every Movement child with a known target is a typed relocation child; a computed target
  stays blocked, reason naming the accepted owner. Evidence: `manifest.json` `relocation_children`.
- [ ] D38: WorldObject children classified into `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` with
  `revert_after_ms`. **Not met on the committed corpus** (checkout denied); schema, converter and
  synthetic-fixture tests (incl. repair round 1's converter-level cases) are ready for the next run.
- [x] CREATE/REMOVE only reference a pre-authored anchor key (never invented coordinates, incl. repair
  round 1's non-literal-position case); RETAG carries no action-id field (coordinator decision 1c).
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus.
- [x] `verify_quest_schema.py`: 160/160 (repair round 1 added 10 converter-level cases).
- [x] Deterministic regeneration: migration is idempotent; full-corpus diff shows only Movement/
  WorldObject children changed — chests/doors/questlog/readiness byte-identical to `origin/main`.

## Excluded scope

- Real per-instance D38 operation classification on the committed corpus (see Blocked/unblock delta).
- `content/**` (incl. `content/interactions/**`), any Rust, protocol, DUR-03/quest-state runtime,
  D39-D42 reward chests, the proposal's own contract text/review: untouched, per allocation.
- SCOPE_HANDOFF: no instance in the corpus signals it (the source servers have no channel concept).

## Exact delta to unblock D38

A worker/session with `opentibiabr/canary` at `47dfd51f` and `zimbadev/crystalserver` at `ff7ede59`
checked out can complete D38 with no further code change: run `ots_interactions.py --canary <checkout>
--crystal <checkout>` (emits typed `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`, literal `from`/`to`/`def`/
`revert_after_ms` where the call is literal, else `value_source_line`/blocked), diff against this
branch's `samples/interactions/*.json`, commit. Tests already cover it.

## Repair round 1 (Codex review on 6d69ccd, 3 P2s on ots_interactions.py ~307-313)

All verified real and fixed, with converter-level synthetic regression fixtures (`run_converter`,
`verify_quest_schema.py`; runs hand-written Lua through the real `Script` class — no real corpus):
1. A revert call (`decay`/`revertItem`/`addEvent(Position.revertItem,…)`) produced its own standalone
   TRANSFORM. Fixed: `children()` emits an internal `_revert` marker (literal delay only from the
   well-known `addEvent(Position.revertItem, delay, …)` argument position, else none); `convert()`'s
   new `revert()`/`append_children()` attach it as `revert_after_ms` on the preceding typed WorldObject
   op in the same statement list, or leave a blocked child (never a standalone TRANSFORM) if none precedes.
2. `Game.createItem(id, Position(...))` dropped the position. Fixed: a literal position in the tail
   binds `self.anchor(...)`; a position-shaped non-literal tail (e.g. `toPosition`) keeps it blocked
   (C3: no dynamic geometry); no position mentioned is unchanged.
3. `local reward = Game.createItem(id)` (a `self.created_items` reward constructor) also produced a
   WorldObject CREATE. Fixed: excluded when the line matches `CREATE_ITEM` and its local is in
   `self.created_items`; a call with an explicit position is never in `self.created_items`, so a
   genuine world placement assigned to a local is unaffected.

`git diff --stat` on `samples/interactions/*` against 6d69ccd is empty — code/tests only.

## Implementation / findings

- `interaction.schema.json`: `$defs.relocation_target`; typed D37 relocate child; D38
  `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` (`revert_after_ms` optional; `RETAG` carries no id field); a
  narrowed blocked branch per owner (computed target / unclassified call).
- `ots_interactions.py`: `children()` emits typed shapes from source signals already read; repair
  round 1 added `revert()`/`append_children()` (§ Repair round 1) and refined CREATE/constructor
  detection; `build()`'s manifest gained `relocation_children`/`overlay_operations(_with_revert_after)`.
- `validate_quest_content.py`: `BLOCKED` reasons refreshed (an owner exists now); blocked-reason check
  scoped to `status == 'blocked'`; anchor-usage check reads a relocate child's `target.anchor`.
- `verify_quest_schema.py`: rebuilt the interaction fixture (one typed + one still-blocked child per
  owner) with D37/D38 schema cases; repair round 1 added `run_converter`/`converter_case`, a harness
  that runs hand-written synthetic Lua through the real `Script` class for 10 converter-level cases.
- `samples/interactions/{interactions,manifest}.json`: migrated (programmatically verified — see
  Validation). Movement 800 → 219 anchor-relocate + 209 previous-tile-relocate + 372 still blocked;
  WorldObject 891 → reason refreshed, 0 reclassified (see Blocked). Unchanged by repair round 1.
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`: §3.3 rows, §6.3 counts, Banshees paragraph
  (20 interactions, 19 Movement → 9/9/1, 27 WorldObject → 0/27).
- `README.md`: updated the `ots_interactions.py`/`interaction.schema.json` rows.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 160/160 (34 D37/D38 schema cases + 10 converter-level regression cases, repair round 1)

### Component/integration
- command/run: `python3 validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json --progress samples/questlog/progress.json --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json`
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE — offline content tooling, no running server.

### Regeneration / non-regression
- Full-corpus programmatic diff (stripping Movement/WorldObject children): 0 of 1,221 interactions
  differ outside those two owners; chests/doors/questlog/readiness samples untouched (`ots_readiness.py`
  reproduced its file byte-identically — its counts key only on owner name, unaffected by shape).
- The migration is idempotent (re-running asserts immediately, writes nothing, sha256 unchanged).
- Repair round 1: `git diff --stat` on `samples/interactions/*` against 6d69ccd is empty (code-only fix).

### Exact-head CI
- final head: c916c5668dab28307afd3fcfbe84f23abf533c97
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending (set at freeze)
- method/reviewer: implementing agent (this session)
- material findings: original round — `oneOf` between `value_source_line` and literal fields needs
  `value_source_line` popped before a literal update, else both alternatives validate (fixed, tested).
  Repair round 1 — 3 Codex P2s on the D38 classifier, all verified real and fixed (see § Repair round 1).
- verdict: no other material findings in either round.

## Independent review

- required: YES — changes an accepted candidate format's committed samples and schema
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending (repair round 1 addresses 3 P2s)
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending — D38 real-corpus classification is residual scope for a checkout-capable
  worker/session ("Exact delta to unblock").

## Context checkpoint

```yaml
last_progress: repair round 1 (3 D38-classifier P2s fixed, converter-level regression tests added);
  D37 complete; D38 real-corpus classification still blocked on denied source-checkout access
status: blocked
branch: claude/quest-relocation-worldobject-transcription
head_sha: c916c5668dab28307afd3fcfbe84f23abf533c97
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: a checkout-capable worker/session must run ots_interactions.py and commit
  the real D38 classification delta
blocker: D38 classification needs the pinned source checkouts; add_repo for canary/crystalserver
  was denied (Untrusted Code Integration)
next_action: push repair round 1, report new head SHA/validation counts to the coordinator
```

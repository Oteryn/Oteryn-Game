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

**Blocked:** D38 per-instance classification (which of `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`, and any
literal `from`/`to`/`def`/`revert_after_ms`) needs the specific Lua call each WorldObject child came
from. The committed transcription never recorded that (only `source_line`), so it cannot be re-derived
from the committed samples alone; it needs a fresh `ots_interactions.py` run against the pinned Canary
(`47dfd51f`) / CrystalServer (`ff7ede59`) checkouts, neither present in this sandbox. `add_repo` for
both was denied by the permission system ("Untrusted Code Integration"); per that denial, no workaround
was attempted. See "Exact delta to unblock" below.

## Architecture and source of truth

- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §3.3, §6.3, §6.6, §6.7 — D36 transcription
  (`PROVEN`, accepted).
- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3 (D37),
  §4 (D38) — CANDIDATE, owner decisions D37/D38 taken 2026-09-27; contract text awaits independent
  review (unrelated to this data-transcription task; not touched here).
- Counts in this task (`PROVEN`, computed from the committed/regenerated samples in this branch):
  interactions 1,221; Movement 800 (219 anchor / 209 previous tile / 372 computed); WorldObject 891
  (0 classified / 891 blocked pending re-transcription).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
```
Offline content transcription tooling; no production mutation, authority fence, PREPARE/COMMIT or
recovery evidence involved.

## Acceptance criteria

- [x] D37: every Movement child with a known target (anchor or previous tile) is a typed relocation
  child; a computed target stays blocked with a reason naming the accepted owner. Evidence:
  `samples/interactions/interactions.json`, `manifest.json` counts (`relocation_children`).
- [ ] D38: WorldObject children are classified into `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG` with
  `revert_after_ms` where the source schedules a revert. **Not met on the committed corpus** (source
  checkout unavailable/denied); schema, converter and synthetic-fixture tests are ready for the next
  run that has the checkouts.
- [x] CREATE/REMOVE only reference a pre-authored anchor key (never invented coordinates); RETAG
  carries no action-id field (coordinator decision 1c).
- [x] Value-bearing removals stay DUR-03 consumption; the 43 creature removals stay unresolved (C4) —
  unchanged, verified by the full-corpus diff (see Implementation).
- [x] Queen of the Banshees worked example documented in §6.3 with exact before/after counts.
- [x] 0 schema/semantic validator errors on the full regenerated corpus.
- [x] `verify_quest_schema.py`: 150/150 synthetic cases pass (34 new D37/D38 cases added).
- [x] Deterministic regeneration: re-running the migration is idempotent (asserts on its own new-shape
  output and writes nothing); the full-corpus diff shows only Movement/WorldObject children changed —
  `samples/chests/**`, `samples/doors/**`, `samples/questlog/**` and `samples/readiness/**` are
  byte-identical to `origin/main`.

## Excluded scope

- Real per-instance D38 operation classification on the committed corpus (see "Blocked" above and
  "Exact delta to unblock").
- `content/**` (including `content/interactions/**`), any Rust, protocol, DUR-03/quest-state runtime,
  D39-D42 reward chests: untouched, per allocation.
- The relocation/overlay proposal's own contract text and independent review: unrelated to this data
  task, not touched.
- SCOPE_HANDOFF (cross-Channel/Instance relocation): no instance in the corpus signals it (the source
  servers have no channel concept); nothing here classifies a case that cannot currently be identified.

## Exact delta to unblock D38

A worker/session with a checked-out (or otherwise readable) `opentibiabr/canary` at `47dfd51f` and
`zimbadev/crystalserver` at `ff7ede59` can complete D38 with no further code changes beyond this
branch: run `python ots_interactions.py --canary <checkout> --crystal <checkout>` (already emits typed
`TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`, with literal `from`/`to`/`def` where the call has a plain
literal argument, else `value_source_line`), diff against this branch's `samples/interactions/*.json`,
commit the delta. `verify_quest_schema.py`/`validate_quest_content.py --interactions` already cover it.

## Implementation / findings

- `interaction.schema.json`: added `$defs.relocation_target`; replaced the Movement/WorldObject
  blocked-only shapes with the typed D37 relocate child, the D38 `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`
  operations (`revert_after_ms` optional on all four; `RETAG` carries no id field), and a narrowed
  blocked branch per owner (computed target / unclassified call).
- `ots_interactions.py`: `children()` emits the typed shapes directly from source signals already read
  (anchor/`fromPosition` for Movement; which call matched for WorldObject); `build()`'s manifest gained
  `relocation_children` and `overlay_operations(_with_revert_after)` counts.
- `validate_quest_content.py`: `BLOCKED` reasons refreshed (an owner exists now; what's blocked is
  narrower); the blocked-reason check only applies to `status == 'blocked'` children; the anchor-usage
  check also reads a relocate child's `target.anchor`.
- `verify_quest_schema.py`: rebuilt the interaction fixture to carry one typed and one still-blocked
  child per owner; added D37/D38 cases (target kinds, scope closed, blocked-reason mismatch, each
  operation's literal-vs-evidence `oneOf`, `revert_after_ms` bounds, anchor usage). 150/150 pass.
- `samples/interactions/interactions.json` / `manifest.json`: migrated by a script equivalent to
  `children()`'s own reclassification (verified programmatically — see Validation). Movement 800 →
  219 anchor-relocate + 209 previous-tile-relocate + 372 still blocked (reason refreshed); WorldObject
  891 → reason refreshed, 0 reclassified (see Blocked).
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`: §3.3 rows for `teleportTo` and map-object
  calls now describe D37/D38 instead of "no owner contract yet"; §6.3 counts table gained the D37/D38
  rows; the Queen of the Banshees paragraph gained the worked-example numbers (20 interactions, 19
  Movement → 9/9/1, 27 WorldObject → 0/27).
- `README.md`: updated the `ots_interactions.py`/`interaction.schema.json` rows.

## Validation

### Focused
- command/run: `python3 verify_quest_schema.py`
- result: PASS — 150/150 cases (34 D37/D38-specific)

### Component/integration
- command/run: `python3 validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json --progress samples/questlog/progress.json --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json`
- result: PASS — `{"valid": true, "error_count": 0}`

### E2E
- scenario: NOT_APPLICABLE — offline content tooling, no running server.

### Regeneration / non-regression
- Full-corpus programmatic diff (stripping Movement/WorldObject children before comparing): 0 of
  1,221 interactions differ outside those two owners. `samples/chests/**`, `samples/doors/**`,
  `samples/questlog/**`, `samples/readiness/readiness.json` are untouched (re-running
  `ots_readiness.py` reproduced its file byte-identically; its counts key only on owner name).
- The migration step is idempotent: re-running it against its own output asserts immediately (no
  matching old-shaped children left) and writes nothing (sha256 unchanged).

### Exact-head CI
- final head: c916c5668dab28307afd3fcfbe84f23abf533c97
- trigger source: push
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending (set at freeze)
- method/reviewer: implementing agent (this session)
- material findings: `oneOf` between `value_source_line` and literal fields (`from`/`to`/`def`) in the
  D38 schema — a literal update must first pop `value_source_line`, else both alternatives validate and
  the branch fails; caught by `verify_quest_schema.py`'s new cases, fixed before commit.
- verdict: no other material findings; the converter change is additive to a per-line `elif` chain that
  already matched only a fixed call set, so branch coverage is exhaustive of the prior generic case.

## Independent review

- required: YES — changes an accepted candidate format's committed samples and schema
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending — D38 real-corpus classification is residual scope for a worker/session
  with Canary/CrystalServer checkout access ("Exact delta to unblock").

## Context checkpoint

```yaml
last_progress: D37 fully transcribed on the real corpus; D38 schema/converter/tests ready; D38
  real-corpus classification blocked on denied source-checkout access
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
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: a worker/session with Canary 47dfd51f / CrystalServer ff7ede59 checkout access
  needs to run the updated ots_interactions.py and commit the real D38 classification delta
blocker: D38 per-instance operation-kind classification needs the pinned source checkouts;
  add_repo for opentibiabr/canary and zimbadev/crystalserver was denied (Untrusted Code Integration)
next_action: open the PR with the D37-complete/D38-ready delta and report LANE_BLOCKED with this
  exact residual scope
```

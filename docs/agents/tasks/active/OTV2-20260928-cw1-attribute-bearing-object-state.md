# OTV2-20260928-cw1-attribute-bearing-object-state

```yaml
task_id: OTV2-20260928-cw1-attribute-bearing-object-state
title: Minimal design for attribute-bearing local-object state (destination/interaction)
mode: CONTRACT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-attribute-bearing-object-state
pr: 1099
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
  - docs/agents/tasks/active/OTV2-20260928-cw1-attribute-bearing-object-state.md
public_contracts: []
depends_on:
  - "owner decision recorded on issue #162, 2026-09-28: §7 open decision 3 is YES"
  - "docs/agents/tasks/active/OTV2-20260928-cw1-revert-acceptance.md (task A, PR #1097, sequenced first)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

New §9 in `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`, `DecisionStatus:
CANDIDATE`: a minimal design for local-object state that carries a teleporter `destination`
attribute, so the four `map_item transform`-at-a-pre-authored-anchor authored timed actions §7
currently rejects (the Depth trio, `the_lord_of_the_lice`) become admissible. **Round 2** (Codex P1s
4121918211/4121918220 + P2 4121918234, all accepted, owner stop rule applies) corrected: (a)
`revert_destination` no longer bakes into the placement's natural source state — lowering creates a
distinct post-revert state instead; (b) narrowed scope to the teleporter-transform shapes only —
`interaction`-bearing `death_position` creates (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`) are
NOT covered, added as §7 open decision 8; (c) `local_object_revert_after_ms` keyed by
`(TransitionKey, LoweredActionId)`, not `TransitionKey` alone. Covers, against re-verified code:

1. Where the attributes live: two new optional per-placement fields on `PlacementRef`.
2. How `prepare`/`Publish` carry and apply them: a pure read of `(state_attributes, current state)`,
   like collision presence; no new field on `PreparedMutation::Publish`/`commit`.
3. How the inverse restores them, including corrected `revert_destination` semantics (post-revert
   state, not source-state bake-in) and how §7's unique-inverse rule stays satisfied (Round 3: §7's
   rule itself widened via a new `LocalObjectStateDefinition.attribute_variant_of` field — the
   post-revert state's key never equals the forward transition's own `source_state`, so §7's rule as
   written before this round rejected all four covered samples outright).
4. How the Round 17/19 lowering rejection is lifted for exactly this one named shape.
5. Open decision 4 (§7) resolved, occurrence-keyed.
6. The lifecycle-record fields added: none — explained why.

No code change; this is a documentation-only architecture design. Not owner-accepted as
implementation-ready (§9's own `DecisionStatus: CANDIDATE`).

## Architecture and source of truth

Full file:line evidence lives in the new §9 (Evidence subsection); this is the index.

- `reference_playable.rs` `PlacementRef`/`local_object_initial_state`,
  `validate_local_object_placement_state`/`validate_placement` — PROVEN, re-verified: the
  per-placement-fact + fail-closed-validation precedent this design mirrors.
- `LocalObjectRuntime`/`bind` (incl. `~620-630`'s `PlacementKey` requirement) — PROVEN, re-verified:
  no attribute field today; requires an existing `content.placements` entry, no dynamic-placement.
- `prepare`/`PreparedMutation`/`commit` — PROVEN: collision presence is a pure derived read from
  `(states, target_state)`, never mutable state.
- `TransitionBinding`, `ProductionKey`/`PlacementKey` — PROVEN: shared, no per-invocation payload;
  existing types this design reuses.
- `validate_encounter.py`, encounter-format line 144 — PROVEN, re-verified.
- §4 "Out of scope," C3 (lines 154-162) — PROVEN: excludes a `CREATE` reserving cells not known at
  bind time.
- `LocalObjectStateDefinition` (~805-813) — PROVEN: exactly `key`+`collision`, no relationship field
  between states today; `attribute_variant_of` (Round 3) is genuinely new.
- Seven authored samples, all re-verified file:line: `the_lord_of_the_lice` lines 58-75,
  `the_duke_of_the_depths` lines 53-68, `the_baron_from_below`/`the_count_of_the_core` lines 63-78
  (covered); `mazzinor` lines 47-57, `gaz_haragoth` lines 123-132, `cult_soul_remains` lines 53-62/
  70-79 (NOT covered — §7 open decision 8).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture design; no production mutation, authority grant, PREPARE/COMMIT,
  controller install/restore or persisted recovery evidence is touched. Not itself an implementation.
```

## Acceptance criteria

- [x] §9 names where the attributes live, how `prepare`/`Publish` carry them, corrected
      `revert_destination` semantics (post-revert state, Round 2 P1), how the rejection lifts for the
      covered shape only, open decision 4 resolved/occurrence-keyed (Round 2 P2), and the
      lifecycle-record fields added (none).
- [x] §9 narrowed to teleporter-transform shapes; `interaction`/`death_position` moved to §7 open
      decision 8 (Round 2 P1), with §8/open-decisions cross-references updated.
- [x] §7's unique-inverse rule and §9's post-revert-state inverse are mutually consistent (Round 3
      P1): `bind` accepts the widened rule's `attribute_variant_of` branch for all four covered
      samples, and the widened rule is inert for ordinary content.
- [x] Every claim verified against code this task (not carried over from memory).
- [x] §9's `DecisionStatus` is `CANDIDATE`.
- [x] Both validators pass.

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Implementing CW3 linker validation, the lowering step, or `LoweredActionId`'s representation.
- Teleportation execution; `interaction` bindings and `death_position` placements (Round 2 exclusion,
  §7 open decision 8 names the gap, does not design a fix).
- A generic attribute system beyond `destination`/`revert_destination`.
- §7's open decisions 1, 2, 5, 6, 7 (task A) and 8 — unaffected, remain the owning lane's.
- `@codex` trigger, PR/issue comments, merge action — coordinator's.

## Implementation / findings

Round 1: branched fresh (`f0710bfb`). Core insight: collision presence is a pure function of
`(states, target_state)` inside `prepare`, never mutable state — attributes reuse the same pattern.
Wrote §9 after §8, untouched §7 (task A's concurrent PR).

Round 22: merged `origin/main` (`9beb6625`, PR #1097), resolved the one expected §8-item-7 conflict.
Fixed Codex P2 4121718203 (reserve-after-`Publish` ordering misattributed to FND-03 §15.4).

Round 2 (head `db49049d`; 2 P1s + 1 P2, owner stop rule): merged `origin/main` (`800e3eb6`) first.
`revert_destination` was baking into the natural source state; fixed via a distinct post-revert
state, natural source untouched. Narrowed §9 to teleporter-transform shapes only (death_position
creates need a `PlacementKey` `bind` lacks and C3 excludes); added §7 open decision 8.
`local_object_revert_after_ms` rekeyed `(TransitionKey, LoweredActionId)`.

Round 3 (PR #1099 Codex, head `d417e860`; 1 P1, thread 4122104484, owner stop rule): merged
`origin/main` (`d17a3826`, unrelated, clean) first. **P1** — Round 2's post-revert state claimed §7's
unique-inverse rule was "satisfied by construction" but never checked the inverse's `target_state`
against the forward transition's *actual* `source_state`: since the post-revert state's key ≠ the
natural source state's key, §7's rule as written (exact equality) finds zero candidates and rejects
all four covered samples. Re-verified §7's rule text directly. Fixed by widening §7's rule itself:
new optional `LocalObjectStateDefinition.attribute_variant_of: Option<ProductionKey>` (content-level,
same `collision`, CW3-validated fail-closed); the search now accepts `target_state` equal to
`source_state` *or* its declared `attribute_variant_of`, uniqueness unchanged. Updated both §7 rule
citations, §9 design point 3, Evidence/PLAYABLE_FIRST-scoping/Open-items, added a Round 23 §7
narrative paragraph and a Round 3 §9-header note. Added test obligations: widened rule accepted for
all four samples; inert for ordinary content; `attribute_variant_of` validated fail-closed. Grepped
for other stale rule citations and markdown defects — none found beyond what was fixed.

Both validators re-run after all edits: PASS (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes."

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 50 workflows)." (round 3, after merging origin/main)

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
- material findings: two markdown formatting defects (heading wrapped across two lines; two
  inline-code spans split mid-identifier) found and fixed in self-review before freeze
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
- related/superseded PRs: sequenced after task A (PR #1097, merged `061b1676`); merged `origin/main`
  three times (round 22 for #1097; rounds 2/3 for `800e3eb6`/`d17a3826`, unrelated, no conflicts)
- protected auto-merge: not requested by this task
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: >
  Round 3 (PR #1099 Codex on d417e860, thread 4122104484): 1 P1 fixed. Section 9's post-revert-state
  inverse (B->C) never satisfied section 7's own unique-inverse rule as written (target_state must
  exactly equal source_state; C != A) -- bind would reject all four covered samples. Fixed by
  widening section 7's rule itself: new optional LocalObjectStateDefinition.attribute_variant_of
  field (content-level, same collision, CW3-validated); inverse search now accepts target_state
  equal to source_state OR its declared attribute_variant_of, uniqueness unchanged. Updated both
  section 7 rule citations, section 9 design point 3, evidence/scoping/open-items, added a Round 23
  section 7 narrative paragraph and new test obligations. Merged origin/main (d17a3826, unrelated)
  first. Validators pass; about to push.
status: ready
branch: claude/cw1-attribute-bearing-object-state
head_sha: null
pr: 1099
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
next_action: coordinator freeze at the reported head + independent review
```

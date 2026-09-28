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
CANDIDATE`: a minimal design for local-object state that carries attributes (teleporter
`destination`/`revert_destination`, `interaction` binding), so the five authored timed actions §7
currently rejects (the Depth trio, `the_lord_of_the_lice`, `mazzinor`, `gaz_haragoth`,
`cult_soul_remains`) become admissible. Covers, against re-verified code:

1. Where the attributes live: two new optional per-placement fields on `PlacementRef`
   (`local_object_state_attributes`, `local_object_revert_after_ms`) — not the shared
   `LocalObjectStateDefinition`/`TransitionBinding`, which cannot represent per-instance variance.
2. How `prepare`/`Publish` carry and apply them: they don't need to — attributes are a pure read of
   `(state_attributes, current state)`, exactly like collision presence already is; no new field on
   `PreparedMutation::Publish`, no new line in `commit`.
3. How the inverse restores them, including `revert_destination` semantics
   (`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md:144`): resolved entirely at lowering time, before `bind`
   ever runs — no lifecycle-record involvement.
4. How the Round 17/19 lowering rejection is lifted for exactly these two named shapes (not a
   generic attribute system).
5. Open decision 4 (§7) resolved as a side effect: `revert_after_ms` and inverse-selection metadata
   live per-placement, not on the shared `TransitionBinding`.
6. The lifecycle-record fields added: none — explained why.

No code change; this is a documentation-only architecture design. Not owner-accepted as
implementation-ready (§9's own `DecisionStatus: CANDIDATE`).

## Architecture and source of truth

Full file:line evidence lives in the new §9 (Evidence subsection); this is the index.

- `apps/game-server/src/content/reference_playable.rs` `PlacementRef` (~1293-1306),
  `local_object_initial_state` — PROVEN, re-verified: the existing per-placement-fact precedent this
  design mirrors.
- `validate_local_object_placement_state`/`validate_placement` (~2093-2132) — PROVEN, re-verified:
  the existing fail-closed both-direction validation precedent.
- `LocalObjectRuntime` (~574-586), `bind` (~590-750) — PROVEN, re-verified: no attribute field today;
  `transitions` map already built per-placement (~679-714).
- `world_runtime.rs` `prepare`/`PreparedMutation`/`commit` (~973-1183) — PROVEN, re-verified:
  collision presence is already a pure derived read from `(states, target_state)`, never separately
  mutable state — the precedent attributes reuse instead of a new payload-application mechanism.
- `TransitionBinding` (~1334-1342) — PROVEN, re-verified: shared, no per-invocation payload; cannot
  represent per-placement `revert_after_ms` variance (§7 open decision 4, Codex 4120487841).
- `content/production.rs` `ProductionKey::new` (~148-166), `PlacementKey` (~1167-1181) — PROVEN,
  re-verified: existing generic content-key and placement-reference types this design reuses instead
  of inventing new ones.
- `tools/content-schema/encounter-authoring/validate_encounter.py` (~188-201) — PROVEN, re-verified:
  `destination`/`revert_destination` validated as anchor references; `effect` untied to state.
- `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144 — PROVEN, re-verified verbatim.
- Five authored samples, all re-verified file:line this task: `the_lord_of_the_lice/encounter.json`
  lines 58-75; `the_duke_of_the_depths/encounter.json` lines 53-68; `the_baron_from_below/
  encounter.json` lines 63-78; `the_count_of_the_core/encounter.json` lines 63-78; `mazzinor/
  encounter.json` lines 47-57; `gaz_haragoth/encounter.json` lines 123-132; `cult_soul_remains/
  encounter.json` lines 53-62 and 70-79.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture design; no production mutation, authority grant, PREPARE/COMMIT,
  controller install/restore or persisted recovery evidence is touched. Not itself an implementation.
```

## Acceptance criteria

- [x] §9 names where the attributes live, with evidence against `reference_playable.rs`/
      `world_runtime.rs`.
- [x] §9 states how `prepare`/`Publish` carry and apply them.
- [x] §9 covers `revert_destination` semantics per encounter-format line 144.
- [x] §9 states exactly how the Round 17/19 rejection is lifted for these two named shapes, not a
      generic system.
- [x] §9 resolves open decision 4 (coupled, per the coordinator).
- [x] §9 states the lifecycle-record fields added.
- [x] Every claim verified against code this task (not carried over from memory).
- [x] §9's `DecisionStatus` is `CANDIDATE`.
- [x] `python3 tools/agents/validate_governance.py` passes.
- [x] `python3 tools/repository/validate_repository_policy.py` passes.

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Implementing the CW3 linker validation or the production lowering step — named as owning-lane
  follow-up.
- Teleportation execution or interaction-domain invocation — explicitly out of scope, unaffected.
- A generic attribute system beyond `destination`/`revert_destination`/`interaction`.
- §7's open decisions 1, 2, 5, 6 (and 7, task A) — unaffected, remain the owning lane's.
- `@codex` trigger, PR/issue comments, merge action — coordinator's.

## Implementation / findings

Branched from fresh `origin/main` (`f0710bfb`, same base task A used, since task A's PR #1097 had
not yet merged — "start task B without waiting" per the coordinator). Re-verified every code claim
directly rather than reusing §7's existing citations by reference: `PlacementRef`/
`local_object_initial_state` and its validator, `LocalObjectRuntime`/`bind`, `prepare`/
`PreparedMutation`/`commit`, `TransitionBinding`, `ProductionKey`/`PlacementKey`, and all seven
sample files (all file:line citations re-confirmed by direct read/grep, not carried over from a
prior round's memory).

Core design insight, verified against `prepare`'s existing handling of collision presence
(`apps/game-server/src/world_runtime.rs` ~973-1050): collision presence is already a pure function of
`(states, target_state)`, computed fresh inside `prepare` and never stored as separate mutable state
on `LocalObjectRuntime`. Attributes (`destination`/`interaction`) can reuse exactly this pattern if
they are made a pure function of `(placement's own state-attribute table, current state)` instead of
something that needs to flow through `PreparedMutation::Publish` or the round-14 lifecycle record.
This is why the design needs zero new `PreparedMutation` fields, zero new `commit` logic and zero new
lifecycle-record fields — the smallest change consistent with PLAYABLE_FIRST.

Resolved `revert_destination` (line 144) as a pure lowering-time concern: since attributes are
per-placement-per-state and static since `bind`, "restore the original" and "apply the
`revert_destination` override" collapse into the same fact once lowering bakes the override into the
source state's own attribute entry — no runtime-level override/restore branch needed.

Resolved open decision 4 as a side effect of design point 1 (per-placement `local_object_
revert_after_ms`, keyed by `TransitionKey`): different placements sharing one content-level
`TransitionBinding` can author independent durations, directly answering Codex 4120487841. The
existing inverse-uniqueness check (§7) is simplified to run against the placement's own already-built
`transitions` map (`bind` ~679-714) instead of a content-level search.

Wrote §9 as a new top-level section after §8 (end of file), avoiding any edit to §7's existing text
(owned by task A's concurrent PR #1097) to prevent a merge conflict between the two sequenced tasks.
Fixed one authoring mistake caught by a markdown-heading-must-be-one-line and inline-code-span
mid-identifier-split self-review (two `this_`/`local_object_state_` splits fixed).

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
- related/superseded PRs: sequenced after task A (`OTV2-20260928-cw1-revert-acceptance`, PR #1097) on
  the same document; expect the coordinator to merge task A first
- protected auto-merge: not requested by this task
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: >
  New §9 added: minimal attribute-bearing local-object state design (destination/revert_destination/
  interaction), all code claims re-verified this task. Two new optional per-placement fields on
  PlacementRef; zero new PreparedMutation/commit/lifecycle-record fields (attributes are a pure read
  of placement-state, mirroring how collision presence already works). revert_destination resolved
  as a lowering-time concern. Open decision 4 resolved as a side effect. Validators pass; about to
  Pushed as 6bd9da69, PR #1099 opened.
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

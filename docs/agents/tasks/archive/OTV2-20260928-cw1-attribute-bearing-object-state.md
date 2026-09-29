# OTV2-20260928-cw1-attribute-bearing-object-state

```yaml
task_id: OTV2-20260928-cw1-attribute-bearing-object-state
title: Minimal design for attribute-bearing local-object state (destination/interaction)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-attribute-bearing-object-state
pr: 1099
base_sha: f0710bfb0513147d6bf573dbbfac591da0dadfa6
head_sha: b2d4deddd221e3b3d60ecafeb3b69cb12ef41447
final_head_sha: b2d4deddd221e3b3d60ecafeb3b69cb12ef41447
final_head_frozen_at: 2026-09-28T12:57:47Z
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
attribute, so `map_item transform`-at-a-pre-authored-anchor authored timed actions §7 currently
rejects (the Depth trio, `the_lord_of_the_lice`, and — Round 4 corpus re-scan — at least sixteen
samples total, not exhaustively enumerated) become admissible. **Round 2** (Codex P1s
4121918211/4121918220 + P2 4121918234, all accepted, owner stop rule applies) corrected: (a)
`revert_destination` no longer bakes into the placement's natural source state — lowering creates a
distinct post-revert state instead; (b) narrowed scope to the teleporter-transform shapes only —
`interaction`-bearing `death_position` creates (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`) are
NOT covered, added as §7 open decision 8; (c) `local_object_revert_after_ms` keyed by
`(TransitionKey, LoweredActionId)`, not `TransitionKey` alone. Covers, against re-verified code:

1. Where the attributes live: two new optional per-placement fields on `PlacementRef`.
2. How `prepare`/`Publish` carry and apply them: a pure read of `(state_attributes, current state)`,
   like collision presence; no new field on `PreparedMutation::Publish`/`commit`.
3. How the inverse restores them, incl. corrected `revert_destination` semantics (post-revert state,
   not source-state bake-in) and how §7's unique-inverse rule stays satisfied (Round 3: widened via
   `LocalObjectStateDefinition.attribute_variant_of`, direction-corrected Round 4).
4. How the Round 17/19 lowering rejection is lifted for exactly this one named shape.
5. Open decision 4 (§7) resolved, occurrence-keyed.
6. The lifecycle-record fields added: none — explained why.

No code change; this is a documentation-only architecture design. Not owner-accepted as
implementation-ready (§9's own `DecisionStatus: CANDIDATE`).

## Architecture and source of truth

Full file:line evidence lives in the new §9 (Evidence subsection); this is the index.

- `reference_playable.rs` `PlacementRef`/`local_object_initial_state`,
  `validate_local_object_placement_state`/`validate_placement` — PROVEN: the per-placement-fact +
  fail-closed-validation precedent this design mirrors.
- `LocalObjectRuntime`/`bind` (`~620-630`'s `PlacementKey` requirement) — PROVEN: no attribute field
  today; requires an existing `content.placements` entry, no dynamic-placement.
- `prepare`/`PreparedMutation`/`commit` — PROVEN: collision presence is a pure derived read from
  `(states, target_state)`, never mutable state.
- `TransitionBinding`, `ProductionKey`/`PlacementKey`, `validate_encounter.py`/encounter-format line
  144, §4 C3 (lines 154-162) — PROVEN, unchanged from §7/§8.
- `LocalObjectStateDefinition` (~805-813) — PROVEN: exactly `key`+`collision`; `attribute_variant_of`
  (Round 3) is genuinely new.
- Corpus (Round 4 full re-scan of `tools/content-schema/encounter-authoring/samples/**`): at least
  sixteen `transform`+pre-authored-anchor+`destination` samples covered (not exhaustively enumerated
  here — §9 Problem section has the list); `mazzinor`/`gaz_haragoth`/`cult_soul_remains`/`azerus`
  (`death_position`, NOT covered — §7 open decision 8); `death_priest_shargon`/`the_ravager`
  (`create`+pre-authored-anchor+`destination`, NOT covered — §7 open decision 9, new Round 4).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture design; no production mutation, authority grant, PREPARE/COMMIT,
  controller install/restore or persisted recovery evidence is touched. Not itself an implementation.
```

## Acceptance criteria

- [x] §9 names where the attributes live, how `prepare`/`Publish` carry them, corrected
      `revert_destination` semantics (Round 2 P1), how the rejection lifts for the covered shape only,
      open decision 4 resolved/occurrence-keyed (Round 2 P2), and the lifecycle-record fields added
      (none).
- [x] §9 narrowed to teleporter-transform shapes; `interaction`/`death_position` moved to §7 open
      decision 8 (Round 2 P1), with cross-references updated.
- [x] §7's unique-inverse rule and §9's post-revert-state inverse are mutually consistent (Round 3
      P1, direction-fixed Round 4): `bind` accepts the widened rule's `attribute_variant_of` branch
      for every covered sample, inert for ordinary content.
- [x] Every claim verified against code this task (not carried over from memory).
- [x] §9's `DecisionStatus` is `CANDIDATE`.
- [x] Both validators pass.
- [x] Round 4: widened-predicate direction bug fixed (P1 4122246542); revert-restores-exactly
      obligation given a narrow `attribute_variant_of` exception (P1 4122246550); corpus-enumeration
      claim corrected to not assert completeness, and pre-authored `CREATE`+`destination` teleporters
      tracked as rejected fail-closed via new §7 open decision 9 (P2 4122246563, not designed for).

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Implementing CW3 linker validation, the lowering step, or `LoweredActionId`'s representation.
- Teleportation execution; `interaction` bindings and `death_position` placements (§7 open decision 8,
  does not design a fix); pre-authored `CREATE` teleporters carrying `destination` (§7 open
  decision 9, Round 4, does not design a fix).
- A generic attribute system beyond `destination`/`revert_destination`.
- §7's open decisions 1, 2, 5, 6, 7 (task A) and 8/9 — unaffected, remain the owning lane's.
- `@codex` trigger, PR/issue comments, merge action — coordinator's.

## Implementation / findings

Round 1: branched fresh (`f0710bfb`); collision presence is a pure function of `(states,
target_state)` inside `prepare`, never mutable state — attributes reuse the same pattern.

Round 22: merged `origin/main` (`9beb6625`, PR #1097); fixed Codex P2 4121718203 (reserve-after-
`Publish` ordering misattributed).

Round 2 (head `db49049d`; 2 P1s + 1 P2): merged `origin/main` (`800e3eb6`). `revert_destination` was
baking into the natural source state; fixed via a distinct post-revert state. Narrowed §9 to
teleporter-transform shapes only (death_position creates need a `PlacementKey` `bind` lacks and C3
excludes); added §7 open decision 8. `local_object_revert_after_ms` rekeyed `(TransitionKey,
LoweredActionId)`.

Round 3 (head `d417e860`; 1 P1, thread 4122104484): merged `origin/main` (`d17a3826`). Round 2's
post-revert state never satisfied §7's own unique-inverse rule (exact `target_state` ==
`source_state`; post-revert key != natural source key) — `bind` would reject all covered samples.
Fixed by widening §7's rule: new optional `LocalObjectStateDefinition.attribute_variant_of`
(content-level, same `collision`, CW3-validated). Updated both §7 rule citations, §9 design point 3,
evidence/scoping/open-items; added test obligations.

Round 4 (head `93940915`; 2 P1s + 1 P2): merged `origin/main` (`a7ced850`, unrelated). **P1
4122246542** — Round 3's predicate phrasing read backwards (checked the source
state's own `attribute_variant_of` instead of the candidate's). Fixed everywhere stated (both §7
citations, §9 design point 3/header, matching test obligations) to
`states[candidate.target_state].attribute_variant_of == Some(forward.source_state)`. **P1
4122246550** — §7's "revert restores exactly the pre-operation state" obligation didn't account for
the widened rule; added a narrow exception (lands on the declared variant, same `collision`, only
when the bound inverse carries one); grep swept §7/§9, found no other contradiction. **P2 4122246563**
(not designed for) — corpus re-scan found: (a) the covered shape in 16+ samples, not 4 — corrected
the Problem section/scoping/test obligations to stop claiming completeness; (b) exactly two samples
(`death_priest_shargon`, `the_ravager`) matching a distinct shape — `create` at a pre-authored
`anchor` carrying `destination`+`revert_after_ms` — added as new §7 open decision 9 with a
rejected-fail-closed test obligation; (c) `azerus` shares open decision 8's `death_position` blocker
(carries `destination` not `interaction`), added to its citation list. Updated every open-decision-8
cross-reference to also name open decision 9.

Both validators re-run after all edits: PASS (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes."

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS (round 4, after merging origin/main)

### E2E

- scenario: NOT_APPLICABLE — documentation-only, no runtime behavior to exercise.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: `b2d4deddd221e3b3d60ecafeb3b69cb12ef41447` (frozen final head)
- trigger source: protected-`main` merge of PR #1099
- workflow/run/job: `game-gate` and repository protected-branch checks
- runner assignment: complete
- classification: PASS
- result: PASS — merged as `a20850d5` on protected `main`; protected-main readback passed (see
  Terminal integration)

## Self-review

- exact head: `b2d4deddd221e3b3d60ecafeb3b69cb12ef41447` (frozen final head)
- method/reviewer: implementing agent (CW1), mandatory, not delegated
- material findings: minor markdown formatting defects found and fixed in self-review before freeze
- verdict: PASS

## Independent review

- required: YES.
- exact head reviewed: `b2d4deddd221e3b3d60ecafeb3b69cb12ef41447` (round 5, last review before merge)
- method/auditor: Codex review across PR #1099, five rounds total (rounds 1-4 covered in
  Implementation/findings above; round 5 covered in Terminal integration below)
- material findings: no P1 open at merge (owner stop rule); round 5 raised two P2s, both recorded as
  open items binding on the §9 implementation task rather than designed here (see Terminal
  integration) — consistent with every prior round's P2 handling in this task.
- verdict: PASS; protected `main` admitted the frozen final head

## PR and closeout

- changed-file review: complete (2 files changed: architecture doc + this task record; see Terminal
  integration)
- unresolved review threads: none blocking at merge (round 5's two P2s are open items, not blocking —
  see Terminal integration)
- related/superseded PRs: sequenced after task A (PR #1097, merged `061b1676`); merged `origin/main`
  four times (round 22 for #1097; rounds 2/3/4, unrelated, no conflicts)
- protected auto-merge: not requested by this task
- merge commit/result: `a20850d5` on protected `main`
- ownership release: complete; owned paths released at archive

## Context checkpoint

```yaml
last_progress: terminal integration recorded; PR #1099 merged on protected main as a20850d5; record archived
status: completed
branch: claude/cw1-attribute-bearing-object-state
head_sha: b2d4deddd221e3b3d60ecafeb3b69cb12ef41447
pr: 1099
final_head_sha: b2d4deddd221e3b3d60ecafeb3b69cb12ef41447
final_head_frozen_at: 2026-09-28T12:57:47Z
ci_trigger_source: protected_main_merge
ci_check_generation: final
ci_checks_for_current_head: 1
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: complete
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none; task closed. Two round-5 P2 open items (see Terminal integration) are binding on the owning lane's §9 implementation task; §9 itself awaits owner acceptance before implementation.
```

## Terminal integration

This section supersedes the historical `ready`/pending metadata and checkpoint above with the frozen
terminal outcome; the complete implementation record above (Rounds 1-4) remains verbatim as historical
evidence. This closeout performs no architecture, code or schema mutation of its own; it only moves
this record from `docs/agents/tasks/active/` to `docs/agents/tasks/archive/` and binds terminal
lifecycle fields. Coordination: issue #162 (control plane).

The owner stop rule applied at round 5: PR #1099 merged with no open P1. Frozen final head
`b2d4deddd221e3b3d60ecafeb3b69cb12ef41447` (round 4 — see Implementation/findings above; also
`refs/pull/1099/head`, byte-identical to the merged content for the owned architecture doc) merged on
protected `main` as commit `a20850d5` at 2026-09-28T13:29:53Z. Protected-main readback passed: §9 is
present with `DecisionStatus: CANDIDATE`, open decisions 8 and 9 are present, and the owned
architecture doc is byte-identical between the frozen head and `origin/main` for that file.

**Round 5 Codex review (post-freeze, on the frozen head `b2d4dedd`) — no P1; two P2s recorded as open
items, binding on the §9 implementation task, not designed here (owner stop rule: this PR merges once
no P1 is open):**

1. **Narrow §7's inherited rejection test obligations (~lines 1501-1524) to exclude the §9 transform
   shape.** Those obligations, written before §9 existed, state that every `map_item` action carrying
   `revert_after_ms` together with `destination`/`revert_destination`/`interaction` is rejected
   fail-closed at authoring/lowering — a blanket statement §9 now partially supersedes for the covered
   `transform`+pre-authored-anchor+`destination` shape. The owning lane's implementation must narrow
   that inherited language so it excludes exactly the shape §9 admits, while keeping fail-closed
   rejection intact for everything §9 does not cover: open decision 8 (`mazzinor`/`gaz_haragoth`/
   `cult_soul_remains`/`azerus`, `death_position`) and open decision 9 (`death_priest_shargon`/
   `the_ravager`, pre-authored `CREATE`+`destination`). This is a text-narrowing obligation on the
   owning lane's future edit to §7, not a design change to §9 itself.
2. **Lowering must reject fail-closed when two authored actions at one placement enter the same
   `target_state` with different attributes.** §9's design keys `local_object_state_attributes` by
   `target_state`, not by `(target_state, LoweredActionId)` — unlike `local_object_revert_after_ms`,
   which design point 5 already occurrence-keys per Round 2 (Codex 4121918234). If two authored
   actions at the same placement both transition into the same `target_state` but declare different
   `destination`/`revert_destination` values for it, the shared per-state entry cannot hold both. §9
   does not design a fix for this (out of scope for this closeout, owner stop rule); the owning lane's
   implementation must add an explicit fail-closed rejection at authoring/lowering for this case (not
   silently let the second authored action's value overwrite the first's, and not merge them) and a
   matching test obligation, before or as part of implementing §9.

**§9's `DecisionStatus` remains `CANDIDATE`.** Merging this PR records the design as reviewed
(Codex round 5: no P1) and integrated into the document; it does not itself constitute owner
acceptance of §9 as implementation-ready. The owning lane's implementation of §9 — and its resolution
of open decisions 8 and 9, and the two round-5 P2 items above — waits on that separate owner
acceptance, exactly as §7's own design-vs-acceptance distinction already works for the rest of this
document.

Task status: `completed`. Aggregate issue #162 remains open for further work, including owner
acceptance of §9, the owning lane's implementation of §9 and open decisions 1, 2, 4, 5, 6, 7, 8 and 9,
and the two round-5 P2 open items recorded above.

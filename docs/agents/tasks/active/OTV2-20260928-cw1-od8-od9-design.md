# OTV2-20260928-cw1-od8-od9-design

```yaml
task_id: OTV2-20260928-cw1-od8-od9-design
title: OD8/OD9 design - runtime-created local objects at death_position and pre-authored CREATE teleporters (proposal §10)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5879302658
base_branch: main
branch: claude/cw1-od8-od9-design
pr: 1182
base_sha: 3dcf3c82abc5d424542388a9f65ca1507e8746a4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world architecture
created_at: 2026-09-28T22:00:00Z
updated_at: 2026-09-28T23:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md (new §10; §7 open decisions 8/9 status; §8 item 7 and §9 pointers; owner decision 3 in §9)
  - docs/agents/tasks/active/OTV2-20260928-cw1-od8-od9-design.md
  - docs/agents/tasks/archive/OTV2-20260928-cw1-teleporter-rearm.md (closeout of #1164, moved from active/)
public_contracts: []
depends_on:
  - "owner decisions OD8, OD9 and decision 3 on #162 comment 5879299188"
  - "allocation comment on issue #162: ALLOCATIONS, item A (5879302658)"
  - "#1164 merged as 195ef53a (D90 re-arm)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task adds a docs-only design as §10 of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`. It is
`CANDIDATE`, pending independent review, and follows the CrystalServer/Canary reference behaviour
(file:line evidence is in §10.1).

- **Shared model (§10.2).** A created object has two states. The first is a synthesized absent
  state: `collision: Absent`, a new `absent` marker, no attributes. The second is the created
  item's present state. A `CREATE` forward and a `REMOVE` inverse connect them. §7/§9 run as they are, except for a
  runtime-placement flag on `PendingRevert` and a terminal-release rule used only by OD8.
- **OD8 (§10.3), runtime placements at `death_position`:**
  - A lowered immutable template per action is bound at run time as an injected `PlacementRef`,
    following the `bind_native_entry_door` precedent, through `bind`'s unchanged validations.
  - Its key is `oteryn-runtime:<action>/<n>`, where `n` is a never-reused scope sequence.
  - Creation happens only in the scope owner's own turn. The object is retired on reaching absent
    (timed removal or step-in consumption), and its records are released at `TERMINAL`.
  - It is scope-ephemeral.
  - C3 is lifted only for collision-`Absent`, one-cell objects.
  - New attribute: `interaction`. New row proposal: `WOBJ-RL-08` (64 live runtime objects per
    scope), named in the text only.
- **OD9 (§10.4), pre-authored `CREATE` teleporters:**
  - The anchor placement starts in the absent state, and the create forward carries `destination`
    and `revert_after_ms`.
  - The revert lands on the natural absent state, so the authored forward re-arms without a
    `/rearm` edge (consistent with D90).
  - A kill while open is a no-op (owner decision 3).
- **Owner decision 3** is recorded in §9, next to D90/D91.
- **Owner questions** on deviations only (§10.7): Q1 is the OD8 cap-and-skip at `WOBJ-RL-08`. Q2
  asks whether decision 3 also covers the `CREATE` teleporters, whose Canary scripts would stack a
  second teleporter.
  Q3 (Round 2) is the same-cell replacement case: Canary's lookup-by-id timer can remove a newer
  object early; this design removes each object only through its own record.
- **Round 2** (Codex on `2a929c9d`, two P1s and one P2, all accepted):
  - `interaction` keys resolve at compile time against the interaction-domain content, and fail
    closed when unresolved or incompatible (DUR-04).
  - A combined `WOBJ-RL-03` check (pre-authored plus live runtime < 486) applies at
    materialization, with a boundary test for a nearly full pre-authored scope.
  - Q3 is recorded as an owner question.
- **Round 3** (Codex P1 4127486973 on `dc239ea9`, accepted):
  - The linker never infers a `WorldObject` target from untyped evidence.
  - A typed `REMOVE` target, `"target": {"kind": "triggering_object"}`, is named as a
    prerequisite owned by the interaction lane (`interaction.schema.json` and quest format §6.3).
    It is not edited here.
  - Until it exists, `mazzinor`, `gaz_haragoth` and `cult_soul_remains` stay rejected with named
    errors, and a test obligation covers rejecting untyped `REMOVE` children.
- **Round 4** (Codex on `35fe1d88`, two P1s and one P2, all accepted):
  - A §10.9 decision test covers the five mandatory questions, with options and trade-offs for
    runtime identity, record retention, capacity, the contract prerequisite and the OD9 model.
  - §10.2 states the `PendingRevert` flag and the terminal-release exception explicitly, and the
    "unchanged" claims in §10 are corrected.
  - The `triggering_object` target is restricted to `ON_ENTER`.
- **Round 5** (Codex P1 4127573494 on `9ec1b1c0`, accepted):
  - Two present → absent `REMOVE` edges, each with a single D91 owner: `/remove` (revert driver,
    the encounter owner) and `/consume` (the interaction).
  - `bind`'s inverse search requires the forward's own origin, so `/remove` stays the unique
    inverse.
  - Consumption still retires the object, and the later timer still fences to a no-op.
- **Closeout** of `OTV2-20260928-cw1-teleporter-rearm`: archived as completed, merged `195ef53a`.

## Excluded scope

- No code, schema, sample or registry change. `WOBJ-RL-08` and the `WOBJ-RL-04` note are named in
  the text only.
- Interaction consumers, the teleport consumer, encounter-trigger wiring and absent-state client
  rendering are out of scope. D91 enforcement is named as a precondition.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`

## Context checkpoint

last_progress: Round 5 repair (Codex P1 4127573494) on PR #1182; new candidate head pending freeze
jira: pending (no mapped Story resolved in this worker session)

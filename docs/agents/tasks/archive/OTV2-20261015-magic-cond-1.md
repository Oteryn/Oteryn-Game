# OTV2-20261015-magic-cond-1

```yaml
task_id: OTV2-20261015-magic-cond-1
title: "MAGIC-COND-1 (COND-1 §3.5 residual and COND-CONTENT-2): the muted, pacified, powerless and agony families, their admission gates, the agony tick and their authored content"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/magic-cond-1
plan_issue: 2051
pr: null # the coordinator writes the positive PR number once the PR is open
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for the MAGIC-COND-1 coordinator
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ability/condition.rs
  - apps/game-server/src/ability/condition_gate_tests.rs
  - apps/game-server/src/ability/condition_tests.rs
  - apps/game-server/src/creature_condition_content.rs
  - apps/game-server/src/creature_condition_content_gate_tests.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/attack_gate_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs
  - apps/game-server/src/spell/actor_conditions.rs
  - apps/game-server/src/spell/actor_conditions_agony_tests.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/cast_gate_tests.rs
  - apps/game-server/src/spell/source_player_conditions.rs
  - tools/content-schema/condition-authoring/README.md
  - tools/content-schema/condition-authoring/authored-conditions.json
  - tools/content-schema/condition-authoring/condition.schema.json
  - tools/content-schema/condition-authoring/condition_authoring.py
  - tools/content-schema/condition-authoring/samples/canary-condition-sources-04b83b51.json
  - tools/content-schema/condition-authoring/samples/conditions-candidate.json
  - tools/content-schema/condition-authoring/test_condition_authoring.py
  - docs/agents/tasks/archive/OTV2-20261015-magic-cond-1.md
public_contracts: []
depends_on:
  - "#2106 (ACCEPT-CONDITIONS-0, merged)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The runtime condition store has the §3.5 keys `muted`, `pacified` and `powerless` (one gate family, refreshed on every application, also when shorter) and `agony` (replace when the new total is at least the remaining one, a replacement following §3.1), with `COND0-RL-01` at 32. None of them is tagged `negative`, no dispel selector takes them, they end only by duration, death or the §6.3 boundaries, and creature and Cleanse immunity do not refuse `agony`; re-entry protection refuses a creature application of every one.
- The gates run once the action is identified, after the CHAT-0 §6 chat-mute pre-gate and before every other check: in the transport ahead of every cast dispatcher, and again first in every `spell/cast.rs` entry point and the ordinary-combat owner. Refusal order is `muted`, `pacified`, `powerless`; a spell answers `SPELL_CAST_DISPOSITION_REJECTED` (10), an attack set-target refused by `pacified` answers `ATTACK_INTENT_DISPOSITION_REJECTED` (6) before the visibility, creature, protection-zone and PvP checks, and clearing a target stays open.
- An `agony` tick skips the whole incoming mitigation stage (damage reduction and the mana shield) and every charm; in a protection zone it is refused and used up. The player cycle deals a non-lethal creature agony tick; every other occurrence goes to the combat owner.
- COND-CONTENT-2: the condition-authoring schema, builder, validator and tests carry the four families, `MAX_CONFLICT_KEYS = 32` and the agony rules; the reference-sourced rows are `spell.swift_foot.pacified` and `monster_spell.soulpit_powerless`, and the candidate is rebuilt.

## Architecture and source of truth

- PROVEN: CONDITIONS-0 §3.5 and §7 as accepted in `docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md` and `OTERYN_GAME_ACCEPT_CONDITIONS0_ACCEPTANCE_DECISION_2026-10-14.md`; the SPELL-D1 `resolve_cast` amendment in `docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`; the ATTACK-0 §3 and §4 amendments. No protocol, wire, persistence or pin change.

## High-risk authority/recovery qualification

- Combat admission and determinism change on the server only; no durable state, wire field or authority changes. The gates fail closed (a missing spell state refuses), and the agony tick is not persisted (a declared `PARITY_PENDING` gap of §3.5).

## Acceptance criteria

- [x] The §3.5 `ConflictKey` values and policies, with `COND0_RL_01_INSTANCES_PER_ACTOR = 32` and a test that a 17th distinct key is admitted.
- [x] The `agony` tick and its replace policy, with a test of the equal-total case; it skips mitigation and charms and is refused but used up in a protection zone.
- [x] The §3.5 gates with their precedence, first after the chat-mute pre-gate in every transport cast path and every cast entry point, with typed reasons server-side only.
- [x] `pacified` refuses a set-target intent first with `ATTACK_INTENT_DISPOSITION_REJECTED` and clears the held target before any further swing.
- [x] COND-CONTENT-2: the four families in `tools/content-schema/condition-authoring` with reference-sourced rows only.

## Excluded scope and residuals

- A lethal agony tick and one of non-creature provenance go to the combat owner; routing them through `spell_periodic` is deferred.
- No `agony` or `muted` content row exists: there is no reference evidence for one (COND-CONTENT-2 authors from reference evidence only).
- The SPELL-PRESENT-0 `MUTED` detail, the typed refusal reasons for `pacified` and `powerless` and the icons of all four families belong to COND-WIRE-2; a rune refusal keeps `REJECTED` until RUNE-CAST-1 builds the RUNE-USE-0 PREPARE with its `REQUIREMENT_NOT_MET` mapping.
- `pacified` clears the held target at the attack owner's next pass, before any swing; a clear inside each condition-application owner is not built here.

## Validation

### Focused

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib` with the filters condition, actor_conditions, spell, attack, source_player and creature_condition: pass
- `python condition_authoring.py build --check`, `validate samples/conditions-candidate.json` and `python test_condition_authoring.py` (12 tests): pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

### Component/integration

- the game-server library suites above; CI runs the full workspace.

### E2E

- `NOT_APPLICABLE`: no client or wire change.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer against the accepted §3.5, §7, SPELL-D1 and ATTACK-0 text.
- material findings: none open; the residuals are listed above.
- verdict: ready for review

## Independent review

- required: combat and determinism review (COND-1 class hard); the control plane owns the review call.
- verdict: pending on the frozen head.

## PR and closeout

- changed-file review: the owned paths above
- unresolved review threads: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: slices A and B and the final conformance pass complete
status: completed
branch: claude/magic-cond-1
pr: null
blocker: null
next_action: the coordinator opens the PR, writes its number above, freezes and requests the exact-head review
```

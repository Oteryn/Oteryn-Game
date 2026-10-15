# OTV2-20261014-accept-conditions-0

```yaml
task_id: OTV2-20261014-accept-conditions-0
title: "ACCEPT-CONDITIONS-0: record owner acceptance of the actor conditions decision with the family scope amendment"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/conditions-0-accept-20261014
pr: 2106
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_CONDITIONS0_ACCEPTANCE_DECISION_2026-10-14.md
  - docs/agents/tasks/archive/OTV2-20261014-accept-conditions-0.md
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md
  - docs/architecture/reviews/OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_CHARM4_INCOMING_EFFECT_ORDER_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_RUNE_USE0_USING_RUNES_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CONDITIONS-0 is recorded as ACCEPTED by the owner (blakinio, 2026-10-14, owner accepted via control plane). A new acceptance decision records it, following ACCEPT-CHAR-POSITION-0. §3.5 adds the muted, pacified, powerless and agony family rules and raises COND0-RL-01 to 32. The CONDITIONS-0 facts record the merged COND-1 slices and the merged SPEED-1 scope (paced player steps), with live creature step pacing left to the GAME-AI-01 amendment; its brief narrows COND-1 to the §3.5 residual and adds COND-CONTENT-2, COND-WIRE-2 and COND-CLIENT-2, and the MAGIC-COND-1 graph of the mechanics plan lists both children. No code, wire, content or pin change.

## Architecture and source of truth

- PROVEN: the acceptance decision names the accepted document and records the owner-accepted scope amendment as rule text: §3.5 (the `muted`, `pacified`, `powerless` and `agony` keys, policies and gates; `exhaust` and `infight` as projections) and `COND0-RL-01` raised from 16 to 32. The acceptance also amends these rules outside §3.5, each named here: CONDITIONS-0 §3.2 (an Agony tick skips mitigation and charms, every other §3.2 rule kept), §3.4 (the overload and instance-limit wording and counts), §4.1 (creature base speed is its content `speed`, per CREATURE-MOVE-1-PACKET-1 §1.2), §5 (the §3.5 keys are valid for no dispel selector), §7 (the §3.5 icons routed to COND-WIRE-2) and §8 (`COND0-RL-03` counts damage and regeneration ticks); CHARM-4 §4 (an Agony tick runs only the commit step); EQUIP-0 §3 (the `SUPPRESS` row: `agony` is not a valid key); the cast precedence keeps the CHAT-0 §6 chat-mute pre-gate first; RUNE-USE-0 §6.1 PREPARE runs the §3.5 gates between resolving the rune and the `resolve_cast` checks; SPELL-D1 `resolve_cast` runs the §3.5 gates before its first check; ATTACK-0 §3 refuses a `pacified` set-target intent first and §4 clears the held target when `pacified` is applied. Every other rule is unchanged.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation only. No mutation, no code, nothing enabled.

## Acceptance criteria

- [x] The acceptance decision records the owner, date and basis.
- [x] Rule text changes are exactly the ones the Architecture record lists: §3.5 and COND0-RL-01, plus the named amendments to CONDITIONS-0 §3.2, §3.4, §4.1, §5, §7 and §8, CHARM-4 §4, the EQUIP-0 `SUPPRESS` row and the CHAT-0 chat-mute precedence, the RUNE-USE-0 §6.1 PREPARE gate order; the SPELL-D1 `resolve_cast` gate order; the ATTACK-0 §3 `pacified` admission gate and §4 target clear; none other. §3.5 names the accepted EQUIP-0 §4 `SUPPRESS` removal among the ways its keys end, so the two accepted rules agree; `agony` is excluded from `SUPPRESS` in both documents (it cannot be cured, blocked or resisted), and §4.1 takes creature base speed from content with no draw (CREATURE-MOVE-1-PACKET-1 §1.2).
- [x] Every child the brief defines (COND-CONTENT-2, COND-WIRE-2 and COND-CLIENT-2 included) is in the mechanics plan's MAGIC-COND-1 graph with its class and prerequisites.

## Excluded scope

- No code, content, migration or protocol change.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer, followed the ACCEPT-CHAR-POSITION-0 pattern (#2007) and ACCEPT-EQUIP-0 (#2090).
- material findings: none open
- verdict: ready for review

## Independent review

- required: the control plane owns the review call; the owner acceptance is recorded here.
- verdict: pending; the control plane runs the exact-head combat, determinism and protocol reviews on the frozen head, and the owner acceptance does not substitute for them.

## PR and closeout

- changed-file review: documentation paths only
- unresolved review threads: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: acceptance recorded
status: completed
branch: claude/conditions-0-accept-20261014
pr: 2106
blocker: null
next_action: control plane runs the exact-head independent review (combat, determinism, protocol), then sends the PR to merge
```

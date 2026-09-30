# OTV2-20260930-conditions0

```yaml
task_id: OTV2-20260930-conditions0
title: "CONDITIONS-0 actor conditions"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-conditions-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 88fb9b01
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-conditions0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

CONDITIONS-0 decides actor conditions (control plane #162 5913838260; charm lane 5913203231).

- **Families:** speed, damage over time, food regeneration, Recovery, mana shield, light; one
  instance per conflict key; values from content in rational thousandths.
- **Policies:** Canary's speed range and paralysis floor; strictly-greater damage-over-time
  replacement with field rules; tick provenance, absent source and in-fight refresh; mana shield
  capacity and stage; random Cleanse with 11 s immunity.
- **Speed:** a pinned step-speed table, 50 ms beat, one-step player buffer with `TOO_EARLY`
  behind `PACED_MOVEMENT_V1`, and a requested GAME-AI-01 step cadence.
- **Lifecycle:** death, PvE re-entry protection, channel transfer continuation; only food time is
  durable in V1 (SPELL-D8 H-1 pattern).
- **Wire:** a new vitals revision and entity `speed` behind `CONDITIONS_V1`.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: GAME-ABILITY-01 and its whole-gate candidate; SIM-DETERMINISM-01; SPELL-D2; SPELL-D8
  H-1 (#1360); ITEM-USE-0 §6.1; D115; `world_spatial.rs`; `actor_spell.rs`.
- `DERIVED`: Canary `04b83b51` and OTClient (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. COND-DUR-1 needs persistence review; COND-1 combat and determinism
review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (combat, determinism, protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code; drunk, invisible, outfit, skill boosts, fear, root; diagonal steps; durable conditions
  other than food time.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only): 10 material and 4 minor findings, all fixed
  before the first push: the speed formula and paralysis floor, the step-speed table and beat,
  pacing and the AI cadence, damage-over-time replacement, tick provenance, determinism (RNG
  purposes, order, bounded catch-up), food per ITEM-USE-0 §6.1, the mana shield capacity and stage,
  the wire revision and bit owners, channel transfer, Cleanse, PvE-only re-entry, light and tick
  values, equipment speed, and rows versus parity values.
- Its two questions are Global-parity applications ruled by the architect (owner rule 5905825574):
  Cleanse is random as in Tibia; persistence follows Tibia as vitals become durable, with food time
  durable now and the rest a declared `PARITY_PENDING` gap.
- Review of `b7848afb` (#1380 5914810790: 3 MEDIUM, 3 LOW), answered in one push: Cleanse
  candidates in instance-sequence order with no draw for zero or one; Cleanse immunity carried
  across reconnect and transfer; the speed charms stay failing closed until SPEED-1 and the AI
  cadence amendment, with one ConditionDefinition per charm; the step table over 10..65,535; the
  paralysis floor for base speeds below 40; the vitals cap of 48 bytes.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-conditions-0
owner_action_required: null
blocker: null
next_action: null
```

# OTV2-20260930-gold-fee-boundary-decision

```yaml
task_id: OTV2-20260930-gold-fee-boundary-decision
title: GOLD-FEE T0 - Character gold fee boundary decision (D174-D178)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gold-fee-boundary-t0
issue: 162
pr: 1313
base_sha: cf251bb2
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
merge_commit: "squash merge of #1313"
owner: "GOLD-FEE T0 hard worker (claude-code-session-01EfiFA9LMuUuzoNkizLfR2R)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §15, §39.1 pointers; §39.3 gold fee amendment
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md   # §7.5 pointer
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md   # §9 pointer
  - docs/agents/DECISION_INDEX.md   # generated
  - docs/agents/tasks/archive/OTV2-20260930-gold-fee-boundary-decision.md
public_contracts: [DUR-03, DUR-02, GAME-ITEM-01]
depends_on: ["#1310 (D165-D173, numbering only)"]
blocks: [GOLD-FEE-1a, GOLD-FEE-1b, CHARM-6]
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner decisions D174-D178 (#162 5905416975 questions, 5905498654 answers) are recorded as
`CHARACTER-GOLD-FEE-BOUNDARY-V1`. DUR-03 §39.3 admits a typed BURN of up to 20 coin stacks with
up to 2 change MINTs (platinum + gold), composed with a Character change in one fenced
transaction. DUR-02 §7.5 and
GAME-ITEM-01 §9 carry pointers. No registry row, migration or code changes here; GOLD-FEE-1a
registers the fee-shape rows (RL-01 22, RL-02 22, RL-06 22/64), because the offline evidence test
and runtime constants bind `DUR03-RL-01` = 2 today.

## Architecture and source of truth

- `PROVEN`: DUR-03 §15/§17/§18/§39; the composition decision §3; migration 0020; B3 D80-D83;
  `RESOURCE_LIMITS_REGISTRY.json` DUR03 rows; content definitions of i3031/i3035/i3043; tibia.com
  manual `world.md` exchange rates and official facts (gold converter "stack of 100").
- `DERIVED`: crystal coin stack of 100; the payment plan, change rule and resource rows.
- `UNKNOWN`: Global's coin selection order and change behaviour.

## High-risk authority/recovery qualification

Applicable at design level. The decision defines which fence authorizes a Character + value
commit; the negative cases bind GOLD-FEE-1b and CHARM-6 (decision §9 `required_revalidation`):
each stale fence part commits nothing, a changed binding conflicts, the same occurrence never
burns twice, and a rejection writes nothing. No runtime path exists in this task.

## Closeout

- Validation: `validate_governance.py` pass; `tools/agents/tests` 36/36 pass; `git diff --check`
  pass. The contracts JSON is unchanged, so the repository policy check does not apply.
- Review: required independent exact-head review, triggered by the control plane on the frozen head.
- T0 repair (control plane, before freeze): Q39's "+1 change output" is read as a wording error;
  D175 governs, so the fee shape admits at most 2 change outputs (RL-01 22). Recorded in decision
  §2 as a control-plane interpretation; the earlier owner question is withdrawn.

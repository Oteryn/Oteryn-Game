# OTV2-20261003-item-sem-2b2-packet

```yaml
task_id: OTV2-20261003-item-sem-2b2-packet
title: "ITEM-SEM-2b-2: slot, hands and requirements lowering rulings and worker packet"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/item-sem-2b2-packet-20261003
issue: 162
pr: null
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_SEM2B2_EQUIPMENT_REQUIREMENTS_PACKET_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-item-sem-2b2-packet.md
public_contracts: []
external_repositories: []
```

## Outcome

Architect-selected follow-up to ITEM-SEM-2b-1. The new decision fixes the TibiaWiki-to-content
mapping for `slot`, `hands`, `levelrequired`, `vocrequired` and `mlrequired`:

- the Extra Slot maps to `ammo`, and the compact `equipment` form is always used;
- occupancy follows `domain/equipment.rs`;
- equip requirements go in `equipment`, use requirements (runes, ammunition, Extra) in
  `requirements` with `on_use`;
- `mlrequired` goes to the existing `requirements.min_magic_level`, so no schema amendment is
  needed;
- `without` maps to the A13 key `none`, and `None` means no restriction.

The worker packet (`OTV2-20261003-item-sem-2b2-equipment-requirements`) starts after #1599 merges.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `python -m unittest discover -s tools/agents/tests`
- `git diff --cached --check`

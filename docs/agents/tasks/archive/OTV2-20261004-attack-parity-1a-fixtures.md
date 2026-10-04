# OTV2-20261004-attack-parity-1a-fixtures

```yaml
task_id: OTV2-20261004-attack-parity-1a-fixtures
title: "ATTACK-PARITY-1a: TibiaTools auto-attack fixture grid"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/attack-parity-1a-20261004
pr: 1775
base_sha: ba8b8df
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-impl-worker (CP #1622)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - tools/combat-parity/capture_tibiapal_auto_attack.py
  - tools/combat-parity/test_capture_tibiapal_auto_attack.py
  - tools/combat-parity/README.md
  - content/combat/parity/tibiapal_auto_attack_v1.json
  - docs/agents/evidence/OTV2-20261004-attack-parity-1a-capture.md
  - docs/agents/tasks/archive/OTV2-20261004-attack-parity-1a-fixtures.md
public_contracts: []
depends_on: [ATTACK-1a]
blocks: [OTV2-20261004-attack-parity-1b-formulas]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ATTACK_PARITY1_AUTO_ATTACK_PARITY_SCOPE_PACKET_2026-10-04.md` §2.1.
The fixture holds 1536 rows (4 vocations x 6 levels x 4 skills x fists plus 15 melee weapons), live-captured
2026-10-04T15:16:19Z to 15:18:59Z, with the Offensive/1.0 binding and `tibiatools_attack` on weapon rows. No monk row, no
missing residue, no Rust change. The deferred #1768 P2 (4177982269) fight-mode gap is recorded in
`docs/agents/evidence/OTV2-20261004-attack-parity-1a-capture.md`.

## Validation

- `python3 -m unittest tools/combat-parity/test_capture_tibiapal_auto_attack.py`: OK
- `python3 tools/combat-parity/capture_tibiapal_auto_attack.py verify`: OK
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Merge result

Recorded on #1622 at merge.

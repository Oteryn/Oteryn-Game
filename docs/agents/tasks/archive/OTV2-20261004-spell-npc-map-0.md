# OTV2-20261004-spell-npc-map-0

```yaml
task_id: OTV2-20261004-spell-npc-map-0
title: "SPELL-NPC-MAP-0: map the spell and NPC source handoffs to accepted owners"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-npc-map-0-20261004
issue: 162
pr: 1754
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SPELL_NPC_MAP0_SPELL_AND_NPC_HANDOFF_MAPPING_DECISION_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-spell-npc-map-0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Owner answers 1a and 2a of 2026-10-04. New decision `SPELL-NPC-MAP0-V1` (candidate).
- §1: five classes (adapter missing, contract gap, runtime gap, source conflict kept, excluded),
  each with a consumer seam and an existing owner.
- M1: the private `source-complete` spell schema is not adopted; its four extensions lower to the
  S27-accepted `native_behavior` keys and stay held until each key's runtime child lands.
- M2: the blocked spell queues are mapped to accepted children by concrete key; the spell scheduler (`delayed_strike`) is the
  one runtime gap without built code.
- M4: Paralyze caster effect follows execution, not the per-target outcome; D.5.3 test 4 split.
- M5-M7: Monk source formulas kept; the example spells excluded; D479 unchanged; NPC dependencies
  mapped to NPC-0, NPC-BEHAVIOUR-0, QUEST-STATE-0, TRAVEL-0 and BANK-FEE-0 owners.
- M8: optional route `departure_text` in TRAVEL-0 §4. M9: local packets publish as draft PRs, no
  merge before acceptance.
- #1754 Codex round 1: P1 4177304303, the extensions map to the concrete accepted keys and
  fields (family labels are never keys); P1 4177304309, §11 decision test for M4 and M8; P2
  4177304316, `departure_text` must be non-empty after trimming.
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

# OTV2-20261004-arch-item-packets-amend-2

```yaml
task_id: OTV2-20261004-arch-item-packets-amend-2
title: "ARCH-ITEM-PACKETS-AMEND-2: capability-mismatch resume fallback, v5 resource bounds, ground-speed admission, capability 4 entry checks"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-item-packets-amend-2
issue: 1622
pr: 1707
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_MAP_WIRE1_MAP_STATE_WIRE_CONTRACT_CANDIDATE_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-item-packets-amend-2.md
public_contracts: []
depends_on: []
blocks: [CAP-NEG-RESUME-FALLBACK-1, SPEED-1, VIS-3, ITEM-VIEW-1b, ITEM-SEM-2b-3, MAP-WIRE-2]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D449:** new hard packet CAP-NEG-RESUME-FALLBACK-1 (§1.13, §2.0c). A resume refused for a
  capability mismatch terminally releases the RECONNECTABLE incumbent, so the fresh admission
  that follows succeeds. The packet starts after CAP-NEG-1 (#1705), leases 0067 if it needs a
  migration, and must merge before SPEED-1, VIS-3 or ITEM-VIEW-1b, so that nothing is offered
  before it (§0.2, §1.9).
- **D448, #1702 P1 4175418427:** ITEM-SEM-2b-3 now owns the resource-profile tool, a v5
  architecture profile, evidence and the registry rows. It recomputes every v5 ceiling, with
  max/max+1 tests, before the codec is released (§1.12, §2.2a).
- **#1702 P2 4175418429:** MAP-WIRE-2 admits ground speed in 1..=1,000 in the schema,
  `world_objects.py`, the bundle compiler and MAP-LOAD-1's reader (§1.11, MAP-WIRE-1 amendment).
- **#1703 P2s 4175400422 and 4175400425:** ITEM-VIEW-1b validates that capability 4 requires 6
  in accepted and resume-accepted, and that `item_handle` is unique per snapshot/delta, before
  capability 4 is offered (§2.4, §0.3).
- **#1707 P1 4175486625:** the capability-mismatch fallback is a terminal release with no
  successor, through the existing fenced release (FND-04B §3, §6, §20), not an
  `EarlyTerminalReplacement`. That mode commits the old session `Active` onto a successor, and
  turning it into the usable connection would widen the selected set on resume (§1.13, §2.0c).
- **#1707 P2 4175486629:** VIS-3, SPEED-1 and ITEM-VIEW-1b carry CAP-NEG-RESUME-FALLBACK-1 in their
  `base` and `depends_on`; CHARM-5-COMP and CHAT-1b-2 list it when they are packeted (§1.9).
- **#1707 P2 4175486632:** ground-speed admission tests 0, 1, 1,000 and 1,001 in the schema, the
  converter, the compiler and the reader (§1.11, MAP-WIRE-1 amendment).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

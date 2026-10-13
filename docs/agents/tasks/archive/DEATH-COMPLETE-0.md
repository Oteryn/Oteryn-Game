# DEATH-COMPLETE-0

```yaml
task_id: DEATH-COMPLETE-0
title: "DEATH-COMPLETE-0: player death module as one coherent package"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death-complete-0-20261013
issue: 1622
governing_issue: 1622
pr: null
base_sha: 59f9fb12
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md
  - docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/agents/tasks/archive/DEATH-COMPLETE-0.md
public_contracts: []
external_repositories: []
```

## Outcome

Owner direction 2026-10-13: deliver the whole death module, lifting the D1022 waiver. The decision
fixes five batches (B DEATH-DUR-1, C DEATH-3, D PVP-DEATH-1, E DEATH-4, optional F wire), one receipt
change, the DUR-03 shapes (death drop, container with contents, amulet burn), the respawn gate and
the registry and migration order. Documentation only; no code, registry or migration change.

## Acceptance criteria

- [x] The waiver is recorded as lifted in the first player death decision.
- [x] The receipt is changed once (DEATH-0 §3.1 pointer).
- [ ] Independent review, then owner acceptance (outside this worker).

## Validation

Documentation only; the governance validator and the agent tests are run before the freeze.

## Self-review

Whole-diff read against the DEATH-0 migration (0016), the first death decision and PARTY-PVP-0 §10.
Unverified: the Ground cell limit, the container enumeration need (recorded as UNKNOWN).

# OTV2-20261003-premium-delivery0a

```yaml
task_id: OTV2-20261003-premium-delivery0a
title: "PREM-DELIVERY-0A: deferred review findings from #1659, #1661, #1391 and #1631"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/premium-delivery0a-followups-20261003
issue: 162
pr: 1668
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_D327_PACKETS_AND_HELD_P1S_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-premium-delivery0-acceptance.md
  - docs/agents/tasks/archive/OTV2-20261003-premium-delivery0a.md
public_contracts: []
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: []
```

## Outcome

Control plane D369 and D370. This is one batch of review findings deferred under D317, and it is a
precondition for allocating PREM-1b.

- #1659 P1 4174238442: PREMIUM-DELIVERY-0 §3.1 item 2 and §11 scope item 7 classify `Unsupported`
  only for a complete envelope. A partial or mistyped body is a recoverable failed pull with no
  durable conflict. A test is added. Review round 1 (Codex P1 4174298225): compatibility is
  compared only after every baseline value-form check (enumerations, timestamps, tokens, closed
  `NONE`, lease form), with a test for an unknown `schema` and a malformed timestamp.
- #1659 P2 4174238447 and 4174238453: the acceptance record's outcome states the migration lease,
  and its `pr` field is 1659.
- #1391 P2 4153900550: §11 scope item 3 ignores a failed pull or quarantine older than the latest
  proof. A test is added.
- #1661 P2 4174254492: the D327 CHAR-REV-SEQ-1 tests cover build and proficiency with no retry.
  Review round 3 (Codex P1 4174342587, D381 exception to D317): an overlong lease stays
  `Unsupported`; malformed covers only structural interval and form failures.
  Review round 2 (Codex P1 4174311683): both PostgreSQL case files are in the packet's
  `owned_paths`, taken from #1661 by merging `main` rather than duplicating them.
- #1631 P2 4173086801: CYCLOPEDIA-0 §6 sends the initial or replacement snapshot with each
  undiscovered POI that is already in range.

The #1662 findings 4174228267 and 4174228270 went to lane 2 (TIMED-PROF-0C, D370).

## Validation

`python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`,
`git diff --check`: pass.

```yaml
status: completed
owner_action_required: null
blocker: null
next_action: "control plane: review the frozen head; allocate PREM-1b after merge"
```

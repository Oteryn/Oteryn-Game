# OTV2-20261004-social-map-packets

```yaml
task_id: OTV2-20261004-social-map-packets
title: "SOCIAL-MAP-PACKETS-1: PARTY-1, GUILD-1, HOUSE-1a/1b and the map overlay and cutover packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/social-map-packets-20261004
issue: 162
pr: 1773
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION0_GAMEPLAY_SWITCH_OVER_DECISION_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261004-social-map-packets.md
public_contracts: []
depends_on: [ACCEPT-SOCIAL-MAP-0]
blocks: [PARTY-1, GUILD-1, HOUSE-1a, HOUSE-1b, MAP-OVERLAY-1a, MAP-OVERLAY-1b, MAP-OVERLAY-1c, MAP-CUTOVER-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane order after D551: detailed packets for PARTY-1, GUILD-1, HOUSE-1 and the map
  overlay and cutover, in one PR. BED-0 and ECON-RET-0 stay out.
- HOUSE-1 splits into HOUSE-1a (acquisition) and HOUSE-1b (tenancy, needs INBOX-1 and
  MAP-OVERLAY-1c). MAP-OVERLAY-1 splits into 1a (overlay), 1b (map item MINT) and 1c (World reset).
- Rulings: the step-4 recheck takes `LOCK TABLE game_item_house_interior_locations IN SHARE MODE`
  after the reset record row; `WorldReset` widens the `0015` retirement tables with a cause
  discriminator; GUILD-1 waits on BANK-1 and PREM-WIRE-1, not INBOX-1 (corrects #1771 §3).
- #1771 P2 4178031083: HOUSE-1a registers `HOUSEOWN0-RL-15`, a deferred guard keeping balance
  plus HELD private house escrow at most the hard ceiling; GUILDHALL-1 extends the same guard.
- #1773 round 1 (4 P1):
  - 4178085295: the HOUSE-RUNTIME-1 cycle is broken (§1.10). HOUSE-RUNTIME-0's row is amended, and an owned house admits its owner only until HOUSE-ACL-1.
  - 4178085301 and 4178085303: the map packets own their audit schema, proto and registry changes (§1.11).
  - 4178085304: stale CorpseDecay reservations are replaced under the new generation (§1.7).
- #1773 round 2 (3 P1):
  - 4178117645: MAP-OVERLAY-1b owns `item_mint_audit.rs` for the new arm and tag 8 (§1.11).
  - 4178117650: `OneItemWorldResetV1` carries `{world_id, reset_epoch, item_instance_id}` (§1.11).
  - 4178117654: guild and house payload schemas, registry fields and an owned-path sweep of every packet (§1.12).
- PREMIUM-ACTIVATION-0 §1.3 gains the house acquisition consumer row.
- Candidate bases are named per packet; none is accepted here.
- No code, migration, registry, event type or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

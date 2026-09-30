# OTV2-20260930-player-trade0

```yaml
task_id: OTV2-20260930-player-trade0
title: "PLAYER-TRADE-0 direct player trade"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-player-trade-0
pr: "#1365"
base_sha: 5dcfb724
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/agents/tasks/archive/OTV2-20260930-player-trade0.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on:
  - "#1344 ITEM-MOVE-WIRE-0 (merged)"
  - "#1354 ITEM-MOVE-WIRE-1 (merged)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PLAYER-TRADE-0 starts direct trade between two players (owner direction, 2026-09-30).

- **Wire.** Capability `PLAYER_TRADE_V1` (number at allocation), one command (offer, accept by
  trade id, cancel), one domain on both sessions.
- **Runtime.** One trade per actor on one channel; one whole item per side; any change cancels
  before `TRANSFERRING`; in `TRANSFERRING` both items and one entry per side are reserved until the
  database outcome.
- **Swap.** One transaction, two items into new entries (no merge, room checked before removal,
  Canary parity); both Characters fenced, the first accepter through its terminal accept with
  pinned generations; fixed rows; a per-line trade receipt as the alternative proof in the item
  guards.
- **Architect rulings (Global parity, Canary default):** `tradeable: UNKNOWN` is tradeable; received
  stacks do not merge.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the scope matrix; DUR-03; the composition decision; migrations `0010`, `0011`, `0023`;
  FND-02; FND-ID-01; content trade flags.
- `DERIVED`: the Tibia manual; Canary `04b83b51` (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. TRADE-1 needs persistence and security review; TRADE-WIRE-1 protocol
review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol, persistence, security).
- [ ] Protected Merge Queue integration, after #1344 and #1354.

## Excluded scope

- Code, migrations and content; containers with contents, ground items, capacity, house tiles.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only): 4 material findings (the first accepter's fence,
  no in-flight state, one-Character persistence guards, simultaneous accepts) and 7 others, all
  fixed: pinned terminal-accept authority, a `TRANSFERRING` state with reservations, a per-line
  trade receipt in every guard, accept by trade id, fixed rows, Canary parity (sight line, self,
  non-pickupable, room before removal, no merge), the container-slot exclusion, a broad cancel rule,
  wire refusals and the full lock order. Its two owner questions were Global-parity applications
  and are recorded as architect rulings.
- Review of `4c83b475` (#1365 5913700136: 1 MEDIUM, 3 LOW; and 5912908965's conditions), all answered
  in one push: the `TRANSFERRING` reservations are in-memory with crash safety from the swap's own
  checks and occurrence replay; logout, death or kick during `TRANSFERRING`; the partner's offer
  shown only after the counter-offer (Canary parity); a 120 s trade timeout; `depends_on` #1344 and
  #1354; the owner direction recorded on #162 5914137757.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-player-trade-0
owner_action_required: null
blocker: null
next_action: null
```

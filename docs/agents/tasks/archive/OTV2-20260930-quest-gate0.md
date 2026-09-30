# OTV2-20260930-quest-gate0

```yaml
task_id: OTV2-20260930-quest-gate0
title: "QUEST-GATE-0 quest gates, triggers, NPC quest dialogue and the quest log"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-quest-gate-0
pr: 1401
base_sha: a6a054e6
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-quest-gate0.md
  - docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - docs/architecture/GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

QUEST-GATE-0 (covering NPC-QUEST-0) wires the QUEST-STATE-0 store into the world, NPCs and the
client (owner direction 2026-09-30: build now, full Tibia Global parity).

- **World gates:** format `Gate` records lowered to QUEST-STATE-0 predicates, bound to the door's
  `placement_key`; checked by the channel runtime at `USE` and at every step onto the door; pass
  opens (D38) and moves through (D37), fail pushes back; no durable write; fail closed.
- **Triggers:** `USE`, `ON_ENTER`, `ON_LEAVE` interactions request transitions with the child
  occurrence as cause; the D39 successor sections (plus §18) and D37 §3 / D38 §4 accepted for them.
- **NPC dialogue:** typed quest conditions and outcomes; talk occurrence and bound confirmation as
  cause; items through a dialogue `RewardClaim` or a `QuestExchangeCause` exchange with a quest
  obligation; XP through a quest XP obligation and the XP writer.
- **Quest log:** capability `QUEST_LOG_V1`, one query command and one domain computed from tracks.
- **Content lanes:** QUEST-CONTENT-2 (gates, triggers), NPC-QUEST-CONTENT-1 (dialogue hooks).
- **Owner answers (2026-09-30, #162):** Q1b gold hand-ins take backpack coins first, then the bank
  balance through BANK-0 / BANK-FEE-0 in the same transaction; Q2a quest journal text is Tibia text
  1:1.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: QUEST-STATE-0, the quest format V1 and its samples, NPC-0, the NPC schema, D37-D42,
  WO-0, ADR-0021, MAP-WIRE-1, ITEM-USE-0, the composition decision, `character_progression.rs`.
- `CIPSOFT_OFFICIAL`: `docs/reference/tibia-manual/quests.md`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The children need persistence, protocol and security review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol, persistence, security).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; key doors; outfit, addon and mount grants; boss rooms;
  cross-scope relocation; party and world quests.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.
- Codex round-1 repair (four P1): fail-closed unknown tracks (§3.3); successor §6.1, §6.2 and §7
  accepted (§4); every transition refusal checked before any burn and the D42 `RewardClaim` row
  in the exchange transaction (§5.4). Validators re-run PASS on the repaired tree.
- Codex round-2 repair (two P1, one P2): over-limit trigger firings refuse their root before commit (§4);
  quest XP passes the active progression policy's `reward_revision`, the quest content revision kept
  as provenance, and content requires `1 <= n <= QUESTGATE0-RL-05` (§5.5). Validators re-run PASS
  on the repaired tree.
- Codex round 3 (#1401, 1 P1, 2 P2): dialogue node predicates revalidated under `character_root` in the value transaction (§5.1); tracker tracks quest lines, missions derived (§7, RL-06); pending XP cap RL-10 enforced before the transition with `OUT_OF_RANGE` (§5.5). Validators re-run PASS.
- Codex round 4 (#1401, 2 P1, 3 P2): DUR-03 supersession admits the rewarded-exchange MINT with BURN
  and its audit aggregate; relocation children are not trigger roots (no cascade); exchange
  preflight checks RL-07 (`OBLIGATIONS_FULL`); active quests render from their pinned revision
  (§7); exchange nodes must declare a transition (§5.2). Validators re-run PASS.
- Codex round 5 (final batched round; #1401, 0 P1, 1 P2): 4149456886 fixed: the composition
  rule 1 amendment requires a quest obligation row only for an exchange and for a claim whose node
  names a transition; a claim-only node writes none (§5.4 says so too). Validators re-run PASS.
- Owner answers applied (2026-09-30, #162): Q1b (§5.4 gold hand-in, coins then bank, rows §9) and
  Q2a (§7 journal text 1:1); the DUR-03 §39.3 quest exchange paragraph names the gold hand-in.
  Validators re-run PASS.

## Closeout

- PR: #1401. Merge commit/result: its squash merge.
- Jira sync: pending (coordinator batch).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-quest-gate-0
owner_action_required: null
blocker: null
next_action: "#162 validates the exact head and routes the independent review"
```

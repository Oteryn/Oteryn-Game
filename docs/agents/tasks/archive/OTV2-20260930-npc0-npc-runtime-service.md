# OTV2-20260930-npc0-npc-runtime-service

```yaml
task_id: OTV2-20260930-npc0-npc-runtime-service
title: "NPC-0 NPC runtime service (talk, trade, travel)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/laughing-goldberg-4gwjfq
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 58429a26
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md
  - docs/architecture/OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/agents/tasks/archive/OTV2-20260930-npc0-npc-runtime-service.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

NPC-0 is the child of the NPC service boundary of 2026-09-09. It answers the NPC lane question
packet (architect ruling on #162, 5909181928, answer 1a).

- **Content.** The runtime reads the generated NPC, Dialogue, Trade and Travel records as a
  pinned, validated input. NPCs without a Dialogue get generated minimal replies.
- **Wire.** Capability 3 `NPC_SERVICE_V1`, command types 7 (talk) and 8 (trade), state domains 7
  (conversation) and 8 (trade window), all runtime-local (#162 5909366267).
- **Value.** BUY, SELL and travel are item-only transactions with a cause record and no
  CharacterRevision advance, on the gold fee plan. Travel adds a pending arrival obligation, which
  a death supersedes.
- **Boundary.** The 2026-09-09 NPC boundary is accepted with this PR; its price evidence gate
  becomes a release parity gate (NPC-0 §7).
- **Amendments.** DUR-03 §15 and §39.3, the gold fee decision §4.3 and §4.4, the composition
  decision §3.1, DEATH-0 §3.4, the NPC boundary. The owner
  admitted NPC value sources (Q1a, "tak a", 2026-09-30, #162 5909366267), as D178 requires.
- **Children.** NPC-CONTENT-1, NPC-PLACE-1, NPC-WIRE-1, NPC-TALK-1, NPC-TRADE-1, NPC-TRAVEL-1.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the NPC boundary; NPC authoring schema (D4, D9) and NPC admission; the gold fee
  decision and migration `0023`; the protocol registry and the charm/proficiency allocation;
  DEATH-0 `respawn_position` (`0016`); ADR-0021 §4.3.
- `DERIVED`: Tibia travel and trade flow through dialogue.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. NPC-TRADE-1 and NPC-TRAVEL-1 need persistence review; NPC-WIRE-1
needs protocol review.

## Acceptance criteria

- [x] Decision and DUR-03 amendment on an exact frozen head with passing validators.
- [x] Owner answer on NPC value sources (decision §9, D178): Q1a.
- [ ] Independent exact-head review (persistence and protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations, content, quest dialogue, the bank, spells, blessings, promotion, NPC movement.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) on `edec015c`: 5 material, 2 evidence gaps, 4
  hardening findings, all fixed before freeze (item-only trade without revision advance, full D178
  amendment, GOLD-FEE-1b and DEATH-1 dependencies, pending arrival, boundary acceptance and price
  ruling, content facts and offer rules, one trade cause, rows, wire rules).
- Second pass on `0eed72b4`: 4 new material findings (D177 text, BUY quantity, death versus
  arrival, compile rule) and 3 others, all fixed: item-only reading of D177 in the gold fee §4.3,
  travel item-only with an obligation, death supersedes arrival, placement fallback, compile rule
  aligned with ADR-0021, offer counts corrected.
- Repair after freeze `9f9e06de` (found by the ITEM-MOVE-WIRE-0 review): domain revisions are
  monotonic per GameSession across reconnect (FND-02 §13.3, §15), not reset per connection.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/laughing-goldberg-4gwjfq
owner_action_required: null
blocker: null
next_action: null
```

# OTV2-20261004-arch-root-packets-1

```yaml
task_id: OTV2-20261004-arch-root-packets-1
title: "ARCH-ROOT-PACKETS-1: root packets for ITEM-USE-WIRE-1, BANK-RET-0, BANK-1 and GOLD-FEE-2; NPC-BEHAVIOUR-0 acceptance; #513 disposition"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-root-packets-20261004
issue: 162
pr: 1733
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-root-packets-1.md
public_contracts: []
depends_on: []
blocks: [ITEM-USE-WIRE-1, BANK-RET-0, BANK-1, GOLD-FEE-2, NPC-VIS-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Packets: ITEM-USE-WIRE-1 (impl, protocol
  review), BANK-RET-0 (control plane, privacy review), BANK-1 (hard, persistence review) and
  GOLD-FEE-2 (hard, persistence review), in that dependency order (§0.2, §2).
- Leases (control plane): capability 15 `ITEM_USE_V1`; migrations 0071 (BANK-1) and 0072 (GOLD-FEE-2);
  event type 3 `BANK_OPERATION`; profiles `ECONOMY_LEDGER_RETENTION_V1` and the successor
  `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2` (§0.1).
- #1733 P1 4176849286: profile identity is immutable, so event type 2's fee events with a bank part
  get a successor profile for future admission only; V1 is never revised; GOLD-FEE-2 admits a bank
  part only after the reviewed activation boundary (§1.7, §2.3, §2.5).
- #1733 P1 4176877703: the registry binds event type 2 to one profile, so every type-2 producer
  moves to V2 at one boundary, GOLD-FEE-2's merge and deploy. The shared constant, the registry
  binding and the `0010` CHECK (V1 or V2) change together. Stored V1 events keep V1 (§1.7, §2.3,
  §2.5).
- #1733 P1 4176877701: GOLD-FEE-2 owns the type-2 proto change: `OneItemFeeBankDebitV1` as field
  13 of the fee burn, schema revision 2, and the revision-1 golden-byte compatibility and codec
  qualification (§2.5).
- #1733 P2 4176849291: each NPC turn, a focus restoration included, has a unique occurrence
  (NPC ref, focus sequence, `NPC_TURN`) (NPC-BEHAVIOUR-0 §5).
- Owner answers 2026-10-04: BANK-0 Q1 = b, recorded in BANK-0; batch scope 2a (§1.1).
- NPC-BEHAVIOUR-0 accepted when this merges, with its Amends line (§1.5).
- #513: limits still correct; recommend closing it and moving `DUR03-RL-08` to stage C (§1.6).
- Next wave (MAP-OVERLAY-1, ITEM-USE-1, NPC-ACTOR-1) goes in the next batch.
- #1733 P1 4176934049: `OneItemFeeBankDebitV1` carries BANK-0 §5's full value line, including
  the payer's historical `AccountId` and the `WorldId`, with codec and compatibility tests
  (§2.5).
- #1733 P1 4176934053: `0072` replaces `0010`'s single-value revision and profile CHECKs with
  one tuple CHECK, `(1, V1)` or `(2, V2)`. Both tuples are qualified and the mixed tuples are
  refused (§1.7, §2.5).
- D491 (owner 5a): MAP-LOAD-1 moved to MAP-LOAD-PACKET-1 (#1744). That covers bundle staging, the
  reader crate, the ground speed source, the Terrain catalogue input (with the P1 4176957737
  fix) and the packet. The open P1 4177026515 and P2 4177026518 moved with it and are answered
  there. §1.2-§1.4 and §2.1 stay here as pointers.
- #1733 P1 4177057911: type-2 audit activation has two phases. In phase 1, GOLD-FEE-2 reads (1,V1)
  and (2,V2) and still emits (1,V1). In phase 2, GOLD-FEE-ACT-1 is the single reviewed switch: an
  activation row plus an outbox trigger, applied only once every node runs phase-1 code, so no V1
  is emitted after the boundary. Qualification tests are listed (§0, §1.7, §2.5, §4).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

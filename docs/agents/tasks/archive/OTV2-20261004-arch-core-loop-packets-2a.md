# OTV2-20261004-arch-core-loop-packets-2a

```yaml
task_id: OTV2-20261004-arch-core-loop-packets-2a
title: "ARCH-CORE-LOOP-PACKETS-2 part A: ATTACK-1 and CHAT-1b-2 splits, ATTACK-WIRE-1 lease, spawn rulings"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-core-loop-packets-20261004
issue: 162
pr: 1735
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-core-loop-packets-2a.md
public_contracts: []
depends_on: []
blocks: [ATTACK-WIRE-1, ATTACK-1a, ATTACK-1b, CHAT-1b-2a, CHAT-1b-2b, SPAWN-CONTENT-1, SPAWN-1a]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D486 item 1:** ATTACK-1 splits into a pure ATTACK-1a (`combat/attack/**`) and the wiring
  ATTACK-1b. CHAT-1b-2 splits into a pure CHAT-1b-2a (`chat/**`) and the wiring CHAT-1b-2b, which
  reuses CAP-NEG-1 instead of CHARM-5-COMP. ATTACK-WIRE-1 holds capability 17 `ATTACK_V1`
  (requires 6), command types 11 and 12 and domain 10 (§0.1, §1.1, §1.2).
- **D486 item 2:** SPAWN-CONTENT-1 needs MAP-BUNDLE-1 only, because spawns are a base bundle
  family. SPAWN-1 splits into SPAWN-1a, the D116 fixture spawn before MAP-LOAD-1, and SPAWN-1b, the
  bundle family and its measurements (§1.3, §1.4).
- Packets: ATTACK-WIRE-1, ATTACK-1a, ATTACK-1b, CHAT-1b-2a, CHAT-1b-2b, SPAWN-CONTENT-1 and SPAWN-1a
  (§2). Shared-file order (§0.3).
- Brief amendments in ATTACK-0, CHAT-0 and CREATURE-AI-0.
- #1735 P1 4176889390: SPAWN-1a realizes the committed content spawn, not the carrier's test
  helper (§1.4). D116 stays one spawn of 2 rats; room revision 2 (#1745 §1.1) gives the content
  spawn two den cells and population 2.
- #1735 P1 4176889394: SPAWN-1a owns the real boot path. The spawn source travels in the
  activated `NativeEntryContentPin` to `ChannelRuntimeV1::from_committed_assignment` in
  `node/serve.rs`, and realizes before the listener binds. `world_runtime.rs` drops out (§2.7).
- #1735 P2 4176889397: the ATTACK-0 brief moves client target selection and the fight-mode
  buttons to ATTACK-CLIENT-1 (part B §2.5).
- #1735 P1 4176940094: SPAWN-1a owns `content/activation.rs`.
- #1735 P1 4176940109: SPAWN-1a ships room revision 2, which moves the rat to a new non-proof
  cell `entry-den` with map and content r2. The amendment is recorded in the product bindings
  decision and takes effect on SPAWN-1a's merge (now SPAWN-1A-PACKET-1, #1745).
- #1735 P1 4176975924: SPAWN-1a owns `entry_chest::CONTENT_REVISION` and
  `tests/content_native_entry.rs`, both room-r2 consumers (§2.7).
- #1735 P1 4176975932: the revision-2 spawn record carries `respawn_delay_ms` 60,000 and
  `occupancy_retry_interval_ms` 5,000, and the qualifier refuses missing or out-of-range values
  (first-creature §4.3, §4.8; §1.4, §2.7).
- #1735 P2 4176975942: CREATURE-AI-0's amendment names the carrier's `d116_definition()` as a
  test helper and points to the content spawn in #1745 §1.1. §6.1 keeps D116 (one spawn, 2 rats),
  restored after #1745 P1 4177068268 (control plane, option a).
- Control plane D492 (owner 7a): room revision 2, the spawn inputs, the activation seam, the
  SPAWN-1a packet and the bindings amendment moved to SPAWN-1A-PACKET-1 (#1745), with #1735
  P1 4177035356 (r2 package and lock identities), P1 4177035359 (`qualification.rs`) and
  P1 4177035362 (`test_arena_map.py`). This PR keeps the SPAWN-1a/1b split ruling (§1.4, §2.7
  stub) and SPAWN-1b.
- No code, contract or wire change in this PR.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

# OTV2-20261004-arch-core-loop-packets-2b

```yaml
task_id: OTV2-20261004-arch-core-loop-packets-2b
title: "ARCH-CORE-LOOP-PACKETS-2 part B: core loop client packets (session push deltas, entities, chat, items, attack, item use)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-core-loop-packets-b-20261004
issue: 162
pr: 1736
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-core-loop-packets-2b.md
public_contracts: []
depends_on: [CLIENT-NEG-1]
blocks: [SESSION-PUSH-1, ENTITY-CLIENT-1, CHAT-CLIENT-1, ITEM-CLIENT-1, ATTACK-CLIENT-1, ITEM-USE-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- D486 item 3. New batch `OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md`.
- §1.1: two tracks. The features land first in the session crate, the dev client and the D93
  harness live mode, where they are playable as soon as their server packet merges. The
  `apps/client` windows follow ADR-0020 N4, and `apps/client` stays fail-closed.
- §1.2: server-initiated deltas are applied by domain revision (FND-02 §14, §15). This fixes the
  latent idle poison: on `main`, a Serene vitals delta poisons an idle session. Each command's
  result and its own deltas are written as one contiguous run, and that server invariant is
  binding.
- §1.3: a capability is advertised only once its domains and commands are complete, and the
  advertised set is closed under `requires`.
- §1.4: one client lane (`crates/session/**`, `tools/dev-client/**`, the harness `live/**`).
  SESSION-PUSH-1, then ENTITY-CLIENT-1, CHAT-CLIENT-1, ITEM-CLIENT-1, ATTACK-CLIENT-1 and
  ITEM-USE-CLIENT-1. API changes are additive only.
- §2: the six packets.
- §3: item batch ITEM-CLIENT-1 to 4 are retargeted to the playable track. The `apps/client`
  windows become the -P packets after N4.
- §5: the control-plane queue: N4P contract-text acceptance (ARCH-N4P-ACCEPT-1), the missing
  ITEM-USE-WIRE-1 packet, and N8.
- #1736 P1 4176908866: the client attributes no delta to a command. Exchanges read push-driven,
  by domain revision only, and the session-side check for a delta between a result and its own
  delta is removed (§1.2, §2.1).
- #1736 P2 4176908870: ITEM-USE-CLIENT-1 has no local food or potion rejection, so the server
  decides (§2.6).
- #1734 P2s: ENTITY-CLIENT-1 decodes the capability-6 type-2 snapshot in `Session::admit`, and
  SESSION-PUSH-1 consumes deltas push-driven without predicting the next domain (§1.2, §2.1,
  §2.2).
- No code, contract, wire, registry or migration change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

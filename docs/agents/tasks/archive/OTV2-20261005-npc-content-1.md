# OTV2-20261005-npc-content-1

```yaml
task_id: OTV2-20261005-npc-content-1
title: "NPC-CONTENT-1: NPC service model"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/npc-content-1-20261005
issue: 1622
pr: 1858
head_sha: "exact frozen head in the FREEZE report to the control plane"
final_head_sha: "exact frozen head in the FREEZE report to the control plane"
owner: claude-code-session_01KKGrnv6QSBXGu6oZ7notzp (oteryn-impl-worker for control plane session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-05
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/npc_catalogue.rs
  - apps/game-server/src/content/npc_catalogue/service.rs
  - apps/game-server/src/content/npc_catalogue/service_tests.rs
  - apps/game-server/src/content/npc_catalogue/replies.rs
  - apps/game-server/tests/content_npc_service_model.rs
  - docs/agents/tasks/archive/OTV2-20261005-npc-content-1.md
public_contracts: []
depends_on: []
blocks: [NPC-TALK-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet §2.2 of `OTERYN_GAME_ARCH_NPC_PACKETS_2026-10-05.md` (decision §1.2). A deterministic NPC
service model over `NpcDataCatalogue`: per-offer admission or hold, routes, held NPCs and generated
replies. No catalogue digest is pinned (PR #1830 changes it); counts are measured on current main.

Measured on `content/world`, revision `g4-npc-provisional-enrichment-r28`: 1,282 NPCs, 0 held
NPCs, 380 Services, 11,903 admitted offers, 195 admitted routes, 446 NPCs on generated replies
(1,282 - 836 Dialogues).

Held offers by reason: `NonGoldCurrency` 31, `SellPriceAboveCoinCapacity` 299,
`ArbitragePaying` 1, and 0 for `CountOutOfRange`, `ParityPending`, `TimedCountMismatch`,
`UnknownItem`.

## Decisions and assumptions

- Rule order: non-gold currency, unknown item, count rules, sell price above 1,009,999,
  `parity_pending`, then arbitrage over the offers still admitted. The packet enum lists
  `ArbitragePaying` before `ParityPending` but says arbitrage runs last; parity is checked first.
- Arbitrage compares per-unit prices exactly (u128 cross-multiplication of `unit_price / count`),
  strictly greater than the lowest admitted sell price for the same item ref and sub_type.
- Every route loads: the source has no gate field, so there is no held-route list.
- An NPC is held whole when its Dialogue or Service reference does not resolve.
- NPC display name is derived from the key slug (`oteryn:npc.a_seagull` is "A Seagull"); only 133
  NPCs have `bulk.name`, so there is no uniform name field. Assumption for NPC-TALK-1 to confirm.
- Generated replies carry exactly one line per referenced service (repair of Codex P2): `Trade`,
  `Travel`, `TradeAndTravel` (offers and routes) or `Idle` (empty or fully held, retained).
- Service recipes are ignored.
- `mod npc_catalogue` is private and `content/mod.rs` is not owned, so the model types are not
  nameable from integration tests; entry points are methods on `NpcDataCatalogue` and the test
  uses only those. NPC-TALK-1 owns the re-export.
- Item facts come through the `NpcItemFacts` trait; the production adapter reads the public
  `ReferencePlayableContentSource.definitions`, so `reference_playable.rs` and `item_admission.rs`
  are untouched.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server npc_catalogue`: pass
- `cargo test --locked -p oteryn-game-server --test content_npc_catalogue`: pass
- `cargo test --locked -p oteryn-game-server --test content_npc_service_model`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

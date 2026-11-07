# OTV2-20261003-item-client-1

```yaml
task_id: OTV2-20261003-item-client-1
title: "ITEM-CLIENT-1: capability 4 item view, corpse open and loot in the client session, the dev client and the live harness"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-client-1-20261003
issue: 1622
pr: 1769
head_sha: "exact frozen head in the FREEZE entry to the control plane"
final_head_sha: "exact frozen head in the FREEZE entry to the control plane"
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - crates/session/src/**
  - tools/dev-client/src/**
  - tools/synthetic-client-harness/src/live/**
  - docs/agents/tasks/archive/OTV2-20261003-item-client-1.md
public_contracts: []
depends_on: [CHAT-CLIENT-1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md` §2.4.

- `CLIENT_SUPPORTED_CAPABILITIES` is `[4, 6, 7, 13]`, closed under the registry `requires`.
  Capabilities 12 and 14 are not advertised; only the plain capability-4 move forms are used.
- Domain 9 (inventory) and domain 11 (open container) snapshots and deltas decode through the
  fail-closed store (revision, registered type); events `Inventory` and `OpenContainer`. A
  duplicate `item_handle` in one snapshot or delta is refused. Without capability 4 either domain
  is refused.
- With capability 4 the domain-1 entity codecs are the `_with_item_handles` forms: field 10 is
  required on corpses and ground items and refused otherwise.
- `Session::use_item` (command 2, item target) and `Session::move_item` (command 9) return at
  their result; a `Moved` result's domain 11 and 9 deltas arrive as pushed deltas. `STALE` and
  every non-`Moved` result change nothing locally. Both are refused without capability 4 before
  anything is sent.
- `dev-client` re-exports the item types and adds `inventory()`, `open_container()`, `use_item()`
  and `move_item()`.
- Harness: clicking a corpse with a handle opens it, backpack and corpse panes render, `loot N`
  moves entry N to the backpack.
- Carried P2s from #1762: 4177772522 (a chat input without capability 7 is a nonfatal notice and
  the help is hidden by capability), 4177772525 (`pm "Al Dric" hello` quoted recipient),
  4177772529 (a selected but empty chat pane renders `chat []`).
- Not in scope: equipment, nested bags, Ground (ITEM-CLIENT-2/-3/-4). No protocol, registry,
  proto, server or `apps/client` file touched.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet`: pass
- `cargo check --locked -p oteryn-game-server --tests`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

# TRADE-PLAYER-1 — player-to-player trade (skeleton task record)

```yaml
task_id: TRADE-PLAYER-1
title: "TRADE-PLAYER-1: player-to-player trade"
mode: IMPLEMENT
status: local_preparation_pending_qualification
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row TRADE-PLAYER-1 (row 82); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (TRADE-WIRE-1, TRADE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/trade-player-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Two players of one channel exchange items safely: capability, trade command and domain (TRADE-WIRE-1), trade session with TRANSFERRING state and the two-Character swap transaction with every persistence delta (TRADE-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

TRADE-WIRE-1 (impl, protocol review, Reg) -> TRADE-1 (hard, persistence and security review). BAGS-TRADE-1 is an integration child released by TRADE-1 and outside this completion set.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a (TRADE-1) | BLOCKING (ITEM-LOOP-1 child) |
| ITEM-VIEW-1b | MERGED (archive OTV2-20261003-item-view-1b) |
| VIS-2 | MERGED (archive OTV2-20260930-vis-2) |
| acceptance of PLAYER_TRADE0 | Effective acceptance on main 3ef900c160c9713092a86bfcd1a8377c7587c898 |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/player_trade.rs`
- `apps/game-server/src/durability/player_trade.rs`
- `apps/game-server/src/durability/player_trade_tests.rs`
- `apps/game-server/src/gameplay_transport/player_trade.rs`
- `apps/game-server/src/gameplay_transport/player_trade_tests.rs`
- `crates/protocol-oteryn/src/player_trade.rs`
- `crates/protocol-oteryn/src/player_trade_tests.rs`
- `apps/game-server/tests/player_trade_postgres.rs`
- `docs/agents/tasks/TRADE-PLAYER-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- connection.rs: command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (see CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs: `GATED` table l.165 and offered sets l.76/104/132 (capability `TRADE_V1` not offered until CP decides)
- crates/protocol-oteryn/src/lib.rs: `pub mod` list l.2545-2568
- durability/mod.rs and domain/mod.rs module registration
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability, command type, state domain)

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests (TRADE-WIRE-1)
- migration + swap writer + durability tests (TRADE-1)
- item move/MINT-free swap: both inventory deltas + audit in one transaction (DUR-03 trade swap amendment)

## Acceptance criteria

- [ ] A trade between two Characters on one channel either commits both item deltas or none (crash/restart test).
- [ ] TRANSFERRING state survives restart and resolves deterministically; concurrent offers lose with a typed refusal.
- [ ] Cross-channel trade is refused.
- [ ] Wire gated by capability; older clients receive no trade domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server trade
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md §3-§6
- DUR-03 §34 trade swap paragraph (amended by PLAYER_TRADE0)
- ITEM-MOVE-WIRE-1 (archive OTV2-20260930-item-move-wire1)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded leaf lifecycle preparation (2026-10-15, draft #2132)

Source `332f8872506133d0ca48f75fc19d0a378140ee50`, branch
`skeleton/trade-player-1`. Exact allocation: `domain/player_trade.rs`,
`gameplay_transport/player_trade.rs`, its `_tests.rs` and this record. No wire
types/IDs, registries, shared actor/connection/module wiring, migration, durability
writer, content or pins are changed. BAGS-TRADE-1 remains an integration child
outside this completion set; no container-tree writer or whole-tree admission is
invented. ITEM-LOOP-1, ITEM-MOVE-2a, TRADE-WIRE-1 and actual swap integration gates
remain open even though the owning PLAYER_TRADE0 decision is now accepted.

### Accepted basis and useful prepared subset

The effective owning text was read on accepted main
`3ef900c160c9713092a86bfcd1a8377c7587c898`: PLAYER_TRADE0 §§4–6. The branch's
CANDIDATE header is historical. The detached code records lifecycle policy over
already independently owner-revalidated actions; it does not resolve a target,
authorize an offer/accept, install reservations or dispatch a production command.

- `OFFERED` contains the first reference; a revalidated counter-offer enters
  `READY`. Both accepts bind that same immutable pair. The internal pair key is
  the native counter-offer `CommandRef`; TRADE-WIRE must allocate/map its actual
  server-issued `trade_id`. No serialization or new public protocol identifier
  is chosen here.
- Native `CurrentCharacterItemFence`, `CommandRef`, Account/Character/World/
  Channel/session types, `CharacterBackpack`, `ContainerEntryPosition`,
  `OneItemStateV1` and `StoredTimedState` are reused. The leaf binding includes
  native definition/count/lifecycle/key state, timed state and immediate position;
  a missing timed row means a proven absent row, not a failed read. Snapshot
  equality detects changed item data, fences, positions or destination contents.
- Self/same-Account and nonmatching Channel scope pairs are refused. An immutable
  first terminal accept cannot be replaced by a later accept from the same side.
  The second side's accept becomes the trigger in a retained transfer binding.
  A stale pair/foreign command does not cancel a different current READY pair.
- `OFFERED`/`READY` expire after exactly 120 seconds without a new offer/accept;
  backward time is refused. The owner must use its actual monotonic clock.
  Pre-transfer invalidations/reconnect cancel; a changed captured binding is stale.
- `TRANSFERRING` retains its immutable binding on unknown outcome, timeout, cancel
  and same-GameSession reconnect. A different session cannot adopt it. Only supplied
  matching terminal outcome data closes the policy state. The helper does not
  authenticate that outcome or release a physical reservation.
- The separate native flag predicate explicitly allows `tradeable: UNKNOWN` and
  Known true, refuses Known false/ambiguous states, and never treats `marketable`
  as the trade flag. It checks only that flag, not complete item eligibility.

### Exact prerequisite and integration seams (REPORT_ONLY)

The available native APIs do not provide an allocated complete trade admission
and reservation seam. `ChannelRuntimeV1::owner_fence`, `player_control_facts`,
`visible_entities` and exact actor/position access live in the unowned Foundation
carrier. Item handle resolution is the session-owned `SessionItemView::resolve`;
the main-backpack observation path is `item_move::observe_inventory`. Neither a
client handle nor a cached snapshot proves current ownership, visibility or reach.
Actual at-use target generation, same-floor 2-tile range, visibility and sightline
must be independently revalidated at start, counter-offer and **every** accept.
No fake visible/sightline boolean, proof constructor or default-success validator
is introduced to bridge this gap. The code's `record_revalidated_*` names express
a caller requirement; they are not evidence that those runtime checks are wired.

The owner must also recheck capability support, actor exclusivity (one open trade
per actor), current live session/lease/scope fences, active Content, source ownership,
pickupability, non-corpse classification, empty offered container and main-backpack
direct-entry location. The main backpack itself, ground/corpse sources, filled bags
and untradeable items remain refused. Equipped sources and room/capacity need their
own accepted integration; no unknown eligibility fact may be promoted by this data
model. Reuse actual native stored state; any additional admitted instance state must
be included before that class is supported. Equality of immutable snapshots does
not establish a current session, active generation, storage read or entitlement.

`runtime_actor_source_reservation` reserves actor combat slots, not offered item
instances and free backpack entries. It must not be substituted for PLAYER_TRADE0's
two-item/two-entry reservations. Under the real Channel owner, the second accept
must atomically stage the policy transition with genuine reservation acquisition,
then issue the one swap cause bound 1:1 to its pending native CommandRef. The first
accept's terminal CommandRef/fence is pinned; only the permitted connection
generation advance can be handled by the actual §5 writer. No policy binding here
is a reservation or writer-authority token.

All issued and committed source/destination mutations (including move/use/drop,
fee/NPC sale, equipment, reward mint and death), range loss, logout, kick, reconnect
and channel transfer must reach the owning open-trade index. Before TRANSFERRING
they cancel; during it, conflicting mutations are refused, channel transfer and
logout/kick are delayed, and death waits behind the outcome. Runtime reservations
stay held on ambiguity until the real writer/reconciler proves the matching outcome.
After a crash safety must come from atomic database source/room checks and replay,
not reconstruction of vanished in-memory reservations from these snapshots.

The actual slot still needs both native fences, canonical two-root/item lock order,
full before-state and pre-removal free-entry checks, no received-stack merge, one
atomic two-Character swap, cause/receipt/placement/entry guards, audit shape and
measured resource rows. No CharacterRevision advances for this item-only swap.
Only the actual writer/reconciler can verify terminal outcome data before policy
closure and physical reservation release. Historical outcome/binding data cannot
reacquire gameplay authority. PostgreSQL crash/restart/replay, concurrent accepts,
current-fence/visibility changes and runtime reservation tests remain unperformed.

This is source preparation, not active runtime, TRADE-1 completion, architecture
approval or a ready candidate. Independent exact-head review and CP/architect
integration gates remain required. No worker commit/push/ready/merge/review trigger
or external action is performed.

### Validation and self-review

- Actual native-library source-bound direct test: **10 passed, 0 failed,
  0 ignored**. The small external harness re-exports actual native modules and
  path-loads the owned domain/lifecycle files and tests once; no native type,
  source excerpt, fake sightline/session authority or storage implementation is
  reconstructed. There was no missing-file RED ceremony.
- Cases cover Unknown tradeable with marketable false, self/same-Account/channel/
  instance refusal, both acceptance orders, pinned first acceptance and triggering
  CommandRef, stale pair/foreign session/fence substitutions, native key/timed/
  position/backpack changes, pre-transfer invalidation/reconnect, exact idle
  boundary and time regression, ambiguous retention, same-session reconnect,
  mismatched terminal binding and all terminal outcome classes. These are pure
  policy/data tests, not current runtime sightline, physical reservation or DB
  authority qualification.
- Strict native `clippy-driver --test --emit=metadata`, owned Rust formatting,
  `cargo fmt --all --check`, governance validation (22 documents, 9 lanes),
  **124 governance tests** and diff/scope checks passed. All Rust checks used
  the shared flock/target, two jobs and `CARGO_INCREMENTAL=0`. No Cargo build or
  implicit full-server link was run.
- Exact sources/native-library/log/test-ELF hashes, commands and local patch are
  retained at `/workspace/oteryn-prep/trade-player-1-evidence/`. Used native type
  and behavior definitions match the preserved library baseline and accepted main;
  accepted main's protocol root additionally registers logout, outside the used
  UUID identity definitions (the exact unrelated diff is retained). The
  `75fc517e...` library is unchanged before/after checks. Full package, wire,
  PostgreSQL swap, replay/restart and integrated actor/reservation qualification
  remain pending and are not implied by these results.
- Whole-diff review confirms exactly four allocated paths and no staged content,
  commit or shared wiring. The 334-line implementation and its 10 focused native
  tests form one lifecycle/binding unit; no separate speculative framework or
  alternative writer is added. Inputs/return values remain policy data, with all
  real authority/reservation/swap obligations above explicitly unimplemented.

Rust SHA-256:

- Domain: `2a7bc36e34503c924501f2da1b98729b0aa8898eed591176f4c896c9526fabbe`.
- Lifecycle: `d01a6b6400c82bf6cb420951c6a260837871c41d35fe5ce8f7b30a3669364780`.
- Tests: `8332e2e8d8cf2d8ffcd35c3ac12693ced7365059824d78cbdd15e0dc7cd6eaa9`.

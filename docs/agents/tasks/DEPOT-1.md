# DEPOT-1

```yaml
task_id: DEPOT-1
title: "DEPOT-1: playable per-town depot open, browse, deposit and withdraw"
mode: IMPLEMENT
status: IN_PROGRESS
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-wire-depot
pr: null
base_sha: 4a5d6c0f7b5a976571796c489fa3b4e8960d4932
owner: Codex standalone task worker
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - apps/client/src/combat_input.rs
  - apps/client/src/depot.rs
  - apps/client/src/lib.rs
  - apps/client/src/play.rs
  - apps/client/src/windows_shell.rs
  - apps/game-server/migrations/0157_depot_transfers.sql
  - apps/game-server/src/lifted_tests/auto/depot_postgres_cases.rs
  - crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs
  - crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/depot.rs
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/depot_dispatch.rs
  - crates/gameplay/durability/src/durability/character_depot.rs
  - crates/gameplay/durability/src/durability/depot_move.rs
  - crates/gameplay/durability/src/durability/item_loop_move.rs
  - crates/gameplay/world/src/map/facts.rs
  - crates/protocol-oteryn/src/capability_tables.rs
  - crates/protocol-oteryn/src/depot.rs
  - crates/protocol-oteryn/src/depot_tests.rs
  - crates/protocol-oteryn/src/lib.rs
  - crates/session/src/depot.rs
  - crates/session/src/domain_dispatch.rs
  - crates/session/src/lib.rs
  - crates/session/src/map_transport.rs
  - crates/session/src/tests/depot_map_tests.rs
  - docs/agents/tasks/DEPOT-1.md
  - docs/contracts/protocol-oteryn/v1/depot_v1.proto
  - docs/contracts/protocol-oteryn/v1/registry/capabilities/0037-depot-v1.json
  - docs/contracts/protocol-oteryn/v1/registry/command_types/0036-depot-intent.json
  - tools/synthetic-client-harness/src/live/model.rs
public_contracts:
  - docs/contracts/protocol-oteryn/v1/depot_v1.proto
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Vertical wiring gap sweep (2026-10-23)

Owner now authorizes the complete playable depot path in this worktree; previous
storage-only activation deferral below is historical. Local AUTHORING only; no
Git mutations or publication. Reuse the native item loop and composed admission.

| Layer | Present | Missing at start of this continuation |
|---|---|---|
| L3 items | 17 boxes, 32-entry pages, 2,000 Free / 15,000 hard limits | Live move admission consuming these bounds |
| L4 durability | Migration 0151 custody/counters; read-only representation | Fenced deposit/withdraw receipts, removal/counter proof and bounded page reader |
| protocol-oteryn | Item handles, inventory and map domains | Negotiated depot request/result codec and append-only registry fragments |
| L5 dispatch | Composed session authority, item loop, map reach | Depot locker resolution, page/move handler and connection routing |
| game-server | Composes the real L5 admission and embedded migrations | Enable the new route through that composition and migration |
| client | Session command exchange, native play input/status surface | Depot request/reply handling, browse/deposit/withdraw controls |
| E2E | Real composed connection/PG harness | Session -> depot command -> receipt/custody -> decoded reply scenario |

## Scope and implementation

Local AUTHORING only, under the owner's depot vertical-wiring task. All changes
remain uncommitted. The task stays IN_PROGRESS for independent review and CI;
no candidate, PR, publication, paid review or protected integration was created.
Bound META 3.1.0 was read at immutable commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

The prior merged foundation was intentionally dormant. This continuation replaces
that activation deferral with the owner's explicit playable-path authorization.
It preserves the owner's per-town override: Character + World + u16 town id,
including zero; Channel never partitions storage. Seventeen virtual boxes reuse
native ItemInstances. Inbox custody and the existing eight other custody arms
remain owned by their original mechanisms.

- Protocol: append-only capability **37 DEPOT_V1** (requires 4), command
  **36 DEPOT_INTENT**, strict bounded request/reply codecs and schema. A command
  result carries the selected box's newest-first page, at most 32 entries.
- Map/admission: resolve the nearest visible authored `Attrs.depot` binding in
  Chebyshev reach 1 on the actor's current floor, deterministically breaking ties
  by position and ordinal. No client-supplied Character, locker authority, item
  identity or Premium tier is accepted. Every request checks the live admitted
  session/generation and native content pin. A move must name the town last shown.
- Dispatch: the production ComposedFreshAdmission implements the route; the
  game-server already composes that owner. Capability negotiation limits the
  offer to bundle Worlds. Unknown commit outcomes disconnect before sequencing.
- Durability: migration **0157** adds depot kind 4 to the existing item-loop
  receipt/proof owner. Deposit and withdrawal retain current Character/session,
  lease, Channel ownership, node/recovery and Content fencing, item row locks,
  conservation, immutable receipts and deferred exclusive custody. Withdrawal
  retains high-water ordinals while decrementing current capacity; zero is valid.
  Runtime writes require a current transfer line, and ordinary raw removal and
  placement remain rejected. The old storage negatives now exercise both the
  original privilege restrictions and the newly granted, proof-gated columns.
- Rules: whole native items/stacks only; nonempty containers remain refused.
  Deposit uses the existing native session Premium gate at the decision point:
  Free/unavailable/not-current admits 2,000 entries per town; current Premium
  admits 15,000. The database independently enforces 15,000. Withdrawal uses
  the existing backpack room, carried-tree and weight planners, including STACK-1
  merges. No new third-party dependency or second transfer subsystem was added.
- Replay: receipts bind the canonical request digest, town, box, direction and
  Character. A lost reply is resolved before ephemeral handles/ordinals. Reusing
  a CommandRef with another request is rejected. A terminal replay outside reach
  returns Moved with the view closed; it grants no locker access. Existing FND-02
  lower-command-id expiry remains unchanged; no duplicate cache was invented.
- Client: the real Session sends and decodes the command and waits for committed
  inventory before accepting another input. The existing native play status line
  exposes controls and selected bag/depot entries. Bundle-map capability 18 was a
  necessary admission dependency: Session now validates its join and bounded
  deltas against Content/bundle/reset binding and revisions, preserving the
  existing renderer. The synthetic harness's exhaustive event match was updated.

## Play controls

Stand beside an authored depot locker. **Ctrl+D** opens/refreshes it.
**Ctrl+B** cycles boxes 1–17; **Ctrl+P** cycles pages; **Ctrl+I** selects a direct
main-backpack entry; **Ctrl+O** selects a displayed depot entry; **Ctrl+J** deposits
one whole item/stack; **Ctrl+K** withdraws it to the main backpack. The play status
line shows town, box, page, selected entry/reference, quantity and refusal text.
Selection is local; every mutation and reply comes from the server. Reconnect
starts with no depot panel. Item names/icons and a graphical container grid are
outside this minimal UI.

## Reference parity

Canary revision: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`. Reference was read
via immutable raw source URLs; no original files/assets are redistributed.

| Wire-visible behavior | Reference | Native behavior / evidence |
|---|---|---|
| Message shape and negotiation | [Canary source at the pin](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/server/network) is behavioral reference, not the runtime wire owner | Native capability 37 / command 36 and `depot_v1.proto`; existing IDs untouched. Codec round trips, malformed/bounds tests, capability refusal and Session-stream tests |
| Locker access and identity | [DepotLocker](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/depot/depotlocker.cpp) is a virtual access container; direct add returns NOTENOUGHROOM | Authored reachable locker binding; private Character depot. Native NoLocker and Stale dispositions; live-dispatch reach, wrong-town and stale-generation negatives |
| Fixed compartments | Owner's accepted DEPOT-0 / DEPOT-1 rule, [Tibia manual](https://www.tibia.com/gameguides/?section=world&subtopic=manual) | 17 boxes; differs deliberately from Canary configuration. L3 reference table and PG storage cases include box 17 |
| Pages | [DepotChest constructor](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/depot/depotchest.cpp), `maxSize=32`, pagination enabled | 32 entries per page, zero-based pages 0..468, descending retained ordinal; strict codec bounds and bounded SQL read |
| Free capacity / full refusal | Same constructor and `queryAdd`: 2,000, DEPOTISFULL | 2,000 across the town's boxes; native Full result. Existing L3 bounds, SQL hard-cap/concurrency tests; runtime reads the native Premium owner |
| Premium / database ceiling | Owner's accepted DEPOT-1 manual ceiling; Canary configuration is not imported | 15,000 only when the native gate proves current Premium; unavailable status is Free. L3 bounds and existing 15,000-entry storage PG case |
| Whole stack cost | Same `queryAdd`: whole external stack adds one entry | Quantity 100 remains one entry; real session/dispatch deposit/browse/withdraw conserves quantity and custody |
| Nonempty bags / partial stacks | Canary permits broader container and partial-stack shapes | Intentional accepted first-slice restriction: whole items/stacks and empty containers only. Existing every-family contents refusal tests remain active |
| Withdrawal admission | Canary delegates to destination container admission | Existing native main-backpack room/tree/weight and stack planner; native Full, NoBackpack, Overweight or Unsupported; no partial write on refusal |
| Retry and stale source | Native DUR-03/FND-02 contracts own receipt and session behavior | Exact-intent durable replay; altered request and stale ordinal/generation fail closed. PostgreSQL connection test exercises the real route |
| Town sharing | Owner's 2026-10-23 explicit override of DEPOT-0 / Reference sharing | Separate Character + World + town custody, shared across Channels; no silently changed global decision |

## Validation

All final Cargo commands use the queued wrapper explicitly because a login shell
puts `/home/mole/.cargo/bin/cargo` first. Before the correction, bare-cargo
route-less results were discarded as PostgreSQL evidence. The wrapper supplies
the PG route and offline dependencies; no URL or credential was printed.

Exact reproducible commands (run inside this worktree):

```bash
export PATH="/home/mole/oteryn-cache/codex-bin:$PATH"
export CARGO_NET_OFFLINE=true
cargo fmt --all -- --check
cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-world -p oteryn-gameplay-dispatch -p oteryn-session -p oteryn-client -p oteryn-game-server -p oteryn-synthetic-client-harness --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-client --lib --quiet
cargo test --locked -p oteryn-gameplay-items --lib --quiet -- depot::tests::
cargo test --locked -p oteryn-gameplay-durability --lib --quiet -- schema::contract_tests::
cargo test --locked -p oteryn-gameplay-world --lib --quiet -- map::facts::
cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- capabilities::tests::
cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- depot_session_dispatch
cargo test --locked -p oteryn-game-server --lib --quiet -- depot_postgres_cases::
cargo test --locked -p oteryn-synthetic-client-harness --quiet
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

The server/dispatch PG cases create private databases using the real embedded
migrations on PostgreSQL 18.6. No unfiltered PostgreSQL suite was run. The new
case is auto-mounted under the existing dispatch library PG CI lane; no new
integration target or CI routing change was made.

Final results: PASS for fmt, all-target Clippy on all eight changed packages,
protocol 390 tests, Session 72, native client 86, item bounds 1, schema contracts
8, world map facts 6, dispatch capability/selection 61, depot real-dispatch PG 1,
existing depot storage PG 6, synthetic harness 37 and governance unit tests 124.
Both Python validators and `git diff --check` pass. The two PG runs took 26.48 s
and 14.83 s respectively; none was route-less. There are no unresolved local
check failures or CI-only contention exceptions.

Whole-diff self-review completed against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: bounded reads, native current fences,
receipt replay and unknown-outcome disconnect, least-privilege writes, append-only
wire IDs, no test weakening, and selected-capability checks on both ends.
The original item-loop admission (0130) and final-state proof (0128) bodies were
compared byte-for-byte after removing only the added depot arms; every old arm
is unchanged. Workspace-wide constructor/match/implementation searches covered
all added enums, fields and traits across `apps/`, `crates/`, `tools/`, including
the new WorldMap session event and synthetic consumer. All added public APIs
have production callers; the real dispatch/PG and Session stream tests cover the
new path. Existing test doubles retain explicit fail-closed trait defaults.

## Unfinished / limits

- Independent high-risk review, Windows runtime verification and protected CI /
  integration remain with the control plane. This worker was explicitly forbidden
  to commit, publish, mutate CI routing or send repository content for review.
- This is the accepted empty-container / whole-stack depot slice. Inbox withdrawal,
  nonempty depot container trees, stash/search, partial stacks, cross-box moves,
  MAIL/MARKET and graphical container grids remain their separate owners' work.
- The native client retains its existing static local map artwork. This change
  admits and validates bundle map traffic needed by depot access; it does not
  claim a complete streamed-map renderer or cross-floor map resynchronization.
- Premium deployment activation is unchanged. Capacity consumes the existing
  current native gate; no client tier or new entitlement authority is introduced.

Next step: control-plane review of the local diff and repository CI; archive the
card only in the eventual completing PR, not as a separate archive-only change.

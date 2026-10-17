# TRADE-PLAYER-1 — player-to-player trade (skeleton task record)

```yaml
task_id: TRADE-PLAYER-1
title: "TRADE-PLAYER-1: player-to-player trade"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
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

## Native runtime owner successor (2026-10-15; bounded #2132 preparation)

This successor starts at published `bd20009b7eb05bc384f7f6e15ba4a4fc6d43b244`.
The exact current allocation is the two original domain/transport sources, the
transport test source and this record. The earlier leaf result above remains
historical evidence. Original acceptance boxes and the actual swap/wire/integration
requirements remain open.

### Implemented native owner and current observation boundary

`TradeOwner` is an owner-local, capacity-bounded book over the original leaf
lifecycle. Each operation borrows the genuine current Channel runtime; an open
trade does not retain an exclusive borrow that would block movement/combat. The
per-actor index prevents a second open trade inside this book. Installation of
exactly one book in the live Channel owner, and routing all issued/committed
physical invalidations into that book, remain CP composition work. Construction
alone does not prove process-wide uniqueness.

At start/counter-offer/every accept the owner checks actual native owner continuity,
player control and bound Character lease, exact actors and Channel/generation,
actual native interest queries in both directions (including the 256-result
cutoff), same-floor Chebyshev range at most two and the native qualified map/LOS
path. Missing or unknown dynamic map facts refuse. No passed visible/current/LOS
boolean or callback grants admission.

The same allocated transport file proposes one private `durability::player_trade`
mount so it can reuse the genuine private Root/equipment/assignment readers.
Outside-tree qualification only adds `domain::player_trade` and that one
`#[path = "../gameplay_transport/player_trade.rs"]` durability mount. Candidate
shared roots remain unchanged. CP must install that one context rather than also
register a duplicate gameplay module or weaken private native visibility.

The read-only Root operation uses actual recovery and session/connection/Character
lease/scope/admission/current Node possession and independently active DB Content
checks through `assert_current_equipment_in_transaction`. Both Character roots are
locked in CharacterId order before inventory reads. Under the native assignment
SHARE lock, the existing `load_assignment` owner reads the actual Assigned scope,
generation, holder/incarnation, source revision and decision identity. The opaque
runtime binding is expected DATA: its comparison receives only independently read
current row facts. No decision string is manufactured from the binding to certify
itself. The separate native row guards couple source/generation updates and forbid
isolated rollback/reuse; tests do not disable those guards to invent current state.

Actual native backpack/entry readers and item/timed rows supply current direct
membership, definition, quantity, lifecycle, key, state revision and original
placement TransactionId for both direct destination-backpack inventories and offered leaves.
A main backpack, foreign/non-direct/ground/equipped/corpse item or item with
contents refuses. Actual activated native item records supply pickupability and
the direct-trade flag; accepted `tradeable: UNKNOWN` remains allowed. Source 2854's
known Container pattern/capacity and source 3031's actual stack facts supply the
native test MINT/TRANSFER inputs; no policy is invented to convert unknown data.

Retained before-values detect intervening changes but authorize no later swap.
The original immutable first accepted CommandRef, exact second trigger, 120-second
idle boundary, pre-transfer cancellation and same-session TRANSFERRING reconnect/
unknown retention remain. A second valid accept computes only two new direct-entry
addresses before removal, with no merge/freed-entry credit. `ReservationIntent`
is DATA: no physical reservation, trade occurrence, item move, ledger or event is
written, and no owner API accepts a supplied terminal enum as proof to release the
book. A real swap writer and receipt reader must close that boundary.

### Qualification and authority matrix

The outside-tree native production context compiled the actual new owner/Root
reader with the repository's configured Clippy denials: exit0. Rust unused-code
warnings remain visible; no blanket `-D warnings` clean result is claimed. The
selected normal cfg(test) native context linted and compiled all 19 exact own test
bodies (16 leaf/native-runtime,3 PG), then an actual native driver ran them:
18 passed,1 failed,0 ignored,0 filtered,676.76s. All3 bodies reached PostgreSQL17.6
(`170006`); the complete owner path and current-authority matrix passed.

The sole failure was a test-fixture registration mismatch: its genuine native
Revoke writer was opened as `trade-revoke` on a Root already bound by the unchanged
native Harness to `item-transfer-writer`. Native `RuntimeScopeAssignmentWriter::open`
correctly returned typed `Unsupported`. The repair changes only that one literal
to the already-bound registration. Between the two native qualification snapshots,
both complete production files, the shared native fixture/predicates, all other
2,281 materialized inputs and the other 18 exact test bodies remain byte-identical.
This record is appended after execution; its documentation delta changes no
compiled Rust. With all 61 captured extern/proc-macro hashes still unchanged, the final 19-body selected native context again passed Clippy and
native compilation; the corrected original PG body then passed alone on170006:
1 passed,0 failed,0 ignored,0 filtered,47.43s. Its original genuine Revoke-commit and
retained-book refusal assertions remain. Qualification is explicitly the composite
of18 unchanged prior passes plus1 fresh repaired native PG pass, **not a fresh
19-case run**. The original18/1 failure and both source generations are retained.

Rustfmt passes for the three owned Rust files. Full-workspace fmt fails only at unchanged base
`apps/game-server/src/durability/item_fee_burn.rs:627/668` (SHA256
`b9aff053159d43b990ec5833cfee728cb1df8c9aa3c560f784c00d6109d511da`);
Git/base/clone/snapshot equality and the actual failing log are retained. No
unowned formatting or lint allowances were changed. Initial missing sparse Cargo/
fixture inputs were setup failures, not behavioral RED. Exact ownHEAD blob/SHA/
consumer manifests preserve their materialization. Cargo's deliberate native-root
capture exit101/sentinel86 is graph capture, not a test pass. Original whole-native
unit lint exceeded this host's8GiB budget in another lane and is not claimed;
protected exact-head CI must supply it. There is no owned standalone PG wrapper
in this allocation.

Native tests independently vary named session/Character/lease/scope, actual control
loss, owner retirement, foreign map, range2/3, floor, actual blocking ray and actual
visibility cutoff. Item state/placedTx/timed/member mismatches include explicit
DATA equality controls. The actual PG bodies seed a complete second Character and
its session/guard rows as independent privileged fixture DATA before genuine Root
reconciliation, then use real Node/assignment/recovery/Content activation and
MINT/TRANSFER. They do not prove two-client admission/controller installation.
Expected fences and book bindings remain fixed while current session generation,
lease, scope, eligibility, actual Node/Content or native Revoke changes; foreign
carrier Node/incarnation/source provenance is tested against unchanged valid DB
facts. Isolated illegal current source/decision mutations are not claimed as
legal independent authority operators. All native predicates stay enabled.

Both validation dispatchers require canonical admin configuration. The original
run records all 3 PostgreSQL17.6 (`170006`) markers and the corrected-body run
records its single 170006 marker; optional fixture early returns cannot qualify.  Native custody operations provide positive controls, a genuine quantity merge invalidates an earlier accept, and footprint comparisons verify that Trade reads/owner transitions add no durable writes.

### Remaining complete-mechanic gates

- CP single-book registration, live tick/clock and command routing; exact native
  item-handle resolution; every physical consumer's invalidation/reservation path.
- TRADE-WIRE capability/IDs/trade-id mapping and replay/rate/transport integration.
- BAGS-TRADE-1 full-tree bindings/destination traversal remain outside this leaf
  component; live physical consumers must route every relevant change.
- Real two-Character trade occurrence, ordered item/source/destination locks,
  current first/second accept fences, cross-command physical reservations, atomic
  no-merge swap/audit/receipts and unknown-outcome/restart reconciliation.
- Qualified production privileges/schema/cause/event/resource rows, migrations and
  integration prerequisites, protected exact-head CI, independent frozen-candidate
  review and CP acceptance. This packet is a bounded native read/runtime component,
  not complete player trade or permission to activate/mark ready/merge.

### CP D1160/D1122: register the actual private component (2026-10-16)

Status NOT_FULLY_QUALIFIED. CP comment6224912485 grants own-module registration without
a slot. Register domain::player_trade and the allocated native owner as one durability
child using its actual gameplay_transport/player_trade.rs path, as previously qualified
outside-tree. Its domain/owner retain the scoped dead-code convention for a component
whose physical reservations and atomic writer are not integrated. No visibility of an
existing API, actual authority check, test predicate or runtime caller is changed.
New-head Cargo/PG/full integration: NOT_RUN; prior bounded native component evidence
is retained only for its original candidate/context. Standalone-root Content/private-domain
composition remains unqualified, not an assumed PASS. D1122 allows the single coherent
existing-draft checkpoint after fmt/diff checks; cause/receipts/guards/wire slots remain CP.

### D1158: owner-local offered-item observation port (2026-10-16)

Implemented `TradeOwner::item_reserved_by_trade` against the actual bounded owner book.
It checks the native runtime carrier fence/full book binding before any empty-book answer,
reports both offered leaves in Offered/Ready/Transferring, and excludes Closed. It adds no
DB reservation table, physical custody, authority token or unused transaction-proof argument.
The caller must retain the actual sole installed book's serialization borrow through the
same MAIL TX and independently resolve native current session/assignment/item fences there.
The port's false is local observation, not global send admission; that caller remains CP.

Added a real runtime-carrier negative test (foreign binding/retired owner) and extended the
existing genuine native inventory case for offered/not-yet-offered/unrelated/backpack IDs,
cancellation release, both Ready offers and retained ambiguous Transferring membership.
Root independent source review and fmt/diff PASS; new native/PG cases **NOT_RUN**. Published
88a47e96 Rust/Clippy and S3-B are RED; concrete log request is #2131 comment6225250171.
Preserve this successor locally in a verified bundle until the concrete compiler finding
can be fixed in the same coherent push. No new runner, repeated suite, ready or merge.

## Complete native assignment binding comparison (2026-10-17; proposed successor)

Source reviewed: `57cb016f1e982ede4f51d4c731406f742aee9d61`. Its native
`ChannelRuntimeAssignmentBinding` contains NodeId and node registration revision
in addition to World/Channel, ownership generation and source revision. The prior
TradeOwner key omitted both Node fields, so the offered-item observation port could
return an empty-book `false` for a different native Node/incarnation with the same
remaining key fields. The durable offer/accept reader already compared those
fields against the independently current database assignment.

The proposed correction retains the actual native assignment binding and uses its
existing equality, together with the existing Content digest. The existing query
test now independently varies Channel, NodeId and node registration revision;
Node/incarnation controls preserve scope, source revision and Content. Valid-owner
and retired-owner controls remain. These carriers use the real native constructor
and the original activated Content pin. No replacement binding representation,
authority constructor, physical reservation, writer or caller wiring is added.

CP D1311, `#2131` comment `6232429005` (2026-10-17), grants the one-hunk
`pub(crate)` re-export of the existing `ChannelRuntimeAssignmentBinding` in
`foundation/mod.rs`. That explicit allocation covers the shared facade delta;
no constructor, getter or public API is added. The new regression and candidate
native compilation remain **NOT_RUN**; status remains NOT_FULLY_QUALIFIED.

The source head's repository checks now show 15 success and one skipped World
bundle check. Rust job `117219324696` (run `38973988492`) records 3,185 passed,
0 failed and 29 ignored; S3-B job `117218884447` (run `38973988483`) succeeded.
These results supersede the preceding historical RED note only for that source
head, not this proposed successor. PostgreSQL fixture names reported as `ok` do
not by themselves prove database execution because those fixtures can early-return.
Original atomic swap, durable TRANSFERRING recovery and live integration criteria
remain open.

## Native command-bound issuance (2026-10-17; owned source preparation)

Base: `6f9379e8ffc977d8a006c3a55976a526bc025bbc`. That published draft includes
the D1311 native assignment-binding fix. Its exact-head validation is recorded at
[#2132 comment6232831422](https://github.com/Oteryn/Oteryn-Game/pull/2132#issuecomment-6232831422):
Rust1.95 all-target Clippy with warnings denied; lib 3,185 passed, 0 failed,
29 ignored; ops 6 passed. Those results qualify that predecessor, not this new
issuance delta. Canonical PG execution remains unqualified.

The accepted PLAYER-TRADE0 brief assigns the runtime issuer to TRADE-1 and the
ID-bearing command/domain to TRADE-WIRE-1; this task owns both leaves. The earlier
local handoff's claim that a preexisting external occurrence producer was needed
is withdrawn. The producer is owned implementation work. D1315 now grants the
bounded paired-fence/root-lock split described below. Actual Channel/session-owner
installation, dispatch, capability and registry remain shared integration work.

The owned gameplay leaf now has a private occurrence-issuance component.
`accept_pending` borrows the real FND `CommandIngress` implementation and existing
native Root/node/runtime/context. The ingress type has no session ownership
field; this leaf does not prove that an arbitrary supplied book belongs to the
admitted session. At this head, the live connection retains only a raw command
counter in its Copy `SessionContinuity`, so the real shared owner installation
is still absent. A wrapper around an arbitrary book and a supplied session id
would not establish that provenance and is not added. The component record is
not yet authenticated session consent or a complete writer proof.

Each newly accepted command goes through the existing actor,
visibility, Content and DB-current two-Character/item reads. Canonical semantic
identity binds the accepted trade id, Content pin and full native CommandRef;
there is no supplied current/pending/accepted flag or substitute authority.

- The first successful accept is installed only inside native FND terminalization
  after its preflight succeeds. The owner captures that terminal consent and its
  fence; later FND terminal-record eviction cannot replace the captured accept.
- A matching second pending command issues one occurrence and retains the
  existing destination intent. It stays pending until the future durable outcome.
  It must be the first command of the actual FND pending suffix; a later pending
  command cannot issue a swap ahead of an earlier one.
- Retrying that pending trigger observes the same issued record. An ambiguous
  outcome and the idle timeout retain it. A terminal enum or an old DATA accept
  cannot create this component's acceptance; the old DATA adapter cannot modify
  its lifecycle after native acceptance starts. Historical terminal replay returns
  a distinct `ReplayedAccepted` outcome. It can outlive a closed book and does not
  revalidate actor/DB generation or recreate a current acceptance.
- Trade ids and occurrences use distinct native command-derived identity tags
  and the existing full 16-byte identity convention. D85 actor ids and item
  handles keep their existing types. No wire opcode, capability or registry is
  assigned by this leaf.

The shared preparation step preserves the previous DATA accept behavior while
allowing FND preflight to fail without installing acceptance. It clones only the
expected next session state, not a runtime owner or an authority implementation.
The occurrence has a private constructor path and immutable accessors; it is not
a committed-swap receipt. The real writer must still consume recovery/node/current
fences and recheck both sources and destinations in its atomic transaction.

New qualification cases use actual FND reserve/terminalize operations around the
existing real native MINT/TRANSFER fixture history and the native TradeOwner read
path. They cover component issuance, independent missing/changed pending data and
CommandRef-session mismatch (not ingress-owner provenance), first/second command
order, first-accept retention, duplicate trigger,
DATA/terminal-enum promotion refusal, and an offered item changed by a real native
TRANSFER. PG cases still require canonical PostgreSQL17.6 and explicit execution
evidence; an early-return test reported as `ok` is insufficient.

CP D1315, [comment6232733238](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232733238),
reaffirmed by [comment6232878773](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232878773),
grants the narrow shared `durability/item_transfer.rs` split. The original
`character_item_fence_is_current` wrapper now calls
`character_item_fence_without_root_lock`, then `lock_character_item_root`.
All original SQL predicates, strict current-generation equality, account/World
comparison and single-Character caller order are preserved. The future TRADE
writer can complete both native fence checks before either root, then acquire
the roots in CharacterId order. The intermediate account string is expected DATA,
not new authority; both phases must run in the same transaction.

A native PG case observes root availability from independent transactions with
`FOR UPDATE NOWAIT`: both fence phases leave both roots unlocked; the ordered
root phase locks them; the legacy wrapper locks its original one root. Independent
stale-generation and account-mismatch cases refuse, and the durable footprint and
CharacterRevision remain unchanged. This qualifies the helper component only;
the atomic writer and migration0100 remain open. That migration must preserve all
latest0095 HOUSE guard arms as well as earlier proofs.

Local Rust1.94 repo-config rustfmt and source/diff checks are recorded in the
candidate audit. New Clippy, unit and native PG execution: **NOT_RUN**. Status is
**NOT_FULLY_QUALIFIED**. This adds no migration, durable cause/receipt/event, atomic
writer, physical cross-command reservation installation, wire dispatch or shared
registry. Those remaining parts of TRADE-1/TRADE-WIRE are still open.

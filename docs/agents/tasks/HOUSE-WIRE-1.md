# HOUSE-WIRE-1 — House codecs, query request and native request adapters (consolidated task card)

```yaml
task_id: HOUSE-WIRE-1
title: "HOUSE-WIRE-1: HOUSE_V1 codecs, HOUSE_INTENT/HOUSE_QUERY and native request adapters"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "impl (protocol review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row HOUSE-WIRE-1 (rows 119, 121); App. 7 HOUSE-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md     # wire section, panel
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md   # guild ACL extension
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md                     # §0.1
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # PROTOCOL_OTERYN_V1_REGISTRY, RESOURCE_LIMITS_REGISTRY
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. No HOUSE_V1 management codec or dispatch handlers from this card are present. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: House commands/query, registry/capability and native ownership/ACL adapters remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2145, branch skeleton/house-wire-1

## Goal

Capability `HOUSE_V1`, `HOUSE_INTENT` (bid, move-out notice, cancel, character ACL replacement,
door-list replacement, kick, leave) and `HOUSE_QUERY` (houses by town), plus the transport
adapters that hand prepared requests to the owning native writers. Older clients without
`HOUSE_V1` receive nothing new and the commands are rejected without the capability. The
client House Management panel is coordinated with the client lane (#1942) and is not allocated
to this task.

## Dependencies (re-verified on origin/main, 2026-10-22)

| Dependency | State |
|---|---|
| HOUSE-RUNTIME-1a | MERGED (#2163) |
| HOUSE-1b (`DurabilityRoot::apply_house_move_out_command`, D1384) | BLOCKING — re-implementation pending (draft #2141 closing) |
| HOUSE-ACL-1 (`DurabilityRoot::replace_house_acl`) | BLOCKING — re-implementation pending (draft #2140 closing) |
| HOUSE-1a (bid writer, ownership/funds) | BLOCKING for the bid mapping — re-implementation pending (draft #2137 closing) |
| GUILDHALL-1 / GUILD-WIRE-1 (guild ACL tags) | BLOCKING for guild ACL entries — re-implementation pending (drafts #2153, #2159 closing) |
| HOUSE-RUNTIME-1b (House exit admission for kick/leave) | BLOCKING for kick/leave — re-implementation pending (draft #2165 closing) |
| Client panel lane | #1942 (coordinate only) |

## Owned paths (ADR-0022 layout)

- `crates/protocol-oteryn/src/house.rs` (new; codecs stay in the protocol crate)
- `crates/protocol-oteryn/src/house_tests.rs` (new)
- `crates/gameplay/dispatch/src/gameplay_transport/house_intent.rs` (new, L5 preparation + native ACL/move-out adapters)
- `crates/gameplay/dispatch/src/gameplay_transport/house_intent_tests.rs` (new)
- `crates/gameplay/dispatch/src/gameplay_transport/house_query.rs` (new, inactive request preparation; one mechanic per file)
- `crates/gameplay/dispatch/src/gameplay_transport/house_query_tests.rs` (new)
- `docs/contracts/protocol-oteryn/v1/house_v1.proto` (new)
- `docs/agents/tasks/HOUSE-WIRE-1.md`

Slot-only (final commit at the queue turn): `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`
(capability, two command types, state domain); `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
(flat body bounds); `crates/protocol-oteryn/src/lib.rs` (`pub mod house;`);
`crates/gameplay/dispatch/src/gameplay_transport/mod.rs`, `connection.rs` (dispatch arm),
`capabilities.rs` (+ `capabilities_tests.rs`). Inseparable bundle: wire + registry + capability
+ handler + tests in one slot commit. No migration.

## Wiring points (verified on origin/main)

- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain (follow the NPC_TRADE dispatch `UseCommand` binding pattern)
- `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` `GATED` table and offered sets
- `crates/gameplay/dispatch/src/gameplay_transport/world_map.rs` `tile_pos` (checked u16 x/y, i8 floor −15..0)
- `crates/gameplay/domain/src/domain/map_position.rs` `TilePos`
- `crates/protocol-oteryn/src/world_spatial.rs` (signed-coordinate codec) and the strict protobuf wire helpers in `crates/protocol-oteryn/src/`
- `crates/gameplay/housing/src/house_scope/handoff.rs` (exit currently `ExitNotAdmitted`)
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (current tail: capabilities up to 30, commands up to 30, domains up to 22)

## Accepted decisions carried from the draft

- **D1170 (provisional numbers):** `HOUSE_V1` capability 27, `HOUSE_INTENT` command 27,
  `HOUSE_QUERY` command 28, state domain 23 as leaf constants. On main today capability 27,
  commands 27/28 and domain 23 are unregistered (capability 28 is ITEM_LOOK_V1). Re-verify every
  number against the registry at the slot; CP leases them at the queue turn.
- **Target binding (D1170):** every complete intent carries one explicit WorldId plus native
  House key (field 8) alongside exactly one action, including ACL, door, kick, move-out, cancel and
  leave; the old Bid-local key is removed. Transport refuses a body World different from the
  actor World before preparing, validates keys with native `HouseId`, and retains the exact actor
  and CommandRef. Prepared values are request data, never current authority.
- **Intent bound:** 4168 bytes = largest native 200-member Door action 3626 + World/key target 536
  + 6 spare. Whole-body bound checked before nested allocation; each list count checked before
  inserting. Unknown/duplicate/missing fields, empty/multiple oneofs, unknown roles, malformed
  varints/lengths/UTF-8/UUID and duplicate list identities refuse. Empty lists clear; revision 0
  and bid max 0 are syntactically valid; list order preserved. Price/funds/eligibility belong to
  the owning writer.
- **Door coordinates:** match the native tile domain — x/y 0..65535, floor −15..0 on encode and
  decode; no sign flip or remap (corrective finding: the first draft used the broader
  ActorPosition shape).
- **D1261 HOUSE_QUERY request:** field tags 1 (WorldId) / 2 (catalogue town) / 3 (optional cursor);
  caps town 512 B, cursor 512 B, body 1056 B (canonical maximum 1048 B). The town is the city Area
  key `oteryn:content.area.city.<slug>` (native `FirstProductionArea.key: ProductionKey`), not a new
  TownId. A present empty cursor means first page, normalized to `None`; duplicate cursor tags
  refuse. A cursor grants no read access or DB currentness.
- **D1350/D1354 ACL adapter:** maps character subowner, guest and door ACL requests to the real
  `DurabilityRoot::replace_house_acl`, taking the owner's retained `ReconciledCharacterAuthority`,
  `NodeIncarnationProof` and complete `CurrentCharacterItemFence`; checks actor, command session
  and World against the fence; preserves House target, door conversion, subjects and expected
  revision; returns the owner's receipt or error unchanged. It creates no transaction, success ACK
  or authority, and never turns an Instance into a Channel.
- **D1384 move-out adapter:** forwards the exact absolute Unix-ms date as
  `HouseMoveOutCommand::Notice { due_at_unix_ms }` or fieldless `Cancel` to
  `DurabilityRoot::apply_house_move_out_command` with real retained authority, node and HouseId;
  returns the native receipt/error unchanged. It mints no operation ID, supplies no ownership
  revision and samples no clock — clock/date validation, revision resolution and replay are
  HOUSE-1b's. Held until HOUSE-1b merges.
- D1354 module order: register `house_intent` after the NPC-ECON/ADMIT-CAP mounts already on
  main; compile and dispatch follow HOUSE-ACL-1.

## Reference behaviour and qualification (draft result)

- Protocol codec tests: 21 PASS (including 10 owned codec tests and 3 door-coordinate regressions
  that first failed against the unfixed codec).
- Adapter conversion tests: 14 PASS (9 preparation + 5 ACL conversion: both character list kinds
  and revision bounds, native door boundaries and empty-list clearing, each mismatched binding,
  non-ACL action refusal, unchanged Instance scope); the proposal raises this to 19 with 5 move-out
  cases (not run). Plus HOUSE_QUERY preparation cases (authored, not run).
- These are conversion tests; they do not qualify a dispatched end-to-end House request.

## Findings and open questions (activation gates)

1. Gate the capability in `connection.rs` **before** interpreting either command; bind the admitted
   actor/session/CommandId; resolve the actor's house/World and current native session/lease/scope
   fence for both Channel and House sessions. Body data never supplies authority. The panel must
   work away from the house.
2. Bid/ACL/move-out call their owning writers under current Character/session and property/ACL/
   ledger locks with exact expected revision and CommandRef binding.
3. HOUSE_QUERY response/projection still needs review: houses by town, public price/end/rent,
   actor-only bid maximum and own house, owner/subowner ACL. Never project other bidders' maxima.
   Needs a native privacy/currentness read port and a resource row.
4. GUILD0 guild matching and character exclusions are not in this character-only body subset;
   resolve the wire shape with the HOUSE-ACL/GUILD wire owners. OWN0 omits a house selector from
   ACL/door intents — the explicit target above is the proposed answer and needs protocol review.
5. Kick/leave need target presence, owner/subowner role and fenced house-exit admission;
   `house_scope_handoff` refuses exit with `ExitNotAdmitted` today. No side-effect ACK precedes the
   durable result; an unknown outcome holds the command, reconciles the receipt and replays the
   same CommandRef without a second effect.
6. Capability-off clients get no new state; unselected intents/queries reject. Panel, projection,
   Premium/funds and the protocol/generation/reconnect/replay matrix need the registered integration.

## Acceptance criteria

- [ ] Older clients without `HOUSE_V1` receive nothing new; commands rejected without the capability.
- [ ] Flat message bounds registered in RESOURCE_LIMITS.
- [ ] Handlers call only the HOUSE-1a/1b/ACL writers; adapters return native receipts/errors unchanged.
- [ ] Body World/House target bound and checked against the actor before preparation.
- [ ] Door coordinates limited to the native tile domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] One mechanic per module file in its ADR-0022 layer.
- [ ] New tests in their own child files.
- [ ] Edits to mod.rs/lib.rs/connection.rs/capabilities.rs are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-protocol-oteryn house
cargo test -p oteryn-gameplay-dispatch house_intent
cargo test -p oteryn-gameplay-dispatch house_query
cargo test -p oteryn-gameplay-dispatch capabilities
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md
- docs/agents/tasks/archive/WORLDINT-WIRE-1.md (wire pattern)
- docs/reference/tibia-manual/houses.md
- Draft #2145 (CP decisions D1170, D1261, D1345, D1350, D1354, D1384 on PR #2131)

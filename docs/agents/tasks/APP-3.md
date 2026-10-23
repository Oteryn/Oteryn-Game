# APP-3 — appearance change command, actor appearance state and wire codec (consolidated task card)

```yaml
task_id: APP-3
title: "APP-3: appearance change command, actor appearance state and wire codec"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-3 (rows 126, 127); §7 supporting briefs row APP-3"
brief: "docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md (§4.2, §4.5, §5, §8)"
class: "hard, protocol, registry"
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # capability, command, state domain allocated by the protocol registry lease
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. No permanent appearance-change codec/dispatch from this card is on main; temporary spell appearance is a different path. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Implement APP-3 permanent selection command and publication over the existing APP-1 owner. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2175, branch skeleton/app-3

## Goal

Appearance change command and actor appearance state on the wire: other players see a
committed look; a rejected change publishes nothing; older clients are gated by
capability. The draft delivered the protocol codec, a connection-local reference
reducer and an unwired server leaf; shared integration (slot A2) was never done.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| CHARACTER_APPEARANCE acceptance | ACCEPTED (#2209); allocates no wire layout or registry ids |
| APP-1 durable appearance writer, migration 0093, recovery | MERGED partial (#2170); `read_current_character_appearance` and the native writer are the consumed APIs |
| APP-2 canonical cosmetics authoring, pinned native ABI | MERGED partial (#2173); supplies `NativeCosmeticsView` |
| ATTACK-1b / DEATH-2 (combat protocol lane) | MERGED (#1798; #1742, #1747) |
| VIS-2 world spatial entities (capability 6) | on main: `crates/protocol-oteryn/src/world_spatial_entities.rs` |
| Protocol registry lease (capability/command/domain ids) | CP lease at allocation; re-check that the draft ids are still free |
| APP-4 (open draft #2176) | sibling; not a consolidation draft — coordinate shared wiring |

## Accepted decisions

- **CP D1094:** revoked card-only and authorised authoring; the layout below was
  proposed for CP protocol review on the frozen head. Capability unoffered until A2.
- **D61:** premium metadata adds no V1 gate; unlocked premium-flagged cosmetics succeed
  whether Premium is current or not.
- Draft wire allocation (proposal; registry ids still unregistered on main at
  59785d76, re-check at the lease): capability **25 APPEARANCE_V1** requiring
  **6 WORLD_SPATIAL_ENTITIES**; command **25 APPEARANCE_CHANGE**; state domain
  **21 APPEARANCE**. Snapshot type 1 = private own catalogue + public appearance
  dictionary; delta type 1 = mapping changes/retirements; delta type 2 = replace the
  private own catalogue. Item-transform Look also requires capability **4
  ITEM_VIEW_MOVE_V1** and that generation's Item dictionary; without it the existing
  unavailable/resync/end-session policy applies, never a guessed old outfit.
- No session, account, sex, authority, unlock provenance or grant receipt is accepted
  from client payloads.

## Wire schema (strict proto3 via existing `charm_wire`)

Every field required (explicit zero scalars too) unless optional/repeated. Unknown,
duplicate singular, wrong wire type, missing required, invalid UTF-8/control chars,
unknown enum, invalid size/count/order and integer overflow refuse on decode and encode.

| Message | Fields |
|---|---|
| Context | 1 client artifact generation bytes32 (= VIS-2 content_generation); 2 cosmetics document digest bytes32 (palette + definitions) |
| Selection | 1 outfit key; 2 colours exactly 8 bytes (head/body/legs/feet u16 BE); 3 addons u8; 4 optional mount key (storage only) |
| Selected | 1 Selection; 2 nonzero committed selection revision u64; 3 content provenance |
| Change | Context 1–2; 3 nonzero expected Character root revision; 4 optional exact prior Selected; 5 new Selection; 6 expected content revision |
| ChangeResult | 1 enum Committed/Rejected; 2 nonzero committed revision iff Committed. Unresolved has no terminal payload |
| OutfitOption | 1 key; 2 nonzero look_type for authoritative sex; 3 permitted addon bits |
| Catalogue | Context; 3 own root revision; 4 optional stored Selected (kept even if incompatible); 5 RGB palette exact triples; 6 repeated OutfitOption; 7 repeated permitted mount key |
| EntityRef | 1 identity bytes16; 2 nonzero actor generation |
| Look | 1 enum Outfit/Item; Outfit: 2 look_type, 3 colours, 4 addons; Item: 5 Item definition_ref. Kind-inconsistent fields refuse |
| Binding | Context; 3 appearance_ref u32; 4 EntityRef; 5 root revision; 6 condition-owner player revision; 7 owner time µs; 8 Look; 9 optional durable selection revision (None = implicit fallback) |
| Mappings | Context; 3 repeated Binding; 4 repeated EntityRef forgets, disjoint from upserts |
| Snapshot | 1 Catalogue; 2 repeated Binding; 3 u32 allocation high-water incl. retired refs |

Lists strictly ascending/unique; Binding Context must equal the message Context; a
selection revision cannot exceed the root revision. Shape validation never
authorises a key, palette index, addon, entitlement or sex.

Bounds: key 512 B; provenance 128 B; Selection/Change/Result 1100/4096/16 B; palette
≤256 colours (actual from content); ≤256 outfits and ≤256 mounts; catalogue 196608 B;
≤256 bindings per batch (VIS-2 ceiling); Binding/Mappings/Snapshot 256/131072/262144 B;
refs 1..u32::MAX monotonic per connection, no wrap/reuse, exhaustion → reconnect-required.

## Reference behaviour from the draft (re-implement, do not copy)

- **Mapping lifecycle.** `SessionAppearance` belongs to one observer connection and is
  not in resume continuity; every reconnect/context replacement starts a new dictionary
  and snapshot. Retirement is by exact EntityRef and never resets the high-water.
- **Publication order.** For each visible change stage both fully encoded frames before
  touching allocation/cursors: domain 21 mapping delta first, then VIS-2 UPDATE (domain 1)
  with consecutive sequence numbers, keeping the current position (no forged position
  revision). The actor must exist as Player in the visibility snapshot; hidden actors are
  never introduced by a mapping. Join/resync queues the full domain-21 snapshot before
  the VIS-2 snapshot that references it; a leave is queued before retiring its mapping.
  A failed/ambiguous write terminates the connection; refs/revisions never roll back.
- **ReferenceStore** is a strict connection-local presentation reducer, not a visibility
  authority: VIS-2 owns enter/leave and generation currentness. Unknown/stale mapping →
  authoritative resync/end-session, never ref 0/look 128/nearby/sex guesses. Duplicates,
  stale bases and gaps refuse atomically. A command result settles its command; it does
  not repaint. Result-first, observation-first and superseding observations converge.
- **Command.** Retains one native `AppearanceChangeOccurrence` with its original fence,
  request and sex; CommandRef is session+CommandId; no UUID from client bytes. Retry
  reuses the retained intent, never re-prepares at a newer root. Preparation uses
  `FreshAdmissionStore::current_session_at`, current Character root and runtime
  player/lease/scope. `PreparedChange::commit/reconcile` call the APP-1 APIs; historical
  AlreadyCommitted remains obtainable after session loss. Known pre-commit rejection
  emits no mapping; storage/COMMIT unavailable stays **Unresolved** with no publication.
- **Current read.** APP-1 `read_current_character_appearance(authority,node,fence)`, with
  the fence derived from current DB session/root reads, never a receipt. Target selection
  re-read after observer preparation; a replacement or root advance suppresses late
  publication. Runtime owner fence, player control/lease/scope, pinned artifact and
  condition state checked before publication.
- **Content.** `NativeGameplayState::cosmetics()` → APP-2 `NativeCosmeticsView`, checked
  against the runtime pin. AccountUnlocks is pure input. Real resolver, creation sex and
  activated authoring facts remain required owners.
- **Effective look** = latest stored base (or validated fallback) composed with
  ChannelSpellStates/ConditionStore at owner time; temporary Outfit/Item states keep
  native clear/expiry rules; no condition mutated by a selection change; no mount
  projected; incompatible stored selection preserved.
- Draft status: protocol codec 14 tests passed with a private registration; seven
  server mechanism tests authored but never run; leaf unwired (`dead_code` expectation).

## Required negative qualification matrix (retained)

| Boundary | Evidence required for the integrated candidate |
|---|---|
| Decoder / bounds | roundtrip; missing/duplicate/malformed/unknown; truncated/oversized key/provenance; palette max/max+1; addon range; unknown capability/type no effect |
| Admission / binding | Character/account binding, World vs Channel, actor generation, sex, catalogue revision, artifact/map/activation; reject before write; unknown sex/catalogue fails closed |
| Authorisation | starter without grant succeeds; locked outfit/addon/mount, incompatible provenance, unavailable Store reject; premium-flagged unlocked succeeds (D61) |
| Durable write | stale session/connection/lease/scope/node/root/prior; bad deadline; DB, revision, receipt and display unchanged on known rejection |
| Commit / replay | exact retry one disposition; rebinding refuses; competing roots typed loser; ambiguity survives restart |
| Publication / owner | visible to own + other observers; owner retirement/generation/pin/condition change after commit → no stale mutation, next owner reloads |
| Receipt substitution | substituted compatibility/binding/revision rejected at writer/reconciler |
| Observation order | result-/observation-first, duplicates, lost ack, unresolved converge; stationary change emits UPDATE; leave/reappear purges; hidden never revealed |
| Client downgrade | wrong mapping/content/palette/session, stale/unknown/missing ref, unsupported capability → native refresh policy |
| Temporary/fallback | base change under Outfit/Item transforms; clear/expiry restore latest base; client timer cannot clear |

## Target layout (ADR-0022)

- Protocol codec: `crates/protocol-oteryn/src/appearance.rs` (new) with
  `crates/protocol-oteryn/src/appearance_tests.rs` (new); `pub mod` + constants in
  `crates/protocol-oteryn/src/lib.rs` (wiring-only).
- Command leaf + publication: `crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs`
  (new) with `appearance_change_tests.rs` (new); reducer may be its own child module.
  Existing neighbour: `crates/gameplay/dispatch/src/gameplay_transport/appearance_atomic.rs`.
- Durable consumers (exist): `crates/gameplay/durability/src/durability/character_appearance.rs`,
  `crates/gameplay/domain/src/domain/appearance.rs`; cosmetics view
  `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader.rs`.
- Wiring (A2, wiring-only): `gameplay_transport/connection.rs` dispatch,
  `gameplay_transport/capabilities.rs` GATED/offer, `gameplay_transport/mod.rs`,
  CharacterRevisionSequencer appearance cursor arm, visibility snapshot/resync/leave
  ordering, registry `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, client consumer.

## Acceptance criteria

- [ ] Other players see a committed look.
- [ ] Rejected change publishes nothing; Unresolved publishes nothing and keeps the occurrence.
- [ ] Older clients gated by capability (25-equivalent requires 6; Item look requires 4).
- [ ] Every concurrent transition serialized and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Negative qualification matrix above covered by native evidence (not mechanism tests only).
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`connection.rs` ~8.3k lines takes dispatch only).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-protocol-oteryn appearance
cargo test -p oteryn-gameplay-dispatch appearance_change
cargo test -p oteryn-game-server capabilities
python3 tools/agents/validate_governance.py
```

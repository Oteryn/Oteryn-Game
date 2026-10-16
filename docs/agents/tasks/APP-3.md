# APP-3 — appearance change command and actor appearance state (implementation candidate)

```yaml
task_id: APP-3
title: "APP-3: appearance change command and actor appearance state"
mode: IMPLEMENT
status: partial
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-3 (rows 126, 127); §7 supporting briefs row APP-3"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-3
needs_migration: false
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Change command and actor appearance state on the wire; other players see a committed look; a rejected change publishes nothing.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-3 (hard, protocol, Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| APP-1 | BLOCKING (Wave 4) |
| combat child E protocol lane (DEATH-2/ATTACK) | ATTACK-1b MERGED; DEATH-2: verify |
| registry lease | CP lease |
| acceptance of CHARACTER_APPEARANCE | ACCEPTED on protected main by #2209 (`e2da9805d296a27b06b32335bf67394509d4910b`); no wire layout or registry IDs allocated by that acceptance |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/appearance_change.rs`
- `apps/game-server/src/gameplay_transport/appearance_change_tests.rs`
- `crates/protocol-oteryn/src/appearance.rs`
- `crates/protocol-oteryn/src/appearance_tests.rs`
- `docs/agents/tasks/APP-3.md` (this record: status and notes only)

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

- apps/game-server/src/gameplay_transport/appearance_atomic.rs (existing)
- world_spatial_entities actor state (crates/protocol-oteryn/src/world_spatial_entities.rs)
- connection.rs dispatch; capabilities.rs `GATED`

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Other players see a committed look.
- [ ] Rejected change publishes nothing.
- [ ] Older clients gated by capability.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §4.5, §5, §8
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Current APP-3 implementation proposal — CP D1094

CP [6219643346](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346),
updated 2026-10-16 16:11:36 UTC, revokes CARD_ONLY and authorizes authoring now. The
layout below is proposed for CP protocol review on this PR's frozen head; a separate
prior layout decision is not required. The previous card-only packet is superseded.
One author edits only the four Rust owned paths above and this record. Shared
connection/module/capability/client/registry changes remain slot A2; the capability
is **unoffered** until that integration. No new PR, ready, queue or merge authority.

### Wire allocation and concrete schema

Leaf constants: capability **25 APPEARANCE_V1**, requires **6 WORLD_SPATIAL_ENTITIES**;
command **25 APPEARANCE_CHANGE**; state domain **21 APPEARANCE**. Snapshot type **1**
contains the private own catalogue and public appearance dictionary. Delta type **1**
changes mappings/retirements; delta type **2** replaces the private own catalogue.
Item-transform Look additionally requires selected capability **4 ITEM_VIEW_MOVE_V1**
and that generation's actual Item dictionary. No Item ref is sent/resolved without
it: the caller must use the existing unavailable/resync/end-session policy, never
show a guessed old outfit. Registry/offer/GATED/lib registration is not authored here.

All messages use the existing strict proto3 varint/bytes parser in `charm_wire`.
Every field below is required, including explicit zero scalar values, unless marked
optional/repeated. Unknown, duplicate singular, wrong wire type, missing required,
invalid UTF-8/control characters, unknown enum, invalid size/count/order and integer
overflow are refused on decode and encode. No native session, account, sex, authority,
unlock provenance or grant receipt is accepted from client payloads.

| Message | Fields (number:type; repeated messages retain wire order) |
|---|---|
| Context, embedded in Change/Catalogue/Binding/Mappings | 1:client artifact generation bytes32, exactly VIS-2 content_generation; 2:cosmetics document digest bytes32 (includes palette and definitions) |
| Selection | 1:outfit key UTF-8; 2:exactly8 bytes head/body/legs/feet u16 big-endian; 3:addons u8 varint; 4:optional mount key UTF-8 (storage only, no activation) |
| Selected | 1:Selection message; 2:nonzero committed selection revision u64; 3:content provenance UTF-8 |
| Change | Context1–2; 3:nonzero expected Character root revision u64; 4:optional exact prior Selected; 5:new Selection; 6:expected content revision UTF-8 |
| ChangeResult | 1:enum1 Committed /2 Rejected; 2:nonzero committed Character revision iff Committed. CommandId/generation remain the existing CommandResult envelope; Unresolved has no terminal payload |
| OutfitOption | 1:key; 2:nonzero look_type u32 resolved for authoritative sex; 3:currently permitted addon bits u8 |
| Catalogue | Context1–2; 3:nonzero current own root revision; 4:optional own stored Selected (kept even if incompatible); 5:nonempty RGB palette bytes, exact triples; 6:repeated OutfitOption; 7:repeated permitted mount key |
| EntityRef | 1:identity bytes16; 2:nonzero actor generation u64 |
| Look | 1:enum1 Outfit /2 Item. Outfit requires2:nonzero look_type u32,3:colours bytes8,4:addons u8; Item requires5:nonzero existing Item definition_ref u32. Kind-inconsistent fields refuse |
| Binding | Context1–2; 3:nonzero appearance_ref u32; 4:EntityRef; 5:nonzero current Character root revision; 6:condition-owner player revision u64; 7:owner time microseconds u64; 8:Look; 9:optional durable selection revision. None means implicit fallback, not a fabricated persisted revision |
| Mappings | Context1–2; 3:repeated Binding; 4:repeated EntityRef explicit forget, disjoint from upserts |
| Snapshot | 1:Catalogue; 2:repeated Binding; 3:u32 allocation high-water including retired refs, zero for a fresh empty dictionary |

Outfit/mount lists are strictly ascending unique keys. Binding batches are strictly
ascending references, with unique full EntityRefs and unique references. Their
Context must equal the message/catalogue Context. A binding's optional selection
revision cannot exceed its current root revision. Shape validation never authorizes
a key, palette index, addon, entitlement, sex or current artifact admission.

### Bounds proposed for this head

| Bound | Value and source/reason |
|---|---|
| key bytes | 512, existing native production-key bound |
| provenance bytes | 128, accepted CHARACTER_APPEARANCE §4.2 |
| Selection / Change / result bytes | 1100 /4096 /16; Change fits both prior/new maximum outfit+mount keys and provenance (over2048), not just a typical request |
| palette colours | 256 allocation ceiling; actual palette size/colours come from content (Reference133 is not an engine palette) |
| catalogue outfits /mounts | 256 each, plus encoded byte bound; over-bound generations cannot offer this capability |
| catalogue bytes | 196608, below existing state-delta envelope262144 |
| active bindings /total batch upserts+forgets | 256, the existing VIS-2 visibility ceiling |
| Binding /Mappings /Snapshot bytes | 256 /131072 /262144; catalogue+256 bounded bindings fit snapshot; all below existing delta/chunk ceilings |
| refs | 1..u32::MAX monotonically allocated per connection; no wrap, no reuse after leave; exhausted allocation returns reconnect-required |

### Mapping lifecycle and publication

`SessionAppearance` belongs to one observer GameSession/connection and is absent from
resume continuity. Every reconnect (including resume), context replacement or new
connection starts a new dictionary and snapshot; no old ref is interpreted there.
Live-context catalogue replacement cannot change the palette/document binding.
Retirement is exact EntityRef and never resets allocation high-water. Snapshot carries
that high-water even if the most recently allocated actor has left.

For each visible appearance change the leaf stages **both** fully encoded native
frames before changing allocation/cursors: domain21 mapping delta first, domain1
VIS-2 UPDATE second, with consecutive existing server sequence numbers. The UPDATE
keeps the actual current visible position; it does not forge a position revision or
require movement. The exact actor must exist as Player in the visibility snapshot;
hidden actors cannot be introduced by a mapping. Initial join/resync must queue the
complete domain21 snapshot before a VIS-2 snapshot that first references its refs.
A leave is queued before retiring its mapping. A failed/ambiguous write terminates
the connection: allocated refs/revisions are never rolled back or reused.

`ReferenceStore` is a strict connection-local presentation reducer, not a visibility
or actor authority. Native Session VIS-2 owns enter/leave, full EntityRef generation
currentness and domain revisions. Mapping arrival never admits an actor. Session
renders only its exact current visible EntityRef/reference after resolving the
matching context/palette. An unknown/missing/stale mapping requires the existing
authoritative resync/end-session route, never ref0/look128/nearby-look/sex guesses.
The reducer consumes the actual domain21 envelope revision: duplicates, stale bases and gaps refuse atomically. Account unlock changes may replace the catalogue without advancing this Character root revision. A command result settles its command; it does not repaint the scene. Result-first,
observation-first and a newer superseding observation preserve the current mapping.

### Real owners and implemented leaf boundaries

The command retains one actual native `AppearanceChangeOccurrence` and its original
native fence/request/sex after first preparation; CommandRef stays the existing
session+CommandId identity. No UUID is manufactured from client bytes/CommandId.
Retry/reconcile consumes that same retained intent; it never re-prepares at a newer
root revision or submits a second occurrence after delay/lost acknowledgement.
Preparation uses the real `FreshAdmissionStore::current_session_at`, current Character
root and runtime player/lease/scope reads. The APP-1 native writer independently
checks recovery, admission, node, session/connection/lease/scope, root/prior state,
content and semantic transaction fencing. Wire `before` is an expectation only.

`PreparedChange::commit/reconcile` call the real APP-1 APIs. An exact historical
AlreadyCommitted remains obtainable after current-session loss; current observation
reads are never inserted before that replay path. Known precommit rejection emits
no mapping. Storage/COMMIT unavailable remains **Unresolved**, no terminal Rejected
payload and no appearance publication; the original occurrence is retained. A
receipt/result is historical disposition, never actor-installation authority.

The actual APP-1 `read_current_character_appearance(authority,node,fence)` is consumed
for current catalogue/observation: its expected fence is independently derived from
current native DB session/root reads, never a receipt. Target selection is re-read
at this native boundary **after** observer preparation awaits; a replacement or root
advance suppresses late publication. These separate transactions are not claimed
jointly atomic. The observer and observed player's connections are distinct.
Runtime owner_fence, exact player control/lease/scope, pinned native artifact and
current condition state are independently checked before staged publication.

`NativeGameplayState::cosmetics()` supplies APP-2's concrete outer-digest-bound
`NativeCosmeticsView`; the leaf checks its source/document identities against the
actual server runtime pin; public Context.content is the separate client artifact generation already used by VIS-2. Current native target/observer positions are compared with the supplied visibility view to refuse stale spatial coordinates. The owned Arc adapter only satisfies the existing native pure
AppearanceContent lifetime, not authority. Catalogue iteration uses actual canonical
keys and the native validator; premium metadata adds no V1 gate (D61). The existing
AccountUnlocks view is pure input, not an admitted account/compatibility proof.
Its real resolver, Character creation sex and activated authoring facts remain
required owners; no client sex, empty guessed catalogue or fabricated unlock is used.

Effective look composes the latest stored base or validated content fallback with
actual ChannelSpellStates/ConditionStore at the owner time. Temporary Outfit/Item
state and explicit clear/expiry retain native conflict/clock rules; no condition is
mutated by a selected-look change, and no mount is projected from selected storage.
An incompatible stored selection is preserved. Item transforms use the actual
native Item key/revision dictionary under capability4, not an appearance_ref cast.

### Qualification and remaining integration

The four owned source files contain codecs, the strict reducer, native writer/read
consumers and publication mechanisms. Codec tests exercise strict shape/bounds and
malformed bytes, maximal native keys, implicit fallback, item downgrade, connection/
EntityRef reset, ordering, tombstones and result/observation separation. Server tests
exercise the actual envelope codecs for mapping-before-stationary-UPDATE, hidden/
context/refusal atomicity, capability gates, exhaustion, leave/reentry/reset,
temporary clear/latest base and unresolved-versus-known-rejection settlement.
These are mechanism tests, not fabricated native authority qualification.

Author validation of the exact protocol source with a private `pub mod appearance;`
registration in the actual protocol crate: **14 passed, 0 failed, 0 ignored**;
strict all-target protocol Clippy passed. The seven server mechanism tests are
authored but not yet run. The shared registration remains absent from this branch.

Fresh native qualification must privately mount the exact APP-1 current-reader
source and APP-2 concrete view/ABI, and only the allowed shared registration lines.
Those dependencies are frozen at APP-1 `29efda15ae08c37a718fcef1eb1d3b34588b7a14`
and APP-2 `cde2fa3ef03ca0e57a7dd852893c1be8e0ef4b90`; no native server test/build
result is claimed before that run. Required additional native evidence
includes independently persisted target replacement during observer preparation,
current actor/owner loss after commit, real condition transitions and actual
admission/caller path. The A2 author must wire the CharacterRevisionSequencer
appearance cursor arm, occurrence/pending settlement, current sex/account-compatible
facts, capability/registry/lib/module/connection routing, visibility/initial snapshot/
resync/leave ordering and actual client consumer. Those shared paths are not edited
here, and all acceptance checkboxes remain unchecked. No full relog/E2E/Windows/
GPU proof, offered capability, completed feature or merge readiness is claimed.

### Full negative qualification matrix (retained)

The matrix remains mandatory for the final integrated candidate. Existing own
mechanism tests cover only their named codec/reducer/publication boundaries;
independent native authority/currentness and actual callers require separate evidence.

| Boundary / invariant | Required evidence before integrated qualification |
|---|---|
| Decoder / identity and bounds | Valid selection roundtrip; missing/duplicate/malformed/unknown variant, truncated/oversized key/provenance, palette boundary/max+1, addon mask range; unknown capability/type has no effect. Values follow actual leased/native bounds, not new constants here. |
| Current admission / binding | Change only Character/account binding, World versus Channel, exact actor generation, sex, request catalogue revision, current artifact/map/frame or current activation; reject before write/publication. Unknown sex/catalogue/source fails closed. |
| Native selection / authorization | Starter without grant succeeds; locked outfit/addon/mount, incompatible provenance and unavailable Store reject. Unlocked premium-flagged cosmetics succeed under both current/not-current Premium (D61); do not add a gate. |
| Durable write / current liveness | Missing/stale/mismatched current session, connection, lease, scope, node incarnation or root/prior state; expired/future/non-monotonic semantic deadline. Verify real DB state, revision, receipt and runtime display remain unchanged on known rejection. |
| Commit / replay and concurrency | Exact retry returns one historical disposition; occurrence rebound to different selection/provenance/Character refuses. Competing root revisions have typed loser; one commit/revision advance only. Actual commit ambiguity/reload survives restart. |
| Publication / current owner | Valid commit visible to own+other current observers; inject owner retirement, generation/connection replacement, stale pin/activation or condition change after commit before publication. No stale actor mutation or credential reuse; next current owner reloads durable state. |
| Receipt / provenance substitution | Substitute source compatibility, occurrence binding or stored revision independently while current fence remains valid; reject at actual writer/reconciler/integrity boundary. A historical match never restores authority. |
| Observation / order and visibility | Result-first/observation-first, duplicate, newer superseding observation, lost ack and unresolved outcome converge without duplicate write. Stationary appearance change emits update; leave/reappear and EntityRef generation reset purge old mapping; hidden actor never revealed. |
| Client domain / downgrade | Wrong mapping/content/palette/session/context, stale sequence/revision, unknown ref, missing mapping and unsupported capability fail/refresh via native policy. Reconnect without previously selected support is refused; no cross-session refs/draft/replay. |
| Temporary/fallback composition | Base changes under active Outfit and Item transforms, explicit clear and owner expiry restore latest base; stale/non-monotonic projection/clear refuses. Incompatible-world fallback preserves durable selection; client timer and missing packet cannot clear state. |

# HOUSE-WIRE-1 — House codecs and native ACL request adapter

```yaml
task_id: HOUSE-WIRE-1
title: "HOUSE-WIRE-1: House codecs and native ACL request adapter"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-wire-1
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-WIRE-1 (rows 119, 121); App. 7 HOUSE-*; §5 Wave 5"
class: "impl (protocol review)"
needs_migration: no
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "crates/protocol-oteryn/src/house.rs (new)"
  - "crates/protocol-oteryn/src/house_tests.rs"
  - "apps/game-server/src/gameplay_transport/house_intent.rs (new)"
  - "apps/game-server/src/gameplay_transport/house_intent_tests.rs"
  - "docs/contracts/protocol-oteryn/v1/house_v1.proto (new)"
  - "client House Management panel (coordinate with client lane #1942)"
  - "docs/agents/tasks/HOUSE-WIRE-1.md"
slot_only_paths:
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability and command numbers leased at queue turn)"
  - "crates/protocol-oteryn/src/lib.rs (module export)"
  - "apps/game-server/src/gameplay_transport/connection.rs (dispatch arm)"
  - "apps/game-server/src/gameplay_transport/mod.rs"
  - "apps/game-server/src/gameplay_transport/capabilities.rs (+capabilities_tests.rs)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## 2026-10-17 D1384 move-out handoff — inert source checkpoint

This **card-only** checkpoint extends #2145
`6886b30617b34067a21aa9ec8db0c57a97bd4d4c`. It preserves the complete frozen
two-leaf D1384 source proposal, pinned preimages, patch and independent review
in the [reproducible appendix](#reproducible-d1384-source-checkpoint) below.
No Rust, protocol export, gameplay mount, dispatch, capability or registry is
changed by this checkpoint. All original card content is retained as history;
its named producer refs and earlier AUTHORING labels belong to those records.

The proposed native adapter verifies the real actor, original CommandRef session
and World against the complete unchanged native fence. It forwards the exact
absolute Unix-millisecond date as `HouseMoveOutCommand::Notice` or a fieldless
`Cancel` to actual `DurabilityRoot::apply_house_move_out_command`, passing the
real retained authority, node and HouseId. The native receipt and error are
returned unchanged. It does not mint an operation ID, supply an ownership
revision, sample a clock, create SQL or turn an Instance into a Channel.

[D1384](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235281112)
assigns clock/date validation, operation/revision resolution and immutable
original-command replay to #2141. Its source-reviewed actual five-file owner
candidate is `d3ddcd782f4f288d4b9e0b85a25e7b4de37da27c76c79ca4f0b4d4c0395988a4`,
based on #2141 `c65dd10af69ce6d2263175a35736fbe531f01a01`.
[D1403](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237071382)
now confirms the complete canonical 0121 disposition producer before its guard,
with `MIGRATOR` as the only installer. That allocation is not native execution
or an actual merge. The WIRE Rust adapter remains HELD until #2141 actually
merges; then it must be rebound to the current branch and native-qualified.

| Qualification | Exact scope and result |
|---|---|
| New move-out source review | **PASS**: handler `2510f019c2e151d9e955bcfe26d185378b820a902bdb567a49f121fdaed9c41e`, tests `d9b8002d159e76539ba1eb62dd73191d2ddbc4840ed538fbac1faca82a1638df`; full independent report is preserved below. |
| New proposal inventory | **19 conversion cases**, comprising the unchanged prior14 plus five new move-out cases. This count describes the embedded proposed source, not a discovered run of the checked-in adapter. |
| New native formatting/compile/execution | **NOT_RUN**. Source review does not qualify native owner execution or dispatch. |
| Existing exact-head protocol | **21/21 PASS** reported by root at published `6886b306...`; this card-only successor is not a new test run. |
| Prior real source assembly | Protocol21, adapter14 and actual nullable ACL owner9 **PASS** at main `f010472b...` plus their exact owned sources and mounts. It does not qualify the five new D1384 cases. |

The checked-in native adapter remains the previously published ACL component.
It requires actual #2140 composition and its ordered shared mount. The proposed
move-out mapping and #2141's 22PG+1pure candidate remain separate native
qualification inputs. [D1399b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236882742)
orders the current MAIL and then HUNT heavy windows; further House runs require
their own coordinated window and the actual D1397/D1397a shared lock. This card
does not start a native job.

Other action writers, query/privacy projection, Guild wire identities, active
capabilities and client House UI remain incomplete. No embedded code is
activated by preserving it here. The PR stays draft / **NOT_FULLY_QUALIFIED**.

## 2026-10-17 D1350/D1354 native ACL packet — AUTHORING

Predecessor after CP's ordinary main merge:
`afd7272eaeecaec443932977b56ec02d090fd2c5`. The single writer and owned-packet
authoring release are [D1345/D1350](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418).
The minimum module allocations are [D1354](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098).

The owned native handler now maps character subowner, guest and door ACL requests
to the actual `DurabilityRoot::replace_house_acl` owner. It accepts that owner's
real retained `ReconciledCharacterAuthority`, `NodeIncarnationProof` and complete
`CurrentCharacterItemFence`. It checks the request actor, command session and
World against the unchanged fence, preserves the explicit House target, native
door coordinate conversion, subjects and expected revision, and returns the
owner's receipt or error without creating a transaction, success ACK or authority.
The writer retains currentness, scope admission, authorization and replay decisions;
an Instance is never changed to a Channel by the adapter.

Five added conversion tests cover both character list kinds and revision bounds,
native door boundaries and empty-list clearing, each mismatched binding, non-ACL
action refusal, and unchanged Instance scope. The existing nine preparation tests
and twenty-one protocol tests remain present. These tests do not by themselves
qualify a current authority grant or execute the PostgreSQL ACL transaction.

The packet adds only D1354's `pub mod house;` line to the protocol crate. It leaves
the gameplay module, dispatch, capabilities and registries to their ordered slots.

| Remaining composition | Exact owner and order |
|---|---|
| Native ACL owner and door placement | #2140 at `20b751c889936cb311ca61d0374512e83e374470`; native composition and its real door helper follow #2167 (D1348/D1350). |
| Gameplay module registration | One `house_intent` line after #2197, before #2198 and A2 #2187; compile and dispatch follow #2140 (D1354). |
| Guild ACL wire identities | #2153 at `4b435e05143f6ea66b90ecb2e2df26174c945d6d`; no invented Guild request tags. |
| Bids, move-out date mapping, kick/leave and query projection | Their actual mutation/read owners remain required. `AclView` observations do not supply a full privacy-filtered House query response. |
| Capability, response projection and client | A2 and the native privacy projection; House panel after #1942 at `fdc20be476e6d38913d254209ccfa710743d6c59`. |

Validation of this packet: the coordinator's Molehill slot B run starting
2026-10-17 13:20:10 UTC passed all twenty-one codec cases, all fourteen native
preparation/conversion cases and all nine actual ACL owner cases (four pure and
five PostgreSQL), with no failed or ignored cases. The protocol build took 4.96s;
the game-server build took 2m03s; the actual ACL cases took 10.12s. Focused native
formatting passed and the source manifest stayed unchanged across the run.

This evidence is explicitly a SOURCE ASSEMBLY: main
`f010472bcb6794a2cbc56dfaf755bd122c10225e`, the four actual WIRE files, the three
actual #2140 ACL files with their reviewed nullable-owner correction, and only
the required module lines. D1342's native environment/slot target and D1329's
shared PostgreSQL wrapper were used; `cargo fetch --locked` passed before the
offline run. The tested adapter SHA-256 is
`7f4dabec44fe7b9c49bbbd394df48548270e8d20df8a52fa912dcea5e4805843` and its
test SHA-256 is
`ef925aa7fcf68835705936d2e7019fec22b3009737ada8d5f1ae6b0c53148603`.

The fourteen adapter cases are conversion tests. The nine separate native ACL
cases exercise the actual writer; they are not a dispatched end-to-end House
request. The published #2145 branch exports the codec only: it does not include
the gameplay mount or copy the unmerged #2140 dependency. Published-head adapter
qualification, package Clippy, native dispatch/privacy/client composition and
complete feature qualification remain open. The PR remains draft.

## D1170 target-binding source checkpoint — historical qualification

This section records the earlier source checkpoint; the current allocation and
implementation are stated above.

CP D1170 assigns leaf constants HOUSE_V1=27, HOUSE_INTENT=27, HOUSE_QUERY=28,
domain23. No registry, offered capability, shared exports or dispatch is activated.
Every complete intent now carries one explicit WorldId plus native House key,
including ACL, door, kick, move-out, cancel and leave. Transport refuses a body
World different from the current actor World before preparing any action, validates
all keys with native HouseId, and retains that identity alongside the exact actor
and CommandRef. Prepared values remain request data, never current authority.
The old Bid-local key is removed to keep one target. Unqualified old bodies refuse.

The proposed full bound is4168 bytes: largest native200-member Door action3626
plus World/key target536 plus6 spare. Target uses field8 alongside exactly one
original action; unknown/duplicate/missing fields and invalid UUID/UTF8 refuse.
Action-level coordinate/list/revision cases remain focused on the private action
codec; new tests exercise the public complete-target codec and actual native
preparation boundary, World mismatch, invalid non-Bid keys and request correlation.

Prior direct-rustc255/6 evidence below applies only to3142194 and is historical.
This source generation's compile, Clippy and test execution are NOT_RUN. Exact
Rustfmt1.94 and full whitespace checks pass. Independent Medium source review
passed the complete six-path batch, including target binding and byte bounds.
HOUSE_QUERY request town/cursor codec and inactive preparation are authored; request tags/bounds
are approved by D1261. Response/projection/native privacy read, guild ACL tags from#2153, full
revisioned writer integration, A2 activation and client panel after#1942 remain NOT_IMPLEMENTED.
Intent layout/bounds still require independent protocol review; no complete feature claim.

## Goal

Capability `HOUSE_V1`, `HOUSE_INTENT` commands (house list, bids, move-out, ACL edits) and the House Management panel.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1b | NOT MERGED | **BLOCKING** |
| HOUSE-ACL-1 | NOT MERGED | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- crates/protocol-oteryn/src/house.rs (new)
- `crates/protocol-oteryn/src/house_tests.rs`
- apps/game-server/src/gameplay_transport/house_intent.rs (new)
- `apps/game-server/src/gameplay_transport/house_intent_tests.rs`
- docs/contracts/protocol-oteryn/v1/house_v1.proto (new)
- client House Management panel (coordinate with client lane #1942)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability and command numbers leased at queue turn)
- crates/protocol-oteryn/src/lib.rs (module export)
- apps/game-server/src/gameplay_transport/connection.rs (dispatch arm)
- apps/game-server/src/gameplay_transport/mod.rs
- apps/game-server/src/gameplay_transport/capabilities.rs (+capabilities_tests.rs)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/connection.rs command dispatch
- apps/game-server/src/gameplay_transport/capabilities.rs offered set
- crates/protocol-oteryn/src/lib.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests (one commit at slot)

## Acceptance criteria

- [ ] older clients without HOUSE_V1 receive nothing new; commands rejected without capability
- [ ] flat message bounds in RESOURCE_LIMITS
- [ ] handlers call HOUSE-1a/1b/ACL writers only
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-protocol-oteryn house
cargo test -p oteryn-game-server capabilities house_intent
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
- docs/agents/tasks/archive/WORLDINT-WIRE-1.md (wire pattern)
- `docs/reference/tibia-manual/houses.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Independent preparation after CP release (2026-10-15)

CP [6193914573](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6193914573)
released the former HELD state for independent draft modules/tests. Exact local source:
`f8b0a276f8bc5cc777660d61f21d0f43f535570c`, accepted-main inspection
`ca5ea5930edf0d609e91d7a3f5436e7ca2fde610`. The exact allocation JSON limits this
preparation to six paths; the historical client path above is **not allocated now**.
The historical permission to add temporary registrations above is superseded by this
allocation: no draft shared registrations, capability numbers, dispatch, client or pins.
All original family acceptance boxes remain OPEN. Preparation does not close dependencies,
make a capability effective or supply production protocol/merge authority.

### Implemented bounded component

`house.rs` provides encode/decode for the seven accepted request variants: bid, move-out,
cancel, character ACL replacement, door-list replacement, kick and leave. It uses the
existing strict protobuf wire helpers, native `CharacterId` UUIDv7 decoder and actual
world-spatial signed-coordinate codec. It rejects unknown fields, repeated singular
fields, empty/multiple oneofs, unknown roles, malformed varints/lengths/UTF-8/identifiers,
duplicate list identities and oversized bodies/lists. Empty lists clear; revision0 and
bid max0 remain syntactically valid. Current-price/funds/eligibility rules belong to the
owning writer and are not reinvented by the codec. List order is preserved.

`house_intent.rs` converts a decoded bid key through real native `HouseId::new` in the
supplied actor World and retains the actual `CommandRef` and acting `CharacterId` in an
immutable request. Other variants keep exact requested data for the owning actor-house
resolver. This prevents the body from nominating another World, session or future owner;
it does **not** prove that the supplied actor/World/session is current. There is no writer,
result-producing handler, database mutation, role callback, capability flag or current-proof
constructor in this component.

`house_v1.proto` explicitly proposes the body field tags, positive signed-range Unix-ms
date representation and flat3632-byte bound. They are **not an accepted wire allocation**.
The bound derives from accepted200-entry lists,16-byte UUID fields, the existing position
codec and u64 revision, rather than an invented count/limit registry row. The parser checks
the whole-body bound before nested allocation and each list count before inserting another
entry. Protocol review and CP registration/measurement remain required before activation.

### Exact remaining consumer and protocol gates

1. At the CP slot, after HOUSE-1b and HOUSE-ACL-1 land on main, review/allocate actual
   HOUSE_V1 capability, two command types, domain and payload bounds; export the owned
   modules in protocol `lib.rs` and gameplay `mod.rs`. No number is selected here.
2. `gameplay_transport/connection.rs` must gate the selected capability **before** interpreting
   either command, bind actual admitted actor/session/CommandId (following the existing
   NPC_TRADE dispatch's `UseCommand` binding pattern), then use the own preparation function.
   Resolve the actor's actual house/World and current native session/lease/scope fence;
   preserve scope semantics for both Channel and House sessions. Body data or a retained
   request never supplies this authority. The panel must still work away from its house.
3. Bid/ACL/move-out commands must call their owning native writers under the same current
   Character/session and property/ACL/ledger locks; preserve exact expected revision and
   CommandRef binding. The prepared private HOUSE-1b notice currently accepts notice-days,
   whereas OWN0 wire says date. Mapping the proposed Unix-ms date must use its owning DB
   clock/selected-date semantics under that writer, without rounding an unqualified client
   or transport clock into admission. No unmerged private schema is consumed here.
4. HOUSE_QUERY still requires reviewed request/projection shapes and flat resource bounds:
   houses by town, current public price/ends/rent, actor-only bid max/own house and authorized
   owner/subowner ACL. Never project other bidders' maxima. Town key/pagination and common/
   domain result shapes are not guessed. Accepted GUILD0 extends ACLs with guild matching
   and character exclusions; this character-only body subset does not invent that wire shape
   or claim full ACL support. OWN0 also omits a house selector from ACL/door intents despite
   remote editing and potentially multiple subowner grants: active dispatch needs a reviewed
   unambiguous house binding, not invented remembered panel state or nearest/owned-house
   selection. Resolve these with HOUSE-ACL/GUILD wire owners before activation.
5. Kick/leave must resolve actual target presence, current owner/subowner role, inside/owner
   condition and real fenced house-exit admission. Native `house_scope_handoff` currently
   exposes entry and refuses exit with `ExitNotAdmitted`; syntactic decoding never overrides
   this. No side-effect acknowledgement precedes the owning durable result. Unknown outcome
   must end/hold the command before sequencing, reconcile exact native receipt and replay
   the same CommandRef without a second effect.
6. Capability-off older clients receive no new state, and unselected intents/queries reject.
   The actual panel, domain projection, ACL/ownership/runtime, current Premium/funds and
   protocol/generation/reconnect/replay matrix require the complete registered integration.
   No current handler, authority liveness, PostgreSQL writer or runtime E2E is claimed here.

### Qualification record

Exact commands, original source hashes, validation-only literal registration/facade files
and actual results are retained in `/workspace/oteryn-prep/house-wire-1-evidence/`.
Protocol tests qualify the original crate plus only a literal snapshot-only `pub mod house`;
native intent tests use the real retained server library's HouseId/CommandRef and native ID
identity. The small validation facade loads the unchanged candidate codec/wire helpers;
it does not replace native authority or reproduce its types. Actual direct-rustc qualification completed: the original protocol crate snapshot passed
252/252 tests (including10 owned codec tests), and the small native public-API harness passed
5/5 owned preparation tests. Original protocol and native-intent Clippy metadata passed
with warnings denied, as did exact owned rustfmt-check. The narrow native facade compile
retains3 unused-function warnings from unchanged shared wire helpers; neither qualification
Clippy run has warnings. There was no failing Rust/setup round and no semantic RED claim.
Governance validator and124 governance tests passed. These are source-bound direct-rustc
checks, **not Cargo/full native library unit, PostgreSQL or current-runtime qualification**.
The original MAP native library was reused without modifications; all9 package/lock/type/
helper compatibility inputs were byte-identical. All27 captured native extern artifact hashes
and an immutable original protocol artifact were retained after validation. Own outputs were
private evidence paths, with no shared-target protocol replacement. The retained native
artifact is released to root after the completed checks; no unchanged test repeat is queued.

Builder/CP rules remain: no ready, auto-merge, merge-main, shared slot edits or review triggers
by this worker. Root may publish only the existing draft; CP owns later queue/integration.


## Corrective coordinate batch (2026-10-15, after candidate c86b862)

Root cross-module review found P2: the initial DoorSet validator and its fixtures used the
broader ActorPosition wire shape rather than the actual native tile domain. The earlier
252/5 direct-rustc passes are retained immutable as a prior qualification that missed this
boundary; they do not qualify native door coordinates. This worker missed that source trace.

The actual `gameplay_transport/world_map.rs::tile_pos` performs checked u16 x/y conversion,
checked i8 floor conversion and accepts floor-15..0. `map::overlay::TilePos` stores u16 x/y
and i8 floor. The corrected codec rejects x/y outside0..65535 and floor outside-15..0 on
both encode/decode; it does not flip a floor sign or remap coordinates. Three new regressions
first ran against unchanged published codec: valid underground positions, invalid positive
floors and invalid axes. Raw RED logs and baseline/test hashes are retained separately in
`/workspace/oteryn-prep/house-wire-1-coordinatefix-1-evidence/`.

Corrected fixtures cover legal native endpoints and an independent signed-floor protobuf
fixture. A real native TilePos type test verifies unchanged coordinate values through the
prepared request; this is public type integration, not execution of the private tile helper
or current door/house authority. The body field layout and3632-byte proposed limit stay
unchanged (canonical largest native door body3626 bytes, six spare bytes). Prototype native
coordinate comments are corrected; all earlier proposal/ACL selector/guild/query/result/
activation/current-authority gates remain open. No shared source, registry, client or active
dispatch changes.

Corrective qualification: against the unchanged published codec, the three new regressions
failed (0 passed/3 failed/252 filtered), exposing underground rejection and acceptance of
negative axes/positive floors. The same regression bodies (whitespace only after formatting)
then passed in the complete original protocol snapshot:255 passed/0 failed. The corrected
native public-API harness passed6/6. Protocol and native-harness Clippy metadata with warnings
denied and owned rustfmt-check passed. The narrow facade retains3 baseline unused-helper
warnings; neither Clippy context has warnings. These are direct-rustc source-bound checks,
not Cargo/full native unit, PostgreSQL, private tile-helper execution or live runtime tests.

The first native attempt did not execute tests: the preserved MAP rlib could not resolve its
changed shared dependency graph (E0463). Exact hashes show12/27 captured direct externs had
drifted; this is a setup failure, not behavioral RED. No dependencies were swapped and no
server rebuild was performed. Qualification resumed once using the freshly built genuine
HOUSE-ACL native rlib (SHA256 5a805f39781821550dad290f2ae8caa37d585af86a597dca5cd48bb8c28512fd):
all11 relevant package/lock/native-type/helper source inputs and27 current captured extern
hashes matched under the Rust lock. Its other candidate ACL code is unused by this value/API
harness. All commands, raw logs, preserved RED baseline, exact regression-body binding,
source compatibility and artifact hashes are in the corrective evidence directory. The old
252/5 packet and original evidence are unchanged. Governance and124 governance tests pass;
owned scope and whitespace checks pass. No publication, registration or authority is claimed.

## D1170 HOUSE_QUERY request successor — LOCAL_CHECKPOINT, NOT_FULLY_QUALIFIED

Adds an inactive strict request codec and native preparation for explicit WorldId, catalogueTown
and an optional bounded opaque cursor. The actual catalogue town reference is the city Area key
(`oteryn:content.area.city.<slug>`); the native type is `FirstProductionArea.key: ProductionKey`,
not a newly invented TownId. Transport validates native production-key syntax/prefix and the
current actor World while retaining the exact CommandRef/actor; this is immutable request data.
Existing HOUSE_INTENT encoding/preparation remains unchanged.

CP D1261 approved field tags1/2/3 and technical caps town512B/cursor512B/body1056B;
encoded maximum1048B leaves8B slack. The town cap matches native ProductionKey's existing512B
cap; the cursor/body caps are technical request bounds, not gameplay values. No response,
projection, active-bundle membership, own-bid maximum or ACL/privacy read is implemented.
The real native privacy/currentness read port, dispatcher, resource row and A2 wiring remain
NOT_IMPLEMENTED. A cursor does not grant read access or establish DB currentness.

Focused actual codec/native preparation cases are authored NOT_RUN. Current-generation
compile/native/PG remain NOT_RUN; fmt/diffcheck alone do not qualify this successor.

Independent source-only review and official fmt/diffcheck pass. D1261 approves the request
protocol and one coherent additional checkpoint beyond D1122. Current-generation compile,
Clippy and tests remain NOT_RUN; this checkpoint does not qualify the complete mechanic.

## D1261 request conditions — CHECKPOINT, NOT_FULLY_QUALIFIED

CP #2131 comment6226929679 approved HOUSE_QUERY tags1/2/3 and technical bounds
town512B/cursor512B/body1056B (maximum canonical1048B). Existing D1170 command28 remains28.
Present empty cursor now means only the first page, exactly like absence; native preparation
normalizes it to None while preserving World/actor/CommandRef and granting no read authority.
Duplicate cursor tags still refuse even when both are empty. Native privacy/currentness read,
response/projection/ownmax/ACL and shared dispatch remain NOT_IMPLEMENTED.

Added actual codec decode boundary1048/1049 and cursor512/513 cases, duplicate-empty/unknown
rejection and native empty/absent normalization cases; all authored NOT_RUN. Official fmt and
diffcheck are cheap source checks only; no current-generation compile/native/PG PASS claimed.
D1261 permits one coherent checkpoint successor after Root review; this work does not publish.

## Reproducible D1384 source checkpoint

This complete UTF-8 source checkpoint is retained as data inside the owned task card. Its exact length is 17946 bytes and SHA-256 is `60e1a95a82d71e4cf4ea45f3ae6a51d808be04ea5bf4163aed62f2dd98cb9d5a`. The source inventory, complete patch and analysis/review can be recovered from it; the recovery instructions do not authorize applying a held runtime change.

<!-- card-artifact:wire2145-d1384-source-checkpoint.md -->
`````markdown
# HOUSE-WIRE-1 — D1384 source checkpoint

State: **reviewed inert source; new native formatting, compilation and 19-case execution NOT_RUN**. The exact published predecessor is `6886b30617b34067a21aa9ec8db0c57a97bd4d4c`. This document preserves two frozen owned leaves while their native owner dependency and the allocated publication order remain open.

## Prepared behavior

The native move-out adapter preserves the actor identity, original CommandRef, complete current item fence and exact World-qualified HouseId. It forwards an absolute Unix-millisecond date as `HouseMoveOutCommand::Notice { due_at_unix_ms }`, or a fieldless `Cancel`, to the actual `DurabilityRoot::apply_house_move_out_command` owner. It does not mint operation IDs, supply an ownership revision, sample a clock, reinterpret the date, start SQL or manufacture current authority. The native receipt and error are returned unchanged. Instance scope remains intact so that the owner handles current-scope admission and historical replay.

The existing fourteen conversion cases remain present. Five additional cases cover exact date/command/House propagation, fieldless cancellation, each request/fence mismatch, refusal of other actions and preservation of Instance scope. Inventory: **19 conversion cases**. The independent source review is embedded below and is **SOURCE REVIEW PASS** for the exact two result hashes. It does not claim execution.

## Actual dependency and activation order

[D1384](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235281112) assigns the real operation/revision/clock/replay port to #2141 and the consumer mapping to #2145. The reviewed #2141 candidate uses `HouseMoveOutCommand`, `HouseMoveOutCommandReceipt` and `HouseMoveOutError` in `house::move_out`, with the actual Root method named above. Its five-file native payload has SHA-256 `d3ddcd782f4f288d4b9e0b85a25e7b4de37da27c76c79ca4f0b4d4c0395988a4` and is based on #2141 `c65dd10af69ce6d2263175a35736fbe531f01a01`. That candidate is a source dependency, not a merged owner or a test PASS. [D1403](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237071382) now confirms the full canonical 0121 producer relocation and single MIGRATOR installer. Actual owner publication/merge and native qualification remain separate requirements.

Publication of these Rust leaves follows the **actual #2141 merge**, then a fresh #2145 baseline/readback and native formatting/qualification. The existing ACL adapter still depends on the actual #2140 nullable-owner sources. Gameplay module mounting/dispatch, other action writers, query/privacy projection, Guild identities, capabilities and client UI remain separately ordered. This checkpoint adds none of those integration points.

## Previously completed qualification

Root reported protocol **21/21 PASS on the exact published #2145 head**. The earlier combined source assembly at main `f010472bcb6794a2cbc56dfaf755bd122c10225e` executed protocol 21, adapter 14 and actual ACL owner 9 cases successfully on Molehill. Those results apply to their recorded sources. They do not execute the five newly recorded move-out cases, qualify #2141's new PG owner, or establish an end-to-end dispatched House request.

## Recovery and integrity

This is inert source evidence preserved verbatim in the existing owned task card. Only that card is proposed for the draft; the embedded Rust changes and dependency sources are not applied or registered. No new repository evidence path or owned_paths expansion is used. Runtime permission, dependency order and qualification gates still apply.

Recover each artifact from the matching section below. Preserve its exact UTF-8 bytes and verify the listed byte length and SHA-256. Reconstruct an isolated review tree from each file's pinned base ref and Git blob in `source-inventory.json`, then check and apply the patch there. Never apply a historical patch to an unspecified current checkout. The checkpoint preparation independently applied the recorded patch to those exact preimages and compared every resulting file byte for byte with the frozen source. That integrity check is not Rust compilation or PostgreSQL qualification.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `source-inventory.json` | 1005 | `bbf63529c517780fe15bf778a61930b21cada7bc752f0c0b71041dc53db98242` |
| `wire2145-d1384.patch` | 9326 | `4e103fc14d48e3a18bd3c87b285b611fca90173a56646de06c817fc1b58f038f` |
| `independent-source-review.md` | 2731 | `3a490e92ea002fcb8d6c1990aa4cd282dc817b66a6affd06221411a784ce1e1d` |

## Embedded artifacts

### source-inventory.json

<!-- checkpoint-artifact:source-inventory.json -->
````json
{
  "predecessor": "6886b30617b34067a21aa9ec8db0c57a97bd4d4c",
  "files": [
    {
      "path": "apps/game-server/src/gameplay_transport/house_intent.rs",
      "base_ref": "6886b30617b34067a21aa9ec8db0c57a97bd4d4c",
      "base_git_blob": "6de13c8a7e054944e942ff894d42ba8f98a03652",
      "base_sha256": "7f4dabec44fe7b9c49bbbd394df48548270e8d20df8a52fa912dcea5e4805843",
      "result_sha256": "2510f019c2e151d9e955bcfe26d185378b820a902bdb567a49f121fdaed9c41e",
      "result_bytes": 13278
    },
    {
      "path": "apps/game-server/src/gameplay_transport/house_intent_tests.rs",
      "base_ref": "6886b30617b34067a21aa9ec8db0c57a97bd4d4c",
      "base_git_blob": "4669cac1c475782db9db6dc985d78d8544874084",
      "base_sha256": "ef925aa7fcf68835705936d2e7019fec22b3009737ada8d5f1ae6b0c53148603",
      "result_sha256": "d9b8002d159e76539ba1eb62dd73191d2ddbc4840ed538fbac1faca82a1638df",
      "result_bytes": 19358
    }
  ],
  "expected_conversion_cases": 19,
  "native_new_candidate": "NOT_RUN"
}
````

### wire2145-d1384.patch

<!-- checkpoint-artifact:wire2145-d1384.patch -->
````diff
--- a/apps/game-server/src/gameplay_transport/house_intent.rs
+++ b/apps/game-server/src/gameplay_transport/house_intent.rs
@@ -1,4 +1,4 @@
-//! HOUSE_INTENT preparation and native character-ACL writer adapter. No dispatch is installed.
+//! HOUSE_INTENT preparation and native ACL/move-out writer adapters. No dispatch is installed.
 //!
 //! Every body supplies an explicit World-qualified target. Its World must equal the current
 //! actor World; native HouseId validates the key. The request selects no owner/session/lease. Inputs are
@@ -12,6 +12,9 @@
 use crate::durability::item_transfer::CurrentCharacterItemFence;
 use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
 use crate::foundation::{CharacterId, CommandRef, WorldId};
+use crate::house::move_out::{
+    HouseMoveOutCommand, HouseMoveOutCommandReceipt, HouseMoveOutError,
+};
 use oteryn_protocol_oteryn::house::{
     HouseAclRole, HouseAction, HouseWireError, decode_house_intent,
 };
@@ -230,6 +233,80 @@
         .map_err(HouseAclHandlerError::Writer)
 }
 
+/// Immutable command/fence binding failures, not current ownership or date decisions.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum HouseMoveOutRequestError {
+    ActorMismatch,
+    SessionMismatch,
+    WorldMismatch,
+    NotMoveOutAction,
+}
+
+#[derive(Debug)]
+#[allow(
+    dead_code,
+    reason = "native adapter outcomes are consumed by the later CP ingress slot"
+)]
+pub(crate) enum HouseMoveOutHandlerError {
+    Request(HouseMoveOutRequestError),
+    Writer(HouseMoveOutError),
+}
+
+/// Preserve the exact absolute date or fieldless cancellation for the native owner.
+/// Only that owner resolves/mints the operation, reads the ownership revision and
+/// samples its database clock. Historical replay retains the original date/result.
+fn native_move_out_request(
+    prepared: &PreparedHouseIntent,
+    fence: &CurrentCharacterItemFence,
+) -> Result<HouseMoveOutCommand, HouseMoveOutRequestError> {
+    if prepared.actor.as_bytes() != fence.character_id.as_bytes() {
+        return Err(HouseMoveOutRequestError::ActorMismatch);
+    }
+    if prepared.command.game_session_id() != fence.game_session_id {
+        return Err(HouseMoveOutRequestError::SessionMismatch);
+    }
+    if prepared.world != fence.runtime_scope.world_id() {
+        return Err(HouseMoveOutRequestError::WorldMismatch);
+    }
+    match &prepared.action {
+        PreparedHouseAction::MoveOut { date_unix_ms } => Ok(HouseMoveOutCommand::Notice {
+            due_at_unix_ms: *date_unix_ms,
+        }),
+        PreparedHouseAction::CancelMoveOut => Ok(HouseMoveOutCommand::Cancel),
+        _ => Err(HouseMoveOutRequestError::NotMoveOutAction),
+    }
+}
+
+/// Execute the selected command through HOUSE-1b's actual same-transaction owner.
+/// The complete native fence and original CommandRef/HouseId pass unchanged; an
+/// Instance remains an Instance. The owner decides current admission and historical
+/// replay, and returns the real receipt or error. This adapter performs no preliminary
+/// notice lookup, date rounding, operation minting, independent commit or success ACK.
+#[allow(
+    dead_code,
+    reason = "native move-out adapter awaits #2141 and the CP-owned HOUSE_V1 ingress slot"
+)]
+pub(crate) async fn handle_house_move_out_intent(
+    root: &DurabilityRoot,
+    authority: &ReconciledCharacterAuthority<'_, '_>,
+    node: &NodeIncarnationProof,
+    fence: CurrentCharacterItemFence,
+    prepared: &PreparedHouseIntent,
+) -> Result<HouseMoveOutCommandReceipt, HouseMoveOutHandlerError> {
+    let request =
+        native_move_out_request(prepared, &fence).map_err(HouseMoveOutHandlerError::Request)?;
+    root.apply_house_move_out_command(
+        authority,
+        node,
+        fence,
+        prepared.command,
+        prepared.house.clone(),
+        request,
+    )
+    .await
+    .map_err(HouseMoveOutHandlerError::Writer)
+}
+
 /// Inactive request data only. ProductionKey is the native FirstProductionArea.key type;
 /// its city prefix comes from the House catalogue contract. This does not prove that a
 /// town exists in the active bundle, or authorize a catalogue/owner/ACL read.
--- a/apps/game-server/src/gameplay_transport/house_intent_tests.rs
+++ b/apps/game-server/src/gameplay_transport/house_intent_tests.rs
@@ -442,3 +442,115 @@
         Err(HouseAclRequestError::WorldMismatch)
     ));
 }
+
+#[test]
+fn native_move_out_adapter_preserves_absolute_milliseconds_and_original_target() {
+    // These are request values, not assertions that the native DB clock admits them.
+    for date_unix_ms in [1, 2_592_000_001, 1_792_000_000_123, i64::MAX] {
+        for house_key in ["oteryn:content.house.thais_1", "oteryn:content.house.thais_2"] {
+            let mut value = request(HouseAction::MoveOut { date_unix_ms });
+            value.house.house_key = house_key.into();
+            for command_id in [1, u64::MAX] {
+                let correlation = CommandRef::new(
+                    command(2).game_session_id(),
+                    CommandId::new(command_id).unwrap(),
+                );
+                let prepared = prepare_house_intent(
+                    &encode_house_intent(&value).unwrap(),
+                    correlation,
+                    actor(),
+                    world(3),
+                )
+                .unwrap();
+                let native = native_move_out_request(&prepared, &acl_fence()).unwrap();
+                assert!(matches!(
+                    native,
+                    HouseMoveOutCommand::Notice { due_at_unix_ms } if due_at_unix_ms == date_unix_ms
+                ));
+                assert_eq!(prepared.command(), correlation);
+                assert_eq!(prepared.actor(), actor());
+                assert_eq!(prepared.house(), &HouseId::new(world(3), house_key).unwrap());
+            }
+        }
+    }
+}
+
+#[test]
+fn native_move_out_adapter_leaves_cancellation_operation_with_the_owner() {
+    let prepared = prepare(&request(HouseAction::CancelMoveOut), world(3)).unwrap();
+    assert!(matches!(
+        native_move_out_request(&prepared, &acl_fence()).unwrap(),
+        HouseMoveOutCommand::Cancel
+    ));
+    assert_eq!(prepared.command(), command(2));
+    assert_eq!(
+        prepared.house(),
+        &HouseId::new(world(3), "oteryn:content.house.thais_1").unwrap()
+    );
+    // The native Cancel type has no caller operation, revision or date to retarget on retry.
+}
+
+#[test]
+fn native_move_out_adapter_refuses_each_mismatched_binding_for_both_commands() {
+    for action in [HouseAction::MoveOut { date_unix_ms: 1 }, HouseAction::CancelMoveOut] {
+        let prepared = prepare(&request(action), world(3)).unwrap();
+        let mut different_actor = acl_fence();
+        different_actor.character_id = crate::domain::CharacterId::from_bytes(uuid(8)).unwrap();
+        let mut different_session = acl_fence();
+        different_session.game_session_id = command(9).game_session_id();
+        let mut different_world = acl_fence();
+        if let RuntimeScopeRefV1::Channel { world_id, .. } = &mut different_world.runtime_scope {
+            *world_id = world(9);
+        }
+        for (fence, expected) in [
+            (different_actor, HouseMoveOutRequestError::ActorMismatch),
+            (different_session, HouseMoveOutRequestError::SessionMismatch),
+            (different_world, HouseMoveOutRequestError::WorldMismatch),
+        ] {
+            assert!(matches!(native_move_out_request(&prepared, &fence), Err(actual) if actual == expected));
+        }
+    }
+}
+
+#[test]
+fn native_move_out_adapter_never_routes_another_action_to_the_notice_owner() {
+    for action in [
+        HouseAction::Bid { max_gold: 1 },
+        HouseAction::AclSet {
+            role: HouseAclRole::Guest,
+            character_ids: vec![],
+            expected_revision: 1,
+        },
+        HouseAction::DoorSet {
+            position: ActorPosition { x: 10, y: 24, floor: -7 },
+            character_ids: vec![],
+            expected_revision: 1,
+        },
+        HouseAction::Kick { character_id: actor() },
+        HouseAction::Leave,
+    ] {
+        let prepared = prepare(&request(action), world(3)).unwrap();
+        assert!(matches!(
+            native_move_out_request(&prepared, &acl_fence()),
+            Err(HouseMoveOutRequestError::NotMoveOutAction)
+        ));
+    }
+}
+
+#[test]
+fn native_move_out_adapter_preserves_instance_scope_for_owner_replay_and_admission() {
+    for action in [HouseAction::MoveOut { date_unix_ms: 1 }, HouseAction::CancelMoveOut] {
+        let prepared = prepare(&request(action), world(3)).unwrap();
+        let mut fence = acl_fence();
+        fence.runtime_scope = RuntimeScopeRefV1::instance(world(3), uuid(7)).unwrap();
+        let unchanged = fence;
+        assert!(native_move_out_request(&prepared, &fence).is_ok());
+        assert_eq!(fence, unchanged);
+        fence.runtime_scope = RuntimeScopeRefV1::instance(world(9), uuid(7)).unwrap();
+        assert!(matches!(
+            native_move_out_request(&prepared, &fence),
+            Err(HouseMoveOutRequestError::WorldMismatch)
+        ));
+    }
+    // Conversion preserves scope semantics; it grants no current owner or replay outcome.
+}
````

### independent-source-review.md

<!-- checkpoint-artifact:independent-source-review.md -->
````markdown
# Independent source review — D1384 WIRE adapter

**Result: SOURCE REVIEW PASS for the bounded two-file change.** Native formatting, compilation and execution were not run by this reviewer. Publication/activation remains gated on the actual #2141 producer and CP ingress allocation.

Reviewed exact sources from `agent-2145-2147-2148/wire2145-moveout/candidate/`:

| File | SHA256 |
| --- | --- |
| `apps/game-server/src/gameplay_transport/house_intent.rs` | `2510f019c2e151d9e955bcfe26d185378b820a902bdb567a49f121fdaed9c41e` |
| `apps/game-server/src/gameplay_transport/house_intent_tests.rs` | `d9b8002d159e76539ba1eb62dd73191d2ddbc4840ed538fbac1faca82a1638df` |

The comparison uses the actual newly authored #2141 API in frozen payload `d3ddcd782f4f288d4b9e0b85a25e7b4de37da27c76c79ca4f0b4d4c0395988a4`, including the native Root transaction envelope and same-transaction owner. No interface stub or replacement authority is used in this review.

`native_move_out_request` preserves the exact signed millisecond value as `HouseMoveOutCommand::Notice { due_at_unix_ms }` and maps cancellation to the fieldless `Cancel`. It neither samples a clock nor rounds, retargets or manufactures an operation/revision. The exact actor byte identity, original CommandRef session and World agree with the complete retained fence before dispatch. The World comparison does not convert Instance or House scope to Channel; native admission and historical replay remain decisions of the actual owner.

`handle_house_move_out_intent` forwards the genuine Root, retained recovery authority, Node proof, untouched fence, original CommandRef and validated HouseId to `apply_house_move_out_command`. Its only error transformation labels the native error as `Writer`; it returns the actual receipt unchanged. There is no independent SQL, commit, preliminary owner/notice lookup, callback, authorization flag or success ACK.

The five added conversion cases cover exact milliseconds/house/command identity, fieldless cancellation, independently mismatched actor/session/World for both commands, refusal to route other House actions, and preservation of Instance scope. The prior fourteen case bodies remain unchanged. These are conversion checks; they do not claim that a syntactically valid absolute date or preserved Instance is admitted by the native writer. The actual #2141 PG suite separately owns current authority, deadline bounds, pending-notice resolution and durable replay qualification.

No material defect found in this bounded adapter. The code uses the exact producer names and typed signatures that were independently reviewed; its five new tests and the older fourteen still require the native source assembly before qualification.
````

`````

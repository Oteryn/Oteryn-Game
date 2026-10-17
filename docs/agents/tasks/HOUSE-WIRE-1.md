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

# HOUSE-WIRE-1 — HOUSE_V1 capability, house list, bids, move-out (skeleton task record)

```yaml
task_id: HOUSE-WIRE-1
title: "HOUSE-WIRE-1: HOUSE_V1 capability, house list, bids, move-out (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-wire-1
base_sha: 5742f30f3
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

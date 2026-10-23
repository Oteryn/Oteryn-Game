# MAP-OVERLAY-1c — World reset and overlay retire (consolidated task card)

```yaml
task_id: MAP-OVERLAY-1c
title: "MAP-OVERLAY-1c: World reset (RETIRING -> drain -> Ground retirement -> atomic epoch N+1 ACTIVATED) and overlay retire"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MAP-OVERLAY-1c; §7 map overlay chain"
brief: "docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (§1.5–1.8, §1.11, §2.7); ADR-0021 §4.7; HOUSE-CUSTODY0 §3.6"
class: "hard, persistence, authority and security review"
base_branch: main
branch: null
pr: null
needs_migration: true   # next free number at allocation time (only one migration PR in the queue); two schema units in the draft
needs_registry: true    # RESOURCE_LIMITS_REGISTRY MAP01-RESET-RETIRE-MS row; GAME_EVENT_FOUNDATION_REGISTRY audit grammar if widened
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Map pickup still uses epoch 0 pending the reset owner; no MAP-OVERLAY-1c activation/drain owner is retained. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs`.

Remaining: World RETIRING/drain/Ground retirement and atomic epoch activation, source fencing and measured reset qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2167, branch skeleton/map-overlay-1c

## Goal

A World reset retires every live Ground root and its contents deterministically with
audit, preserves World-scoped HouseInterior custody, and atomically activates epoch
N+1 bound to content activation. Third of MAP-OVERLAY-1a → 1b → 1c; releases
MAP-CUTOVER-1c. Accepted sequence (SOCIAL_MAP §2.7, ADR-0021 §4.7):
1. Publish RETIRING (closes admission for the World).
2. Drain: every Channel gets a fresh ordinary ownership generation via the existing
   assignment writer; players logged out and released.
3. Retire every Ground root and its contents, entries before parent, one item per transaction.
4. Lock the reset record, recheck Houses, then write target digest + epoch N+1 + ACTIVATED
   bound to the content activation in one commit.
A crash while RETIRING resumes from step 2 and never boots the old bundle over
partially retired Ground. The first bundle cutover is also a reset (legacy-map Ground
included). Ground on an unowned house tile is still Ground and is retired.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| MAP-OVERLAY-1a / 1b | MERGED (#1800, #1809) |
| MAP-CUTOVER-1b (domain 17 bundle World) | MERGED (#1916) |
| HOUSE-RUNTIME-1a (HouseInterior custody, migration 0095 shape guards) | MERGED (#2163) |
| ADMIT-CAP-1 (sealed drain / admission gating, "I/#2187") | MERGED (#2187) |
| NPC-ECON-1, ITEM-LOOP-1 (main-merge compile neighbours; `loop_move` passes `reset_epoch`) | MERGED (#2197, #2190) |
| HOUSE-1b catalogue-revision jobs | not a prerequisite: preflight refuses a changed target House catalogue on a World with property rows |
| D1447 deploy batch (grant the two reset capabilities to the node / ops logins) | external (oteryn-vps deploy task), after merge |

## Accepted decisions

- **D1134/D1135:** this leaf owns the all-Channel census, the atomic next-epoch ACTIVATED
  writer and the active revision reader; close/logout/drain composition uses #2187.
- **D1152:** durable reset epoch/month history and its migration belong here; DAILY and
  RANK consume the port. **D1257:** epoch 0 is written only by an explicit Control
  initializer (owner-only SECURITY DEFINER); identical facts replay, conflicts refuse; no
  SQL bootstrap seed. Calendar `TIBIA_SERVER_SAVE_BERLIN_10` (PARITY_PENDING) with a
  positive revision: month = `(instant AT TIME ZONE 'Europe/Berlin') - 10h` → YYYY-MM using
  the transaction's `clock_timestamp()`, never process time or a request instant. Missing
  state is `NotInitialized` (DAILY maps to Unknown).
- **D1189/D1189a/D1215:** caller-transaction content-issuance seam (private native
  per-Channel issuance core/CAS) so activation and issuance commit together; a local
  `world_reset_port` resolves the cross-root dependency.
- **D1218/D1269/D1272:** caller-TX House preflight: one root Area, unique Houses only via
  explicit house_tiles, World/map/server identity from native scope compared with actual
  active history and the locked target; Unknown refuses. **D1270/D1283:** a single physical
  placement encoder (content spatial position, mounted via Foundation). **D1245/D1245a/
  D1246:** per-House placement digests use the D1245 codec; full-bundle House production
  is OUT_OF_SCOPE and its metadata is Unknown (a bundle-World reset refuses).
- **D1233:** complete World drain binding (no witness, no current authority).
- **D1416:** schema slot; production orchestration (admission close/drain, census, boot
  resume, limit row) is part of this task. Any later migration that redefines
  `game_item_decay_retire_consistency_guard`, `game_item_instance_change_proven`,
  `game_item_ground_removal_proven` or `game_item_corpse_entry_removal_proven` must keep
  the `game_world_reset_item_proven` early-return; `CREATE OR REPLACE` drops SECURITY
  DEFINER, so re-apply it where main has it and pin `prosecdef` for all four.
- **D1436:** boot recovery and running-node drain wiring (below).
- **D1447:** capability grants go to the oteryn-vps deploy batch (no Synology change).
- **D1451 (option a):** source-map Item receipts keyed by the active reset epoch: add
  `reset_epoch` to `game_native_map_item_receipts`, unique key
  `(world_id, channel_id, reset_epoch, content_digest, placement_key)`, stamp trigger refuses
  a receipt outside the active epoch; epoch 0 keeps its intent bytes, N > 0 binds the epoch
  into intent and audit. Operator command in `oteryn-game-ops`.
- **D1453:** the reset migration must be the last definition of each extended guard and its
  generator extends the latest guard body (in the draft a later main migration redefined
  `game_item_instance_change_proven` and dropped the early-return); a unit test scans
  `migrations/` to prove it. Role split (below).
- **Q2 (CP option A):** the epoch keeps the content-scope map revision; the retirement commit
  does not compare it with the runtime guard's map revision (the guard carries the bundle
  digest — comparing them refused every production commit). One namespace (option B) is a
  follow-up.
- **ACCEPT-BATCH item 9:** no periodic Ground cleanup between resets; Ground root capacity
  64 ("not enough room") is admission-only and separate from `HIDDEN_REACH`'s 64-bit base mask.
- **CP decision (a):** boot run releases crash-left sessions of a retired holder (below).

## Reference behaviour from the draft (re-implement, do not copy)

- **Schema:** World epoch history + active pointer; reset job (RETIRING/ACTIVATED, World,
  epoch N, expected active identity, target digest, nullable all-or-none target
  map/server/client/frame/calendar columns) and its Channel cohort; reservations widened
  in the existing decay-retire tables (SOCIAL_MAP §1.7; receipt uniqueness across both
  causes, no parallel receipt table); additive WorldReset cause and Ground-container-entry
  location grammar (§1.11) with existing CorpseDecay compatibility. The revision of a
  RETIRING job advances only via a SECURITY DEFINER trigger on each `WORLD_RESET` receipt;
  runtime/control direct updates refused.
- **Capabilities (Codex P1s):** two disjoint NOLOGIN NOINHERIT roles —
  `oteryn_game_world_reset` (EXECUTE `game_world_reset_activate`, node login) and
  `oteryn_game_world_reset_publisher` (EXECUTE `game_world_reset_publish`, ops login, plus
  read-only `SELECT ON _sqlx_migrations` for ready-root). Migration refuses a pre-existing
  role with login/privileged attributes, memberships or direct ACLs. Activation refuses
  (42501) if the session holds the publisher capability; publication refuses if a Game group
  holds it or the login holds activation. Runtime/control keep SELECT only.
- **Publication** `game_world_reset_publish` (sole RETIRING publisher): House preflight of the
  proposed target under the admission fence and SQL recheck; takes `SHARE ROW EXCLUSIVE` on
  the job table (waits out and excludes ordinary content activations' `SHARE` fence), then
  refuses unless the latest activation of every World Channel issues the active epoch's
  server/client/frame digests; refuses a World with no Channel or more than
  `WORLD_RESET_CHANNELS_MAX` (128 = platform `MAX_CHANNELS_PER_WORLD`). Holds admission
  relations and the reconnect-session table EXCLUSIVE so nothing straddles its commit.
- **Admission closure triggers (OTR03 while RETIRING):** new reconnect sessions/attempts
  and House entries (`game_world_reset_admission_closed`); a reconnect session entering
  state 2 (`game_world_reset_readmission_closed`); a Channel assignment first inserted while
  RETIRING (`game_world_reset_cohort_closed`, takes `SHARE` on the job table; remint of an
  existing scope stays open); content activation (`game_content_record_activation`).
- **Running node:** watches every 5 s for a RETIRING job of its World, requests
  `GameplayDrain`: listener stops accepting, entry work cancelled, each session loses
  control immediately and is released through its grace lifecycle; then readiness is
  withdrawn and the node exits `BOOT_READINESS_FAILED` (retryable, exit 17).
- **Boot:** `recover_world_reset` runs before content activation or any Channel runtime;
  while the census says `NotCurrent` it waits (bounded by the assignment wait); first calls
  `release_retired_holder_sessions` (terminal release only for sessions whose holder
  registration is no longer current while the proven incarnation holds a newer generation;
  state 1 after its grace expires, state 2 as abandoned; never a live holder's session).
  `run_world_reset` takes a fresh Spell lane per drained Channel, resumes any frozen
  candidate, retires every Ground item one transaction each, re-censuses, activates N+1
  atomically (unknown commits retried with the same identity), logs `elapsed_ms` against
  `MAP01-RESET-RETIRE-MS` (600,000 ms; needs real-data measurement) and raises
  `world_reset_alarm` over budget. Returns the map reset epoch from the active pointer (none
  = 0); after activation it must equal N+1. Boot passes it via
  `GameplayDrain::at_reset_epoch` into the domain 17 map view source (pickup handles and
  replacement baselines are stale across epochs; doors resolve by digest/placement/revision).
  Target catalogue from `OTERYN_WORLD_RESET_TARGET_MANIFEST` (qualified before recovery; a
  refused manifest refuses readiness), else the active room.
- **Census** `census_retiring_world_reset(tx, node, world)`: job epoch/target/revision under
  row lock, current incarnation, all-Channel replacement/closed readiness/session predicate,
  actual Ground roots and corpse entries; drain binding reads at most
  `WORLD_RESET_CHANNELS_MAX + 1` assignments (`CapacityExceeded` above). Returned data is
  not authority beyond the transaction. No request supplies epoch, generation or Channel list.
- **Retirement order (Codex P1):** every content of a Ground container (corpse-entry family
  only) retires in ordinal order before the container; a nested entry (0128
  `game_item_nested_entries`) or a character container entry under a Ground root is
  `InvalidStoredState` and the root is not attempted. Each item conserves full quantity
  into RETIRED/0 with one location removed and same-transaction receipt/outbox/audit.
- **Activation:** lock reset record `FOR UPDATE`, then `LOCK TABLE
  game_item_house_interior_locations IN SHARE MODE`, recheck every live HouseInterior
  `(house_key, position)` belongs to the same House in the target (via
  `QualifiedHouseTiles`; a numeric tile marker is not proof), then write digest/N+1/
  ACTIVATED with the per-Channel issuance in one commit. Recheck failure leaves RETIRING
  (no rollback of retired items). NewEpoch and Historical are distinct dispositions;
  historical replay grants no authority. The retirement commit requires the typed runtime
  guard, not ready, at the local generation.
- **Operator command** (`oteryn-game-ops world-reset`): `initialize --world --calendar-revision`
  (control login) and `publish --world --epoch --calendar-revision` (separate
  `[world_reset.database]` login, must differ). Bundle identity only from qualified content
  (`OTERYN_NATIVE_GAMEPLAY_MANIFEST`, target manifest); retirement binding = domain-separated
  SHA-256 of the target, so the identical re-run is `replayed=true` and another cause is
  rejected. Unit tests pin which login each command connects; malformed commands connect none.
- **Spell lane:** the reset phase holds a `SpellLanePermit` and deletes corpse entries, so it
  joins the reviewed key-33 inventory.
- **Remaining fail-closed seams at the draft:** bundle-World House catalogue Unknown (reset
  refuses); one Channel per node (multi-Channel World waits `NotCurrent`); kill-reward and
  actor settlement do not run for released crash-left sessions; a state-1 session without a
  loss epoch waits.
- **PG cases (draft names):** crashed-reset resume retiring Ground and keeping Houses
  (`reset_run_resumes_a_crashed_reset_retires_ground_keeps_houses_and_activates`; fixture
  5 items in 678 ms); three retired-holder release cases; operator e2e
  `reset_operator_command_publishes_retiring_and_the_boot_run_remints_under_n_plus_one`
  (initialize → publish/replay/conflict → boot run → source Item reminted under epoch 1, second
  boot replays; prepared reconnect refused while RETIRING; admission open/closed/open); D1257
  calendar cases (09:59/10:00 Berlin, DST, unsupported calendar, no initialization); role
  denial matrix. PG cases skip when `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent (repo
  convention) and must run in the protected PostgreSQL lanes. Fixture gotchas: record the
  predecessor Channel activation before publication; remint through the node login's own Root.

### Required negative matrix (kept from the draft)

| Boundary | Must prove |
|---|---|
| Preflight/House | Removed/re-keyed House, removed/reassigned tile, unknown membership, wrong World/frame/content, changed catalogue with property rows refuse before any mutation; HouseInterior/provenance survive; Ground on unowned house tiles retires. |
| World/generation | Two Channels + another World; fresh generation per Channel; old MINT/TRANSFER/Ground/decay commits fail; stale incarnation, wrong Channel/epoch/target/phase refuse. |
| Item/census | Ordinary, map-materialized and source-created Ground; corpse with 0/max contents; already-retired; moved-away items untouched; entries before parent. |
| Cross-cause/replay | Old CorpseDecay reservation superseded; no duplicate receipts; same reset/item replay returns the original outcome; conflicting binding refuses. |
| SQL/audit | Direct DELETE/UPDATE without same-xact receipt/outbox, stale proof, forged location fail deferred guards; legacy events stay valid. |
| Crash/unknown | Crash after each phase and between child/root retirements resumes; lost COMMIT replies reconcile with no duplicate item, receipt, audit or epoch increment. |
| Step-4 concurrency | Concurrent HouseInterior writes wait under the record-then-SHARE locks; recheck mismatch leaves RETIRING. |
| Activation/bounds | Target/sequence mismatch and epoch overflow refuse; first legacy cutover and same-digest reset; no mixed old/new Channel readiness; worst-case audit envelope measured. |

## Target layout (ADR-0022)

- Reset owner/census/activation: `crates/gameplay/durability/src/durability/world_reset.rs`
  (new), run `world_reset_run.rs` (new), command `world_reset_command.rs` (new), tests in
  child files (new); module lines in `durability/mod.rs` (wiring-only).
- Retire arms: `crates/gameplay/durability/src/durability/item_decay_retire.rs` (1,406 lines;
  put the WorldReset admission in `item_decay_retire/world_reset_cause.rs` (new)) and
  `item_decay_retire_audit.rs`.
- Session release: `crates/gameplay/durability/src/durability/fresh_admission.rs` (3,296
  lines: call-site only; logic in a new sibling file).
- Epoch-keyed receipts: `crates/gameplay/durability/src/durability/native_map_items.rs`,
  `crates/gameplay/spell/src/spell/world_execution.rs` (1,170 lines: lookup change only).
- Map consumers: `crates/gameplay/dispatch/src/gameplay_transport/world_map.rs`, `map_door.rs`,
  `loop_move.rs`; `GameplayDrain` in a new `gameplay_transport/gameplay_drain.rs` (new) with
  `gameplay_transport/mod.rs` (8,167 lines) wiring-only; overlay
  `crates/gameplay/world/src/map/overlay.rs`.
- House membership: `crates/gameplay/content/src/content/project/native_house_tiles.rs`,
  `crates/gameplay/durability/src/durability/house_interior.rs`.
- Node: `apps/game-server/src/node/serve.rs` (3,769 lines: boot/watch hunks only),
  `apps/game-server/src/node/config.rs` (`[world_reset]`), `apps/game-server/src/bin/oteryn-game-ops.rs`.
- Pins/tests: `apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs` (row only),
  `apps/game-server/tests/world_reset_postgres.rs` (new),
  `apps/game-server/tests/support/world_reset_postgres_cases.rs` (new).
- Migrations: next free number at allocation time (only one migration PR in the queue). The
  draft's 0131/0132 collide with reserved BAGS-USE-1/TIMED-RT-1c and must not be used.
- Registry: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` `MAP01-RESET-RETIRE-MS` row.

## Acceptance criteria

- [ ] Reset retires exactly the Ground overlay items the packet names; HouseInterior untouched.
- [ ] Idempotent and crash-safe: restart mid-reset resumes; no old bundle over partial Ground.
- [ ] Epoch N+1 activation atomic with per-Channel content issuance; map views, pickup and source receipts bound to the active epoch.
- [ ] Publisher/activator capability split enforced in SQL; runtime and control denied.
- [ ] Retire duration logged against MAP01-RESET-RETIRE-MS (registry row added).
- [ ] Negative matrix above covered by real PG cases in the protected lanes.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`item_decay_retire.rs`, `fresh_admission.rs`, `world_execution.rs`, `gameplay_transport/mod.rs`, `serve.rs`, `oteryn-game-ops.rs`).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-durability world_reset
cargo test -p oteryn-gameplay-durability corpse_decay
cargo test -p oteryn-game-server --bin oteryn-game-ops world_reset
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test world_reset_postgres
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres world_reset
python3 tools/agents/validate_governance.py
```

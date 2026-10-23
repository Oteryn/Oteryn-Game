# MAP-OVERLAY-1c — World reset and overlay retire (consolidated task card)

```yaml
task_id: MAP-OVERLAY-1c
title: "MAP-OVERLAY-1c: World reset (RETIRING -> drain -> Ground retirement -> atomic epoch N+1 ACTIVATED) and overlay retire"
mode: IMPLEMENT
status: waiting   # MAP-OVERLAY-1c slice merged (#2327); remaining slices per card
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MAP-OVERLAY-1c; §7 map overlay chain"
brief: "docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (§1.5–1.8, §1.11, §2.7); ADR-0021 §4.7; HOUSE-CUSTODY0 §3.6"
class: "hard, persistence, authority and security review"
base_branch: main
branch: w2/map-overlay-1c
pr: null
needs_migration: true   # 0148_world_reset.sql; owner allocation in run 1c-e supersedes earlier provisional slots
needs_registry: true    # RESOURCE_LIMITS_REGISTRY MAP01-RESET-RETIRE-MS row; GAME_EVENT_FOUNDATION_REGISTRY audit grammar if widened
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Map pickup still uses epoch 0 pending the reset owner; no MAP-OVERLAY-1c activation/drain owner is retained. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs`.

Remaining: World RETIRING/drain/Ground retirement and atomic epoch activation, source fencing and measured reset qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2167, branch skeleton/map-overlay-1c

## CI regression fix (2026-10-23)

Local repair on `port/map-overlay-1c` is in AUTHORING; no commit or publication.
The two reported `AuthorityRejected` failures come from fixture epochs, not a
missing-state rejection. Migration 0148's `game_world_reset_map_epoch_current`
already admits epoch 0 when `game_world_reset_active` is absent and refuses every
fresh map MINT during RETIRING. The Ground-limit fixtures requested epoch 1 without
an active pointer; the cross-epoch MINT case advanced its request to epoch 1 while
leaving the World at epoch 0. These formerly arbitrary fixture epochs deliberately
became invalid under D1451 and ADR-0021 §4.7.

Use epoch 0 in both Ground-limit map-MINT fixtures. In the cross-epoch case, seed
the immutable epoch-one history and active pointer atomically before requesting
epoch 1, and use that current epoch for the second Channel. This is map-consumer
qualification, not a substitute for the existing activation-writer tests. Keep
the production admission predicate and receipt triggers intact. No assertion is
removed or relaxed.

The small child module `tests/support/map_item_mint_world_reset_cases.rs`, included
by the existing shared MINT cases, exercises a real MINT into Ground with no reset
rows, rejects a caller-selected future epoch without writes, and rejects both a
fresh MINT and a pre-reset reservation's commit while RETIRING. It changes only
reset state for the negative fence cases, retaining the session and assignment.

Validation uses `/home/mole/oteryn-cache/codex-bin/cargo +1.99.1`: bare `cargo`
resolves to rustup and has no PG URL, so its initial no-database run is not PG
evidence. The queued wrapper reproduced both reported failures on PostgreSQL
18.6 (`180006`), then the exact two-filter reproducer passed **2/2** (10.97 s).
The new reset fence case plus the sibling Ground readiness case passed **2/2**
(18.48 s). Durability's `--lib world_reset` passed **4/4**; real-dispatch
`--lib world_reset_map_pickup` passed **2/2** (28.86 s). Rustfmt check and
clippy for durability and game-server, all targets with `-D warnings`, passed.
Governance validation and its **124** Python unit tests passed.

Exact focused commands (with the queued wrapper in place of `cargo`):

```sh
cargo +1.99.1 test --locked -p oteryn-game-server --test character_authority_postgres -- map_mint_writer_counts_actual_roots map_item_mint_takes_an_entry_into_ground --nocapture
cargo +1.99.1 test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- world_reset_map_mint_admits genuine_not_ready_publication_refuses_fresh_mint_corpse_and_map --nocapture --test-threads=1
cargo +1.99.1 test --locked -p oteryn-gameplay-durability --lib world_reset --quiet
cargo +1.99.1 test --locked -p oteryn-gameplay-dispatch --lib world_reset_map_pickup --quiet -- --nocapture --test-threads=1
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy --locked -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings
```

Unfinished: the full unfiltered `character_authority_postgres` target is CI-only
under the owner's 2026-10-22 shared-PC PG-scope rule, which overrides the task's
validation list. No implementation work remains for this CI repair. Whole-diff
self-review against the owner's `CODEX-CHECKLIST.md` found no remaining hit; no
public API, enum variant, struct field or trait method was added.

## Current scope — integrator decision 2026-10-22 (run 1c-f)

Deliver 1c as a publishable, fail-closed slice: explicit calendar initialization,
Ground retirement, atomic epoch/content activation and final boot composition.
Production defaults remain closed. HOUSE-1a is not on main: only qualified,
explicitly empty House catalogues are admitted, and a reset touching a House is
refused. Capability/operator-command qualification is held behind the owner-only
SQL seam. All other open implementation/qualification work is in **MAP-OVERLAY-1d
follow-up** below. This scope and the final run checkpoint supersede every older
"unfinished", "next" and acceptance list in the retained history.

## Goal (original programme scope; remainder routed to 1d)

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

### Original negative matrix (remaining qualification routed to 1d)

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
  reset cases mounted once in dispatch `--lib` and the existing
  `character_authority_postgres` target; no new PostgreSQL target.
- Migrations: next free number at allocation time (only one migration PR in the queue). The
  draft's 0131/0132 collide with reserved BAGS-USE-1/TIMED-RT-1c and must not be used.
- Registry: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` `MAP01-RESET-RETIRE-MS` row.

## Acceptance criteria (scoped 1c slice)

- [x] Sealed planned drain, fresh-generation census and resumable one-item Ground retirement use the existing owners.
- [x] Explicit epoch-zero calendar initializer replays identical facts and uses the database clock; installation seeds nothing.
- [x] Epoch N+1, calendar month, ACTIVATED and all Channel issuances commit atomically; failure/retry/replay are qualified on PostgreSQL.
- [x] Boot completes recovery before content/runtime activation, uses the active epoch and refuses an unqualified target.
- [x] House/Unknown metadata refuses before retirement; the final transaction locks and rechecks House custody. Full bundle House metadata remains Unknown.
- [x] Runtime/control have no EXECUTE on initializer/publisher/activator; no reset capability or membership is adopted or installed by this slice.
- [x] Map view, LOOK, pickup and native source receipts use the active epoch; reset/remint runs through composed dispatch.
- [x] Recovery elapsed time and the existing limit/alarm are logged; full duration qualification is a 1d follow-up.
- [x] One mechanic per module, hub wiring only, single test mounts and recorded reasons for existing large extractions/migration.
- [x] Required local clippy, no-route library suites and filtered PostgreSQL checks passed; exact commands and counts are below.

## Validation

The authoritative local results and commands are in **Run 1c-f** below. Historical
planned command lists are superseded by the owner's filtered-PG rule. Repository
CI, independent frozen-head review, publication, task-record archival in the
completing authoring commit and Merge Queue remain control-plane work; this worker
has no PR/frozen candidate and leaves every change uncommitted.

## Worktree prerequisite check (2026-10-22)

Base: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`; exclusive local
authoring branch: `w2/map-overlay-1c`. No candidate SHA or PR was created.

**BLOCKER — prerequisite contract mismatch.** The dependency table and
D1134/D1135 require reuse of the sealed close/logout/drain composition attributed
to ADMIT-CAP-1 / #2187. Live PR metadata confirms #2187 merged as
`94288ef41a3dece278b30eedf82b6c2d0ea7bc09`, but its implementation gates fresh
admission and V2 recovery by required capabilities. It supplies no reset drain.
This is a missing prerequisite capability, not an assertion that #2187 is unmerged.

The current implementation was checked across `apps/`, `crates/` and `tools/`:

- `gameplay_transport/mod.rs::serve_accepted` explicitly treats shutdown as no
  evidence of lost playable control, so shutdown does not start session release.
- `control_loss_lifecycle` cancels its grace-release work on shutdown.
- `gameplay_transport/input_gate.rs::InputGate::quiesce` drains admission and
  command permits; it does not perform control loss or terminal session release.
- No existing `GameplayDrain` or sealed reset-drain composition was found.

The new `GameplayDrain` module is explicitly allocated to this lane. The unresolved
point is which existing, accepted drain composition it must wrap, as required by
D1134/D1135. Do not substitute ordinary shutdown for that composition. The
coordinator must identify its actual merged prerequisite or correct the card to
state that this lane implements the missing composition using the existing
control-loss/grace-release owners. Resume implementation after that dependency
contract is reconciled; no runtime, schema, capability or registry changes were made.

Planned module layout remains the target layout above: durable reset owner,
census/activation children, run and command modules; a WorldReset retirement-cause
child; a retired-holder session-release sibling; a dispatch `gameplay_drain.rs`;
dedicated child tests and the new `world_reset_postgres` suite. Shared slots receive
wiring only. Migration `0132` is provisionally allocated by the current lane packet,
superseding the historical collision note above; no migration was created.

Reuse investigation: existing `InputGate`, `control_loss_lifecycle` and the
`FreshAdmissionAuthority` control-loss/release methods were inspected. No helper,
fixture, harness or test registration was added or copied. No enum variants,
struct fields, trait methods or public functions were added.

Validation: documentation-only prerequisite checkpoint; no Rust crate was touched.
The cargo filters in Validation remain required and unrun, and all implementation
acceptance criteria remain open. Governance validation passed (22 policy documents,
9 lanes); all 124 `tools/agents/tests` unit tests passed. Whole-diff review against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and `git diff --check` passed; no added
Rust symbols require a constructor/match/impl sweep. This task record is the only
changed file (302 lines); no file grew by more than 200 lines.

## Integrator decision 2026-10-22: prerequisite allocated to this lane

The continuation packet resolves the earlier ownership blocker: this lane must
implement the sealed reset drain using `InputGate::quiesce`,
`control_loss_lifecycle` and `FreshAdmissionAuthority` control-loss/release owners,
then implement the remaining card. Migration `0132` remains provisionally allocated.
The prerequisite audit above is retained as historical evidence, not a request to
identify a different merged composition. The prerequisite has **not** been
implemented; the requested implemented-modules record cannot yet be asserted.

Planned modules before coding: dispatch `gameplay_drain.rs` owns the sealed reset
signal and transport drain; a dispatch serving child composes that drain with the
existing input gate and lifecycle; a game-server `serve/world_reset.rs` child owns
the RETIRING watch and boot orchestration. The durable reset, retirement-cause,
retired-holder release, run, command and child-test modules remain as planned in
Target layout above. Shared parents receive wiring only.

### Pending owner question: planned reset versus unexpected-loss authority

**BLOCKER at the prerequisite authority boundary.** The continuation authorizes
implementing the composition, but the current owning contract and adapters admit
only an unexpected loss, while the packet requires a planned reset to immediately
lose control for every live session, including a healthy controller:

- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`
  sections 3 and 5 restrict RECONNECTABLE and a new ControlLossEpoch to
  authoritative **unexpected** playable-control loss. Section 5 excludes socket
  close alone and process restart alone; section 6 starts grace at that epoch.
- `crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs`
  `ControlLossObservationV1::validate` requires
  `ControlLossCauseV1::AuthoritativeUnexpectedLoss`. There is no reset cause.
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
  `write_fresh_loss` rejects every other cause; its reader admits only cause tag 1
  and reconstructs `AuthoritativeUnexpectedLoss`.
- Dispatch `commit_control_loss` always constructs that unexpected-loss cause.
  Calling it with zero wait after intentionally closing a healthy transport would
  classify planned shutdown of control as unexpected loss without the contract's
  liveness proof. Ordinary shutdown explicitly avoids this classification today.

Owner decision required: **Does a sealed drain bound to a durable RETIRING World
reset explicitly supersede FND-04B sections 3/5 and authorize a new loss epoch and
the existing grace lifecycle for a currently healthy session?** If yes, authorize
that narrow exception and its explicit reset cause through Foundation and the
durable loss codec. No ordinary shutdown, transport close, client input or caller
boolean should acquire that authority. If no, the packet needs the accepted
planned-reset terminal-release policy instead of the requested loss/grace route.
The card and ADR-0021 define the reset order, but do not specify this exception to
unexpected-loss authority. This is a session/recovery authority choice rather
than a module-layout or helper implementation choice.

Per the continuation's explicit stop rule for an unanswered security/authority
policy choice, no runtime, schema, capability, registry or contract mutation was
made. No new helpers, fixtures, harnesses, symbols or test registrations were
introduced. Reuse findings remain `InputGate::quiesce`,
`control_loss_lifecycle`, `FreshAdmissionAuthority::lose_control`,
`expire_control_loss`, `release_abandoned` and `reconcile_terminal`; these owners
were traced, not copied or reimplemented.


## Owner decision 2026-10-22

YES. A sealed, RETIRING-bound planned World reset is a second authorized cause that may open a loss epoch and grace release for a healthy session. Amend FND-04B section 5 (docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md) with a dated owner decision: the cause is a distinct, explicitly typed variant (planned World reset, bound to the sealed RETIRING record), never inferred from an unexpected loss, and the foundation authorization plus durable encoding admit it only with that sealed binding. Unexpected-loss behaviour stays unchanged; ordinary shutdown semantics are still not reused.

This supersedes the pending owner question above. The original prerequisite audit
is retained as historical evidence. Implementation follows the planned module
layout recorded above, with a Foundation planned-reset binding child and a durable
loss-cause codec child, rather than extending the large owning files with logic.

## Integrator decision 2026-10-22: prerequisite implemented in this lane

The planned-loss prerequisite source now uses the existing owners:
- Foundation `planned_world_reset.rs` admits a distinct World/epoch/target/binding cause.
- Durability `fresh_admission/control_loss_codec.rs` is the extracted single loss codec;
  `world_reset.rs` seals a current database row and rechecks it under the loss transaction.
- Dispatch `control_loss.rs`, `listener.rs`, `serving.rs` extract the existing owners;
  `gameplay_drain.rs` carries the sealed signal through input quiescence and grace release.
- Node `serve/world_reset.rs` watches every five seconds and retains retryable exit 17.
  Boot refuses RETIRING before content activation; the actual reset recovery runner is pending.
- Migration `0148_world_reset.sql` installs the initial record schema and admission fences.
  It is unfinished: no initializer/publisher/activator capability or Ground writer is exposed.
- Tests are single child registrations in Foundation, node and the existing dispatch PG fixture.
  The latter covers a healthy session, stale phase/target, codec reload and real grace release.
  The PR PostgreSQL `durability_postgres` / `rest` leg invokes the exact dispatch filter once.

### Current validation, reuse and remaining work

Status remains `in_progress`; no frozen SHA, PR, publication or Git mutation exists.
Foundation (333), durability (384) and dispatch (903 passed, 14 pre-existing ignored)
unit suites passed; server library (307 passed, 3 pre-existing ignored) passed.
All four touched packages passed clippy `--locked --all-targets -- -D warnings`.
Rustfmt, governance (22 policies, 9 lanes), 124 agent tests and repository policy passed.
The PG case compiled but its existing harness skipped it: this session has no
`OTERYN_TEST_POSTGRES_ADMIN_URL`, and `cargo` resolves directly to rustup rather than
D1466's queued wrapper. The pending environment question asks only for the wrapper
invocation, never credentials. No PostgreSQL success is claimed.

Exact focused cargo filters: Foundation `planned_world_reset` and `owning_loss`;
dispatch `planned_world_reset`, `shutdown_completes_admission_in_flight_and_cancels_entry_work`,
`resource_loss_lifecycle_holds_no_connection_slot`, `an_unknown_grace_expiry_is_retried_until_final`;
server `reset_drain_exit_is_retryable_and_retains_the_observed_reset`.

Reuse: existing Foundation owning-loss test `owner`; existing loss/protection/budget codec
(moved, old definitions deleted); `InputGate::quiesce`; the single existing listener and
`control_loss_lifecycle`; all `FreshAdmissionAuthority` loss/release/reconciliation paths;
`Harness::create_with_admission_seed`, `prepare_current_session`, `Dependencies`, `track`,
`run`, `reset`, `debug` from the monk/character-revision fixture; native node boot retries.
No fixture/harness bodies were copied and no test-only wrapper or duplicate mount was added.
Files growing >200 lines: dispatch `control_loss.rs` (246) and `listener.rs` (279),
existing transport owners extracted from the shared slot; durability
`fresh_admission/control_loss_codec.rs` (316), the one durable historical codec extracted
from its large owner. Every other code file grows by fewer than 200 lines.

The complete tracked/untracked diff was re-read against the owner's checklist; new cause,
seal/watch fields and the changed `lose_control` trait signature were searched throughout
`apps/`, `crates/`, `tools/`. Every new public function has a production caller. PG dispatch
qualification remains pending, so the prerequisite is implemented but not qualified.
The original card remains unfinished: publisher/activator split and operator command,
all-Channel census/fresh generations, crash-left release, Ground retirement/audit,
House preflight/recheck, atomic epoch/content activation, epoch-keyed source receipts/map
consumers, resume runner, duration registry/measurement and full negative PG/MQ matrix.
Next: restore the advertised D1466 wrapper, run the prerequisite PG case without skipping,
then implement and qualify the remaining original scope. No new owner policy question exists.


## Continuation 2026-10-22: PostgreSQL qualification, Ground retirement and drain

Status: **in_progress**, branch `w2/map-overlay-1c`, HEAD `459965d4e`.
No frozen SHA, PR, publication, Git mutation or external review trigger exists.
The earlier missing-wrapper/PG-skip checkpoint is superseded by the actual runs below.
The owner decision above remains verbatim and the prerequisite audit remains retained.

### Layout and implemented modules

The pre-coding layout remains one mechanic per child in the owning layer:
- Durability `item_decay_retire/retirement_workflow.rs` and `retirement_event.rs`
  extract the existing frozen one-item workflow/event builder; `world_reset_cause.rs` and its `ground_admission.rs` child extend that same owner, receipt and outbox.
- Durability `item_decay_retire_audit/world_reset.rs` owns the additive typed reset
  cause and Ground-container-entry grammar; its adjacent child tests mount once.
- Durability `world_reset/{census,ground_step,ground_retirement}.rs` owns the complete
  bounded World census, deterministic child-before-root selection and resumable passes. The census checks the active pointer, exact job binding, current incarnation,
  every Channel's newer ordinary generation, decoded closed Runtime guard, and no nonterminal World sessions. It reads 129 assignments to distinguish CapacityExceeded.
- Durability `fresh_admission/{terminal_release,retired_holder_release}.rs` extracts
  the existing release writer and adds only its sealed-reset retired-holder proof. The deciding release transaction rechecks the immutable old assignment receipt,
  ended old registration, proven newer current holder and unchanged session. ACTIVE crash-left sessions release without inventing a loss; RECONNECTABLE sessions keep
  the original database-clock grace. A state-1 session without a loss epoch waits.
- Node `serve/world_reset.rs` now waits within the assignment budget, cancels through
  the existing boot composition, and invokes recovery before content/runtime boot. After Ground retirement it still refuses readiness with activation required.
  A fresh Spell lane is retained per drained Channel for the whole retirement run.
- The lane's unmerged migration 0132 widens the existing retirement tables, stamps
  current-xact Ground-removal provenance, advances job revision from receipts, and extends the installed latest guard bodies instead of copying old definitions.
  Four guards retain SECURITY DEFINER, pinned in PG. Channel replacement via INSERT/ON CONFLICT stays open during RETIRING; expansion of the cohort stays closed.
  World-filtered assignment/session census queries have supporting partial indexes.
- Proto and event registry record the additive cause/location fields. Legacy corpse
  bytes and deadline behavior remain admitted. New tests remain beside their owners.

### Qualification and review evidence

All cargo invocations used `/home/mole/oteryn-cache/codex-bin/cargo`, with `--locked`.
The wrapper supplies PostgreSQL internally; no shell credential was read or printed.
- Prerequisite dispatch case passed with a real created database
  `crs_planned_reset_loss_1792679934492991800`. The corrected `world_reset` filter runs both real dispatch PG cases (2 passed, 77.10 s); the initial file-name filter
  matched zero and is not evidence. The existing debug owner needs a 64 MiB test scenario stack, as neighboring kill/death tests do; default tests now set it in
  their own thread and heap-pin the production lifecycle.
- Ground PG case passed (32.15 s), database `it_reset_ground_1792680261406247495`,
  PostgreSQL 170006. It uses real MINT, ordinary assignment replacement and retirement; verifies three conserved retirements, child-before-parent audit, missing-outbox
  deferred rollback, durable resume, replay uniqueness and receipt-driven revision. Its raw Item-fixture terminal row is not used as an oracle for session release.
- Retired-holder PG case passed (116.94 s): live-holder refusal, healthy crash-left
  release, open-grace refusal, expired-grace release and exact terminal replay, using the actual composed dispatch admission/loss and existing claim release.
- Existing corpse-decay PG module: 5 passed, five created PostgreSQL 17.6 databases
  (latest qualified run 47.99 s). Existing owning fresh-loss PG case: 1 passed, 97.37 s.
- Foundation: 333 unit tests plus doc tests passed. Server library: 307 passed,
  3 pre-existing ignored. Durability: all 388 tests plus doc tests passed serially in 160.74 s. A parallel run hit two Database(Pool) failures; no tests were changed
  or skipped for the serial run. The extracted reservation insert was moved with its Type2 opener to satisfy the unchanged grandfather/fence inventory assertion.
- Maximum admitted reset audit, including key metadata: payload 4,049 bytes,
  envelope 4,362 bytes (registered maxima 7,936 / 9,216); all four root/entry/key combinations use the same existing bounded codec and gates.
- Final source-batch checks passed: four-package clippy all targets (-D warnings), fmt,
  focused PG, 8 key-33 pins, 5 lifecycle cases and shutdown. Full diff/checklist and global
  constructors/impls reviewed; governance (22 policies, 9 lanes), 124 agent tests and repository policy passed.

Exact focused test commands (prefix each with the wrapper path above):
```sh
cargo test --locked -p oteryn-gameplay-dispatch world_reset -- --nocapture
cargo test --locked -p oteryn-gameplay-durability world_reset -- --nocapture
cargo test --locked -p oteryn-gameplay-durability item_decay_retire --quiet
cargo test --locked -p oteryn-game-server --test character_authority_postgres world_reset_ground -- --nocapture
cargo test --locked -p oteryn-game-server --test character_authority_postgres corpse_decay -- --nocapture
cargo test --locked -p oteryn-game-server --test durability_postgres owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority -- --nocapture
```
Superseded in run 1c-e: the protected workflow is restored exactly to `origin/main`.
Its existing PostgreSQL library step already runs dispatch/durability/game-server
`--lib`; every reset PG case is mounted there or in the existing Character-authority target.

### Reuse, size and unchanged authority boundaries

Reused: the existing freeze/charge/commit/reconcile retirement workflow and
`apply_retire`; the original event builder; `AdmissionGuardStore::load_locked`;
`prove_current_incarnation`; ordinary `RuntimeScopeAssignmentWriter`; existing
`FreshAdmissionStore` current-session, claim preparation, terminal release and
lifecycle reconciliation; `InputGate::quiesce` and `control_loss_lifecycle`;
`Harness::create_with_admission_seed`, `prepare_current_session`, `Dependencies`,
`track`, `run`, `reset`, `debug`; Item `Harness::mint`, corpse `mint_corpse` /
`put_loot`; and the existing private audit message builders.

The RETIRING-row fixture was moved once into `world_reset/retiring_fixture.rs`
and reused by both PG owners; no fixture body is duplicated. Test-only access to
private builders uses `item_decay_retire_audit/world_reset_fixture.rs`; private
node registration uses the one-line wrapper in
`tests/support/character_revision_node_registration.rs`. No duplicate test mount
or registration was introduced. Every new public runtime method has the node
production caller and real dispatch/PG coverage. All changed enum, field,
constructor and trait-implementation sites were searched in apps/crates/tools.

Files growing over 200 lines across the complete unmerged task:
- `item_decay_retire/retirement_workflow.rs`: the single existing one-item durable
  pipeline/reservation writer, extracted from its parent (L4), not a parallel implementation.
- `fresh_admission/control_loss_codec.rs`: the single historical durable codec
  extracted in the preceding WIP (L4).
- Dispatch `control_loss.rs` / `listener.rs`: existing transport/lifecycle owners
  extracted in the preceding WIP (L5).
- Migration `0148_world_reset.sql`: the task's one allocated additive schema unit,
  owning SQL constraints, trigger fences and receipt proofs in the existing migration owner.
- This task record: retained prerequisite audit/owner decision, validation and
  continuation state; no runtime logic belongs here. Other existing large code files shrink or receive only bounded wiring / call-site changes.

### Still unfinished and next action

This is **not** the full card's acceptance or a ready candidate. Pending: qualified
House preflight / step-4 recheck; caller-transaction content issuance and atomic
N+1 ACTIVATED writer; explicit epoch-zero calendar initializer; publisher/activator
capability split and operator command; epoch-keyed native source receipts and map
consumers; final boot activation orchestration; duration registry / real-data
measurement; and the remaining multi-World/Channel, House, cross-cause, stale-proof,
unknown-commit, concurrency and capacity PG matrix. No new owner-only policy question
has been found. Next in card order: House target qualification/preflight using the
existing native House-tile and HouseInterior owners, followed by atomic activation.

### Files changed in this continuation

- `.github/workflows/pr-postgres-harness.yml`
- `apps/game-server/migrations/0148_world_reset.sql`
- `apps/game-server/src/node/serve.rs`
- `apps/game-server/src/node/serve/world_reset.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/character_revision_node_registration.rs`
- `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/gameplay_drain.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/terminal_release.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_event.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_workflow.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause/ground_admission.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_fixture.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset.rs`
- `crates/gameplay/durability/src/durability/world_reset/census.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_step.rs`
- `crates/gameplay/durability/src/durability/world_reset/migration_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/retiring_fixture.rs`
- `docs/agents/tasks/MAP-OVERLAY-1c.md`
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`


## Run 1c-e (2026-10-22): publication repair and active-epoch consumers

Status: **in_progress; full MAP-OVERLAY-1c acceptance remains open**. Branch
`w2/map-overlay-1c`, HEAD `459965d4e`; no frozen SHA, PR, CI run, paid review,
commit, push or Git index mutation was created by this worker.

### Publishing repairs

- Restored `.github/workflows/pr-postgres-harness.yml` byte-for-byte from
  `git show origin/main:.github/workflows/pr-postgres-harness.yml` (SHA-256
  `5b811c0f6cdc5fae0793f20203559900397c24dafffee215070f0f1ac8547e6c`).
  There is no workflow delta against `origin/main`. No other protected-path edit
  exists. Earlier notes proposing a workflow filter change are superseded.
- Renamed the task migration to `apps/game-server/migrations/0148_world_reset.sql`
  and changed the latest-guard registry test from version 132 to 137. All earlier
  provisional migration-number allocations in this record are superseded by the
  owner's run 1c-e allocation; 0132 and 0133 belong to other mechanics.
- Kept every existing reset PostgreSQL assertion. No new `tests/*_postgres.rs`
  target exists. The shared RETIRING fixture now has one explicit path mount,
  in durability's test-support module, and both PG owners import that helper.

### Implemented in this batch

- `world_reset/active_epoch.rs` reads the actual World active pointer under the
  semantic deadline, preserving `None` as NotInitialized. Boot supplies the read
  epoch through `GameplayDrain::at_reset_epoch`; only legacy map boot maps absence
  to epoch zero. Calendar consumers must retain the missing-state distinction.
- Dispatch's `bundle_map_position.rs` owns the extracted domain-17 composition.
  Map snapshots/deltas, LOOK, pickup eligibility and Ground overlay reconstruction
  all use the booted epoch. The new epoch-one command-9 test caught and repaired
  the remaining epoch-zero `read_map_item_mint_placements` call. Door identities
  continue using digest/placement/revision, as the accepted contract specifies.
- Native source receipts use `(world, channel, reset_epoch, content_digest,
  placement_key)` uniqueness. Epoch zero retains the original intent bytes; later
  epochs bind the epoch in the intent and its existing audit. Replay only reads
  this epoch's receipt; the durable owner does not resurrect moved/retired items.
  The native Ground origin reader also filters source placements by active epoch.
- Fresh map MINT admission and both SQL receipt writers check active epoch and
  RETIRING. A narrow, pinned SECURITY DEFINER predicate holds a share lock on the
  active pointer, while runtime retains SELECT-only reset-table privileges.
  Historical terminal receipt replay does not reacquire mutation authority.
- Added `MAP01-RESET-RETIRE-MS = 600000` to the resource registry. Boot reports
  elapsed drain/Ground retirement time and `world_reset_alarm` on Ground completion,
  with `activation=pending`. This is phase evidence, not a full-reset measurement.

### PostgreSQL mounts (each test file mounted once)

| PG test file | Mount / existing CI route |
|---|---|
| `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset`; dispatch `--lib` |
| `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset_retired_holder`; dispatch `--lib` |
| `crates/gameplay/dispatch/src/gameplay_transport/world_reset_source_map_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset_source_map`; dispatch `--lib` |
| `crates/gameplay/dispatch/src/gameplay_transport/world_reset_map_pickup_postgres_tests.rs` | `loop_map_pickup_tests.rs::world_reset`; dispatch `--lib` |
| `crates/gameplay/durability/src/durability/world_reset/ground_retirement_postgres_tests.rs` | `apps/game-server/tests/character_authority_postgres.rs::world_reset_ground_postgres` |
| `apps/game-server/tests/support/corpse_decay_postgres_cases.rs` | existing `character_authority_postgres.rs::corpse_decay_postgres_cases` |
| Owning fresh-loss regression in `apps/game-server/tests/durability_postgres.rs` | existing registered `durability_postgres` target |

The restored workflow already runs dispatch/durability/game-server library tests
with PostgreSQL in its rest leg, and the two registered targets above in their
existing legs. The ordinary Rust library lane without PostgreSQL remains unchanged.

### Validation

All cargo commands used `/home/mole/oteryn-cache/codex-bin/cargo`, `--locked` for
clippy/tests, and PostgreSQL filters. No whole PostgreSQL suite ran.

- Four touched crates: clippy `--all-targets -- -D warnings` passed; final fmt
  `--all --check` passed. The existing vendor Tokio documentation warning is
  dependency output; all commands exited successfully.
- Dispatch `world_reset`: 5 passed before the final narrow privilege repair,
  including the unchanged planned-loss and retired-holder cases (230.45 s).
  Final `world_reset_map_pickup`: 2 passed (40.18 s), covering snapshot epoch,
  successful pickup/MINT/TRANSFER, LOOK, delta hiding, replay, and cross-epoch
  pickup/LOOK refusal with an unchanged durable footprint.
- Final `world_reset_source_map`: 2 passed (93.73 s). Each created a real PostgreSQL
  database and minted/replayed 57 qualified native placements. They check epoch-zero
  compatibility, epoch-one receipt/audit binding, stale-epoch SQL refusal, and the
  definer/EXECUTE/SELECT-only runtime privilege invariant.
- Ground retirement: 1 passed (4.28 s), database
  `it_reset_ground_1792691387470770619`, version 170006. The three conserved
  retirements, child-before-parent order, missing-outbox rollback, resume and
  receipt-driven revision assertions remain unchanged.
- Corpse decay: 5 passed (40.50 s), five real databases, each explicitly reporting
  PostgreSQL 170006. Owning fresh-loss regression: 1 passed (48.73 s).
- Unit filters: foundation `planned_world_reset` 1 passed; durability `world_reset`
  4 passed and `item_decay_retire` 15 passed (overlapping audit cases); server
  `reset_drain_exit_is_retryable_and_retains_the_observed_reset` 1 passed.
- Audit maximum remains 4,049 payload bytes / 4,362 envelope bytes, within the
  registered 7,936 / 9,216 limits. Whole-diff/checklist review and workspace-wide
  enum, field, constructor, trait implementation and caller searches completed.
  Both new public runtime APIs have node production callers and real command-9
  dispatch/PG coverage. Test-support fixture export is test-only.
- Governance validation and all 124 agent unit tests passed. `git diff --check`
  passed. No CI-only contention deferral exists in this run.

Exact focused commands (use the wrapper path above in place of `cargo`):
```sh
cargo test --locked -p oteryn-gameplay-dispatch --lib world_reset -- --nocapture --test-threads=1
cargo test --locked -p oteryn-gameplay-dispatch --lib world_reset_source_map -- --nocapture --test-threads=1
cargo test --locked -p oteryn-gameplay-dispatch --lib world_reset_map_pickup -- --nocapture
cargo test --locked -p oteryn-game-server --test character_authority_postgres world_reset_ground -- --nocapture
cargo test --locked -p oteryn-game-server --test character_authority_postgres corpse_decay -- --nocapture --test-threads=1
cargo test --locked -p oteryn-game-server --test durability_postgres owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority -- --nocapture
cargo test --locked -p oteryn-gameplay-durability --lib world_reset -- --nocapture
cargo test --locked -p oteryn-gameplay-durability --lib item_decay_retire --quiet
cargo test --locked -p oteryn-gameplay-foundation --lib planned_world_reset --quiet
cargo test --locked -p oteryn-game-server --lib reset_drain_exit_is_retryable_and_retains_the_observed_reset --quiet
```

### Unfinished / qualification blocker

This is not a ready candidate or complete reset implementation. House preflight
and the step-4 recheck, caller-TX issuance and atomic N+1 ACTIVATED writer,
explicit calendar initializer, publisher/activator capability split and operator
command, final boot activation, full-reset/real-data duration qualification, and
remaining multi-World/Channel, House, cross-cause, stale-proof, unknown-commit,
concurrency and capacity matrix remain open. Boot still refuses readiness with
`World reset activation required` after Ground retirement.

A trial strict capability-role adoption check refused the shared test cluster's
pre-existing authority. A bounded read-only inventory confirmed one node login
already belongs to `oteryn_game_world_reset` and five operator logins belong to
`oteryn_game_world_reset_publisher`; all six have INHERIT and SET capability.
These are real login memberships, not PostgreSQL's administrative creator grant.
No existing membership or credential was changed. The unqualified capability,
initializer, publisher and activation prototype was not retained. Qualification
of that slice needs an isolated PostgreSQL role fixture or control-plane-owned
reconciliation of stale test memberships; do not relax role adoption to accept
this shared-cluster state. The temporary inventory probe is not a product test.

House carrier/codec evidence was located without an owner question:
[D1245, accepted CP carrier correction](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226519383).
It identifies the native entry overlay's `house_tiles`, qualified canonical
placement digest, and full-bundle Unknown refusal. Further House implementation
must also retain the task's D1245a/D1246 and caller-TX invariants.

Next: obtain test role isolation for strict capability qualification, then finish
House target qualification and atomic activation using the accepted owners.
No publication, external review or production operation is authorized by this
checkpoint.

### Changed-file manifest (complete unmerged task, including retained earlier runs)

The workflow entry is the owner's restoration only, with zero delta from
`origin/main`. The migration entry is the final 0148 path; the provisional 0132
file has been removed. Files retained unchanged from the preceding run remain
listed so a reviewer can inspect the complete task.

- `.github/workflows/pr-postgres-harness.yml`
- `apps/game-server/migrations/0148_world_reset.sql`
- `apps/game-server/src/node/serve.rs`
- `apps/game-server/src/node/serve/world_reset.rs`
- `apps/game-server/src/node/serve/world_reset_tests.rs`
- `apps/game-server/src/qualification.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/character_revision_node_registration.rs`
- `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bundle_map_position.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/control_loss.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/gameplay_drain.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_look.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/listener.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/serving.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_reset_map_pickup_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_reset_source_map_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/core_modules.rs`
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/control_loss_codec.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/terminal_release.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_event.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_workflow.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause/ground_admission.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_fixture.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_tests.rs`
- `crates/gameplay/durability/src/durability/map_item_mint.rs`
- `crates/gameplay/durability/src/durability/native_map_items.rs`
- `crates/gameplay/durability/src/durability/spell_item_transaction.rs`
- `crates/gameplay/durability/src/durability/world_reset.rs`
- `crates/gameplay/durability/src/durability/world_reset/active_epoch.rs`
- `crates/gameplay/durability/src/durability/world_reset/census.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_step.rs`
- `crates/gameplay/durability/src/durability/world_reset/migration_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/retiring_fixture.rs`
- `crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs`
- `crates/gameplay/foundation/src/foundation/control_loss_durability_tests.rs`
- `crates/gameplay/foundation/src/foundation/planned_world_reset.rs`
- `crates/gameplay/foundation/src/foundation/planned_world_reset_tests.rs`
- `docs/agents/tasks/MAP-OVERLAY-1c.md`
- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`


## Run 1c-f (2026-10-22): locally qualified, publishable fail-closed slice

**Status: scoped implementation complete; waiting for CP publication, independent
review and MQ.** Branch `w2/map-overlay-1c`, existing WIP HEAD `459965d4e`.
No new commit, Git index mutation, push, frozen SHA, PR, CI run, paid review or
production operation was created. Prior completion blockers that belong to the
larger programme are now explicitly routed to 1d below.

### Final implementation

- `world_reset/activation.rs` composes the qualified preflight, existing retirement
  runner and one final semantic transaction. It rechecks the typed closed Runtime
  guards, current incarnation and complete fresh-generation cohort. The SQL writer
  locks the reset record, then House custody, assignments and Ground; it rechecks
  target/drain/empty Ground and atomically writes N+1, its calendar history, the
  active pointer, ACTIVATED and all per-Channel issuances. `NEW_EPOCH` and
  `HISTORICAL` remain distinct; historical replay creates no authority or issuance.
- Migration remains **0148_world_reset.sql**. Its explicit owner-only initializer
  records epoch zero from the database clock under `TIBIA_SERVER_SAVE_BERLIN_10`,
  never a seed or caller time. Publication checks the issued predecessor content,
  freezes the bounded cohort and preserves exact replay/conflict semantics.
- The installed content issuance CAS/replay body is extracted into a private,
  caller-transaction SQL core. The ordinary scoped issuer retains its grant check
  and reset fence. A fresh content cutover of an initialized World requires reset;
  exact historical replay is still decided by the existing CAS owner.
- `world_reset/content_preflight.rs` uses the existing qualified House-tile seam.
  Only `is_known_empty()` is admitted for both source and target; World and actual
  old/target map/server/client/frame identities must match locked history/job facts.
  House tiles, Unknown metadata, House custody and property rows refuse. No House
  implementation is introduced. Full bundle reset still refuses before Ground work.
- Node recovery now calls `finish_world_reset` and returns the completed epoch,
  then activates the actual current issuance with the qualified target manifest
  before runtime/readiness. A later boot selects that target at a positive epoch.
  Recovery logs elapsed time through activation against `MAP01-RESET-RETIRE-MS`;
  it makes no full-duration or production-scale qualification claim.
- The new dispatch PG case initializes/publishes, refuses a House target and
  Unknown source before retirement, waits for ordinary replacement, retires 57
  source items, injects an issuance failure after epoch/phase writes, proves atomic
  rollback while Ground retirement survives, retries/replays epoch 1 once and
  remints 57 source items through `ensure_source_map_initialized` at epoch 1.
  Berlin month boundaries are checked before/at 10:00 in summer and winter.
- Existing map-consumer fixtures now bind their active epoch to the actual bundle
  and issued content identities, replacing placeholder digest facts. The exact
  `SeamRooms` construction inventory includes the two reset test files; its bundle
  refusal assertions remain intact. No test was removed, weakened or newly ignored.

### Capability/operator qualification — blocked and closed by default

The fixed migration/harness has no per-database capability-role namespace hook.
Unique test memberships alone would not isolate adoption of the production-named
roles; qualifying that strict adoption needs a harness/migration-role substitution
seam. Per the integrator's fallback, no global role counts are asserted, no existing
cluster membership is changed and no capability qualification is claimed. The
unqualified capability/operator-command implementation is not installed. SQL
initializer, publisher, activator and issuance core are owner-only; runtime/control
EXECUTE denial is asserted. Enabling the production role split/operator route and
its isolated qualification is a blocked 1d follow-up, not a relaxation of adoption.

### Validation (PROVEN local evidence)

Every cargo invocation used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`;
clippy/tests used `--locked`. No whole PostgreSQL suite ran.

- All five touched crates passed `clippy --all-targets --quiet -- -D warnings`:
  `oteryn-gameplay-content`, `oteryn-gameplay-foundation`,
  `oteryn-gameplay-durability`, `oteryn-gameplay-dispatch`, `oteryn-game-server`.
  Dispatch was refreshed after the fixture/inventory repair. Final rustfmt passed.
- All five `test --lib --quiet` suites passed with PostgreSQL **unset in the test
  process**: content 339 passed/1 pre-existing ignored; foundation 333 passed;
  durability 388 passed; dispatch 909 passed/14 pre-existing ignored; server 307
  passed/3 pre-existing ignored. Since the wrapper always exports the PG route,
  the cargo test runner was explicitly
  `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='env -u OTERYN_TEST_POSTGRES_ADMIN_URL'`.
  This verifies the required no-route skip, separately from real PG qualification.
- Final dispatch `--lib world_reset`: **7 passed**, 297.82 s. Real created-database
  markers cover both pickup cases, planned loss, both retired-holder scenarios
  (`lost=false/true`, PostgreSQL 170006), both source epochs (57 placements each)
  and atomic activation/remint. Final activation database:
  `crs_reset_activation_1792701302500258351`, PostgreSQL **170006**, 57 retired and
  57 reminted. Every PG body ran; none of these results is a no-route skip.
- Ground retirement: **1 passed**, 4.55 s, database
  `it_reset_ground_1792700818977728835`, PostgreSQL **170006**.
- Corpse-decay/item-retire regression filter: **5 passed**, 20.46 s; each case
  printed its own created database and PostgreSQL **170006**.
- Owning fresh-loss regression: **1 passed**, 34.50 s with the wrapper's PG route;
  its owning SQL admission/loss assertions ran against the harness database.
- Initial failures were repaired in this batch: readiness bootstrap and assignment
  revision in the new fixture; exact room-construction inventory; placeholder epoch
  digests in the inherited pickup fixture; rustfmt layout of the inventory entries.
  Final reruns pass. No unrelated failure or CI-only contention deferral remains.
- Whole task diff (previous WIP plus uncommitted/untracked files) was reviewed
  against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Added variants, fields,
  constructors and trait methods were searched across `apps/`, `crates/`, `tools/`;
  public runtime methods have production callers and real dispatch/PG coverage.
  Governance passed (22 policies/9 lanes); 124 agent unit tests passed;
  `git diff --check` passed.

Exact final PG commands (replace `cargo` with the wrapper path above):

```sh
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib world_reset --quiet -- --nocapture --test-threads=1
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres world_reset_ground --quiet -- --nocapture
cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres corpse_decay --quiet -- --nocapture --test-threads=1
cargo +1.95.0 test --locked -p oteryn-game-server --test durability_postgres owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority --quiet -- --nocapture
```

### Final PostgreSQL mounts

All new PG files have exactly one mount; no new `tests/*_postgres.rs` target exists.
All library cases use the existing `run` helper, and Ground uses `configured_admin`:
when `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent they return `Ok(())`.

| PG file | Single mount and existing CI route |
|---|---|
| `dispatch/.../planned_world_reset_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset`; dispatch `--lib` |
| `durability/.../fresh_admission/retired_holder_release_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset_retired_holder`; dispatch `--lib` |
| `dispatch/.../world_reset_source_map_postgres_tests.rs` | `monk_save_postgres_tests.rs::planned_world_reset_source_map`; dispatch `--lib` |
| `dispatch/.../world_reset_activation_postgres_tests.rs` | `monk_save_postgres_tests.rs::world_reset_activation`; dispatch `--lib` |
| `dispatch/.../world_reset_map_pickup_postgres_tests.rs` | `loop_map_pickup_tests.rs::world_reset` (that fixture mounts once in monk tests); dispatch `--lib` |
| `durability/.../world_reset/ground_retirement_postgres_tests.rs` | `character_authority_postgres.rs::world_reset_ground_postgres` |
| Existing `tests/support/corpse_decay_postgres_cases.rs` | existing `character_authority_postgres.rs::corpse_decay_postgres_cases` route |
| Existing owning fresh-loss case | existing `tests/durability_postgres.rs` target, exact filter above |

The shared RETIRING fixture remains mounted once by
`world_reset.rs::test_support`; all consumers import it. The existing protected
workflow already runs the library route and registered targets. This run made no
protected-path edit. The pre-existing workflow restoration was preserved and is
byte-identical to `origin/main`; it is not a new workflow change by this worker.

### Size/reuse and next step

The new activation test is 253 lines, activation owner 73 and content preflight 50;
shared reset test operations stay in the existing fixture (122 lines). No new Rust
module exceeds ~300 lines. The migration is 526 lines because this is the single
allocated additive schema unit and cannot be split into a second migration;
large historical workflow/codec extractions and the retained task history keep
their previously recorded reasons. Existing one-item retirement, authority,
assignment, qualified content, admission publication and source initialization
owners are reused. Next: CP prepares the final authoring/archive commit and
candidate, then obtains required independent review/CI/MQ; no Git write is done here.

## MAP-OVERLAY-1d follow-up

- HOUSE-1a-dependent House catalogue/tile identity preflight and final recheck, HouseInterior/provenance preservation, and Ground on unowned House tiles; 1c refuses all such Worlds.
- Full bundle House qualification and first legacy-to-bundle cutover; Unknown bundle metadata stays closed in 1c.
- **Blocked:** isolated strict reset-capability adoption/denial matrix and operator initialize/publish command qualification require a per-harness role namespace seam; never assert global membership counts.
- Complete/install the disjoint activation/publisher capability and operator-command route only after that qualification, then coordinate the separately owned D1447 deployment grants.
- Full real-data reset duration qualification, including all phases, the 600000 ms budget and alarm behavior under representative load.
- Remaining multi-World/multi-Channel and multi-holder coordination, stale incarnation/Channel/generation/epoch/target/phase cases, and cohort capacity limits.
- Remaining Item matrix: map-materialized Ground retirement, zero/max corpse contents, already-retired/moved-away items and every unsupported nested-container shape.
- Remaining cross-cause matrix: superseded CorpseDecay reservations, conflicting reset/item bindings and receipt uniqueness across causes.
- Remaining raw SQL/audit negatives: forged or stale provenance, direct mutation without complete same-transaction proofs and all legacy compatibility cases.
- Full crash/lost-COMMIT qualification after every phase and between every child/root step, including real process restart and operator replay.
- Remaining House/activation concurrency, target/sequence mismatch, epoch overflow, mixed Channel readiness and changed-digest historical replay matrix.

### Final changed-file manifest (whole unmerged task)

The manifest is relative to the pre-task base `9f814e72a`, and includes the earlier
WIP plus all current tracked/untracked work. The protected workflow entry is the
pre-existing restoration described above; its delta against `origin/main` is zero.
The staged deletion of the provisional 0132 from the earlier WIP is preserved;
the task's final migration is 0148. No index state was changed in this run.

- `.github/workflows/pr-postgres-harness.yml`
- `apps/game-server/migrations/0148_world_reset.sql`
- `apps/game-server/src/node/serve.rs`
- `apps/game-server/src/node/serve/world_reset.rs`
- `apps/game-server/src/node/serve/world_reset_tests.rs`
- `apps/game-server/src/qualification.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/character_revision_node_registration.rs`
- `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`
- `crates/gameplay/content/src/content/project/native_house_tiles.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bundle_map_position.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/control_loss.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/gameplay_drain.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_look.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/listener.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/planned_world_reset_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/serving.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_reset_activation_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_reset_map_pickup_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_reset_source_map_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/core_modules.rs`
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/control_loss_codec.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/retired_holder_release_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/terminal_release.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_event.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/retirement_workflow.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire/world_reset_cause/ground_admission.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_fixture.rs`
- `crates/gameplay/durability/src/durability/item_decay_retire_audit/world_reset_tests.rs`
- `crates/gameplay/durability/src/durability/map_item_mint.rs`
- `crates/gameplay/durability/src/durability/native_map_items.rs`
- `crates/gameplay/durability/src/durability/spell_item_transaction.rs`
- `crates/gameplay/durability/src/durability/world_reset.rs`
- `crates/gameplay/durability/src/durability/world_reset/activation.rs`
- `crates/gameplay/durability/src/durability/world_reset/active_epoch.rs`
- `crates/gameplay/durability/src/durability/world_reset/census.rs`
- `crates/gameplay/durability/src/durability/world_reset/content_preflight.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_retirement_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/ground_step.rs`
- `crates/gameplay/durability/src/durability/world_reset/migration_tests.rs`
- `crates/gameplay/durability/src/durability/world_reset/retiring_fixture.rs`
- `crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs`
- `crates/gameplay/foundation/src/foundation/control_loss_durability_tests.rs`
- `crates/gameplay/foundation/src/foundation/planned_world_reset.rs`
- `crates/gameplay/foundation/src/foundation/planned_world_reset_tests.rs`
- `docs/agents/tasks/MAP-OVERLAY-1c.md`
- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`

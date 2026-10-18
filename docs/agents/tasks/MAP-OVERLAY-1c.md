# MAP-OVERLAY-1c — world reset and overlay retire (skeleton task record)

```yaml
task_id: MAP-OVERLAY-1c
title: "MAP-OVERLAY-1c: world reset and overlay retire"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MAP-OVERLAY-1c; §7 map overlay chain"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/map-overlay-1c
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Completion and wiring — 2026-10-18 (D1436)

- CI root cause: the reset PG cases returned `Err` when `OTERYN_TEST_POSTGRES_ADMIN_URL`
  was absent, in the `cargo test --workspace` lane that has no PostgreSQL. They now follow
  the repo convention (`configured_admin()` absent means no run) and execute in the
  protected PostgreSQL lanes. Migration 0096 creates `oteryn_game_world_reset` (NOLOGIN)
  idempotently with EXECUTE on `game_world_reset_activate`.
- Codex P2: the runtime group no longer holds `UPDATE(revision)` on the job. A
  `SECURITY DEFINER` receipt trigger advances the revision for each `WORLD_RESET` receipt
  of the RETIRING job; runtime and control direct updates are refused (PG case).
- Running node (step 1): `serve.rs` watches for a RETIRING job of its World every 5 s,
  then requests `GameplayDrain`. The listener stops accepting and cancels entry work.
  Each served session loses control with no wait and is released through its grace
  lifecycle. Once all are released the node withdraws readiness, shuts down and exits
  with `BOOT_READINESS_FAILED` (retryable, exit 17).
- Boot (steps 3 and 4): `recover_world_reset` runs before content activation or any
  Channel runtime. While the census says `NotCurrent` (operator Replace not yet done,
  session not released) it waits, bounded by the assignment wait. Then
  `run_world_reset` takes a fresh Spell lane per drained Channel and resumes any frozen
  candidate. It retires every Ground item, one transaction per item, re-censuses and
  activates epoch N+1 atomically (unknown commits retried with the same identity). It
  logs `elapsed_ms` against `MAP01-RESET-RETIRE-MS` and raises `world_reset_alarm` over
  budget. Content is then activated from the new issuance.
- Integration case `reset_run_resumes_a_crashed_reset_retires_ground_keeps_houses_and_activates`
  runs the real path through PG: a crash after one frozen candidate, Ground items, a real
  HouseInterior placement, NewEpoch, and a second boot that does nothing.
  The case also holds a Ground container with two contents. The crash leaves its first
  content frozen, and resume retires both contents (ordinal order) before the container.
  Fixture measurement: 5 items retired in 678 ms.
- Codex P1 (retire order): every content of a Ground container retires before the
  container. Ground contents exist only in the corpse entry family. A character container
  entry's parent is a backpack slot item (FK), never Ground; the PG case asserts 23503.
  Such a row under a Ground root is refused as `InvalidStoredState` and the root is not
  attempted.
- Codex P1 (reset epoch plumbing): `recover_world_reset` returns the World's map reset
  epoch, read from the active epoch pointer (`read_map_reset_epoch`; no pointer means
  never reset, epoch 0). After an activation it must equal the activated record's N+1,
  otherwise readiness fails. Boot passes it in `GameplayDrain::at_reset_epoch`, and
  `serve_gameplay_draining` binds it into the domain 17 map view source, so an epoch-0
  object handle is stale after a reset. Doors resolve targets by digest, placement and
  revision and do not read the epoch. The PG case asserts 0 while RETIRING, then N+1 = 1
  after the run (equal to the `NewEpoch` record) and on the second boot. The new
  `ComposedFreshAdmission` field forces `reset_epoch: 0` in two test-only constructors
  outside owned_paths (`monk_save_postgres_tests.rs`, `kill_reward_postgres_e2e_tests.rs`);
  this is a lead-directed compile fix, disclosed as a scope extension.
- Spell lane pin: `world_reset_phase` (holds a `SpellLanePermit`) deletes corpse entries, so
  it is added to the reviewed key-33 inventory in `spell_lane_key33_pin.rs`.
- Remaining fail-closed seams (not in this packet):
  - The production login must be granted `oteryn_game_world_reset`. Until then
    activation refuses with 42501 and the node stays not ready. Follow-up (CP D1447): the
    grant goes into the oteryn-vps batch deploy task (no Synology change).
  - The bundle-world House catalogue is Unknown, so a bundle World reset refuses
    (HOUSE-RUNTIME owner).
  - Only one Channel per node: a multi-Channel World waits as `NotCurrent`.
  - The boot House source uses the active qualified room for both sides, so only a
    same-server-digest target activates.
  - A crash-left reconnect session in state 1 or 2 blocks the cohort until it is released.
  - The control-owned reset publisher is not built.
  - `cross_domain_finding` (LOOP-GROUND-1, REPORT_ONLY): native map source-item receipts
    (`game_native_map_item_receipts`, migration 0049, `native_map_items.rs`) are keyed
    without a reset epoch. So a source item the reset retired is treated as a historical
    replay and is not reminted on epoch N+1. Fixing it needs an epoch in the receipt key
    (a migration) or a reset-time re-materialization, which is a LOOP-GROUND-1/CP decision.
  - `MAP01-RESET-RETIRE-MS` (600,000 ms) needs a real-data measurement.

## Current authoring checkpoint — D1260/D1269/D1270/D1272/D1283

D1269 preserves exactly one root Area and admits only unique Houses referenced
by explicit house_tiles, retaining all existing House identity/cell/entry/door
qualification. Six actual-canonical positive/negative cases are authored.
D1270 moves the single physical placement encoder to content/spatial_position.rs;
D1283 mounts it privately through Foundation; Content and the standalone shim
only re-export the crate-visible encoder, and the actual death caller uses
Foundation. All three compiled crates have exactly one physical encoder input.
There is no net map/boot change from the published 6b455ad8 checkpoint. Full-bundle
House production remains OUT_OF_SCOPE; full-bundle metadata is explicitly Unknown.

D1272 returns native-scope-derived World/map/server identity with House facts,
compared with actual active history and locked target before mutation. The
borrowed qualified-catalogue adapter refuses Unknown and does not grant current
activation or transport-drain authority. Per-House placement-definition digests
use the same D1245 codec independently of the other Houses.

The uninstalled0096 candidate now contains the atomic caller-TX primitive and
bounded Root commit owner: exact occurrence replay, native admission/cohort/node
locks, complete drain/House recheck, native per-Channel issuance core/CAS, immutable
N+1 history, active-pointer CAS and ACTIVATED in the same transaction. Success
returns only after commit; unknown commit retains the exact occurrence identity.
NewEpoch and Historical are distinct immutable dispositions; historical replay
never grants current controller authority or readiness. SQL NULL/malformed input
refuses before locks; direct SQL independently takes the
native durable fences. Retirement binding remains separate from target artifacts.
Calendar is PG-clock Berlin10 with retained revision and PARITY_PENDING.

Independent source and final qualification reviews PASS for source
`c9b3ceefbdfc0a755e4cc29d7b6e314e908959ab`. The D1283 packet compiled the
actual native library and both existing standalone Cargo targets
`combat_death_reward_postgres` and `durability_postgres` (their original roots
without Content). The standalone targets were compiled/listed only. Native
execution passed 23 pure cases and 13 PostgreSQL cases, each on actual 170006.
Four private cfg(test) registrations (Reset and three existing support modules)
were disclosed; they are not published production integration. No wrapper,
source substitution or second encoder was introduced. Sources remained frozen,
all process groups closed, and the owned PostgreSQL service stopped.

Evidence outside the product checkout: `evidence/2167-D1283-native/` contains
source-plan.json, private-assembly.patch, compile-outcome.json, pure-outcome.json,
outcome.json and independent-final-outcome-review.json. Native ELF SHA256:
`e6e772c6010aa9bb5fe1e24c58c6aa9c86439c74f408bfbf01b7513253ed86b8`.
The final publication adds only this documentation bridge to qualified c9b3ceef;
its new SHA does not relabel the compiled artifact. Earlier compile failures and
historical e60b9b8f 10PASS/2FAIL and PG11PASS remain retained under their exact
sources; they do not replace this packet or establish broader qualification.

Production I/#2187 sealed drain implementation/caller, final slot after C/#2163
and B/#2197, migration installation, in-memory publication, positive composed
activation/replay/concurrency/overflow/rollback, strict/all-target/full-package and final
repository gates remain unqualified. No fake positive drain certificate, shared
caller, schema mount or readiness transition is added. The sections below retain
history; these CP decisions supersede earlier dependency descriptions.

## D1134/D1135/D1152 owned authoring checkpoint

[CP D1134/D1135](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930839)
assigns the all-Channel census, atomic next-epoch ACTIVATED writer and active
revision reader to this leaf in `durability/world_reset.rs`. Close/logout/drain
remains with #2187 at A2. [D1152](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224947337)
assigns durable reset epoch/month history and migration0096 after #2163's0095;
DAILY and RANK consume the eventual port. Allocation is not schema installation.

This checkpoint adds a real caller-transaction census port,
`census_retiring_world_reset(tx, node, world)`. It reads the retained job's epoch,
target digest and revision under its row lock; proves current native incarnation;
reuses the native all-Channel replacement/closed-readiness/session predicate;
and counts actual World Ground roots and their corpse entries. Admission,
assignment, Ground and entry relation locks remain held by the caller transaction.
House assignments do not enter the ordinary Channel count. No request supplies
a current epoch, generation, list of Channels or close certificate. `None` means
no RETIRING job; current-fact failures return `NotCurrent`. Returned data is not
an activation/drain authority and must not survive transaction end as proof.

The existing native PG foreign-holder/House-exclusion case now exercises this
port before and after retirement. It expects a foreign resetting node to refuse,
the actual current cohort to count one Channel/one Ground root, and then zero
Ground roots after native retirement while the foreign House assignment remains.
These assertions are authored and **NOT_RUN**; no multi-Channel or restart pass
is inferred from them.

**Exact remaining source gap / CP-ready request:** atomic activation requires
all of the following in the same owner transaction, and the allocated source
has no complete input proof for them yet:
- #2187's genuine close/logout plus actor/pending-work drain and live quiescence
  boundary. Durable closed readiness and absent sessions do not prove that state.
- The accepted target House catalogue/membership/property eligibility and final
  locked recheck. The existing branch has no target-qualified reset preflight
  token/API. An absent table/document or caller boolean cannot certify it.
- Exact current content activation issuances for every Channel and their target
  digest/client/frame binding. `content_activation.rs::record_content_activation`
  opens/commits its own transaction; SQL `game_content_record_activation` requires
  independently granted control-role authority. Copying a request is no grant.
  The sealed live content activator has only boot quiescence on this branch.

Consequently no successful ACTIVATED writer or `read_active_world_revision(tx,
world)` is fabricated here. Candidate job rows can currently be inserted as
ACTIVATED without the content-activation binding, and their update trigger
forbids actual phase transition. Reading such a row as accepted active authority
would bypass ADR-0021. Migration0096's final phase/activation/history guards must
be authored with the actual target/drain/issuance source; this checkpoint neither
installs nor pretends to qualify that migration. D1152's month history must follow
that same genuine committed activation rather than a timer, caller month or mirror.
CP must identify/allocate the exact target/preflight and quiescence source APIs
(and a caller-TX content issuance seam if needed), retaining #2167 as sole reset
writer. The pre-existing owner assessment contains the full boundary inventory.

**NOT_FULLY_QUALIFIED:** explicit Rustfmt for the two changed Rust files and
`git diff --check` PASS. Compilation, strict lint, revised PG runtime, all-Channel
race/empty census, House recheck, activation, month history, restart and full
package validation are **NOT_RUN**. Existing component qualification below is
historical source evidence, not a result for this changed census. Production
acceptance stays **RED**. No shared audit/mod/root/boot file, registration,
numbered migration installation, registry, external mutation, push or freeze.
D1122 permits the truthful draft checkpoint, not a readiness transition.

## Historical goal and qualification

World reset retires overlay items (Ground, corpses, decayed items) deterministically with audit, honouring house interiors (packet §2.7).

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Third of MAP-OVERLAY-1a -> 1b -> 1c; releases MAP-CUTOVER-1c.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAP-OVERLAY-1b | MERGED (#1809) |
| HOUSE-RUNTIME-1a | MERGED (#2163, cecbfda3); real HouseInterior untouched case registered with `world_reset_postgres` in the D1416 slot |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/world_reset.rs`
- `apps/game-server/src/durability/item_decay_retire.rs (world reset arms)`
- `apps/game-server/src/durability/item_decay_retire_audit.rs (world reset arms)`
- `apps/game-server/tests/world_reset_postgres.rs`
- `apps/game-server/tests/support/world_reset_postgres_cases.rs`
- `docs/agents/evidence/MAP-OVERLAY-1c-*.md`
- `docs/agents/tasks/MAP-OVERLAY-1c.md` (this record: status and notes only)
- `apps/game-server/src/durability/world_reset_run.rs` (reset run, boot reads, `MAP01_RESET_RETIRE_MS`)
- `apps/game-server/src/durability/mod.rs` (world reset module registration)
- `apps/game-server/src/node/serve.rs` (boot recovery, reset watch and drain exit)
- `apps/game-server/src/gameplay_transport/mod.rs` (`GameplayDrain`, `serve_gameplay_draining`)
- `apps/game-server/migrations/0096_world_reset.sql`
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (`MAP01-RESET-RETIRE-MS` row only)
- `apps/game-server/tests/spell_lane_key33_pin.rs` (`world_reset_phase` pin row only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/NNNN_world_reset.sql
- proto tag-5 fields (packet §2.7)
- MAP01-RESET-RETIRE-MS limit row
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

- map/overlay.rs (existing)
- durability/item_decay_retire.rs arms
- RESOURCE_LIMITS_REGISTRY.json MAP01-RESET-RETIRE-MS; GAME_EVENT_FOUNDATION_REGISTRY.json

## Inseparable bundles (never split across PRs)

- migration + world_reset writer + retire arms + audit + registries

## Acceptance criteria

- [x] A world reset retires exactly the overlay items the packet names; house interiors untouched.
- [x] Reset is idempotent and crash-safe (restart mid-reset completes).
- [x] Retire duration within MAP01-RESET-RETIRE-MS (measured per run; fixture value only, see below).
- [x] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server world_reset
cargo test -p oteryn-game-server corpse_decay
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md §2.7
- PR #1809
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Prior REPORT_ONLY reset/cutover packet — 2026-10-15 (superseded)

This section records the earlier card-only design phase; its no-Rust/no-PG
qualification applies only to that phase. The implementation and current
qualification are appended below.

This is `REPORT_ONLY_NATIVE_INTEGRATION_PACKET` for existing draft #2167 at
`593ae93237cb0a15bc85e72e7ebd58356d15d16a`. Only this card is allocated for edits.
The original plan, open criteria, historical dependency table and CP builder rules
above are preserved. This packet does not implement a reset, mint a current reset
epoch, activate a map, alter custody/audit schemas or qualify a production cutover.
It identifies the actual reusable owner components and their inseparable remaining
caller/schema/guard work for CP and independent persistence review.

### Accepted scope and dependency boundary

Protected-main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a` supplies accepted
SOCIAL_MAP §§1.5–1.8, 1.11 and 2.7, ADR-0021 §4.7 and HOUSE-CUSTODY0 §3.6.
Packet §2.7 has `candidate_bases: []` and depends on MAP-OVERLAY-1b and
HOUSE-RUNTIME-1a. HOUSE-RUNTIME-1b/1c and HOUSE-1b are not prerequisites for this
child. Until HOUSE-1b supplies catalogue-revision jobs, the accepted narrower rule is
preflight refusal when the target house catalogue changes on a World with property
rows, not a dependency on tenancy implementation or permission to guess its schema.

The accepted reset closes every Channel's admission, logs out players, obtains fresh
ordinary Channel ownership generations through the existing assignment writer,
retires every live DUR-03 Ground root and its contents (entries before parent), then
atomically writes the target digest, epoch N+1 and ACTIVATED bound to content
activation. Crash while RETIRING resumes from step 2, never boots the old bundle
over partially retired Ground. First bundle cutover is also a reset and includes
legacy-map Ground. World-scoped HouseInterior custody is preserved; Ground on an
unowned house tile is still Ground and is retired.

ACCEPT-BATCH item 9 and its per-tile ruling mean **no periodic ground cleanup between
resets**, Ground root capacity 64 with “not enough room” on overflow. That cap is
separate from `map/overlay.rs::HIDDEN_REACH`'s 64 base-entry bitmask. Neither is a
World-wide reset census bound or permission to retire a crowded/old Ground item.
Existing admitted corpse decay and field/timed-item lifecycles remain their own
causes; a planned reset is not a timer sweep or the overlay's volatile expiry index.

### Existing native spine and exact reset residue

| Real source/caller | Current behavior worth retaining | Required integration delta |
|---|---|---|
| `durability/item_decay_retire.rs::freeze_decay_retire` (`:324`), `commit_decay_retire` (`:520`), `reconcile_decay_retire` (`:616`) | Real durable reservation, per-item advisory serialization, exact frozen event/IDs, work-unit accounting, independent recovery/current node/current Channel assignment checks and immutable terminal receipt. Receipt history can be read without granting a new mutation. | These public APIs admit CorpseDecay, not WorldReset. Add the accepted reset cause path through the same per-item retirement uniqueness and physical transaction proofs, with a real admitted reset owner/epoch and fresh ordinary Channel generation. Do not pass a fake corpse or fabricate reconciled Character authority to reuse a signature. |
| `item_decay_retire.rs::admit` (`:891`) and `apply_retire` (`:1017`) | Locks the live item, checks its actual corpse/Ground location and deadline; removes exactly one location, sets LIVE positive quantity to RETIRED/0 and commits receipt/outbox atomically. The lane is checked against the actual stored Channel before DELETE. | WorldReset admission must read actual live Ground root/container membership under the current reset record and scope fence; it has no corpse deadline requirement. Preserve exact before-state, one-location removal, quantity conservation and same-physical-transaction audit proof. Parent retirement waits until live child entries are gone. |
| `read_corpse_decay_schedule` (`:654`) and `retire_decayed_corpse` (`:722`) | Database-clock recovery census limited to materialized live corpses and bounded direct entries, then N+1 one-item transactions. An entry legitimately moved away is not retired. | This census cannot enumerate all reset Ground. A reset-specific bounded root/entry census must include ordinary, map-materialized and source-created Ground, regardless of age/deadline. Resume from durable state, not an in-memory corpse list. |
| Migration `0015`, current retirement audit module and proto | Reservations keyed by `(item, fence generation)`; terminal receipt primary key is item. Current audit `OneItemDecayRetireV1` admits exactly one CorpseDecay cause and Ground/corpse-entry location. | Implement accepted SOCIAL_MAP §1.7 widening in the existing tables; keep receipt uniqueness across both causes. Add only the already accepted additive WorldReset and Ground-container-entry grammar (§1.11), with full event bounds and existing CorpseDecay compatibility. No parallel reset-retirement receipt table or invented reason. |
| `runtime_scope_assignment.rs::RuntimeScopeAssignmentWriter::{submit,reconcile}` and `publish_runtime_readiness` (`:758`) | Existing single-writer assignment queue, node-incarnation checks, ordinary per-Channel generation and readiness publication. | Reset coordinator must persist RETIRING/admission closure before driving fresh assignments for every Channel of the World; preserve idempotent assignment/reconciliation on crash. No World runtime scope or special reset fence kind. Old generation commits must fail. |
| `map/overlay.rs::ChannelOverlay::rebuild` (`:386`), pickup `rehide_taken_origins` | Rebuilds exact persisted stack order and rejects World/Channel/map mismatch; origin hides are restored from committed map-pickup receipts. No overlay journal or snapshot is the reset owner. | Check durable reset phase before any rebuild/readiness. RETIRING resumes retirement, not old-map overlay rebuild. After ACTIVATED start the new base with empty overlay; ordinary crash outside reset rebuilds the same active map and epoch. |
| `map_item_mint.rs` request/receipt and `read_map_item_mint_placements` | Existing pickup cause includes World, Channel, base digest, placement and reset epoch; exact CommandRef retry returns original item. | Bind pickup/re-hide to the actual reset owner's current epoch. The request's `u64 reset_epoch` is data, not proof of current reset authority. Retained receipt history cannot admit the target epoch or justify a new pickup. |
| `native_map_items.rs::initialize_in_transaction` (`:30`) and `adopt_current_ground` (`:214`), transport `source_item_cycle.rs::ensure_source_map_initialized` (`:104`) | Real current source-map initialization, qualified native owner/content/map/frame binding and same-generation installation check. History is keyed by World/Channel/content/placement; restart adoption never recreates moved, transformed or retired instances. | This is restart/adoption, not the reset controller. A same-digest planned reset cannot be implemented by replaying this initializer and treating old retired receipts as new map Items. CP must explicitly join the admitted reset/map-source model without weakening replay or resurrecting identities. No second initializer or implicit epoch default. |
| `durability/content_activation.rs::{record_content_activation,read_current_content_activation}` and `content/activation.rs::ContentActivationController::activate` (`:285`) | Real per-Channel durable issuance/CAS sequence and sealed quiescent in-memory generation publication. Activation checks target/current identity, sequence and zero live scopes before publishing. Restart does not auto-activate from a receipt. | Neither API atomically closes a World reset. The reset final SQL transition must bind the actual current activation record/target and epoch in the same commit; process-local publication/readiness follows proven durable activation. Do not stitch two unrelated commits and call them atomic. |

Source search finds no `durability/world_reset.rs`, reset phase/epoch owner or native
WorldReset cause implementation at this exact head. That is a concrete missing
integration, not absence of the retirement/assignment/activation components above.
The decay APIs also require a real `ReconciledCharacterAuthority`; the reset caller
must obtain its admitted native authority through the owning integration, not invent
a Character/session witness because no player remains attached.

### House exclusion is custody, with current target membership checks

Migration `0025_house_item_custody.sql` already supplies World-scoped
`game_item_house_interior_locations`, reclaim provenance, placement-transaction
history and cross-location exclusivity. HouseInterior has no ChannelId and cannot
hold contents. The reset enumerates Ground roots and their live contents only; it
must not retire HouseInterior or delete provenance. Skipping all coordinates bearing
a house marker would wrongly spare ordinary Ground on unowned house tiles.

Before RETIRING/admission closure, preflight must independently verify every live
HouseInterior `(house_key, position)` still belongs to that same house in the target
bundle. Removed/re-keyed house, removed tile or tile reassigned to another house
fails with no reset record and no closed admission. The active and target catalogue,
map/content/frame identities, World and current property state must be qualified by
their actual owning sources. `QualifiedHouseTiles` in `content/project/native_house_tiles.rs`
is an existing typed membership seam: omission is Unknown; source-region proven absence
is explicit. A numeric `sector::Tile.house` marker or collision `house_tile` boolean
alone does not prove stable House key membership in the target catalogue.

The current native ownership schema uses `game_house_ownership` (`0038`) and
World-house instance admission (`0047`), whereas the accepted future house jobs name
`game_house_properties`. Do not interpret a missing future table as proof that the
World has no property/owned-house state. The owning house adapter must qualify the
actual current property/catalogue view for the accepted preflight. Reset does not
borrow a caster's old ACL result as current house membership or property evidence.

Step 4 repeats the target check in exactly SOCIAL_MAP §1.6 order: lock the reset
record `FOR UPDATE`, then `LOCK TABLE game_item_house_interior_locations IN SHARE MODE`,
then recheck and write target digest/epoch N+1/ACTIVATED. This blocks inserts, updates
and deletes through activation commit; a House scope is World-scoped and survives
Channel closure, so a row lock alone or admission closure is insufficient. Recheck
failure leaves RETIRING, requiring the accepted evacuation path; it does not roll
back already committed item retirements or reopen the old map. Current property/
catalogue eligibility also needs its owning serialization at final activation.

Prepared HOUSE-RUNTIME-1a (#2163), retained base
`a9e0277c8c79071bcfe7b8aa7ec7b78f2592232d`, is needed at the final integration
boundary. Its exported `house_interior.rs` validates structural location/provenance/
role relationships only; it expressly is not current authority or a production
shape writer. `house_interior_audit.rs` is likewise partial immutable audit facts.
The three SECURITY DEFINER shape writers, current access/tile checks, EXECUTE-only
runtime grants and same-transaction proofs remain its CP bundle. Its prior eight
native/PG tests are historical export evidence, not tests run here or qualification
of the reset's house lock. HOUSE-RUNTIME-1b/1c are not added to this dependency.

### Concrete schema/guard and bounded pass handoff

The sole new reset record is already delegated by the accepted packet; its admitted
World, retiring epoch N, expected active identity, target digest and RETIRING/ACTIVATED
state drive progress. CP must supply the actual reset authority/epoch source, current
node and Channel assignment cohort. Do not infer an epoch from wall clock, activation
sequence, client/map-view value or a request integer. CAS/unique transition binding
must reject a different target for an in-progress reset and checked increment must
refuse epoch overflow before mutation. No retirement may start before phase closure
and every affected Channel's old generation has ended.

For a minimum bounded implementation, select the next live Ground root in stable
Channel/item order, then the next admitted direct entry in placement/item order,
and issue at most one one-item logical retirement per owner pass. This is a proposed
scheduling choice, not a newly accepted numeric resource row. Re-read live custody
under the actual item/parent locks before each step; a census descriptor does not
authorize DELETE. Current schema's generic `game_item_container_entries` is constrained
to an equipped main backpack by its FK in `0011`; live corpse contents use the separate
corpse table. The additive Ground-container audit arm does not admit arbitrary nested
bags or turn Character inventory into reset contents. Any additional native Ground
container custody must be concretely admitted by its owner before that branch is used.

Census must not filter Ground to the **new** generation or target map digest: old
stored location generations/revisions are exactly what reset, including first legacy
cutover, must retire. The resetting node's independently current new Channel fence
is authority; the actual old location is before-state. Include every live Ground root
of the World, including expired-but-not-yet-retired items; exclude retired instances
and items now in Character, HouseInterior or other non-Ground custody. Re-query durable
state after progress/unknown outcomes; only a verified empty census permits step 4.
Do not collect the entire World in memory, reuse the corpse-only count as a World
limit or add a periodic cleanup job. Query/index/pass duration and the total reset
bound must be measured on real admitted data before MAP01-RESET-RETIRE-MS is registered.

At the slot, widen `0015` reservations/receipts with closed `cause_kind` values
CORPSE_DECAY/WORLD_RESET and conditional `reset_epoch`; preserve old rows/defaults and
all corpse deadline CHECKs. A stale old-generation CorpseDecay reservation remains
history; reserve a fresh WORLD_RESET row for the same item under the new generation.
Never adopt its old frozen IDs/event bytes or add another terminal receipt. The
same-transaction item-change, location-removal, exclusivity and outbox guards must
prove the selected reset cause, actual before-location, current reset phase/epoch,
node/scope generation and exact item/event binding at commit.

Do not restore an obsolete guard body from `0015`: later migrations extend these
unions. The static definition inventory identifies current candidate bodies:
`game_item_instance_change_proven` and generic container removal in `0083`, Ground
removal in `0050`, corpse-entry removal in `0042`, plus `0015`'s retirement-consistency
guard. Ground removal still contains the corpse-only `d.corpse_item_instance_id =
OLD.item_instance_id` proof; it needs the accepted cause-specific reset arm without
losing spell/temporal/TRANSFER clauses. A reset root/entry and receipt/outbox must be
one physical transaction; old receipts cannot justify new DELETEs. Legacy 0015 rows
must remain CORPSE_DECAY and every existing guard failure/positive case must survive.

Audit uses the existing one-item operation tag 5/type-2 envelope and accepted
SOCIAL_MAP §1.11 additive fields: `world_reset = 7` with World/epoch/item and
`ground_container_entry = 8`, valid only for reset. Exactly one retirement cause,
exactly one admitted location, same item/World/definition before and after, full
positive quantity to zero, no remaining location, and item equality in the reset
cause are mandatory. No new cause name/table/event number is invented. Measure new
worst-case payload/envelope against DUR03-RL-07 limits and retain the old CorpseDecay
encoding/decoder compatibility; no zero/dummy corpse/deadline to satisfy old fields.

### Activation, unknown outcomes and real map consumers

`gameplay_transport/mod.rs::BundleMapPosition::update` and
`map_door.rs::use_door` currently construct `MapViewSource` with `reset_epoch: 0`.
`world_map.rs` already binds handles and replacement-baseline decisions to
`(content generation, bundle digest, reset epoch)`. These consumers need the actual
activated reset epoch from the same owner that binds map pickup/re-hide; assigning a
constant or using content digest as an epoch is not a cutover integration. This is a
shared caller change for CP, not an edit within this card allocation.

A failure before preflight success changes nothing. Once RETIRING has committed,
partial one-item retirement is expected durable progress and the World stays closed.
A failed later step never claims a rollback of earlier items. A lost per-item COMMIT
reply reconciles its immutable receipt before retry; a lost activation reply re-reads
the actual reset row and bound activation record under current authority before
publishing or starting another reset. ACTIVATED with matching identity/epoch is
historical success, not fresh reset authority. A conflicting target, mismatched
activation or unknown phase fails closed. The current sealed in-memory activation
controller cannot by itself prove that durable two-record atomicity.

### Required native negative/cutover matrix once implemented

| Boundary | Qualification and conservation evidence |
|---|---|
| Preflight/house | Same-house target preserved; removed/re-keyed house, removed/reassigned tile, unknown target membership, wrong World/frame/content and changed catalogue with actual property rows each refuse before record/admission mutation. HouseInterior/provenance survive; Ground on unowned house tiles retires. |
| World/generation | Two Channels of one World plus another World; fresh generation for every affected Channel; old MINT/TRANSFER/Ground/decay commit fails; stale node incarnation, wrong Channel, current reset epoch/target/phase independently mismatched; no request-derived current authority. |
| Item/census | Ordinary Ground, materialized map Item, corpse with 0/max contents, already-decayed retired item, expired live Item, moved-to-inventory/HouseInterior item and legacy-map Ground. Entries retire first, parent waits; no inventory/nested-bag expansion; every one-item transaction conserves full quantity into retired/0 with no location. |
| Cross-cause/replay | Old-generation CorpseDecay reservation replaced by fresh reset reservation; old commit refused; existing CorpseDecay receipt not duplicated. Same reset/item replay returns original terminal outcome, conflicting binding refuses, concurrent resets/decay cannot produce two receipts or reuse frozen identities. |
| SQL/audit adversarial | Direct DELETE/update without same-xact receipt/outbox, stale historical proof, wrong item/World/epoch/generation, partial audit/retirement or forged location fails deferred guards. Event both/neither causes, wrong reset item and reset-only location under corpse cause refuse; legacy events/rows remain valid. |
| Crash/unknown | Crash after each reset phase and between child/root retirements; restart RETIRING resumes step 2; no old bundle over half-retired Ground. Lost item/activation COMMIT responses reconcile exact durable outcome without a duplicate item, receipt, audit or epoch increment. |
| Step-4 concurrency | Real concurrent HouseInterior insert/update/delete waits under reset-row then SHARE-table lock; recheck mismatch leaves RETIRING. Competing property/catalogue change serializes under its owning contract. Empty census checked under final activation authority, not a stale caller boolean. |
| Activation/client/bounds | Target/expected-current/sequence mismatch and epoch overflow refuse; first legacy cutover and same-digest next-epoch reset; no mixed old/new Channel readiness. Pickup handles/re-hide/client baseline use actual epoch. Ground64/65 remains admission-only, separate base bitmask; no timed cleanup. Measure bounded memory/pass latency/total MAP01-RESET-RETIRE-MS and worst-case audit payload/envelope. |

### Evidence and qualification limit

`/workspace/oteryn-prep/map-overlay-1c-evidence/` retains source extraction scripts,
43 exact Git-source hashes (39 candidate sources and four protected-main decisions),
five current guard-definition families and 11 verified HOUSE-RUNTIME-1a/LOOP-GROUND
export bindings. Prepared dependencies' tests were not rerun or promoted to reset
proof. Raw governance validation/124-test, whitespace and exact card-only scope logs
plus final source/diff bindings are retained. No Rust/Cargo, PostgreSQL, reset,
activation, runtime, client or performance tests were run. Whole-diff worker
self-review is complete; CP's inseparable migration/writer/audit/registry/caller
bundle and independent hard/persistence/determinism review remain required.

## Bounded native implementation preparation — 2026-10-15

This implementation phase supersedes the preceding card-only lease and its
`REPORT_ONLY_NATIVE_INTEGRATION_PACKET` conclusion. The original plan, dependency
history, open acceptance criteria and CP builder rules remain unchanged. The
previous exact card, local patch, source hashes and qualification output are
retained separately in `map-overlay-1c-evidence/design-phase/`; they are not runtime
evidence for this implementation. This phase owns only the six paths in the local
allocation and does not install a migration, register a module, publish a registry
or claim a complete World reset.

The private added prost fields are not admitted production payload merely because
the genuine native Type2 tuple/audit SQL accepts them in a fixture. Normative
WorldReset proto/schema/event publication and its activation are explicit CP
gates; this local qualification does not cross them.

### Implemented native boundary

`durability/world_reset.rs` supplies actual Root APIs:

- `freeze_next_world_reset_item` resolves a control-published RETIRING job and the
  complete ordinary-Channel cohort, then reserves one actual live Ground root or
  the first real live corpse entry under it. It persists the exact transaction,
  event, timestamp, envelope, epoch, parent, generation, holder and retry budget
  before mutation; re-freezing the same generation resumes those bytes.
- `commit_world_reset_item` takes that opaque candidate, independently rechecks
  current Root recovery, node incarnation, native admission relations, complete
  fresh closed cohort, no active/reconnectable sessions, actual local assignment
  and typed native runtime guard/map revision. It durably charges one existing
  RL-08 work unit in a separate transaction before the atomic retirement pass.
  Same-generation retries retain IDs/envelope and stop at three charges. The
  native later-generation reservation policy is preserved: a new owner generation
  reserves afresh, and the former generation cannot mutate.
- A candidate-specific terminal receipt is read after the native exclusive
  admission-relation fence and before another charge or census. Concurrent replay
  therefore returns the same historical result. Unknown outcomes retain the same
  opaque candidate; the API never silently advances to another item. The public
  identity accessor and `read_world_reset_retirement` expose historical data only.
  Network-loss/unknown-COMMIT fault injection is not claimed.

Mutation uses the actual native Type2 activation fence and frozen-tuple seam,
item-decay advisory lock, one-item/location removal, quantity evidence, shared
retirement reservation and globally unique item receipt, exact outbox bytes/hash,
P90D expiry and job-revision CAS in the same transaction. A failed mutation rolls
back the item/location/receipt/outbox/revision together; the earlier frozen bytes
and spent work unit remain durable. A pre-activation V1 reservation retains its
native grandfather path after activation; new reservations use V2.

The same owned module contains the **uninstalled** proposed reset job/cohort SQL
and conditional WORLD_RESET extensions to the existing retirement tables. Runtime
cannot publish jobs/cohorts or alter their binding. The four native retirement,
instance-change, Ground-removal and corpse-entry-removal guards retain their exact
latest source bodies, with a reset-specific positive clause requiring the actual
same-transaction native evidence and complete reservation/receipt/audit. No native
trigger is disabled. Legacy corpse APIs also work before this proposed SQL exists:
owned cause classification reads the optional column through `to_jsonb`, retaining
CORPSE_DECAY as the absent-column default and rejecting reset rows before decoding
nullable corpse fields.

The audit adds only the accepted reset/generic-entry arms to the existing one-item
DECAY_RETIRE shape. Reset before/after state preserves the real immutable numbered
key; retired quantity zero does not call a live quantity-one key checker. The
selection is exactly native live Ground plus actual corpse entries, entries before
parent. It excludes inventory and HouseInterior; no fictitious nested bag storage,
base-map item census, combined base+Ground cap or periodic cleanup is introduced.

### Current-source and remaining owner obligations

The uninstalled job/cohort is a proposed **privileged control source**, not a caller
assertion of a current epoch. Runtime verifies actual current facts independently,
but the actual pre-reset predecessor and admission/phase provenance must be
published by the eventual native control owner. That owner must capture the real
pre-reset assignment and bind fresh assignment operations/cohort to the retained
reset. Fixtures use the genuine control role, actual assignment SELECT and native
Replace operations; a manually chosen number is not production authority.

ADR-0021 explicitly requires each ordinary Channel on **the resetting node**.
Foreign ordinary-Channel holders are refused until fresh native reassignment to
that node. House assignments (`scope_kind=2`) are outside that cohort and may
remain on another current node. Distributed reset ownership would require a
contract change. The component does not interpret World as another Channel.

CP still owns the admission-close/logout and reset publication/orchestration,
preflight of target digest and accepted House catalogue/membership/property fences,
actor/pending-work quiescence, crash-resume assignment choreography, all-Channel
final census, HouseInterior lock/recheck, atomic N+1/ACTIVATED/content activation,
legacy first-cutover route and reset-aware ground initializer/consumer recovery.
The present schema deliberately refuses changing a job binding/phase; phase4 must
arrive with its inseparable guarded activation owner. No standalone value writer
or current content/epoch authority is claimed. Numbered migration, accepted proto
publication, registries/limits, production wiring and independent architectural
acceptance remain slot work; HOUSE-RUNTIME-1a remains the accepted dependency.

There is no genuine current native Ground-drop route for numbered keys, so the
key regression proves the typed key7 grammar and actual key7 inventory exclusion;
it does not fabricate keyed Ground custody. Likewise a positive HouseInterior
value row awaits its admitted native writer. A real House scope assignment is
qualified without manufacturing HouseInterior rows. Full reset-duration/resource
acceptance and fault-injected unknown COMMIT remain unqualified.

### Executed qualification and source binding

Exact draft source is `593ae93237cb0a15bc85e72e7ebd58356d15d16a`; accepted-source
readiness remains protected-main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.
Implementation evidence is outside the branch at
`/workspace/oteryn-prep/map-overlay-1c-evidence/implementation-phase/`.

The final `attempt-5/` genuine native driver ran **10/10 PASS in 62.48 s**:
eight isolated PostgreSQL cases on **17.6 / server_version_num=170006**, plus two
selected audit bodies. All native guards remained enabled. Coverage includes
actual Ground retirement/conservation/history and key7 inventory preservation;
legacy corpse APIs before and after proposed DDL with retained original bytes;
real deadline, entry-before-parent, stale generation and cross-cause refusal;
committed frozen IDs and max3/max+1 work budget across authority reload; deferred
mutation rollback retaining budget, job CAS and successful replay; concurrent
same-candidate replay through two independently maintained genuine Roots and
separately opened authorities; actual V1 grandfather/new-V2 activation; native
foreign-holder refusal and real foreign House-assignment exclusion; runtime
publication denial, control publication, no-truncate and unproven deletion.
The assignment fixture reuses the actual existing per-Root writer registration;
no single-writer or one-connection bound is relaxed.

Final owned Rust formatting/check, original untransformed native unit metadata
Clippy with `-D warnings`, enabled original standalone wrapper metadata Clippy,
native cfg(test) normal rlib and selected driver compilation all passed under the
shared Rust flock with incremental off/jobs2/debug0 and one coherent fresh Cargo
graph. Cargo capture deliberately stops the root with sentinel86/Cargo101 before
codegen; that is graph capture, not a test pass. The tracked standalone wrapper
is explicitly disabled until native registration; its metadata check proves an
inactive zero-test target only. The enabled validation wrapper removes just that
gate and uses the literal outside module registration in its actual private Root
context. Executable qualification removes only selected owned test attributes,
widens their visibility and adds forwarders outside the candidate; native Root,
fixture bodies, dependencies and guard definitions stay genuine. Transformation
and source-before/after manifests retain those precise limits. Normal rlib
baseline unused-helper warnings are retained; the clean Clippy claim applies to
original untransformed test source and the enabled original wrapper.

Failed authoring/qualification attempts are preserved, not reclassified as green:
initial metadata fixture-context/import issues, an unreachable new fixture arm,
then candidate SQL's nonexistent global map mirror and noncanonical House key,
then a fixture writer-registration conflict. Their raw logs and completed
artifacts/restore hashes remain separate from final attempt5. No full global
unit ELF, full workspace suite, production wiring, reset duration, whole-reset
restart or network-loss/unknown-COMMIT qualification is claimed. Governance124,
owned-path scope, original-card-prefix and final source/fixture bindings are
recorded in `implementation-phase/final-checks/`; Rust hashes remain identical to
`attempt-5/source-before.json` and `source-after.json` after this documentation
update.

### CP gap-review follow-up — 2026-10-16

The remaining whole-reset work was checked against source
`59172979bc3c333fb2eda43b6dae52065bbbf9ef` and CP comment `6216366289`.
The concrete missing owner/consumer allocations, shared CI-list slot and
qualification boundary are retained in
`docs/agents/evidence/MAP-OVERLAY-1c-reset-owner-ports-2026-10-16.md`.
This source assessment changes no Rust or runtime contract and does not complete
the still-open acceptance criteria. Whole-reset orchestration needs actual
transport/node, content activation and initializer consumers outside owned paths;
HOUSE-RUNTIME-1a remains the dependency for real positive HouseInterior custody.


### D1189 caller-transaction issuance seam (source authoring)

CP2131 comment6225612898 assigns the real reset activation composition to G.
`record_content_activation_in_tx(tx, request)` now performs the existing checked
issuance SQL within the caller transaction. The public method retains its early
request validation and delegates without changing SQL bindings/refusal behavior;
it commits only on success. A refusal may abort PostgreSQL's transaction, so a
caller must roll back, never continue as though issuance succeeded.

This primitive does not authorize reset or mint current facts. The final reset
caller must derive the request only from its locked RETIRING job and retain all
proofs through atomic commit. House preflight, I2187's sealed WorldDrainProof,
0096 locked-job wrapper/history and final activation/read ports remain to be
implemented/composed in their assigned order. New strict/runtime/PG checks are
NOT_RUN; earlier reset/census evidence is preserved without relabelling.


D1215 (CP2131 comment6225922014) resolves the cross-root dependency with a new
`world_reset_port.rs`, registered once under D1160. It contains the exact local
House source/drain traits and data, importing no content/transport modules.
G authors the local contracts now; genuine Content implementation must call
C2163's actual native House encoder, and I2187 implements its sealed drain proof.
No production fixture implementation or synthetic proof is added. Target
catalogue revision/digest still needs an actual source to enforce the D1189
same-key revision-change refusal (CP request6225990483). New port compilation,
preflight/activation and PostgreSQL execution remain NOT_RUN.


The actual native crate successfully compiled the exact0c745974 issuance helper
with Events/Weather components (ELF89fb3206f720c42b168379022f37198e77b8b4785638ec6b0813e6589fe883d8).
Dep-info/source-at-close prove that helper file once. This is compile evidence
only, not reset/PG runtime qualification. D1215's newer local port was not part
of that frozen compile and remains compile/runtime NOT_RUN.


### D1218 caller-TX House preflight source checkpoint

The local House port now requests explicit Active/Target catalogues and retains
one canonical definition_digest per House from the native source owner. The
owned preflight locks 0025 custody,0038 ownership and0047 Instance relations in
SHARE mode through the caller TX. It compares every live World custody key and
opaque native position against Target; removed/rekeyed/reassigned positions
refuse. Only keys with ownership or0047 rows compare Active/Target per-House
definition digests and return HouseCatalogueChanged{house_key} on mismatch or
absence. Unowned/unallocated definition changes alone are accepted; no whole
bundle digest comparison or blanket catalogue refusal is used.

Source validation enforces actual House key/position bounds, consistent per-House
digests and unambiguous tile membership. Three authored unit cases cover scoped
catalogue changes, exact custody membership, and malformed/ambiguous source.
All new compilation/unit/PostgreSQL checks remain NOT_RUN; this is
NOT_FULLY_QUALIFIED. The native C2163 encoder/source impl and sealed I drain
implementation remain pending; no encoder copy or production proof is invented.
The caller must run this preflight before RETIRING and repeat under its locked
job in final activation; no activation/history/0096 integration is claimed here.

The review repair streams both SQLx result sets and checks each row immediately;
no live custody/protected-key result Vec is retained. Client indexes retain only
qualified source catalogues, bounded by the native source's1024 Houses and4096
cells per House. Unit success setup propagates Result with ? rather than unwrap.
No added dependency or manifest change; native compile/runtime remains NOT_RUN.

Lead source review PASS after repairs: native-source catalogue bounds remain
1024 Houses/4096 tiles each; SQLx database rows stream one at a time with no
new dependency or collected live-row set. Positive unit tests return Result
without unwrap/allow. New preflight compilation/three tests/PG remain NOT_RUN
until actual native qualification; no current-authority token is minted.

The preflight consumer requires `&(dyn WorldResetHouseSource + Sync)` because its
borrow spans SQLx awaits in the durable semantic lane. The CP local trait is
unchanged and no production implementation is invented. The prior three native
case PASS remains exact to6bd7824e; this signature's renewed compile is NOT_RUN.

### D1233 complete World drain binding — source checkpoint

The durability-local port now borrows WorldDrainBinding: World, exact
NodeRegistrationFact, complete sorted per-Channel scopes/generations and a
canonical cohort digest. Only I/#2187 implements WorldDrainWitness from its actual
sealed proof; this checkpoint adds no production witness implementation or proof
constructor. I must call the single owned world_drain_cohort_digest helper rather
than copy its encoding. Its internal fixed domain/version hashes World16,Node16,
registration u64BE,count u64BE,then sorted Channel16/generation u64BE entries;
wrong-World/non-Channel/duplicate/unsorted or zero-generation input refuses.

The caller-TX consumer first uses the existing native census, retaining admission,
RETIRING job and assignment locks and independently revalidating current node,
complete replacement cohort/closed readiness/session absence. It then streams
all ordinary Channel assignments for this World in byte order, checking every
holder node/revision and scope/generation against the entire borrowed binding.
Missing/extra channels, changed generation/node/World or digest refuse; no
representative Channel or request-derived authority is accepted. No data row
buffer, activation SQL write/commit or new schema is added.

One authored data-only unit case checks exact binding/digest agreement and each
mismatch axis, plus malformed ordering/duplicate/foreign-World entries. It mints
no witness and establishes no current authority. New D1233 compile/unit/PG/
activation/integration qualification is NOT_RUN and NOT_FULLY_QUALIFIED. The
separate native0771f0c5 source freeze covers the earlier House/Sync inputs only,
not this later binding/consumer delta. Genuine C encoder and I sealed logout/
pending-work drain remain required before final composition.

## D1233 native component qualification

Actual native Cargo no-run at source2fca2ae7 plus exact2152 donation leaf:
compile exit0,3185unique global cases; four Reset cases executed literally
(three House preflight +one complete drain-binding data case), each1PASS,0ignored.
The fifth case belongs to2152 and is not counted as Reset. Only private cfg(test)
module registrations; all frozen source hashes/actual dep-info match. Independent
outcome review is retained with the external evidence packet. PG,strict,full
package,canonical wiring and final activation remain NOT_RUN. I2187 alone supplies
the sealed production drain proof. CP6226416344 requests the actual Cencoder/
canonical House key mapping/fullbundle owned hunks; no ENTRY substitution.


## D1241 canonical position encoder and exact bundle-carrier gap

The owned map/boot.rs helper encode_spatial_position extracts death_reward's exact
x i32BE | y i32BE | floor i16BE encoding. The single granted death_reward caller
now uses it. checked_movement_position converts actual LogicalCell x/y unchanged
and checks z fits the real MovementLocalPosition i16 floor; overflow refuses,
with no clamp, sign flip or guessed native-frame conversion. One authored unit
case covers exact signed bytes, floor boundaries and overflow. New D1241
compilation/tests are NOT_RUN, distinct from earlier Reset component results.

The full-bundle House producer is blocked at an actual carrier boundary:
world-bundle::bundle::Manifest is deny_unknown_fields and has no native House
document field; Bundle carries manifest,sectors,spawns,npcs,digest only.
map::boot receives bytes/pins/item resolver, not WorldProject declarations or
OTERYN_NATIVE_HOUSE_TILES/v1. Existing QualifiedHouseTiles::qualify requires that
WorldProject plus FirstProductionCell data and qualified source scope. No source
can honestly read the named document out of the current binary bundle. Neither
QualifiedHouseTiles nor QualifiedHousePlacement stores qualified_placement_digest;
0047's durable read validates an existing32byte nonzero value but supplies no
canonical producer algorithm here. No digest accessor/value is invented.

Thus no ENTRY substitution, numeric tile.house→House-key mapping, fake full-bundle
catalogue or new hash is introduced. Actual House metadata/declaration carrier and
0047 placement-digest provenance are still needed before the D1241 producer can
qualify tile.house nonzero/same-within/distinct-between checks and reset source
binding. The source remains NOT_FULLY_QUALIFIED pending that genuine composition.


## D1245 carrier correction and qualified placement digest

D1245 CP6226519383 supersedes D1241's full-bundle producer allocation and the
historical carrier gap above. The actual House carrier is the existing
NativeFirstEntryDocument.house_tiles optional document, serialized/hashed with
its native overlay and admitted by NativeEntryProject::movement_cells through
QualifiedHouseTiles::qualify. This checkpoint adds a test-only authored overlay
through CanonicalProjectDocuments::from_native_entry_draft and parse_native_entry.
That parser automatically validates movement_cells; the test returns a second
explicit qualification against the same parsed source and derived scope.
NativeGameplayInput has no supplied House-overlay field: this fixture does not
claim an operator-configurable or selected-manifest House producer. No production
House allocation, pinned manifest alteration, new runtime seam or full-bundle
metadata/schema is added.

The canonical encoder, checked LogicalCell conversion and exact-byte test move
to content/project/native_house_tiles.rs; the death_reward caller follows it.
Only this PR's earlier map/boot helper/test additions are removed in this
successor, respecting D1245's withdrawal. The c89880eb commit/bundle and its six
actual native cases remain historical exact-source evidence; relocated code/new
digest tests are NOT_RUN pending new native qualification.

QualifiedHouseTiles::placement_digest returns None for Unknown metadata and
SHA256 for explicit qualified placement data, including explicit empty data.
The documented internal v1 serialization sorts Houses by key/revision, retains
sorted cell/door coordinates, and binds key,revision,cells,entry and door numbers
with fixed-width BE coordinates/counts and length-prefixed UTF8. It uses no
request/current authority facts and invents no observed0047 value or external
hash vector. Authored overlay tests cover reordered Houses/cells, revision/
entry/door mutation, None versus SomeEmpty and an invalid physical cell. An
independently computed fixed SHA256 vector for the literal domain plus zero
u64BE House count pins the explicit-empty canonical encoding.

The no-argument digest identifies the retained qualified catalogue only. Current
native_entry::from_bundle retains entry.house_tiles under its original scope;
house_at correctly refuses that catalogue for bundle scope. It is not literally
housesNone when the input entry overlay had metadata. D1245a/D1246 CP6226622415/6226640326 now allocate the exact from_bundle
successor: QualifiedHouseTiles::unknown_for_scope(scope.clone()), retaining the
now-unused entry argument as _entry for caller compatibility. The granted
6916c5f9 patch was applied only after verifying exact before df1b9490 and
intermediate after1e851e4d hashes; no other caller changes. Lookup refusal alone
cannot protect a no-argument digest consumer, so bundle metadata is now explicitly
Unknown under its actual scope. Full-bundle House production remains a separate mechanic, not a
blocker of the admitted overlay fixture. Reset source/durable preflight/current
0047 binding, I's sealed drain and final activation remain NOT_FULLY_QUALIFIED.


D1245a/D1246 adds a real full-bundle regression in native_entry.rs. It first
qualifies a test-only House overlay through canonical documents and the actual
NativeEntryProject movement path, proves ENTRY has a digest/positive lookup, then
boots the existing real binary-bundle fixture through production map::boot and
movement_cells/from_bundle. The resulting bundle catalogue has digestNone,
refuses retained ENTRY scope and reports Unknown in its own bundle scope. No
mock bundle index or current-authority witness is constructed. All new D1245/
D1245a/D1246 compilation and test execution remain NOT_RUN before lead review.

## D1189a private native issuance core candidate

CP6226612398 approves the0096 private SQL issuance-core extraction. The owned
candidate_schema_sql now derives exactly one core from native0008, retaining its
advisory lock/replay/CAS/INSERT bytes. The existing public control wrapper keeps
the exact session_user/op4 check and delegates to that core. Both functions have
pinned search_path; core EXECUTE is revoked from PUBLIC/runtime/control, leaving
the migration owner. No general runtime op4 or second INSERT writer is added.
This remains UNINSTALLED candidate SQL for0096 promotion, not a migration install
or successful reset activation. The reset-specific authorized wrapper and atomic
ACTIVATED/epoch/history/read composition remain separate required work.

One authored pure source case checks the exact native write-body preservation,
single INSERT, public authorization/delegation and private ACL/search_path. An
owned PG case installs the real candidate DDL, records a genuine granted control
issuance and independently verifies runtime cannot execute core even for that
valid exact replay (42501), with schema/function existence checked first. New
source compile/pure case and PG case are NOT_RUN until their recorded execution;
no role/SQL authority result is inferred from source strings.


## D1257 explicit epoch initialization and PG calendar — source checkpoint

The UNINSTALLED0096 candidate now owns immutable World epoch-history plus an
active pointer. Epoch0 is written only by the explicit Control initializer,
modeled on native descriptor issuance's owner-only SECURITY DEFINER/ACL and
semantic transaction pattern. Identical World/map/three-digest/calendar facts
replay without changing history; map/digest/calendar conflicts refuse. No SQL
bootstrap seeds or runtime EXECUTE grant are introduced. Fixture admit uses the
same Control operation. New job publication refuses an uninitialized/current-
revision mismatch, and census checks actual active history instead of assuming0.

ResetCalendar explicitly carries TIBIA_SERVER_SAVE_BERLIN_10 and a positive
revision; history retains that policy/revision and PARITY_PENDING. Pure PG
calculation uses (instant AT TIME ZONE 'Europe/Berlin') minus10hours then YYYY-MM,
with upstream PostgreSQL timezone/DST behavior. Initial history and the current-
month reader obtain clock_timestamp() in their own actual TX, never process time
or a request instant. Missing active state is NotInitialized, for DAILY to map
to Unknown; no absent-state epoch/month default is supplied. Active/history and
locked-target readers return actual stored facts only, not current authority.

Locked jobs have separately nullable target map/server/client/frame/calendar
columns, with all-or-none shape; existing target_digest/active_map_revision stay
separate predecessor/legacy facts. A legacy retirement-only job may still retire
but the target reader returns UnqualifiedTarget, so it cannot supply activation
facts. No final ACTIVATED writer, epoch successor/history atomic composition,
reset-specific issuance wrapper or migration install is claimed in this tranche.

Two authored PG cases cover09:59/10:00 Berlin month starts with winter/summer
UTC offsets, spring/fall DST transitions, unsupported calendar, no initialization,
Control bootstrap/idempotent exact replay and per-fact conflicts, immutable
history and runtime42501. The month reader's observation is recomputed from its
PG instant/calendar; no synthetic current-state authority is used. All new
compile/PG/calendar/initializer/runtime checks are NOT_RUN and
NOT_FULLY_QUALIFIED. Earlier isolated qualification freezes are preserved and
do not qualify this later source delta. No new Cargo dependency or runner.

### D1416 slot commit — 2026-10-17

CP D1416 granted the schema slot on #2167. This commit merges main, registers
`durability::world_reset`, installs the reset schema as migration
`0096_world_reset.sql` (free on main and in every open PR head) and enables the
`world_reset_postgres` target (Cargo `test = false`, cases also in the
`character_authority_postgres` aggregate). The migration is the exact output of
`candidate_schema_sql()`; a unit test pins the bytes. The native item-change
guard source moved from 0083 to 0095, so the reset early-return extends the
current House-shape guard body instead of reverting it. PG fixtures now verify
the migrated schema instead of running DDL. A real HouseInterior case places an
owner-committed House item and shows a full Channel pass retires Ground only,
leaving location, provenance, shape proof and item state byte-identical;
the pass is refused while the House destination session is still attached.

Moved to a follow-up packet (D1416 (2)): the production reset orchestration —
admission close and drain, the all-Channel census, boot resume and the
MAP01-RESET-RETIRE-MS limit row — plus the proto/registry/content slot-only
rows. Later migrations that redefine any of the four reset-extended guards
(`game_item_decay_retire_consistency_guard`, `game_item_instance_change_proven`,
`game_item_ground_removal_proven`, `game_item_corpse_entry_removal_proven`)
must keep the `game_world_reset_item_proven` early-return.
`CREATE OR REPLACE` resets `SECURITY DEFINER`, so 0096 re-applies it to the
two guards that hold it on main (0015 header, 0095 `ALTER`); the PG fixture
pins `prosecdef` for all four.

### Codex P1 4305018171 — activation owner capability — 2026-10-18

Accepted and repaired. The SQL rechecks of the drain digest and House evidence
compare caller values with rows a runtime login can read, so they cannot stand
in for the in-memory drain witness or the qualified catalogues. 0096 now
creates the plain NOLOGIN group `oteryn_game_world_reset` and grants EXECUTE on
`game_world_reset_activate` to it alone (revoked from PUBLIC, runtime and
control). The migration and every call refuse (42501) if a Game group holds the
capability. The rechecks remain as defense in depth. Deployment must grant the
group only to the reset owner's login (also a runtime member), which reaches the
call only after the Rust checks; that deployment row lands with the production
reset wiring (D1416 (2)). The PG case proves runtime and control are denied
with the exact evidence, and the owner capability still refuses each single
changed input.

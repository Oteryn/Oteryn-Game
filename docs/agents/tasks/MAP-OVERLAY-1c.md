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
| HOUSE-RUNTIME-1a | BLOCKING (not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/world_reset.rs`
- `apps/game-server/src/durability/item_decay_retire.rs (world reset arms)`
- `apps/game-server/src/durability/item_decay_retire_audit.rs (world reset arms)`
- `apps/game-server/tests/world_reset_postgres.rs`
- `apps/game-server/tests/support/world_reset_postgres_cases.rs`
- `docs/agents/evidence/MAP-OVERLAY-1c-*.md`
- `docs/agents/tasks/MAP-OVERLAY-1c.md` (this record: status and notes only)

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

- [ ] A world reset retires exactly the overlay items the packet names; house interiors untouched.
- [ ] Reset is idempotent and crash-safe (restart mid-reset completes).
- [ ] Retire duration within MAP01-RESET-RETIRE-MS.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

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

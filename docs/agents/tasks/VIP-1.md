# VIP-1 — VIP list store, consent lifecycle and wire (skeleton task record)

```yaml
task_id: VIP-1
title: "VIP-1: VIP list store, consent lifecycle and wire"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row VIP-1 (row 114); §7 'VIP-*' row VIP-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/vip-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2143](https://github.com/Oteryn/Oteryn-Game/pull/2143), [#2280](https://github.com/Oteryn/Oteryn-Game/pull/2280). Historical VIP metadata source merged in #2143 but was removed in the #2280 restructure; no current VIP owner or offered capability is retained. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Restore the native VIP owner in the accepted layout, consent lifecycle/migration and complete wire/handle/relay custody; do not count historical source as current backend. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Goal

VIP store, command receipts, add-by-name invitation, accept/reject/revoke/withdraw, `vip_status` setting on PARTY-1's social settings row, groups, Premium switch-over, rate bucket and `VIP_V1` wire (not yet offered).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

VIP-1 (impl; persistence, protocol, privacy). VIP-2 (presence) is Wave 5.

## Dependencies (refreshed against protected main@3ef900c1, 2026-10-15)

| Dependency | State |
|---|---|
| VIP-0 acceptance | ACCEPTED: effective ACCEPT-BATCH #2209 §1 on protected main |
| PARTY-1 (`game_character_social_settings`) | IMPLEMENTED: migration 0046 and native world_party.rs |
| PREMIUM-1 | MERGED (#2110) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `crates/gameplay/domain/src/domain/vip.rs`
- `crates/gameplay/durability/src/durability/vip.rs`
- `apps/game-server/src/lifted_tests/durability_vip_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/vip.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/vip_tests.rs`
- `crates/protocol-oteryn/src/vip.rs`
- `crates/protocol-oteryn/src/vip_tests.rs`
- `apps/game-server/tests/vip_postgres.rs`
- `docs/agents/tasks/VIP-1.md` (this record: status and notes only)

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
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- crates/gameplay/durability/src/durability/core_modules.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- PARTY-1 social settings table (add `vip_status` column in this migration)
- connection.rs dispatch chain; capabilities.rs `GATED` (`VIP_V1`, NOT in offered sets)
- crates/protocol-oteryn/src/lib.rs `pub mod`

## Inseparable bundles (never split across PRs)

- migration + VIP writer + durability tests
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Invitation ids never reused; consent required before status is visible.
- [ ] Only the inviting character sees status (V2).
- [ ] Caps and rate bucket from RESOURCE_LIMITS_REGISTRY; `VIP_V1` not offered.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server vip
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md §3-§7, §9, §13, §14
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## Owned preparation (2026-10-15)

This round is authoring on exact draft base `c09e640f7fce59f7b373c7eb5636a35b98ba34e4`;
no merge/rebase or shared registration is performed. Protected acceptance snapshot
`3ef900c160c9713092a86bfcd1a8377c7587c898` contains ACCEPT-BATCH-2026-10-15 §1 and VIP0's
accepted §§4–6/§9. The skeleton's candidate/absent-PARTY assumptions are superseded above:
`0046_world_parties.sql` already owns the actual social settings and blocks tables.
This round's exact lease covers only domain/vip.rs, durability/vip.rs, vip_tests.rs,
tests/vip_postgres.rs and this record; broader historical skeleton paths do not authorize
wire, protocol or shared module changes. The allocated root wrapper is intentionally not
created while the private native storage module is unregistered.

### Concrete completed subset

Preparation implements only **Withdraw by entry_no, Reject and Revoke** of existing consent.
It does not implement invitation creation, acceptance, entry/group editing, hide-status,
Premium-sensitive growth, presence or the wire. Existing consent must have the target view
row produced when its invitation was created (§5.1); missing view state rolls the whole
transaction back rather than manufacturing PARTY defaults. The proposed settings columns and
VIP tables exist only in the explicit test fixture, not an installed migration.

The private native `DurabilityRoot::end_vip_consent` opens/commits the existing semantic
transaction. It obtains recovery provenance from the real root-bound capability and calls
native `assert_recovery_fence` **before** reading the immutable receipt. A matching historical
receipt returns before session, node, lease/scope, locks, rate or state checks; changed actor,
operation or canonical SHA-256 request binding conflicts. New commands call unchanged native
`assert_session_fence` after admission-relation locks. That checker resolves the actual
session Account+World, generation, Character lease, distinct WorldId/ChannelId, scope
assignment, registered node incarnation and admission guards from the database.
No receipt or request reconstructs current authority.

Withdraw resolves the stable local entry number from the actual owning Account+World list,
then rechecks its mapping under the list lock. A disappearing entry becomes NOT_FOUND;
a concurrently appearing/different target retries without touching an unlocked root.
Reject/Revoke bind the full private invitation key and exact never-reused invitation number;
the target is always the independently fenced acting Character. No new public ID is created.
All involved Character roots lock in ascending CharacterId, then both possible list rows lock
in ascending (AccountId, WorldId), followed by receipt recheck. No root revision is read as
an authority fence or advanced; the code touches no items.

Fresh outcomes, including NOT_FOUND and EXHAUSTED, are receipted atomically. Every fresh
command advances the acting receipt/rate list once; successful cross-account target operations
also advance the watcher list once. Successful erasure advances the target consent-view
revision once and retains its shared entry/groups and other inviters' consent. Expired pending
rows read as absent; accepted consent does not expire with its invitation deadline. The
persisted bucket is 20 writes, one refill per 3 seconds, shared by Account+World; the clock is
read after blocking list locks. Future/corrupt bucket times fail closed. No cap/Premium check
blocks consent termination: these operations never grow entries or groups (VIP0 §6).

### Qualification and remaining integration

Native tests reuse the existing `bestiary_postgres_harness`: its real recovery root, current
session/guards, registered node and assigned scope are checked by unchanged native functions.
Administrative relationship rows are explicitly test-only consent fixtures, not gameplay
invitation/acceptance producers. The fixture extends the existing PARTY settings table and
creates the proposed private VIP tables/sequence; no production role receives DML rights.
The first overlay compile failed because a test used unavailable tokio::join!, a concrete
new-test compiler failure rather than product/regression or PostgreSQL setup evidence.
The second run executed 11 tests (4 passed, 7 failed): added fixture Characters lacked their
native creation receipts, so the unchanged recovery integrity check correctly refused them
before the VIP writer. This was a fixture-integrity setup failure. The repaired fixture uses
the actual bootstrap-intent decoder/binder to persist matching historical creation receipts;
it still obtains recovery/session authority through the native database checks.

Overlay-3 then passed all 11 focused native library tests (2 pure and 9 PostgreSQL 17.6;
zero skipped). Cases cover exact invitation/state/expiry, retained shared entries/groups and
other inviters, Account+World isolation, independently stale native fences, recovery-before-
receipt and receipt-before-ended-session ordering, immutable refusal/rate receipts, rollback
on missing target-view state, and competing distinct/identical commands. Native roots retain
CharacterRevision 1. The final source differs only by two Clippy-requested nested-if to
equivalent let-chain rewrites, recorded with both source hashes and an exact syntax diff;
overlay-4 checks the final bytes with full native library Clippy. The executed-test hash
remains explicitly bound to overlay-3 rather than claimed as an execution of overlay-4.
Final native library Clippy (`-D warnings`), workspace/new-source formatting, governance
validation and its 124 unit tests passed; the complete owned patch passes whitespace checks.

The disposable native validation overlay contains exact copied source plus only literal
VIP module registration lines in its own domain/mod.rs and durability/mod.rs. These are
validation changes, not candidate wiring. Source hashes, original shared-file controls,
logs and the overlay manifest remain in `/workspace/oteryn-prep/vip-1-evidence`.
No auto-discovered root wrapper with invalid private imports is published.

**cross_domain_finding / CP slot:** production VIP migration/table constraints, per-operation
SECURITY DEFINER implementation/grants (runtime must not DML these tables), receipt retention,
module registration, relay-listener readiness and list/consent hints, per-session handle
resolution and VIP_V1 wire/capability/registries must land under their owning slot. The current
private Rust transaction is storage/authority preparation tested with the isolated admin-owned
fixture; it is not a production privilege-boundary qualification. Complete VIP-1 and every
full acceptance checkbox above remain pending, including invitation producers and Premium caps.


## Additional owned Accept preparation (2026-10-15)

This increment starts from published draft head `595cf1fb7ba22783236aed1068578d9c5c47d8c1`
with the exact five-path `vip-1-accept-native-2` allocation. The effective owning contract
remains accepted VIP0 §§4/5.1/9 (the historical brief's CANDIDATE heading does not supersede
ACCEPT-BATCH acceptance). No main merge, shared registration, installed migration, production
role grant, public identifier or capability change belongs to this increment. The original
preparation and its qualification above remain historical evidence.

### Bounded native implementation

`DurabilityRoot::accept_vip_invitation` accepts a typed private request with the acting
Account+World, CommandRef and complete retained invitation binding. It reuses the original
`end_vip_consent` native semantic transaction spine, including root-bound recovery provenance,
recovery-before-receipt and receipt-before-live-fence ordering. Fresh operations independently
resolve the actual current target session, connection/Character-lease/scope generation,
Channel assignment, node incarnation and admission guards through unchanged native code.
The acting target is derived from the fence; a caller supplies no target authority boolean.

The writer locks the participating live Character roots in ascending CharacterId and both
Account+World lists in ascending key order, then rechecks receipts. Under those locks it reads
the database clock, locks the actual matching consent row and compares its full key plus
never-reused invitation_id. Only an unexpired PENDING row becomes ACCEPTED. Already accepted,
expired, absent and replaced invitation bindings answer NOT_FOUND. The target-root lock
serializes the count of accepted consent rows across watcher lists; count 100 or greater
answers LIST_FULL. This does not grow the shared entry/group list and invents no Premium
activation rule. A successful Accept changes the watcher-list revision and target consent-view
revision exactly once; the acting receipt/rate list advances once (also on fresh refusals),
with no second watcher-list increment when the keys coincide. CharacterRevision remains
unchanged. Existing Withdraw/Reject/Revoke bindings and digest prefix/codes are preserved.

Accept's operation code 4 and LIST_FULL result code 4 are private proposed receipt values,
not wire tags or registry reservations. Only the existing UNINSTALLED test schema's closed
kind/result checks are extended. A pending invitation fixture is administrative relationship
DATA with complete native root history; it proves neither an invitation producer nor
production consent authority. PARTY0046 settings remain actual native rows; a missing target
view is corrupt producer state and rolls back the entire Accept rather than manufacturing
settings defaults. Add/block lifecycle, relay hints and final socket delivery remain outside
this component. No auto-discovered root wrapper is created while native modules remain
unregistered; the allocated tests/vip_postgres.rs remains absent.

### Exact qualification

On the final three Rust sources, the exact native validation overlay passed all **17 tests**
(3 pure and 14 PostgreSQL 17.6; zero failed, ignored or filtered), including the six new
Accept bodies and all eleven existing controls. The real native harness asserts server version
`170006`; no database skip is counted as a pass. New cases cover exact invitation identities
and replacement, pending/accepted/expiry and count 99/100/101 decisions, actor/session/lease/
scope/node/account/World mismatches with pending state/view unchanged, receipt replay before
ended-session checks but after recovery, immutable binding conflict, rate/refusal replay,
missing-view rollback, and two independently created and warmed native Roots competing on
identical commands and distinct watcher lists at the cap. Ninety-nine relationships are
explicit historical fixture DATA with valid native root history, not 99 admitted actors.
Native admission relation locking also serializes those Root passes; the test is not an
isolated row-lock stress/throughput qualification.

Fresh Cargo dependency capture intentionally stops the global native unit root with sentinel
86 (Cargo 101); that is setup capture, not a test pass. Under the same shared Rust lock,
original whole-native unit metadata and Clippy (`-D warnings`), owned/workspace formatting,
normal cfg(test) native rlib compilation, driver linkage and the 17 original bodies all
returned zero. The normal rlib discloses baseline unused-test-scaffolding warnings because it
has no global harness; the clean original-unit Clippy result is separate. Direct dependency
artifact hashes match before/after execution. No whole-server/global unit ELF or native
production runtime qualification is claimed.

The outside-tree snapshot contains 6024 exact original files, only literal domain/durability
VIP registrations, and three recorded runner deltas: remove the 17 owned test attributes with
parent visibility, append the private module dispatcher, append the library forwarding entry.
Test bodies, actual private native authority functions, real protocol dependency and native
fixture remain unchanged. Draft shared files are byte-identical to the allocated base.
Evidence, commands, complete source/transform/dependency bindings and raw results are retained
in `/workspace/oteryn-prep/vip-1-accept-native-2-evidence`.

Governance/scope validation is recorded in the final bounded evidence manifest; all original
acceptance checkboxes above remain pending.

The existing full VIP-1 acceptance checkboxes remain open. Invitation production, entry/group
editing/removal, hide-status, admitted name resolution, Premium-sensitive list growth,
production migration/SECURITY DEFINER functions/grants and receipt retention, shared native
registration, relay/hints, per-session handle mapping, VIP_V1 wire/capability and full runtime
activation remain their actual CP integration gates. This is a bounded Accept storage
component, not complete VIP-1 or production privilege-boundary qualification.

## Additional owned native Add preparation (2026-10-15, authoring)

This increment starts from published draft `7df915ac0bae99292c7625d4a570d9f789de6269`.
Its exact three-file lease covers durability/vip.rs, durability/vip_tests.rs and this record.
Accepted VIP0 §§4/5/5.1/6/9 and PREMIUM-ACTIVATION0's sealed pre-delivery exception govern
the component. No shared registration, production migration/grants, public identifiers,
wire dispatch, main merge or runtime activation is part of this increment.

`DurabilityRoot::add_vip` now composes the actual closed `SpellPremiumCoordinator` gate
internally. It accepts no Premium status/boolean supplied by a caller. The sealed
`NotActivated` branch uses only the gate-supplied physical transaction, after the native
shared latch lock; its nested savepoint rolls back all VIP writes on callback error even
though the enclosing native gate commits callback output. Delivered branches use the
actual consumer and registered trusted clock: Current permits 100 entries and every
NotCurrent classification permits 20. Loaded historical ACTIVE evidence alone cannot
authorize growth. Lapse leaves existing entries and memberships intact.

Recovery and a bound historical receipt are checked before the Premium gate or new live
session/cap/rate checks. Fresh writes reuse native recovery, admission, current session,
connection/lease/scope-generation, assignment and node checks. The acting Account+World
must match the actual current session. The native CharacterName parser checks the name;
the reservation primary key selects a bounded candidate only. Actual same-World live
root name/lifecycle facts are revalidated under sorted actor/target root locks. Offline
targets need no online spell/session lookup; unknown and other-World names return the
same receipted UNKNOWN_NAME, and self-add is REJECTED.

Under the list lock, a second receipt check precedes the shared rate bucket, cap and
entry/invitation changes. The Account's other characters share an entry but retain their
own consent rows. Fresh PENDING rows expire at the database clock plus seven days;
expired rows receive a new NO CYCLE invitation identity. Target blocking and the global
20-pending cap leave the same successful UNCONFIRMED entry without a consent row or a
target consent-view revision increment. Only a newly visible PENDING advances that view;
CharacterRevision never advances. Entry numbers and invitation identities are not reused.
The shared receipt decoder preserves prior End/Accept bindings, private codes and outcomes;
the private uninstalled fixture proposal adds closed Add result/entry-number fields only.

The target must already have an actual PARTY0046 settings row with the admitted source
policy and valid settings. Missing/corrupt settings fail unavailable with full rollback.
This component never manufactures PARTY defaults or target SpellItemAuthority. The real
first-admission initializer requires target authority and its production caller remains
a CP gate. Native0022 currently admits only live immutable names; legitimate deletion/
rename operators are not available to fabricate as fixture mutation authority. Future
owners must retain the same locked-root identity/lifecycle checks.

### Source qualification and pending execution

All 17 prior test function bodies remain byte-identical to the allocated base: three pure
and fourteen native PostgreSQL bodies. Eight authored Add cases bring the selected family
to 25 cases (three pure, twenty-two PostgreSQL). They cover current/free/lapse/historical
Premium and exact 20/100 boundaries, actual offline/same-World identity and charged unknown/
invalid refusals, shared/per-actor rows, expiry/fresh identities, block/cap privacy, callback
savepoint rollback, replay after live-fence loss, independent assignment revocation, two
actual Root races and the actual Premium latch writer. The latch observer verifies the
entry-writing backend holds the real shared latch lock before permitting the writer race.
New PostgreSQL bodies require the real configured PostgreSQL17.6 route; no missing database
skip is counted as Add qualification. Historical root/relationship/settings rows are
explicit fixture DATA; Premium current evidence uses the actual snapshot validator,
durable fence and in-process consumer ingest, never a fixture current-authority flag.

At this authoring checkpoint, no new Rust, Clippy or PostgreSQL execution is claimed.
Heavy qualification and large hydration are held for the shared validation slot. Exact
source, original-body preservation, authority/operator matrix and subsequent raw results
belong in `/workspace/oteryn-prep/vip-1-add-native-3-evidence`. The existing acceptance
checkboxes remain open: groups/edit/remove/hide, target baseline caller, production schema/
definers/grants/retention, native registrations, handles/relay, protocol and VIP_V1 offering
remain incomplete. This is native Add component preparation, not full VIP delivery.

### Final native Add component qualification (2026-10-15)

The preceding authoring checkpoint is superseded by actual generation 4 qualification:
**25 original selected test bodies passed**, three pure and twenty-two PostgreSQL 17.6,
with zero failed, ignored, measured or filtered in 36.07 seconds. The unchanged native
database fixture asserts `server_version_num=170006` for every PG case; all eight new Add
bodies additionally log that exact version and require a configured database route.
None is counted through a missing-database skip. All seventeen original complete test
functions and intervening helpers remain a literal 36,304-byte tail prefix from the
allocated base (SHA256 470facaba44bb360aec1448fe2d897e13700365e9e2009bc3a290cb334e6eeef).
The root fixture helper gained a World parameter for the other-World negative; its original
entrypoint still supplies the same native World and historical bootstrap facts.

Fresh Cargo capture intentionally returned 101/sentinel 86 before global root codegen; it
is dependency capture, not a test pass. Under one shared Rust lock and the same captured
dependency graph, selected native cfg(test) metadata/Clippy, normal native rlib compilation,
driver linkage and the 25 bodies returned 0. The configured Clippy denials remain active;
baseline unused Rust scaffolding warnings remain visible. This is not a blanket `-D warnings`,
original global-unit metadata/full-server, production registration or gameplay E2E claim.
Source and extern hashes match before/after execution. The selected snapshot binds 7,670
original input rows to this exact HEAD or owned candidate, plus traced literal/macro inputs.
It adds only literal validation-only domain/durability VIP registrations and mechanical
selected-test attribute/parent-visibility changes with dispatch/forwarding glue; test bodies
and native authority/fixture implementations remain unchanged. No tracked shared root changes.

Actual PG outcomes include exact free 20/current 100/pre-delivery 100 boundaries, native
validated/fenced/ingested Premium versus loaded-only ACTIVE history, failed pull/missing clock/
expired lease, offline and other-World names, malformed-name and self refusals, shared entries
and per-actor consent, expiry/new identity, block/pending-cap-20 privacy, missing target baseline,
after-entry failure rollback, historical replay before lost live fences, independently revoked
native assignment, duplicate/cap races through two warmed Roots, and a real activation-latch
writer race. Native admission EXCLUSIVE relation locking also serializes the two-Root tests;
this is not isolated fine-grained row-lock stress or throughput qualification. The observer
uses only numeric native/test lock constants and verifies the actual writing backend's
shared latch lock; waiter observation is scoped to its fixture database. Its generated DDL
uses pinned SQLx's explicit owned AssertSqlSafe<String> API, never request data.

All earlier stops are retained honestly: two pre-Cargo resource-gate setup stops, two actual
selected compile failures (E0597 from the fixture's dynamic borrowed SQL), then one actual
Clippy failure for identical new REJECTED branches. The borrowed raw_sql change alone did
not satisfy pinned SqlSafeStr; its real owned-string contract and existing native fixture
pattern provided the correction. The identical refusal predicates were combined without
changing outcomes. These were not behavioral RED tests; no PG body ran before generation 4.
No lint suppression, dependency/feature change or native predicate relaxation was used.

All commands, raw logs, exact input/body/source/extern bindings, resource observations and
failure dispositions remain under `/workspace/oteryn-prep/vip-1-add-native-3-evidence`.
Generation 4's three completed custom metadata/rlib/driver outputs were losslessly archived
only after execution ended and current/future-consumer checks, with raw/archive/decompressed
hashes, modes/mtimes and exact restore argv. Their source, logs, shared dependencies and
native anchor remain intact. Final card governance and three-file scope/source manifest
bind this frozen packet; independent non-author review and exact immutable candidate C
binding precede root-controlled publication. All full-family acceptance checkboxes and
production schema/definers/grants/target-baseline/wire/relay/runtime integration gates stay open.

## Additional owned native Remove preparation (2026-10-15, source phase)

This successor starts from the published Add candidate
`4efa8e4addd4f8ddf6eb355934a184fadab44cbd`; the frozen Add sources and evidence remain unchanged.
The exact three-file lease prepares accepted VIP0 §§5, 5.1 and 6 `Remove {entry_no}` through
`DurabilityRoot::remove_vip_entry`, reusing the existing native consent transaction spine.
Its dedicated typed request selects only the acting Account+World's current entry number.
Actual recovery, receipt replay, current session/node/assignment, sorted Character roots,
list locks, locator revalidation, persisted rate and immutable receipt all remain native.
There is no Premium argument or benefit/current-authority callback. The private SQL receipt
arm 6 is a proposed local persistence discriminator, not an allocated protocol opcode.

The same transaction deletes that shared entry. Existing FK cascades remove all attached
Account-character pending/accepted relationships and entry group memberships; group definitions,
other entries/Accounts, list history/rate, retained receipts and never-reused allocators survive.
An independently read physical consent row, including retained expired Pending, requires exactly
one target consent-view increment under the locked target root. An unconfirmed entry with no
consent requires neither a settings row nor a new target baseline. A missing required view or
its overflow aborts the whole transaction. No CharacterRevision/item/presence mutation occurs.
The first locator read is DATA for root selection; the locked-list recheck cannot reinterpret a
new mapping as permission to delete an unlocked target. Unknown numbers are receipted NOT_FOUND.

Six appended genuine PG families are authored: Account-wide cascades with mixed consent and
retained custom group definitions/other relationships; no-consent and expired/blocked entries
plus overcap removal without Premium; rate/refusal/historical replay and strict post-Remove
entry/invitation identity; missing-view/overflow/after-delete rollback; current native fence,
node, independent assignment revocation and recovery-before-replay; and two warmed Roots for
duplicate/distinct Remove and Add/Remove serialization. Their full-row snapshots include
accepted state, target view, list/rate/allocator, memberships and receipts. The custom group
catalogue is minimal, explicitly UNINSTALLED test DATA, not a group writer or admission claim.
The existing real PG wrapper asserts 170006 and refuses missing database configuration. New
bodies additionally emit VIP-REMOVE-PG version markers. Native admission relation EXCLUSIVE
locking also serializes these transactions; no isolated row-contention/load claim is made.

All twenty-five inherited test bodies and the complete intervening helper tail are preserved
literally. Source-phase total is 31 authored cases (three pure, twenty-eight PG), not an
execution result. Heavy qualification remains on hold behind the allocated native lanes;
source/authority matrix, hashes, subsequent actual logs and failure dispositions belong in
`/workspace/oteryn-prep/vip-1-remove-native-4-evidence`. No source-only check is called a native
PASS. Production migration/definers/grants, target baseline admission, groups/edit/block/hide,
shared module/caller/relay/handles/protocol/capability and full VIP integration remain open CP
gates. Existing acceptance checkboxes and slot-only builder requirements are unchanged.


## Remove: actual native qualification and final checkpoint — 2026-10-16

The complete private Remove component now has **actual 31/31 native bodies passing**:
3 pure and 28 genuine PostgreSQL 17.6 cases, including all 25 inherited bodies and
six new Remove bodies. The unfiltered driver reports zero failed, ignored or
filtered tests. Its raw log contains exactly 14 Add fixture and six Remove
`server_version_num=170006` markers; the 14 earlier PG bodies independently retain
the real native harness version assertion. The actual runtime invocation records
its explicit local PG admin route, so their optional-unconfigured branches were
unreachable. Raw test-log SHA-256: `bdd91a7df79503845678d8dfb15d76996df63dbd029b85ffb674e2e2a14e9623`.

Fresh native Cargo capture reached the intentional sentinel86 (Cargo101, setup
capture only). Configured selected-native Clippy, same-graph native cfg(test) rlib,
driver link and actual tests each exited 0. Existing configured Clippy denials and
unsafe prohibition remained active. The selected context retains 2,303 ordinary
Rust warnings; **strict production/global lint and the global native unit metadata/
ELF are not qualified**. No lint suppression or alternative authority was added.
The full inherited 69,163-byte test/helper tail remains literally unchanged
(SHA-256 `8343e9a317c144dc21e95ef9e9c2f9b28a1648592eac4222f81cd4bf6673495c`).
Signature-only selection reconstructs the whole formatted 31-body source exactly;
the actual Root, recovery/session/Node/current-assignment checks and fixture bodies
remain native. Source and all captured extern/proc-macro hashes stayed stable.

Failures remain evidence. Initial explicit1.94 formatting check exited1; the
Root-authorized style-only repair was independently reviewed before qualification.
Attempt1 then failed during Cargo's `rustc -vV` setup because the direct Cargo
launcher passed a relative compiler name to `execv`; no native metadata or tests
ran in that attempt. Its failure and exact input proofs remain intact. The bounded
repair explicitly pins the verified absolute1.94 `RUSTC`. Attempt2 reuses the same
fully verified 7,673-file hydration, six detached mutable roots and retained exact
preimages, with separate new logs/results; no second hydration or product repair
was performed.

Root budget9320 and typed-reuse96a9 authorized the operating stages. The historical
pre-hydration entry1,598,439,424B passed1.59GB; the later current-reuse check is
separately >=1.584GB and is not presented as another1.59GB PASS. The actual critical
pre-codegen check passed at1,521,684,480B, above1.50GB. Raw native rlib+driver total
643,171,400B, below800MB. Metadata/rlib/driver capped lossless archives succeeded
with complete producer, consumer, identity, raw/gzip/stream SHA and restoration
proofs. The512MB floor and fresh shared cgroup0OOM/0kill held. No shared dependency,
anchor, source, log or failure evidence was retired.

Coordinator evidence is `vip-1-remove-native-4-evidence/attempt-2/`; the frozen
recipe is v2e (independent review3577eaa2), and attempt1 is explicitly SETUP_FAILURE.
This append follows actual validation; the two validated Rust files are unchanged,
and the prior card is retained byte-for-byte as this card's prefix. Final source/
card, input/body/extern/archive bindings are recorded in the new final checkpoint.
Whole final independent execution/source review and Root exact-C publication remain
separate. Existing production schema/definer/grant, shared caller/wire/relay and full
VIP-family CP gates still apply; this component does not complete the VIP mechanic.
No worker commit, push, PR-state change or shared wiring edit was performed.

## Family continuation: native groups, Edit and status — 2026-10-16

Local AUTHORING starts at draft `bcb366b8a641f19d1901aa11f12ce01a6863bced`, on
`skeleton/vip-1`, for existing PR #2143. The user packet assigns the complete nine
owned paths to this task's one writer; historical narrower preparation leases do
not restrict that current allocation. No shared registration, migration, manifest,
registry, capability, connection, production, commit or publication change is made.
The root lead owns publication and integration remains CP_ONLY / NOT_REQUIRED here.

The appended native `DurabilityRoot::modify_vip_list` implements custom GroupAdd,
GroupRename and GroupRemove, complete replacement Edit and `vip_status` SHOWN/HIDDEN.
Every new command resolves native recovery/session/node/scope authority, locks the
acting live Character root before its Account+World list, rechecks the immutable
receipt, samples database time after blocking locks, charges the existing persisted
bucket, advances the acting list once, and commits outcome/state/receipt atomically.
No CharacterRevision or item changes. Private fixture receipt kinds 7–11 and results
8–9 are proposed storage discriminators, never wire IDs or public reservations.

Group storage is custom numbers 4–8. Fixed memberships 1–3 remain valid. Group removal
atomically deletes only the selected custom definition and its memberships, retaining
entries, consents and fixed memberships. Duplicate names use the same database
`lower(name)` comparison as the fixture unique index. Text/icon/group shape rejects
invalid data; an unknown custom membership refuses Edit before changing any field.
Only GroupAdd/Rename consult the real Premium coordinator, including its shared-latch
pre-delivery transaction and rollback savepoint. Delivered Free allows neither;
Edit/GroupRemove/Status do not need a Premium read. The conservative rename rule
requires count below the current cap, including count==5 refusal, following §6's
literal "until it is under the cap"; an exact boundary test pins that interpretation.

Status writes update the genuine PARTY settings row with its accepted source_policy,
under the acting root lock. A missing baseline rolls back rather than manufacturing
PARTY fields. Its genuine admission initializer, proposed VIP columns and subsequent
presence hint remain integration dependencies; the local writer does not claim the
accepted missing-row/default/full status lifecycle is delivered. A status write does
not change the target consent view or its revision; visibility publication is VIP-2.

Eight appended tests (one pure, seven native PG) cover text/icon/group boundaries,
fixed/custom group bounds, case-insensitive names, current caps, free retention and
metadata edits/removal, immutable replay, rate refusal, unknown membership, injected
membership/receipt failures, missing-setting rollback, all native fence inputs,
node mismatch, direct-core stale recovery before receipt, and two warmed native Roots
for duplicate-command and case-insensitive-name races. Delivered GroupRename and
non-growth Status separately reject a stale generation, an actually ended session
and a real assignment-owner revoke. All 31 inherited tests and helpers remain a
literal byte prefix. No new test reports unconfigured PostgreSQL as a pass; the
existing required native runner asserts PostgreSQL 17.6. Native execution results
belong to the lead's source-bound validation evidence, not source-phase claims.

**CP finding disposition:** comment 6215954832 requests receipt replay after the live
session fence. Accepted VIP0 §4 explicitly mandates current recovery, historical
receipt, then fresh-command session checks. Its replay preserves a stored outcome
after session end and changes no state. This continuation preserves the accepted
contract and inherited replay tests. CP must correct its P2 or obtain an accepted
contract amendment before that ordering changes; this is not silently declared fixed.

**Remaining own scope / recheck triggers:** no approved wire field layout/numeric
reservations or complete handle/relay custody contract is present in the bounded
sources. No speculative protocol/transport stub or invalid private-import PG wrapper
is created. Full VIP_INTENT ingress/snapshot/deltas, session handles, committed reply
and relay publication, installed schema/definers/grants/receipt retention, genuine
social-baseline initialization and production socket tests remain open. Recheck on
the accepted wire/handle/listener decision and exact CP wiring/migration slot. VIP-2
owns presence/index/reconcile/delivery. Full VIP-1 acceptance remains pending, and
this partial native metadata component does not authorize ready, queue or merge.
# Native qualification of the 2026-10-16 metadata increment

The lead executed the unchanged native library source in an isolated validation checkout,
with only the intended domain/durability VIP registrations added outside this branch.
Rust 1.94 `cargo check --locked -p oteryn-game-server --lib` and strict
`cargo clippy --locked -p oteryn-game-server --lib --quiet -- -D warnings` passed.
All 39 VIP cases passed (4 pure, 35 PostgreSQL 17.6; zero failed or ignored;
3179 unrelated native cases filtered). PostgreSQL reported `server_version_num=170006`.
The original 31 cases and all original source bytes remain literal prefixes.
The native executable and a manifest of its 876 compiler-reported source inputs were retained.

The required broader `--all-targets` Clippy check was attempted and remains RED in
the registration overlay: standalone integration-wrapper crate roots do not provide
the real gameplay/Premium/test-harness modules now required by the registered VIP
owner. The native library and its VIP tests compile and execute; this is not a
global strict pass. Coordinated module/wrapper activation remains a CP slot task.
Canonical shared registration files were not changed. Wire, production grants,
runtime callers, final socket custody and the contradictory CP replay request remain
the specific gates recorded above; no ready/merge or full-feature completion is claimed.

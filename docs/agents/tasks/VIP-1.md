# VIP-1 — VIP list store, consent lifecycle and wire (skeleton task record)

```yaml
task_id: VIP-1
title: "VIP-1: VIP list store, consent lifecycle and wire"
mode: IMPLEMENT
status: waiting
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

- `apps/game-server/src/domain/vip.rs`
- `apps/game-server/src/durability/vip.rs`
- `apps/game-server/src/durability/vip_tests.rs`
- `apps/game-server/src/gameplay_transport/vip.rs`
- `apps/game-server/src/gameplay_transport/vip_tests.rs`
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
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

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

# LOOP-GROUND-1 — Ground cell limit (task record)

Current rule: ACCEPT-BATCH-2026-10-15 §§2–3 accepts **64 physical Ground roots**,
overflow **"not enough room"**, and **no periodic ground cleanup between resets**
(ADR-0021 D191). Earlier decay wording and policy-decision blockers below are
historical. The current implementation remains incomplete: production writer
integration and DEATH-DUR-1 are blocking, not deferred slot-only work.

```yaml
task_id: LOOP-GROUND-1
title: "LOOP-GROUND-1: ground loop: dropped item decay and cell limit"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-GROUND-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/loop-ground-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Items on the ground (drops, corpses' spill) decay under one durable policy and each Ground cell enforces a fixed item limit, closing the ground loop after death batch B.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; split only after the decay-policy decision lands.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| DEATH-DUR-1 (death module batch B) | BLOCKING (not merged) |
| decision on dropped-item decay policy and Ground cell limit | BLOCKING (explicitly not decided: DEATH-COMPLETE-0 §6, remaining_unknowns; architect/owner decision needed) |
| acceptance of DEATH-COMPLETE-0 | BLOCKING (CANDIDATE) |
| MAP-OVERLAY-1a Ground rebuild | MERGED (#1800) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/item_decay_retire.rs`
- `apps/game-server/src/durability/item_decay_retire_audit.rs`
- `apps/game-server/src/durability/ground_cell_limit.rs`
- `apps/game-server/src/durability/ground_cell_limit_tests.rs`
- `docs/agents/evidence/LOOP-GROUND-1-*.md`
- `docs/agents/tasks/LOOP-GROUND-1.md` (this record: status and notes only)

### D1339 scope amendment

D1339 grants the fixture repair of the 90 foreign `character_authority_postgres`
failures caused by Ground64 admission: harnesses publish runtime readiness through
`ground_native_readiness_enforcement::prepare` and opaque Ground positions become
canonical 10-byte Ground64 addresses. No assertion, skip or production change.

- `apps/game-server/tests/support/item_mint_postgres_cases.rs`
- `apps/game-server/tests/support/item_transfer_postgres_cases.rs`
- `apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs`
- `apps/game-server/tests/support/map_item_mint_postgres_cases.rs`
- `apps/game-server/tests/support/corpse_transfer_postgres_cases.rs`
- `apps/game-server/tests/support/combat_death_reward_postgres_cases.rs`
- `apps/game-server/tests/support/type2_audit_activation_postgres_cases.rs`

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

- map/overlay.rs and map/overlay/pickup.rs (Ground overlay, MAP-OVERLAY-1a)
- durability/item_decay_retire.rs existing retire arms
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json (cell limit)

## Inseparable bundles (never split across PRs)

- decay schedule + retire writer + audit
- cell limit (RESOURCE_LIMITS) + drop refusal + tests

## Acceptance criteria (current accepted rule)

- [ ] Ground items survive crashes and follow D191 planned-reset retirement; no periodic dropped-item cleanup is introduced.
- [ ] A drop beyond the cell limit is refused with a typed "not enough room" code.
- [ ] Admission never removes or hides existing custody, including overfull tiles and owned corpses.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ground
cargo test -p oteryn-game-server corpse_decay
cargo test -p oteryn-game-server item_decay
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md §6 (remaining_unknowns)
- PR #1800 (MAP-OVERLAY-1a)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local bounded preparation — 2026-10-15 (not runtime acceptance)

Allocation: existing draft #2158, `skeleton/loop-ground-1`, authoring base
`efc7427095af92b969ca9341da5845d7bcca695b`. This allocation permits only
`durability/ground_cell_limit.rs`, its adjacent tests, and this record; the
historical broader owned-path list above is not an active write grant. No
commit, push, module registration, migration, registry, content or pin edit.

The independently retained protected-main decision at
`e2da9805d296a27b06b32335bf67394509d4910b`,
`OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` §§2–3, releases the
old undecided policy gate: **64 items per tile**, overflow **"not enough room"**,
and **no periodic ground cleanup between resets**. It accepts DEATH-COMPLETE-0
and retains ADR-0021 D191 (crash survival and planned reset). Historical
decay/checklist/dependency text above is preserved as history, not an instruction
to introduce a timer or retire writer. No new timed cleanup is prepared here.

### Verified native gap and prepared source

- Existing native Ground MINT locations in `item_mint.rs`, `map_item_mint.rs`,
  `native_map_items.rs` and `spell_item_transaction.rs` have no per-tile 64-root
  admission check. The spell transaction's 500-row snapshot bound is a read-work
  limit. `map/overlay.rs::add_ground` intentionally preserves durable truth on
  rebuild; neither that overlay nor custody writers were duplicated or edited.
- `ground_cell_limit.rs` checks actual native `GroundItemInstance` data against
  independently supplied typed World/Channel, ownership generation and native
  `SpellGroundTarget`. It reuses the native signed 10-byte Ground codec and
  rejects mismatched cell/revisions/placement context or duplicate item IDs.
- It counts physical top-level Ground roots, not stack quantity. The caller
  supplies complete remaining live roots after actual removals and actual new
  roots after the native stack planner has decided merges. Contained descendants
  belong to the separate ITEM-MOVE §5/BAGS per-channel counter, not this root
  count. Static base-map decoration has its separate ADR-0021 base-item bound;
  materialized map items in Ground custody are ordinary supplied roots.
- The helper only checks a plan. It does not establish current authority,
  completeness, ordering, an allowed merge, a destination or reserved capacity.
  It has no mutable owner, expiry, cleanup, writer, grants or persistence.

### Remaining CP/architect integration obligations

Runtime acceptance remains unchecked. CP must register and integrate the check
at every actual root-insertion boundary, including player drop, death spill,
loot/map/spell MINT and inventory overflow, with complete live reads under the
accepted tile/custody locks and the same physical transaction as the inserts.
Current ownership, session-generation, recovery, admission and content fences
remain each existing writer's independently resolved obligations. Caller data,
prepared rows and a successful helper result are not current authority.

Preserve native top-stack ordering/merge rules and per-channel counter updates;
do not guess an overflow destination or redirect items on refusal. A retry or
reload must independently reread occupancy and recheck. Removal-only and rebuild
paths must preserve durable rows, including preexisting overfull tiles, without
using admission refusal as cleanup. Death writer/schema integration, resource
registry/error mapping and slot-only wiring remain later CP work. No PostgreSQL
concurrency, rollback, restart durability, full-server, wire/E2E, architect
acceptance or independent exact-head review is claimed by pure helper tests.

### Actual local validation

- **8/8 passed** in an exact-source standalone Rust harness: 64/65 admission,
  batched insertion, physical stack/root counting and zero-new-root merge,
  actual removal/replacement, duplicate IDs, independently varied World/Channel/
  generation, each tile metadata field, opaque/truncated addresses, and repeated
  reload evaluation without mutable state. The last case is pure determinism,
  not proof of database restart durability. These new tests have no executed
  behavioral RED predecessor; no setup failure is reported as a RED test.
- Standalone `clippy-driver` passed with workspace Rust/Clippy deny settings
  and `-D warnings`; owned `rustfmt --check` and `cargo fmt --all --check` passed.
  Rust commands ran under `/tmp/oteryn-prep-rust.lock`, shared target/jobs 2,
  `CARGO_INCREMENTAL=0`. No server/Cargo test binary or migrate link was built.
- Governance validation and **124/124** agent unit tests passed. Whole owned diff
  reviewed; no new cleanup, authority source, writer or guessed stack merge.
  Scope and whitespace checks passed for exactly the three allocated paths.
- Durable evidence: `/workspace/oteryn-prep/loop-ground-1-evidence/`, including
  real raw logs, reproduction scripts and before/after bindings of five actual
  source files. The harness imports both new files directly and uses nine
  hashed, verbatim native type/ID/codec excerpts; no writer/authority stub.
- Source SHA-256: helper
  `54cae6c36bd6bef274673e3c5bb5d68fe6fa7e414d1f34bc51da5afec701c984`;
  tests `d3b3f1fa32acce8ca5909966c6f42fffe20825dd33c60b7c8491c5e08ac48c5c`;
  harness `f25b85af35bc9ac04e4194d103964ae111232e97c6dc163977b292e73f5fb08b`.

The detached owned subset is prepared, not accepted as the full LOOP-GROUND-1
implementation. The original runtime acceptance boxes remain unchecked.

## Native capacity owner, generation 2 (authoring checkpoint; 2026-10-15)

Existing draft #2158 at `8c2e00b6dd3f74fe89cf72acb9980ad1d14061c7`
has a new exact four-path local allocation: the Ground helper, its tests, the
original PG wrapper and this card. Accepted Batch §§2–3 remains the rule owner:
64 physical Ground roots, “not enough room”, no periodic cleanup. The original
decay-policy/candidate gate is historical; no new decay implementation is made.

`check_ground_cell_root_admission_in_transaction` consumes the existing sealed
native `SpellItemScopeAuthority` and an actual borrowed physical transaction.
It reuses `locked_tile_rows_for_scope`, whose genuine producer independently
checks Root recovery, Node incarnation, current scope assignment/readiness and
native admission, and holds the existing Channel key-33 lock. The reader checks
the physical transaction, actual live row placement/generation/order and its
existing complete 500-visited-item bound. Unknown or incompatible rows are never
filtered into an empty/cached/truncated fallback. Actual current rows become the
native remaining inputs to the unchanged count/identity policy; the caller
cannot supply a remaining/removal DTO.

The caller must read after its actual removals and before its exact prospective
insertions in that same transaction. The result neither writes nor commits,
reserves room beyond the transaction, authorizes an insertion, nor qualifies
current Character/item/content/map/frame facts. In particular, a target matching
an empty tile is still target DATA; only the eventual actual writer can prove
the current admitted target. No new authority wrapper, getter, guard predicate,
cause, schema, clock, role, wire code or migration is introduced.

Zero new roots incur no capacity charge, even for preexisting overfull rows;
native transaction/source checks remain. Removal-only and rebuild paths preserve
truth rather than using a capacity refusal to hide/retire it. The actual writer
still owns stack merging and overflow destinations. Every player drop, death
spill, map/loot/spell MINT and overflow insertion must later compose this guard
with its own real source/current authority and exact inserts. No claim of global
enforcement or complete atomic drop integration follows from this callable part.

### Source qualification plan (not executed)

The original eight pure test bodies remain byte-identical. Eight native PG
bodies are authored for 63+1/64+1 and 65-root zero admission; an actual native
Ground-to-container-slot transfer observed without supplied remaining rows;
foreign physical transaction; stored placement/prospective identity mismatch;
actual stale scope generation and independently registered nonholder Node;
native row locks through rollback; opaque stored source/empty-target authority
distinction; and the native 501-row complete-read refusal. The removal case
qualifies a real committed source removal followed by a fresh guarded read,
not an unimplemented removal-plus-insertion compositor.

PG fixtures use installed native migrations, genuine bestiary Root/recovery/
Node/scope/readiness fixtures, actual MINT/TRANSFER writers and unchanged native
guards. MINT causes and definition facts are explicitly native test fixtures,
not gameplay-produced kills or admitted Content. The existing test-only fresh
spell lane is fixture setup; no fabricated sealed DB authority constructor is
added. The private-root standalone target is disabled until CP registers the
module; zero tests there is not a runtime pass. Outside-tree literal native
registration/selected-body qualification is planned, with its transforms and
source bindings recorded.

No new Rust or PG result is claimed at this checkpoint. Heavy qualification
awaits the root's existing Trade/VIP/Guild queue and storage budget. Evidence
is retained in `/workspace/oteryn-prep/loop-ground-1-native-2-evidence`.
Early independent review corrected a fixture assertion to the actual native
`game_item_container_slots` Character/World/item binding; it was caught before
compilation and is not runtime RED evidence. A wrapper-context concern was
explicitly retracted after verifying this exact draft's native source, with no
source expansion. Original full-family acceptance and CP no-ready/merge/shared
slot rules stay open and unchanged. The worker performs no commit or push.

The source-only phase-3 review removes a newly added module-level
`allow(dead_code)` rather than hiding the missing production insertion callers.
The real strict-production result remains pending and any actual unused-chain
failure will be retained as RED/unqualified. Root permits a separately recorded
ordinary-Rust-warning metadata context, retaining the configured Clippy denials,
to qualify the original private-root consumer and selected native bodies after
that result. This scoped evidence cannot complete the strict/global CI gate or
substitute for the real future insertion callers. No fake caller or public
visibility expansion is used.


## Native generation 4: actual focused qualification (2026-10-16)

The bounded same-transaction Ground admission component now has an actual
**16/16 PASS**, comprising the original eight pure cases and eight mandatory
native PostgreSQL cases. All eight PG fixtures independently reported
`server_version_num=170006`; zero tests were ignored. The selected native rlib
and driver compiled and linked successfully. Actual current Root/recovery/Node/
scope constructors, native MINT/TRANSFER writers, row locks, complete-reader
bounds and negative authority assertions remained enabled.

The first genuine native run was **10 PASS / 6 FAIL**: six MINT-based fixtures
supplied an empty `corpse_ref`, which the unchanged native technical-byte guard
rejects. The shared PG fixture now supplies bounded canonical native test DATA
`id(3)`. This changes neither authority nor admission rules. All sixteen test
function bodies and the original eight-case pure prefix remain unchanged.
The actual failing run and successful successor are both retained.

Production strict metadata remains **RED/unqualified** for six unused private
Ground-chain items. The enabled original consumer strict check likewise remains
**RED/unqualified** for two unused existing native harness helpers. Separately
named ordinary-Rust-warning production and original-consumer metadata contexts,
and fresh selected-native metadata, passed with every configured Clippy denial
preserved. No allowance, fake caller, visibility widening or global strict PASS
was introduced. Production metadata reuse proves the exact retained 1,964-file
context, all 61 extern/proc-macro artifacts, actual captured argv/environment,
toolchain and restored artifact hash; the repaired leaf is excluded by native
`cfg(test)` in the unchanged production graph. Fresh consumer/selected metadata
and the native sixteen cases were executed after the fixture repair.

The root-approved measured Ground-only operating budget retained the 512 MB
reserve and all product/test gates. Earlier setup and resource-check failures
remain evidence. Runtime code hashes before and after qualification are equal;
this card update is a literal append after that completed execution. Evidence,
commands, original-body mappings and logs are in
`/workspace/oteryn-prep/loop-ground-1-native-2-evidence/`, especially
`attempt-4-tests.log`, `attempt-4-status.json` and `overlay-manifest-attempt-4.json`.

This qualifies the bounded native component and its original fixtures. It does
not complete every insertion writer, shared module registration, destination/
stack planning, current Content/map authority, full-server/wire/E2E, global strict
CI or the complete mechanic. Those CP and runtime acceptance gates remain open.
Final independent execution review and root-owned candidate publication follow
this checkpoint; the worker has made no commit or push.

## CP repair, 2026-10-16 — authoring, integration still blocked

Authority/currentness: CP review comment `6215954253` is against source head
`e1eab53df8e14ed07c10835735d2327176ed52d3`; the lead allocated one author on
`skeleton/loop-ground-1`. The active four-path allocation remains the helper,
adjacent tests, PostgreSQL wrapper and this record. Accepted Batch §§2–3 was
read from retained protected-main commit
`e2da9805d296a27b06b32335bf67394509d4910b`. No periodic decay is authorized.

P2 disposition: **fixed in the pending local delta**. The pure policy must allow
zero new roots on an already-overfull tile while still checking native identity,
scope/generation and target placement. The old 65+0 rejection assertion is
removed; separate regression cases cover 65/255+0 and malformed rows beyond the
64th root. Existing 64+1 and 65+1 refusals remain. This capacity-only repair
introduces no writer, authority constructor, persistence change or cleanup.

### Mechanic integration blockers (cross-domain findings; report only)

An exact-source sweep found no production admission caller. Four native Ground
INSERT boundaries require composition after actual removals and before actual
new roots in the same locked physical transaction, with the writer's independent
current authority, target qualification, stack plan and typed refusal:

- `item_mint.rs::insert_mint_with_corpse_attribution`: creature/corpse root MINT;
  its existing native death/scope fences must remain authoritative.
- `map_item_mint.rs::apply_mint`: map item MINT; retain its admitted map source.
- `native_map_items.rs::initialize_in_transaction`: source-map materialization;
  retain its `NativeMapInitializationProof`, current scope and Node checks.
- `spell_item_transaction.rs::apply_physical_item_line`: Player and Creature
  Ground MINT; qualify complete batches after removals rather than charging
  individual lines before their actual removal/merge plan is applied.

These files are outside the active allocation. The existing sealed
`SpellItemScopeAuthority` is not a permission to substitute caller-derived
authority for the MINT/map writers' native fences. CP must allocate the owning
durability/spell/map integration packet (F); this is mechanic implementation,
not merely module registration or queue-slot wiring.

Player drop is itself absent at this head:
`gameplay_transport/item_move.rs::step` rejects Ground destinations with
`NotSupported`. Its accepted owner is ITEM-MOVE-2b (ITEM-MOVE-WIRE-1 §5).
`character_death.rs` writes Character state only; the death spill remains owned
by DEATH-DUR-1 and remains a blocking dependency. No guessed drop/spill/overflow
destination or fake consumer was added to bypass either missing owner.

Runtime acceptance, strict full-package checks and native PostgreSQL
requalification remain pending. The existing PostgreSQL wrapper remains
disabled; executing it with zero tests would not qualify anything. Focused pure
qualification uses literal native type/codec/function/test excerpts and excludes
all async transaction code and PostgreSQL; its provenance is retained separately
under `/workspace/oteryn-family-g/evidence/2158-ground-fix/`.

Focused repair validation: the ten literal pure test bodies first ran against
the unmodified helper (**8 PASS / 2 FAIL**, the two new regressions), then against
the repaired helper (**10 PASS / 0 FAIL**, zero ignored). The same Rust 1.94.0
compiler and unchanged test bodies were used. This is behavioral RED→GREEN,
not a setup failure. The identity set now accommodates more than 64 existing
roots without suppressing validation or indexing beyond a 64-entry array.
Owned Rust formatting, workspace `cargo fmt --all --check`, focused pure
`clippy-driver -D warnings`, governance validation and all **124/124** governance
unit tests passed. Full Cargo/Clippy, PostgreSQL and global integration qualification
are not claimed by these bounded results. No commit or push was performed.

## D1096/D1097 enforcement authoring — 2026-10-16 (not runtime acceptance)

Current source is local candidate `aa91ae5b2dccb68171069e3770640c71f06f1dfe`
plus the pending owned delta. CP comment
[6219643346](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346),
updated 2026-10-16 16:11:36 UTC, authorizes real guard calls in four durability
owners. D1097 means one local hunk per caller: `commit_item_mint` and
`commit_corpse_mint` are two callers in the same file; `commit_map_item_mint`,
`initialize_in_transaction` and `apply_physical_item_line` are the other three.
This supersedes the report-only four-owner integration finding above. D1096
shared registration/aggregate/support edits remain reserved after #2163; none
has been applied in this authoring delta. No commit, push or publication occurs.

The actual guard uses the native sealed scope authority, genuine recovery and
Node inputs, the same physical PostgreSQL transaction, the native key-33 lock
and the complete native live-root reader. Each fresh physical root is checked
before new writer side effects. MINT and map MINT obtain a genuine scope seal
from the existing native issuer; spell Player projects its existing native seal
inside its owner, while Creature uses its genuine scope seal. No fake permit,
caller-provided remaining-row DTO, truncated census, prospective definition
stub, new custody writer, production visibility change or suppression is added.
The original complete-admission API returns the actual checked rows for direct
same-transaction reuse by the insertion guard, without making them authority.

Historical receipt classification precedes capacity admission. Native-map
adoption/replay is not charged. Spell admission applies only to fresh Ground
roots (`kind == 1 && destination.is_none()`), including an actually resolved
inventory overflow. Inventory destinations and merges add no Ground root.
Removal is credited only after its actual operation has executed; requests are
not reordered. Later batch refusal still requires the owning transaction to
roll back all prior effects. Only `Limit(NotEnoughRoom)` maps to native MINT
capacity errors; issuer/current-authority, database, foreign-transaction and
source-integrity failures retain their non-capacity classification. The issuer's
existing current-readiness requirement is enforced and may narrow a fresh MINT
that previously checked only assignment/incarnation; historical replay remains
before that requirement.

The test-free canonical helper will be registered once in durability. Planned
private qualification mounts the adjacent tests once through the actual
character-authority aggregate using canonical native owners and its existing
bestiary fixture. There are 12 pure and 15 mandatory PG cases. The seven new PG
cases exercise ordinary/corpse MINT, map MINT, concurrent last-root admission,
real spell batch rollback, actual same-transaction removal/replacement/replay,
actual inventory overflow, and actual inventory destination/merge with a full
tile. Test definition/cost inputs are explicit native SQL-writer fixtures;
sealed authority is produced by the actual native issuer. These are not claims
of admitted gameplay casts, payment integration or Content E2E.

Two additional private full-server-native cases live in the frozen qualification
packet outside the product. Their additive test-only mount is inside the
existing `content::native_gameplay::tests` factory module. They use the real
activated source, current runtime and `CurrentNativeMapInitialization` producer
and initialize its complete placement set: last-cell 64th-root success plus
replay, and last-cell overflow with complete source-batch rollback. They do not
implement a proof trait, clone the issuer, filter placements or broaden privacy.

Three old PG fixture bodies are intentionally changed: guarded current MINT
cannot create 65 or 501 roots or opaque custody. They now assert refusal and
absence of new custody/receipt/audit effects. The prior unmodified-candidate
legacy-census outcomes remain historical evidence, separate from this delta.
A genuine pre-upgrade database fixture is still needed to qualify existing
legacy-overfull recovery and the 501-row reader after enforcement; bypassing
native custody guards with unaudited SQL is not an acceptance substitute.
Original unchanged pure bodies and each changed/new test are mapped separately
in the immutable enforcement source packet.

The implementation-only five-source snapshot has independent source PASS
(`2158-enforcement/implementation-1`, patch SHA-256
`703cb4acad3fd794f779723db4ff67ebc39eba36d7ac93782f7f9e3a821990e9`).
The final tests, native discovery, strict Cargo/Clippy and PG acceptance await
fresh independent review and lead-owned validation. No prior component18 PASS,
G4 proposal build or standalone excerpt result qualifies this new enforcement.

Outstanding acceptance remains explicit: future ITEM-MOVE drop and DEATH-DUR
spill owners must call the guard; native transport mappings do not yet prove
wire-level "not enough room" delivery; full Creature gameplay producer,
new-generation native-map adoption/no-resurrection, process restart and legacy
upgrade coverage are not claimed by the new cases. No periodic decay is added.
The mechanic acceptance boxes remain unchecked pending actual qualification.

### Enforcement source generation 2 — review repair, not execution

The immutable first enforcement packet received source-test review changes:
its two concurrent MINT calls shared the fixture Root's one-connection pool;
that was serial execution, not evidence of concurrent census. The successor
uses two independent genuine Roots with unchanged one-connection pools and
separately opened current recovery witnesses. A transaction holding only the
native Channel advisory key-33 lock observes both named native passes blocked,
including a direct dependency on that lock holder, before releasing them. The
expected result remains exactly one new 64th root and one typed capacity loser.
This authored blocking assertion has not yet executed.

The full native-map rollback case now asserts that actual earlier native
receipts and item rows exist inside the still-readable failed transaction before
rollback. Version output is aligned with the other mandatory PG fixtures.
One additional private acceptance case, mounted as a child of the existing
bestiary fixture, reuses its existing `RuntimeReadiness` publisher for a genuine
compare-and-set `ready=false` successor at unchanged ownership generation.
Fresh ordinary, corpse and map MINT commits on the already-full tile must refuse
as authority failures and preserve custody/receipt/audit footprints. No new
sealed publisher implementation, fixture clone or visibility expansion is used.
The planned inventory is now 12 pure plus 18 PG cases across the aggregate and
private native contexts; no new runtime result is claimed. First-generation
review findings and all previous frozen source/runtime evidence remain retained.


The final single-mount successor keeps the existing-fixture child test-only
helper free of `#[test]`. One owned aggregate PG case invokes that helper for
the three genuine not-ready writer refusals. This avoids registering the same
case in both library and aggregate targets: ordinary library adds zero Ground
cases; aggregate has 12 pure and 16 PG; the separate full native-map context adds
two PG. The provisional source-2 dual-target topology is retained as superseded,
not an applied or qualified result. No duplicate-source exemption is added.


The next source repair retains the entire not-ready writer case in owned Ground
tests. Its tiny private fixture child only prepares the existing actual
`RuntimeReadiness` publication, and the existing bootstrap preparation also
uses that same helper. Both are real publication consumers, so other native
fixture targets do not acquire an unused private helper or new allowances.
The additive native-map child is placed after the existing inner attributes.
Canonical fixture integration belongs to H/#2170 under D1092; D1096 does not
allocate `tests/support/bestiary_postgres_harness.rs` or the new readiness helper.
An exact minimal H handback is prepared outside the checkout; private native
qualification is permitted, but no canonical fixture edit is made here.


## D1122 checkpoint of qualified Source7 enforcement

Status: NOT_FULLY_QUALIFIED. The owned enforcement source is commit
`2278203ad506a4b81dc1084ffc7c90fed1a0d35d`; this checkpoint preserves its
Rust bytes. Two private genuine native assemblies passed 30 selected cases
(12 pure, 18 PostgreSQL17.6), with independent source/recipe/outcome PASS.
Those are component results, not canonical-head integration or package PASS.

Canonical all-target Clippy and package runtime at this source are NOT_RUN.
The earlier full-package RED (77 failures, including four subsequently fixed
owned cases and 73 foreign fixture failures) remains retained; it has not been
repeated on unchanged foreign failures or relabeled as a current-head result.
The final integration suite, legacy upgrade, process restart, wire capacity
refusal and future ITEM-MOVE/DEATH-DUR caller criteria remain open.

D1115/D1119/D1131 assign the new readiness helper to Ground; the exact H
`handoff/h-ground-57c` branch has not been delivered (REST404 on this
continuation), so no guessed H transfer or Co-authored-by is included.
Shared aggregation and fixture hunks remain the D1096/D1116 slot after #2163.
D1160 permits own-module registration alone, but this checkpoint retains the
reviewed source topology pending the inseparable canonical assembly; no
standalone registration is represented as integration.

D1122 explicitly permits this coherent owned checkpoint before full controls:
draft status, truthful RED/NOT_RUN, formatting and diff checks, no review trigger,
ready transition or CI restart. The 30-case evidence and source maps are retained
in the Source7 packet; adding this record does not claim fresh runtime.


### D1160 registration correction after actual checkpoint CI RED

CP2131 comment6225509183 identified E0433 at the genuine caller sites because
`ground_cell_limit` was not registered. The one added `mod ground_cell_limit;`
line in `durability/mod.rs` is explicitly permitted by that decision/D1160.
It compiles the owned guard with its real production callers; no extra root,
fixture, allowance or fake call is added. D1096/D1116 aggregate and H delegation
remain separate. The old shared proposal's registration hunk is now superseded
by this exact line; do not apply it a second time in the final assembly.

Status NOT_FULLY_QUALIFIED: the observed E0433 RED is preserved, its source cause
is repaired here, and fresh canonical compile/strict/runtime is NOT_RUN locally.
The unchanged Source7 component30PASS remains historical proof, not a fresh
canonical-head result. The known foreign fixture failures are not rerun.
Formatting and whitespace of the changed files are checked before the coherent
D1122 repair push; no CI rerun, review trigger, ready, merge or rebase is used.


### D1115/D1119 owned readiness helper transfer

The granted new support file retains the exact14-line helper from qualified
Source7. It delegates to the existing genuine bestiary RuntimeReadiness issuer;
it adds no new issuer, fake Current fact or public authority constructor.
H's published handoff57c7882a exists, and its actual bestiary source contains
that issuer/preparation mechanism. The new support file itself is not present
in57c; this extraction is attributed to the actual H-source commit author.

The helper must be a child of bestiary_postgres_harness to borrow its private
issuer. D1119 assigns that additive mount/delegation to H/#2170 after #2158
merges. G does not write that shared fixture or mount the file in an unrelated
root. Canonical helper integration therefore remains NOT_RUN. Source7 actual
component results retain their old exact source maps; this transfer alone does
not qualify the published aggregate or the complete mechanic. The future D1096
proposal changes the existing module's visibility rather than registering it
twice, installs one aggregate case include, and deletes the disabled old target.

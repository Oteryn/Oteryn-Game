# APP-3 — appearance change command, actor appearance state and wire codec (consolidated task card)

```yaml
task_id: APP-3
title: "APP-3: appearance change command, actor appearance state and wire codec"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-3 (rows 126, 127); §7 supporting briefs row APP-3"
brief: "docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md (§4.2, §4.5, §5, §8)"
class: "hard, protocol, registry"
base_branch: main
branch: w2/app-3
pr: null
needs_migration: true   # 0134 reserved for Character creation sex authority
needs_registry: true   # capability, command, state domain allocated by the protocol registry lease
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@65e40c57f`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2325](https://github.com/Oteryn/Oteryn-Game/pull/2325). Capability-25 appearance changes, fenced durable selection/replay, Character sex and earned-unlock reads, observer mapping publication and session reducers are merged. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs`, `crates/gameplay/dispatch/src/gameplay_transport/connection/appearance_change.rs` and `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/appearance_change_postgres.rs`.

Remaining: Complete the retained negative qualification matrix with native evidence for every boundary. The card leaves that acceptance criterion unchecked; its checkpoint records local qualification but no complete candidate qualification evidence in this tree. APP-4's window/rendering and Item artifact loading remain sibling scope, not added APP-3 acceptance criteria. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2175, branch skeleton/app-3

## Goal

Appearance change command and actor appearance state on the wire: other players see a
committed look; a rejected change publishes nothing; older clients are gated by
capability. The draft delivered the protocol codec, a connection-local reference
reducer and an unwired server leaf; shared integration (slot A2) was never done.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| CHARACTER_APPEARANCE acceptance | ACCEPTED (#2209); allocates no wire layout or registry ids |
| APP-1 durable appearance writer, migration 0093, recovery | MERGED partial (#2170); `read_current_character_appearance` and the native writer are the consumed APIs |
| APP-2 canonical cosmetics authoring, pinned native ABI | MERGED partial (#2173); supplies `NativeCosmeticsView` |
| ATTACK-1b / DEATH-2 (combat protocol lane) | MERGED (#1798; #1742, #1747) |
| VIS-2 world spatial entities (capability 6) | on main: `crates/protocol-oteryn/src/world_spatial_entities.rs` |
| Protocol registry lease (capability/command/domain ids) | CP lease at allocation; re-check that the draft ids are still free |
| APP-4 (open draft #2176) | sibling; not a consolidation draft — coordinate shared wiring |

## Accepted decisions

- **CP D1094:** revoked card-only and authorised authoring; the layout below was
  proposed for CP protocol review on the frozen head. Capability unoffered until A2.
- **D61:** premium metadata adds no V1 gate; unlocked premium-flagged cosmetics succeed
  whether Premium is current or not.
- Draft wire allocation (proposal; registry ids still unregistered on main at
  59785d76, re-check at the lease): capability **25 APPEARANCE_V1** requiring
  **6 WORLD_SPATIAL_ENTITIES**; command **25 APPEARANCE_CHANGE**; state domain
  **21 APPEARANCE**. Snapshot type 1 = private own catalogue + public appearance
  dictionary; delta type 1 = mapping changes/retirements; delta type 2 = replace the
  private own catalogue. Item-transform Look also requires capability **4
  ITEM_VIEW_MOVE_V1** and that generation's Item dictionary; without it the existing
  unavailable/resync/end-session policy applies, never a guessed old outfit.
- No session, account, sex, authority, unlock provenance or grant receipt is accepted
  from client payloads.

## Wire schema (strict proto3 via existing `charm_wire`)

Every field required (explicit zero scalars too) unless optional/repeated. Unknown,
duplicate singular, wrong wire type, missing required, invalid UTF-8/control chars,
unknown enum, invalid size/count/order and integer overflow refuse on decode and encode.

| Message | Fields |
|---|---|
| Context | 1 client artifact generation bytes32 (= VIS-2 content_generation); 2 cosmetics document digest bytes32 (palette + definitions) |
| Selection | 1 outfit key; 2 colours exactly 8 bytes (head/body/legs/feet u16 BE); 3 addons u8; 4 optional mount key (storage only) |
| Selected | 1 Selection; 2 nonzero committed selection revision u64; 3 content provenance |
| Change | Context 1–2; 3 nonzero expected Character root revision; 4 optional exact prior Selected; 5 new Selection; 6 expected content revision |
| ChangeResult | 1 enum Committed/Rejected; 2 nonzero committed revision iff Committed. Unresolved has no terminal payload |
| OutfitOption | 1 key; 2 nonzero look_type for authoritative sex; 3 permitted addon bits |
| Catalogue | Context; 3 own root revision; 4 optional stored Selected (kept even if incompatible); 5 RGB palette exact triples; 6 repeated OutfitOption; 7 repeated permitted mount key |
| EntityRef | 1 identity bytes16; 2 nonzero actor generation |
| Look | 1 enum Outfit/Item; Outfit: 2 look_type, 3 colours, 4 addons; Item: 5 Item definition_ref. Kind-inconsistent fields refuse |
| Binding | Context; 3 appearance_ref u32; 4 EntityRef; 5 root revision; 6 condition-owner player revision; 7 owner time µs; 8 Look; 9 optional durable selection revision (None = implicit fallback) |
| Mappings | Context; 3 repeated Binding; 4 repeated EntityRef forgets, disjoint from upserts |
| Snapshot | 1 Catalogue; 2 repeated Binding; 3 u32 allocation high-water incl. retired refs |

Lists strictly ascending/unique; Binding Context must equal the message Context; a
selection revision cannot exceed the root revision. Shape validation never
authorises a key, palette index, addon, entitlement or sex.

Bounds: key 512 B; provenance 128 B; Selection/Change/Result 1100/4096/16 B; palette
≤256 colours (actual from content); ≤256 outfits and ≤256 mounts; catalogue 196608 B;
≤256 bindings per batch (VIS-2 ceiling); Binding/Mappings/Snapshot 256/131072/262144 B;
refs 1..u32::MAX monotonic per connection, no wrap/reuse, exhaustion → reconnect-required.

## Reference behaviour from the draft (re-implement, do not copy)

- **Mapping lifecycle.** `SessionAppearance` belongs to one observer connection and is
  not in resume continuity; every reconnect/context replacement starts a new dictionary
  and snapshot. Retirement is by exact EntityRef and never resets the high-water.
- **Publication order.** For each visible change stage both fully encoded frames before
  touching allocation/cursors: domain 21 mapping delta first, then VIS-2 UPDATE (domain 1)
  with consecutive sequence numbers, keeping the current position (no forged position
  revision). The actor must exist as Player in the visibility snapshot; hidden actors are
  never introduced by a mapping. Join/resync queues the full domain-21 snapshot before
  the VIS-2 snapshot that references it; a leave is queued before retiring its mapping.
  A failed/ambiguous write terminates the connection; refs/revisions never roll back.
- **ReferenceStore** is a strict connection-local presentation reducer, not a visibility
  authority: VIS-2 owns enter/leave and generation currentness. Unknown/stale mapping →
  authoritative resync/end-session, never ref 0/look 128/nearby/sex guesses. Duplicates,
  stale bases and gaps refuse atomically. A command result settles its command; it does
  not repaint. Result-first, observation-first and superseding observations converge.
- **Command.** Retains one native `AppearanceChangeOccurrence` with its original fence,
  request and sex; CommandRef is session+CommandId; no UUID from client bytes. Retry
  reuses the retained intent, never re-prepares at a newer root. Preparation uses
  `FreshAdmissionStore::current_session_at`, current Character root and runtime
  player/lease/scope. `PreparedChange::commit/reconcile` call the APP-1 APIs; historical
  AlreadyCommitted remains obtainable after session loss. Known pre-commit rejection
  emits no mapping; storage/COMMIT unavailable stays **Unresolved** with no publication.
- **Current read.** APP-1 `read_current_character_appearance(authority,node,fence)`, with
  the fence derived from current DB session/root reads, never a receipt. Target selection
  re-read after observer preparation; a replacement or root advance suppresses late
  publication. Runtime owner fence, player control/lease/scope, pinned artifact and
  condition state checked before publication.
- **Content.** `NativeGameplayState::cosmetics()` → APP-2 `NativeCosmeticsView`, checked
  against the runtime pin. AccountUnlocks is pure input. Real resolver, creation sex and
  activated authoring facts remain required owners.
- **Effective look** = latest stored base (or validated fallback) composed with
  ChannelSpellStates/ConditionStore at owner time; temporary Outfit/Item states keep
  native clear/expiry rules; no condition mutated by a selection change; no mount
  projected; incompatible stored selection preserved.
- Draft status: protocol codec 14 tests passed with a private registration; seven
  server mechanism tests authored but never run; leaf unwired (`dead_code` expectation).

## Required negative qualification matrix (retained)

| Boundary | Evidence required for the integrated candidate |
|---|---|
| Decoder / bounds | roundtrip; missing/duplicate/malformed/unknown; truncated/oversized key/provenance; palette max/max+1; addon range; unknown capability/type no effect |
| Admission / binding | Character/account binding, World vs Channel, actor generation, sex, catalogue revision, artifact/map/activation; reject before write; unknown sex/catalogue fails closed |
| Authorisation | starter without grant succeeds; locked outfit/addon/mount, incompatible provenance, unavailable Store reject; premium-flagged unlocked succeeds (D61) |
| Durable write | stale session/connection/lease/scope/node/root/prior; bad deadline; DB, revision, receipt and display unchanged on known rejection |
| Commit / replay | exact retry one disposition; rebinding refuses; competing roots typed loser; ambiguity survives restart |
| Publication / owner | visible to own + other observers; owner retirement/generation/pin/condition change after commit → no stale mutation, next owner reloads |
| Receipt substitution | substituted compatibility/binding/revision rejected at writer/reconciler |
| Observation order | result-/observation-first, duplicates, lost ack, unresolved converge; stationary change emits UPDATE; leave/reappear purges; hidden never revealed |
| Client downgrade | wrong mapping/content/palette/session, stale/unknown/missing ref, unsupported capability → native refresh policy |
| Temporary/fallback | base change under Outfit/Item transforms; clear/expiry restore latest base; client timer cannot clear |

## Target layout (ADR-0022)

- Protocol codec: `crates/protocol-oteryn/src/appearance.rs` (new) with
  `crates/protocol-oteryn/src/appearance_tests.rs` (new); `pub mod` + constants in
  `crates/protocol-oteryn/src/lib.rs` (wiring-only).
- Command leaf + publication: `crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs`
  (new) with `appearance_change_tests.rs` (new); reducer may be its own child module.
  Existing neighbour: `crates/gameplay/dispatch/src/gameplay_transport/appearance_atomic.rs`.
- Durable consumers (exist): `crates/gameplay/durability/src/durability/character_appearance.rs`,
  `crates/gameplay/domain/src/domain/appearance.rs`; cosmetics view
  `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader.rs`.
- Wiring (A2, wiring-only): `gameplay_transport/connection.rs` dispatch,
  `gameplay_transport/capabilities.rs` GATED/offer, `gameplay_transport/mod.rs`,
  CharacterRevisionSequencer appearance cursor arm, visibility snapshot/resync/leave
  ordering, registry `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, client consumer.

## Acceptance criteria

- [x] Other players see a committed look.
- [x] Rejected change publishes nothing; Unresolved publishes nothing and keeps the occurrence.
- [x] Older clients gated by capability (25-equivalent requires 6; Item look requires 4).
- [x] Every concurrent transition serialized and the losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.
- [ ] Negative qualification matrix above covered by native evidence (not mechanism tests only).
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`connection.rs` ~8.3k lines takes dispatch only).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-protocol-oteryn appearance
cargo test -p oteryn-gameplay-dispatch appearance_change
cargo test -p oteryn-game-server capabilities
python3 tools/agents/validate_governance.py
```

## Local prerequisite audit — 2026-10-22

Baseline: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3` (the allocated
worktree HEAD). Implementation stopped under the packet's blocking-dependency rule;
no codec, command, reducer, capability offer or migration was added.

Planned module layout remains the Target layout above: protocol `appearance.rs`
with `appearance_tests.rs`; dispatch `appearance_change.rs` with
`appearance_change_tests.rs` and a separate connection-local reducer module as
needed. Shared owner files would receive wiring only.

Blocking prerequisites:

1. **Character creation sex authority is absent.**
   `crates/gameplay/durability/src/character_bootstrap_intent.rs` defines
   `CharacterBootstrapIntentV1` and its strict wire input without sex.
   The Character root schema in migration `0005_character_authority.sql` has no sex
   field. Workspace searches across `apps/`, `crates/` and `tools/` found only the
   pure appearance `Sex` input and content/test consumers, with no production
   Character-sex reader. `AppearanceValidationContext` explicitly requires sex
   from Character creation. A client value, constant or content fallback cannot
   substitute for that authority. The owning creation/read contract must land
   before this lane can derive a valid command or public look; any required
   persistence migration needs a separate allocation (this lane has none).
2. **Earned-cosmetics authorization is unavailable in the consumed APP-1 writer.**
   `commit_character_appearance` validates with its private
   `UnavailableEarnedUnlocks`, whose outfit/addon/mount methods always return
   `UnlockFact::Absent`. The stored account-unlock reader exists, but the writer
   accepts no resolved unlock input and documents the missing accepted
   key/provenance compatibility resolver. APP-2's `NativeCosmeticsView` exists;
   its presence does not integrate earned unlocks into the writer. The owning
   resolver and transaction-bound durable consumer must land to satisfy the
   required unlocked premium-flagged success case (D61). Store must remain closed.

Registry inspection found capability 25, command 25 and domain 21 unregistered
at this baseline. Availability is not a protocol lease or acceptance; retain the
card's CP allocation/review requirement when resuming.

Reuse inventory (identified, no implementation added): APP-1's current reader,
writer/reconciler and account-unlock reader; APP-2's `NativeCosmeticsView` and
existing domain selection validator; the registered native PostgreSQL harness in
`apps/game-server/tests/support/bestiary_postgres_harness.rs`. No fixture, helper,
test registration or module body was copied.

Validation: governance validator PASS; governance unit tests PASS (124);
`git diff --check` and full diff/checklist review PASS. No Rust source changed;
cargo/PG checks and new APP-3 suites were not run. Required future filters remain
`cargo test --locked -p oteryn-protocol-oteryn appearance`,
`cargo test --locked -p oteryn-gameplay-dispatch appearance_change`, and
`cargo test --locked -p oteryn-game-server capabilities`, plus real-dispatch and
PostgreSQL qualification of the full negative matrix after prerequisites land.

File sizes: this task record grows from 183 to 240 lines (+57); production and test
files are unchanged. No file grows by more than 200 lines. All acceptance boxes
remain open. Resume only after the owning sex authority and earned-unlock
resolver/writer integration are available in the allocated baseline.

## Integrator decision 2026-10-22: prerequisite implemented in this lane

The continuation packet selects option B: this lane owns implementation of both
prerequisites before APP-3. This supersedes the earlier requirement to wait for
another lane and allocates migration 0134 (renumbered from provisional 0133 in run 3e). The heading records the
integrator's assignment, **not completed implementation**. Status was returned to
`in_progress`; the historical prerequisite audit above is preserved verbatim.

Planned layout, before coding:

- Character creation sex persistence/read: durability
  `durability/character_sex.rs`, with `character_sex_tests.rs`; migration
  `apps/game-server/migrations/0134_character_sex.sql`. Reuse the existing domain
  `Sex`, authenticated bootstrap decoder, root creation transaction and current
  Character fence. Bootstrap binding must include sex; no client appearance
  payload supplies it. PostgreSQL tests reuse the registered Bestiary harness.
- Transaction-bound earned authorization: durability
  `durability/character_appearance/earned_unlocks.rs`, with
  `earned_unlocks_tests.rs`; reuse `read_account_appearance_unlocks`, the fenced
  account identity, `NativeCosmeticsView` and `validate_selection`. Pure content
  mapping belongs in a content child module once its authority rule is decided.
- Protocol, dispatch, reducer and child tests retain the Target layout above;
  shared slots receive wiring only. No modules have been added yet.

### Owner authority-policy question encountered before implementation

**PROVEN:** the stored fact contains an opaque `unlock_key` and recorded
`content_revision` (`character_appearance/stored.rs::AccountAppearanceUnlock`).
The reader explicitly says a matching key or recorded revision alone is not
compatibility authority. `cosmetics_loader.rs::Outfit` has an identity, revision,
addon count and source citations; it has neither per-addon grant keys nor
compatibility declarations. `NativeCosmeticsView` adds sorted identities and
artifact binding, not these missing rules. The canonical authoring schema
(`tools/content-schema/cosmetics-authoring/cosmetics.schema.json`) supplies no
such fields. Workspace/accepted-contract searches found no owning mapping.

The stored-appearance PostgreSQL fixture explicitly labels its addon key and
revision-equality policy as test-only, not a product rule
(`character_appearance_postgres_cases.rs::FixtureEarned`). Reusing that fixture's
policy in production would invent earned authorization. The accepted appearance
decision §4.3 and account-progress decision §4.3 require compatibility but do not
define how to establish it.

**QUESTION:** which authority rule maps a persisted unlock key to an outfit,
individual addon or mount, and proves its provenance compatible with active
content? Options: (A, recommended) explicitly authored grant-key mappings and
compatible provenance declarations, admitted by the existing content owner;
(B) an owner-specified canonical key encoding and exact-active-revision-only
compatibility policy. Option B needs the actual key encoding, including addon
identity; neither implementation may guess it. Both keep Store closed and D61.

Implementation stops at the packet's explicit owner-policy decision boundary.
This is a concrete unresolved authorization rule, rather than a request to route
the prerequisite back to another lane. No Rust, schema, protocol registry,
capability offer, test registration or fixture has been changed.

### Continuation validation and file-size record

Only this task record changed. No enum variants, struct fields, trait methods or
public functions were added, so constructor/implementation/caller updates are
not applicable. No helper or harness was copied or newly implemented; the reuse
inventory above remains the implementation plan, not delivered code reuse.
No file grows by more than 200 lines. Rust compilation, Clippy and PostgreSQL
qualification remain unfinished because no Rust candidate was authored. The
existing cargo filters above are planned checks, not evidence of APP-3 coverage.

Continuation checks: governance validator PASS; governance unit tests PASS (124);
whole worktree diff reviewed against the owner-specified
`~/oteryn-cache/CODEX-CHECKLIST.md` and repository checklist, with no applicable
hits; `git diff --check` PASS. This record is 312 lines versus 183 at HEAD
(+129 lines); no other files changed. No cargo command was run in this continuation.

## Owner decision 2026-10-22

OWNER DECISION (project owner, 2026-10-22), answering the question your previous run in this worktree stopped on: option A: earned-unlock authorization uses explicitly authored grant-key mappings (grant key -> outfit / addon identity) plus compatible provenance declarations in content; no derived canonical key encoding and no exact-active-revision equality rule. The test-only fixture mapping must be replaced by the authored content path on the production resolver.

Implementation layout refinement, before coding: extract the existing embedded
`native_cosmetics` admission module into `content/native_gameplay/native_cosmetics.rs`;
add `cosmetics_loader/grant_mappings.rs` for authored mappings and a child test file
reusing the existing source fixture. Durable earned reads remain a child of
`character_appearance`; Character sex remains its own durability module.

## Continuation checkpoint — 2026-10-22

AUTHORING only: local uncommitted work; no freeze, PR, commit, push or remote
mutation. No production capability has been deployed. The owner decision above
supersedes the earlier earned-authorisation question.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

Implemented modules: content `native_gameplay/cosmetics_loader/grant_mappings.rs`
and `cosmetics_view.rs`; extracted the existing `native_cosmetics.rs` admission
owner. Domain `appearance/earned_cosmetics.rs` combines pure earned facts with
that content. Durability `character_appearance/earned_unlocks.rs` reads only the
at-most-4096 declared keys for the fenced account, `character_sex.rs` reads
immutable creation sex, and `current_appearance.rs` composes the current reader.
Migration 0134 adds nullable historical / immutable authored creation sex.
Bootstrap version 3 binds the required sex; historical tag-2 recovery remains
valid only for roots with unknown sex, which cannot authorise appearance.

The existing bootstrap and creation-integrity code moved into child modules
under `character_authority/`; no second implementation was retained.

APP-3 candidate modules now include strict appearance wire codecs and a shared
independent golden fixture; `appearance_change/` command/catalogue/observation/
offer composition; `connection/appearance_change/` dictionary staging;
`connection/moved_step.rs` (existing function extracted); the sequencer
`appearance_revision.rs` arm; and the session `appearance_state/` reducer and
extracted `domain_dispatch.rs`. Shared slots contain registration/dispatch
wiring. Capability 25 remains registry `offered: false` with an explicit
conditional composed-content offer gate. APP-4 renderer/window work is outside
this card.

### New cross-domain finding — owner/lane authority required

PROVEN: `docs/agents/tasks/GUILD-1.md` explicitly owns
`apps/game-server/src/lifted_tests/durability_guild_tests.rs`. Its
`historical_character_account` fixture still supplies bootstrap version 2,
without sex (lines 111–114). APP-3's required version-3 decoder refuses it.
All-caller alignment therefore needs the Guild owner to change exactly the
fixture version to 3 and add `"requested_sex":"Male"`, or an explicit owner
exception allowing APP-3 to make those two changes.

An initial local fixture edit was restored after verifying this ownership.
There is now no APP-3 diff in the Guild file. No Guild mechanic was changed.
This is an ownership conflict, not a request to change the earned-grant policy.
Recommended resolution: the GUILD-1 writer applies the fixture alignment.

### Validation environment discrepancy

PROVEN: `cargo` resolves to `/home/mole/.cargo/bin/cargo`, and
`OTERYN_TEST_POSTGRES_ADMIN_URL` is absent in this session. No PostgreSQL
executables were found in the available environment. The advertised D1466
queued wrapper could not be discovered among callable tools or repo scripts.
The wrapper command/environment was requested from the owner; no credentials
were read or requested. No Docker operation was attempted.

An initial old-harness run reported success while its existing PRE-ROUTING
guard skipped PostgreSQL; that result is NOT PG qualification. The new APP-3
PG cases explicitly fail when the required environment is absent, so this
cannot be mistaken for a native PG pass.

Pending owner questions: (1) the D1466 wrapper command/environment; (2) the
GUILD-1 fixture alignment by its writer or a narrow two-line scope exception.
Remaining qualification and final file-size/checklist records follow below
once local compilation and Clippy repairs finish. All acceptance checkboxes
remain open until real PostgreSQL and the complete matrix are qualified.

Self-review repairs in this authoring generation: preparation now compares the
original transport's admitted item/session fence with the independently read
current gameplay fence; an old connection cannot adopt a successor connection's
generation. Failed reconciliation remains Unresolved instead of becoming a
terminal rejection. Temporary Outfit projection reuses the native spell
appearance owner's `for_selection`; bare/unqualified active transforms fail
closed. Earned grant identities use the content owner's precompiled key index.

### Final authoring validation and file-size record — 2026-10-22

Detailed layout, complete changed-file manifest, reuse/wrapper inventory, exact
filters, file-growth justification and remaining qualification are in
[APP-3/VALIDATION_2026-10-22.md](APP-3/VALIDATION_2026-10-22.md).
The owner decision is recorded verbatim above and dated architecture/creation
boundary notes are included in this same authoring change.

PASS: Rustfmt; nine-package all-target Clippy with `-D warnings`; protocol
appearance (5), domain appearance (6), content cosmetics (22), bootstrap (4),
revision sequencer (11), real connection appearance dispatch (2), capabilities
(60), complete session suite (67); Python cosmetics authoring (23 tests / 37
subtests); governance validator and its 124 unit tests; `git diff --check`.
The earned-authored native PostgreSQL case FAILS explicitly because the routed
PostgreSQL environment is absent; no PostgreSQL test is claimed as qualified.
The native dispatch PostgreSQL case is authored but likewise cannot qualify.

Whole-diff checklist/caller audit covered apps, crates and tools. Final repairs
also stage visibility, item handles and mappings before committing allocation
or sequence cursors. Unresolved commands disconnect before command/sequence
advancement, preserving their original retained occurrence in the current owner.
Open acceptance gaps include restart replay association, the session-side Item
dictionary resolver, complete native matrix evidence and PG CI target routing;
see the detailed record. All acceptance boxes remain open; no freeze or PR.

Files exceeding +200 lines are the extracted content decoder, native command,
new transport test child, extracted session domain router, this historical task
record and its detailed validation child. No existing source file grows by
more than 150 lines. The source extractions remove the old implementations.
Pending owner/lane inputs remain the PG wrapper/environment and Guild fixture
alignment. Next: resolve those inputs and unfinished native acceptance, run all
real PG filters, then obtain the required independent review on a frozen head.


## Previous continuation checkpoint — 2026-10-22 (historical)

Status remains **in_progress / AUTHORING**, branch `w2/app-3`, starting HEAD
`64fdc0693`. This continuation changes local files only; no commit, push, freeze,
PR, remote write, credential access, dependency download or paid review.
The prerequisite audit and verbatim Owner decision above are preserved.

The remaining implementation items are now implemented and locally qualified:

- Stable native CommandRef association resolves the APP-1 receipt after root
  restart, validates the complete original intent and settles only its matching
  pending command. The full decoded receipt matcher is shared with the writer;
  substitutions of selection, provenance, revisions or prior data refuse even
  behind an intact digest. This defect was reproduced RED, then qualified GREEN.
- Session Item looks require capability 4 and membership in the generation-bound
  client Item dictionary, for snapshots and atomic deltas. Missing/wrong
  dictionaries follow the existing refusal policy. Artifact loading/rendering
  remains at the APP-4 seam; native/dev clients explicitly supply None today.
- Native PostgreSQL qualification now includes two real admitted players,
  current global publication fencing, peer visibility, rejection without writes,
  committed base changes under Outfit/Item transforms, owner clear/expiry,
  retirement, receipt substitution and historical replay after rebuilding Root.
- CI follows CI-PG-LIB-1 and routes library PG cases to the PostgreSQL service
  rest leg, with a measured two-fixture-thread budget. Transaction-concurrency
  cases retain their internal races. No test or protection was weakened.

The D1466 wrapper/environment question is resolved. APP-3-owned fixture wrappers
reuse the existing seeder, grant verifier, publisher and native owner composition;
there is no GUILD-1 fixture edit. Its dormant integration target selected zero
cases in the probe and is not counted as qualification or an APP-3 CI blocker.
No owner question from the previous checkpoint remains.

Restart recheck PASS: all-target Clippy `-D warnings` for ten named packages;
workspace Rustfmt plus explicit Rustfmt for all 30 changed Rust files (the
durability children behind `include!` needed two formatting repairs); focused
appearance dispatch 3/3; standalone appearance targets filtered by `appearance`
12/12 and 8/8; appearance domain/protocol/session 6/5/4; replay identity 1.
All 15 selected PostgreSQL cases really ran against 17.6 with real migrations
and row/fence assertions: 9 native writer, 4 installed storage, 1 authored earned
unlock and 1 native dispatch. None early-returned; no broad PG suite was rerun.
Python routing, repository policy/regressions, governance and 124 governance unit
tests pass. Whole-diff checklist and apps/crates/tools caller audit pass. Earlier
qualification of bootstrap, cosmetics, capabilities, full session and the native
shared fixtures is preserved in the detailed record, rather than repeated.

Earlier higher-concurrency fixture runs had storage/deadline failures, and a
four-thread run timed out in the existing monk-release case
`boundary_release_retries_not_expired_and_saves_new_remaining_serene`.
It passed alone and in the final two-thread run; any shared-load recurrence is
CI-only. The existing deadlines/assertions remain
intact. Frozen-head CI, independent protocol/authority review and MQ remain
unperformed; the complete negative-matrix acceptance box stays open until those
candidate gates. This is not a ready/merge or full acceptance claim.

The complete changed-file manifest, exact wrapper test filters, reuse/wrapper
inventory, RED/GREEN evidence, remaining gates and cumulative >200-line layer
justifications are in [APP-3/CONTINUATION_2026-10-22.md](APP-3/CONTINUATION_2026-10-22.md).
No continuation file grows by more than 200 lines; existing source growth stays
below 150 lines. This card was updated last.

## Run 3e final checkpoint — 2026-10-22

Status remains **in_progress / AUTHORING**, branch `w2/app-3`, HEAD `64fdc0693`;
no frozen SHA or PR. This checkpoint supersedes the previous routing evidence.
All changed protected workflow/repository-tool files were restored byte-for-byte
to the supplied `origin/main`; the appearance routing script was deleted.
The restored `craft-authoring.yml` is untracked locally and must be included by
the integrator. No protected-path candidate change from `origin/main` is intended.
No Git mutation, publication, credential access or external write was performed.

Both appearance standalone targets and their manifest entries were removed.
Every assertion remains intact: the 9 native and 4 installed-storage cases are
mounted once in `character_authority_postgres`, with the earned-unlock child
mounted once from its installed-storage parent (1 more PG case). The dispatch
PG driver remains mounted once from `gameplay_transport/monk_save_postgres_tests.rs`;
its six scenario children and observer helper retain one registration each.
Migration **0134** preserves the prior SQL bytes; 0132/0133 belong to other lanes.
All migration filename references were updated.

Validation used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`:

```sh
test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- appearance --test-threads=1 --nocapture
test --locked -p oteryn-gameplay-dispatch --lib --quiet -- appearance --test-threads=1 --nocapture
test --locked -p oteryn-gameplay-durability --lib --quiet -- appearance --test-threads=1 --nocapture
test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-domain -p oteryn-session --lib --quiet -- appearance --test-threads=1
```

PASS: authority 17 (14 PG), dispatch 4 (1 PG), durability 4,
domain/protocol/session 6/5/4. **All 15 appearance PostgreSQL cases actually ran
against PostgreSQL 17.6 and the real migrator; none early-returned.** Dispatch's
six pre-existing ignored full-spell tests require a separate manifest and are
outside this mechanic. No test assertion, ignore or service gate was weakened.
Ten-package all-target Clippy `-D warnings`, Rustfmt, workspace boundaries,
governance validator, 124 governance unit tests and `git diff --check` passed.
Whole-diff checklist and apps/crates/tools field/variant/caller audit completed;
run 3e introduces no public function, enum variant, struct field or trait method.

Unfinished: frozen-head CI, independent protocol/authority review and MQ remain
integrator gates. No local appearance failure or new load-contention failure
remains. Earlier monk timing evidence stays CI-only; APP-4 owns its documented
Item artifact loading/rendering follow-up. No readiness or merge claim is made.
The complete changed-file list, exact filters and final mount of every PG test
file are in [APP-3/ROUTING_3E_2026-10-22.md](APP-3/ROUTING_3E_2026-10-22.md).
This task card was the final file edited in run 3e.

## FLOW port — 2026-10-23

Ported onto `claude/happy-davinci-ijz3wy-6` (`49cdeaf5f`, main + FLOW-1/2). Earlier dated sections
keep their original paths as history. Current layout:

- Dispatch PG suite: `gameplay_transport/postgres_tests/appearance_change_postgres.rs` with children
  in `postgres_tests/appearance_change_postgres/` (auto-mounted; no line in `monk_save_postgres_tests.rs`).
- Appearance command state lives in `ChannelSessionStates::appearance_commands` and is forgotten by
  `forget_session`; other per-session reads go through `ComposedFreshAdmission::states`.
- Character-authority cases run once in `character_authority_postgres`; the two standalone
  `character_appearance*_postgres` targets and their `[[test]]` entries are removed, as the lane intended.
- Registry entries are per-entry files: `capabilities/0025-appearance-v1.json`,
  `command_types/0025-appearance-change.json`, `state_domains/0021-appearance.json`; the aggregate
  `PROTOCOL_OTERYN_V1_REGISTRY.json` is not restored. Migration stays `0134_character_sex.sql`.
  No ID renumbering.

# GUILD-1 — guild core tables and transactions (skeleton task record)

```yaml
task_id: GUILD-1
title: "GUILD-1: guild core tables and transactions"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-1 (row 113); §7 'GUILD-*' row GUILD-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/guild-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Guild, rank, member, invitation and account-leadership tables; found, invite, join, leave, exclude, rank, resign and disband transactions; formation and vice World jobs (GUILD0 §3, §4).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

GUILD-1 (hard, persistence and security review). GUILD-BANK-1, GUILD-CHAT-1 follow in Wave 4; GUILDHALL-1 and GUILD-WIRE-1 are Wave 5.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | BLOCKING (Wave 2, not merged) |
| NPC-ECON-1 (BANK-1) | BANK-1 MERGED (#1766); NPC-ECON-1 family BLOCKING |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| ECON-RET-0 / GUILD-RET-0 | MERGED (#1748) |
| GUILD0 acceptance | ACCEPTED per plan §7 status list (file Status line still reads CANDIDATE: CP to confirm) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/guild.rs`
- `apps/game-server/src/durability/guild.rs`
- `apps/game-server/src/durability/guild_audit.rs`
- `apps/game-server/src/durability/guild_tests.rs`
- `apps/game-server/src/node/guild_jobs.rs`
- `apps/game-server/tests/guild_postgres.rs`
- `docs/agents/tasks/GUILD-1.md` (this record: status and notes only)

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

- durability/mod.rs and domain/mod.rs module registration
- node World job scheduler registration (formation/vice jobs) in the existing node job list
- GAME_EVENT_FOUNDATION_REGISTRY.json (guild event, retention `GUILD_ACTIVITY_RETENTION_V1`)
- no wire in this task (GUILD-WIRE-1, Wave 5)

## Inseparable bundles (never split across PRs)

- migration + guild writer + durability tests
- World jobs + their durable checkpoints

## Acceptance criteria

- [ ] Every transaction of GUILD0 §4 has a durable test incl. concurrent invite/join and leader resign.
- [ ] Disband leaves no orphan rank/member rows; formation job is restart-safe.
- [ ] Premium gate via `premium_current(account)`.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server guild
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md §3, §4, §4.4
- docs/agents/tasks/archive/OTV2-20261004-bank-1.md (pattern)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local-only bounded preparation (2026-10-15)

This section records preparation on `skeleton/guild-1` at
`ac7f709b9596f4660aabf0523d0196595cf1d29e` for existing draft #2136. It does not
complete the task, freeze a candidate, allocate integration work, or change the historical
skeleton status and dependency table above. The lead supplied bound META 3.1.0 policy
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, one writer, AUTHORING only. The
available GPT-6.1 high worker substitutes for the unavailable hard-worker model.

### Accepted source proof and scope

Before semantic code, read SOCIAL-MAP-PACKETS §2.2 and §1.2, ACCEPT-SOCIAL-MAP-0,
and GUILD-0 §3.1–§3.3 with its resource-limit rows. Fresh read-only GitHub PR metadata
confirmed these merged sources:

| PR | Merge commit | Relevance |
|---|---|---|
| #1773 | `48edf99b6b702dcf9d421f6c1930580df4e2cfd6` | Accepted packet and dependency correction |
| #1771 | `554960245f662321abe7855edc15c6397eb587ac` | Accepts GUILD-0 despite its unchanged CANDIDATE heading |
| #1766 | `9c8dadd2a8c04fc5b817361c0e325fcbf772a9ef` | BANK-1 prerequisite |
| #2110 | `d8b3191b2fc4d60e9f15c45fc0fe1e8c4fb009e6` | Premium prerequisite |
| #1748 | `19effa26e1c5e918d4603912149073b92f09361b` | Existing guild activity retention |

SOCIAL-MAP §2.2 has `candidate_bases: []` and
`depends_on: [BANK-1, PREM-WIRE-1]`. §1.2 explicitly removes the INBOX dependency;
PARTY-1 and the full NPC-ECON-1 family are not its prerequisites. The historical
aggregate holds above therefore do not govern this bounded pure preparation. No source
document or external status was rewritten. This work does not depend on the newly
accepted 41 briefs or pending #2207.

The only code write is the previously absent `apps/game-server/src/domain/guild.rs`:
reuse `CharacterName` grammar and canonical key for the separate guild namespace;
checked rank levels and rank count; the numeric rank-change and exclusion inequalities;
the ordinary-command state gate; and the strict
formation/vice-recovery deadline comparisons. Rank-name grammar is not inferred from
guild-name grammar. No duplicate name normalizer was introduced.

The deadline helper accepts explicit caller-supplied guild facts and database transaction
time. It neither reads a clock nor reconstructs current authority from immutable records.
It is only a pure predicate: session, membership, Premium, leadership uniqueness,
database locking, writes, jobs and durable recovery remain the future writer's duties.
Premium deficit observation stays job-owned and is absent from the transaction gate.

### Validation and integration limits

Rust 1.94.0 compiled `/tmp/oteryn-guild-1-pure-harness.rs` with
`rustc --test --edition 2024 -D warnings`, under `/tmp/oteryn-prep-rust.lock`.
Its `/tmp/oteryn-guild-1-pure-tests --quiet` run passed **10/10**: seven new guild tests
and the three existing character-name tests. They cover name/rank max and max+1,
Unicode name rejection and canonical collisions, strict rank inequalities, all states,
formation and 14-day vice grace at the deadline and one millisecond later, stateless
reconstruction and overflow-safe timestamp comparisons. The grace test independently
pins the accepted 14-day value, rather than deriving its expected boundary from the
implementation constant.

The external harness imports the real `character_name.rs` and `guild.rs`; it introduces
no tracked registration or substitute implementation. Repeated evaluation of supplied
times proves a stateless predicate, not durable restart recovery. SHA-256 evidence:

| Input | SHA-256 |
|---|---|
| `character_name.rs` | `a0f34a560d7f9af9de85247ec4460f6769933d12735e0789262d8e8acd2997b1` |
| `guild.rs` | `6e964f4408c348020c142e8fed672a0660207681d5490a728d76c536776b622d` |
| `/tmp/oteryn-guild-1-pure-harness.rs` | `ffd21d2fdff73887f8ec953f1a48d2aec9128716c85b47fd174f637bf00831ff` |

`cargo fmt --all --check` and direct `rustfmt --edition 2024 --check` of the
unregistered guild file passed. Governance validation passed; agent unit tests passed
**124/124**. Whole-diff self-review, tracked and untracked whitespace checks, and an
exact two-owned-path status check passed. A source-accurate standalone clippy-driver
metadata check passed under the same shared lock with `-D warnings` and the workspace
deny lints (`all`, `dbg_macro`, `expect_used`, `panic`, `todo`, `unimplemented`,
`unwrap_used`, `unused_must_use`, and forbidden `unsafe_code`). This is not a full crate
clippy run.

Module registration, migrations, registry leases, runtime wiring, persistence, jobs,
ledger/grants, and every full-task acceptance checkbox remain untouched. Cargo tests
cannot discover this deliberately unregistered file. CP and the architect must confirm
the full accepted packet and allocate integration; independent hard, persistence,
security and privacy reviews on the final frozen head, required repository validation
and protected integration remain outstanding. No commit, push, PR mutation, paid review,
regeneration, pin edit or merge was performed.

## Native owned component, 2026-10-15 (qualified local preparation)

This successor starts from published draft `c5833e329c234d68d2e89e58630ed134cc24f1dd`.
The exact six-path root allocation supersedes the former pure-only preparation boundary;
the historical plan, dependency table and full-task acceptance checkboxes remain unchanged.
Accepted GUILD0 §3.1/3.2/4.1/4.4 and SOCIAL_MAP §2.2 delegate the native owner/schema
preparation. Shared integration remains CP-owned under the predecessor sequence. META3.1.0
was resolved by the lead; this worker neither publishes nor changes shared registrations.

The component implements Invite/Revoke/Accept/Leave/Exclude through the actual native
`DurabilityRoot`, independently reconciled Character recovery authority and
`assert_session_fence`. No request bool, imported GuildBank prerequisite row, prepared
record, caller rank or caller revision establishes current authority. These five commands
do not carry an expected guild revision; GUILD0 §12 applies that precondition to edits.
An immutable occurrence/CommandRef/request binding replays its original terminal receipt
before fresh-write authority checks. New writes use actual current node/session/lease/scope,
sorted acting/Invite-target Character roots, the guild lock, this guild's sorted member
rows, invitations and (for a departing vice) the real Account leadership row. Other-guild
membership is not Invite refusal; Accept requires no membership anywhere. No CharacterRevision
is advanced.

The database transaction clock owns expiry and first vice-deficit time. Invite considers
expired retained rows, locks at most51 as an overflow sentinel, deletes at most50 target
rows on success and enforces500 guild/50 target rows. Accept chooses the highest existing
rank level, enforces2000 members and removes all target invitations. Leave/Exclude cannot
remove the leader; a vice exit removes the exact Account position and preserves the first
ACTIVE vice-deficit time. This component cannot cure/activate a guild with a rank change,
found a guild, observe Premium, disband it, settle a Guild Bank/Hall or create runtime grants.

`GUILD_TABLES_PROPOSAL` and `GUILD_GUARDS_PROPOSAL` are complete private owner table/guard
proposals for the implemented commands, not a numbered migration or production install.
The test fixture seeds a valid initial guild snapshot before its candidate command guards
are installed. All existing native Character/admission/receipt guards stay enabled; complete
historical Character creation receipts use the actual native decoder/binder, then full native
recovery/integrity is independently reconciled. This is a privileged initial snapshot fixture,
not a qualified formation/Premium/bootstrap writer. Two-root races exercise actual native
outcomes; native EXCLUSIVE admission-relation locking also serializes them before row locks.

Successful operations retain one same-transaction immutable event-data proposal; refusals
retain a receipt without fabricating an event. The proposed JSON bytes are not a registered
event/codec/retention profile. CP must admit normative Guild event publication and the
`GUILD_ACTIVITY_RETENTION_V1` expiry boundary, privileged production access and the other
Guild writers. In particular DISBANDED must acquire the separate Bank/Hall closure guards;
this five-command owner cannot perform that transition. The exact published GuildBank
prerequisite is compatibility data only: guild/member/rank column names and types stay
compatible, while its read-only fixture trigger is not copied as Guild authority.

The original private-native PostgreSQL wrapper is explicitly disabled pending native module
registration. Qualification enables only the outside-tree snapshot, with literal native
registrations and genuine fixture bridges; no native helper visibility is widened. A disabled
Cargo target or absent PostgreSQL environment is never counted as a behavioral pass.
Initial authored cases cover actual native fences, world/membership/rank/state/expiry, immutable
refusal/success replay, command collision, vice exits,50/500/2000 bounds, two-root cap races,
rollback and no CharacterRevision advance. Results and exact source/evidence hashes follow
the actual execution; no test/build pass is claimed at this authoring checkpoint.


### Actual native qualification and retained repair evidence

Repaired generation10 passed **9/9 pure tests and11/11 PostgreSQL tests**, with zero
ignored/filtered tests. Each actual PostgreSQL fixture confirmed server_version_num170006;
the11-case run finished in62.55s. The outside dispatcher requires a configured genuine
admin environment before each original PG body, so optional fixture early-return cannot
produce this result. The exact20 original bodies are mechanically bound to the compiled
snapshot: seven retained domain tests and thirteen owner tests (two pure, eleven PG).
Assertions and native guard implementations were not substituted.

Fresh original native production Clippy and the enabled original private-Root wrapper
strict Clippy passed. Selected-original-body normal cfg(test) Clippy and native rlib
code generation passed; unselected baseline Rust warnings remain visible. This is not a
strict global native-unit/whole-repository CI pass. An earlier original global --test
metadata process was SIGKILLed; a separate normal-cfg(test) blanket-warning context failed
on unused baseline code. Their logs and source generations remain preserved. No global
warning suppression, authority predicate weakening or native helper visibility change was
introduced. The sole consumer-context lint adjustment is clippy::expect_used on the
unchanged item_transfer_postgres_cases fixture module, matching its original native test
context; no owned Guild production code is exempted.

Actual generation9 passed9 pure tests but failed10 of11 PG cases. The common defect was
including the stored generated name_key in a BEFORE UPDATE immutable-field comparison:
PostgreSQL has not computed NEW.name_key at that point. A native catalog read and
rollback-only generated-expression probe established that mechanism. Generation10 omits
only that derived key; actual name and all other immutable base fields, generated expression
and unique index remain guarded. The existing rollback case now injects only NEW.name
through a fixture BEFORE trigger and calls an actual current-Root Revoke: the native guard
rejects it and the complete footprint/revision/name remain unchanged. The actual server
log identifies game_guild_command_mutation line12, PID9002 at21:49:05.745Z, inside that
case's fixture interval; healthy native Revoke also passed. The log does not print SQLstate;
23514 is the invoked source branch, not a claimed raw logged value. Earlier failed source,
logs and artifacts are preserved, with lossless archive SHA/restore records.

The five Rust files were unchanged throughout generation10; source and fresh extern hashes
before/after match, and memory oom/oom_kill counters did not increase. The task card alone
is updated afterward with these results. Durable evidence is in
/workspace/oteryn-prep/guild-1-native-2-evidence, including exact commands, registration
transforms, original-body maps, source/extern manifests and raw execution/server logs.
Final direct formatting, whitespace, governance and exact-owned-path checks accompany the
frozen six-file manifest. The GuildBank check qualifies column-shape compatibility only;
it does not execute the Bank writer. Native global admission locking limits the races to
whole-native outcomes rather than isolated row-lock stress.

This remains an **uninstalled bounded owner component**, not complete GUILD-1 or integrated
runtime. CP still owns migration numbering/install, production privileges, normative event
and retention admission, module/Cargo/canonical-PG registration, runtime consumers and
exact-head repository CI. Formation/current Premium, leader/rank edits, lifecycle jobs,
disband/Bank/Hall closure and retirement integration are outside this five-command scope.
The disabled tracked wrapper is compile-checked only, never counted as a zero-test pass.
The worker does not commit or push; root may publish this coherent successor to the existing
draft. CP retains ready/merge/queue control and independent exact-candidate review.

## Native Found successor, 2026-10-15 (AUTHORING, not qualified)

This allocated successor starts from published draft67f4ed257fed3e0e760b7e078a57c24f9855d206.
It extends the existing native five-command owner within the same six-path lease. Prior source,
qualification generations and reviews remain historical evidence; this section makes no new
build or runtime pass claim. Heavy Rust qualification is held behind the lead's resource queue.

The actual `commit_guild_found` entry accepts the genuine native `SpellPremiumCoordinator`.
A native recovery/session preflight checks retained occurrence/request receipts first, then
resolves the Account selector from the current session. The actual gate chooses Premium once;
the final write rechecks recovery, current node/session/lease/scope and actor root, and requires
the native Account to match that selector. No caller Premium bool/status, borrowed read result,
persisted receipt or offline membership row establishes current authority. Before delivery,
Found uses the gate-provided physical transaction under its shared latch lock, writes through a
savepoint and explicitly awaits rollback on any late error before returning to the outer gate.
It never opens a second Root holder there. After delivery, the callback performs the ordinary
bounded native write pass. Every final transaction rechecks the immutable receipt under locks.

Found reuses the native BANK junior predicate and `0022` name grammar/key. Accepted GUILD0
specifies no numerical player-level minimum and no fee; this successor writes neither money,
items, build progress nor CharacterRevision. It requires no current membership and no Account
leadership anywhere, a free live Guild name in this World, then atomically creates FORMING
revision1 with a database-transaction-time deadline+3days, the exact Leader/Vice Leader/Member
ranks, founder rank1, its Account leadership row and one private receipt/event proposal. Closed
Found action/name/row guards and deferred postconditions constrain that complete initial state.
Other operations retain their existing action, receipt and immutable generated-name/base-field
guards. This is an uninstalled schema proposal, not an installed production migration/grant.

The newly authored original cases include real native Premium consumer ingest and coordinator
classification, a registered controlled clock fixture (not a production trusted-clock claim),
wrong-Account evidence and pull failure; pre-delivery real advisory-latch interleaving; a late
native member guard refusal after partial creation with zero-trace rollback and exact fresh
retry; deferred event COMMIT failure and fresh-Root retry/replay; stale native fences/wrong Root
and node; namespace/membership/global leadership cases; name, single-Account and occurrence
two-Root races. At least one Guild created by actual Found drives all five existing operations.
A second single-field late rank-name substitution nominates cause-specific server-log
attribution to the native Found-default-ranks guard, separately from generic rank2 rollback.
No guard execution is inferred from a sanitized Unavailable result alone.
The new nine bodies plus the prior twenty nominate10 pure and19 PostgreSQL cases; none has
been executed on this successor yet. PostgreSQL optional-environment early-return must never
count as actual qualification; the eventual dispatcher must require the real canonical fixture.

The Account guard has a single current presence Character/session per Account. Tests do not
fabricate simultaneous current Characters of one Account across Worlds: a valid offline leader
of that same Account on another World proves global ACCOUNT_HAS_POSITION, while same-current-
actor two-Root formation races prove only one owner/position is created. Native EXCLUSIVE
admission locking also limits race claims to complete native outcomes, not isolated row locks.
The event-failure case is a proven database refusal, not a claimed physical COMMIT-unknown
injection; fixed request/receipt replay still follows the native Root's actual outcome contract.

The owned, disabled PostgreSQL wrapper now loads literal original native root modules and their
unchanged module attributes, so its Root and private Premium coordinator share the genuine full
native context. No public authority facade, helper visibility widening or copied Premium owner
is introduced. Its exact declarations/path provenance are recorded outside the repository in
`guild-1-found-native-3-evidence/wrapper-native-root-declarations.json`. Production module/Cargo/
canonical-test registrations remain untouched, and enabling this wrapper has not yet qualified
it. Existing native fixture expectation context is retained; no new blanket warning exemption
is added. Resource-bound original selected-body qualification and repository CI limits will be
reported from actual outcomes.

Formation activation/four-vices, daily Premium/vice/deadline jobs, rank/leader changes, disband,
Bank/Hall/retirement closure, normative event/retention admission, production installation and
privileges, runtime Guild routing/admitted Account selection, and shared module/Cargo/protocol/
canonical-PG wiring remain CP work. No new product eligibility, fee, clock authority or numeric
registry lease is assumed. The worker does not commit or push; root alone may publish the
coherent successor to the existing draft after independent exact-source review. CP alone owns
ready/merge/queue actions.

## Native Found successor qualification, 2026-10-15 (owned component only)

The Found successor now has source-bound native execution evidence in
`guild-1-found-native-3-evidence/attempt-4`: **10 pure and 19 PostgreSQL bodies passed**,
zero failed/ignored/filtered, PostgreSQL `170006`, PG duration 90.25s. These are the
original seven domain and 22 owner bodies, with only recorded outside-tree test
attribute/visibility/forwarding transforms; the drivers require configured canonical PG
before each PG body. All five Rust hashes and actual dependency hashes stayed unchanged.
The old five-command bodies remain compatible, and a genuinely Found-created Guild runs
all five commands. Native admission's EXCLUSIVE relation locks serialize complete passes;
two-Root results do not claim isolated row-lock stress.

Fresh enabled native-wrapper and selected-original-body configured Clippy, native normal
cfg(test) rlib and both drivers passed. Production configured Clippy passed in attempt 2
and was reused only after complete production-input and fresh extern byte proof. Its seven
unused private Found-chain Rust warnings remain visible: **warning-free production and
repository all-target CI are not qualified**. Attempt 1's actual strict failure (seven
unused diagnostics plus the subsequently repaired complex tuple) remains preserved.
Attempts 2/3 retain actual wrapper module-path and owned test error-type compile failures;
no runtime RED is invented from them. No suppressions, fake caller or helper visibility
changes were added. The owned wrapper loads the two native non-mod.rs trees through
ordinary declarations under `../src`, retaining native child resolution and crate aliases.
Its tracked `cfg(any())` remains pending CP registration; disabled metadata is not a test pass.

The actual late-savepoint case proves zero retained footprint after each injected failure,
then a fresh commit of the same original request and terminal receipt-first replay. Bounded
server evidence identifies `game_guild_command_mutation` line 65 at 23:22:44.422 UTC,
PID 9974, the native rank INSERT and `Found must create its three bound default ranks`.
The otherwise-valid `Deputy` substitution therefore reaches the intended Found guard.
SQLSTATE `23514` is bound to that source branch; the server log does not print a raw state.
The member-rank substitution also reaches its immediate founder guard. Deferred event
failure is proven abort plus genuine fresh-Root retry, not physical COMMIT-unknown injection.

Premium cases use the real coordinator/consumer and registered fixture clock, not production
clock deployment. NotActivated holds the actual shared latch and same physical transaction;
explicit savepoint rollback prevents late errors from leaking creation. The native BANK
junior predicate currently reads its accepted NoStarterIsland fact until a real starter
source exists; no numeric level, fee, or caller-supplied eligibility fact was introduced.

The component remains private, UNINSTALLED preparation. Numbered migration, production
privileges/definers, normative event/retention admission, module/Cargo/canonical-PG and runtime
wiring, formation activation/deadline/Premium jobs, rank/leader changes, disband, Bank/Hall,
retirement and exact-head repository CI remain open. This does not close the full mechanic
or claim production/merge readiness. Root alone may publish the frozen existing draft;
CP retains ready/merge/queue authority. Independent exact-candidate review follows freeze.


## Native title/message successor, 2026-10-16 (SOURCE ONLY, UNVERIFIED)

This successor starts at published draft `017b71d6912c8171ebaf6013b125b6b59b9135f1`
under root's exclusive five-path lease. It adds actual native `SetTitle` and `SetMessage`
entries to the existing Guild owner. Accepted GUILD0 §§3.1, 3.2, 4.1, 4.4 and 9 authorize
these bounded edits; production installation and the full Guild mechanic remain open.
The domain data file is unchanged. Historical Found source, evidence and qualification
above describe that predecessor only; they do not qualify this successor.

`commit_guild_text` accepts bounded text and an expected Guild revision as DATA. The same
native semantic transaction prelude checks actual recovery before receipt replay, then
uses the existing native admission/session/Node fences, Character roots, locked Guild
state/revision and ordered current member rows. Only the current leader sets a current
member's title; the current leader or vice sets the message. Other-Guild membership or a
request target never supplies authority. A fresh permitted edit compares the expected
revision, returns typed `StaleRevision` on mismatch and advances one Guild revision on
success. The old membership entry rejects the text operation kinds rather than supplying
missing text. Existing Found and all five membership commands retain their own paths.

Titles preserve their received text, allow an empty value and contain at most29 Unicode
scalar values. NUL is refused because PostgreSQL TEXT cannot represent it. No character-name
grammar or message-only control/trim rule is imposed on titles. Messages use the actual
native `crate::chat::ChatText::parse`: received UTF-8 bytes at most1020, no control characters,
then trim, nonempty and at most255 Unicode scalar values. Empty/whitespace-only messages
are refused; this component introduces no clear-message exception.

The immutable occurrence/CommandRef binding includes operation, target, normalized exact
stored text and expected revision. Terminal receipt replay precedes fresh role/session
checks after valid recovery, so replay never changes later text. Successful edits retain
one immutable operation and one private exact text/audit proposal in the same transaction.
The closed private SQL CHECKs, immediate field/role/revision guards and deferred transition/
event proof extend together; only the bound title or message and Guild revision can change.
These are uninstalled owner proposals, not a production migration, grants, admitted codec,
event ID, retention implementation or runtime caller.

Seven additional original test bodies are authored (one pure and six PostgreSQL), appended
after the complete inherited29 bodies/helpers. The inherited66856-byte owner-test prefix
is preserved literally; the seven domain bodies are unchanged. The intended combined
qualification is11 pure and25 PostgreSQL bodies, all **UNEXECUTED** on this successor.
The authored cases cover an actual Found-created Guild, title29/message255 Unicode bounds,
empty title and invalid messages, leader/vice/member/nonmember/foreign-Guild roles, missing
target and foreign World, both closed Guild states, expected-revision and occurrence races,
restart replay, exact target/text/revision/operation/Guild/CommandRef conflicts, both native
mutation boundaries' stale session/lease/connection/scope/Node fences, real native assignment
revocation, wrong Root and negative recovery data, deferred event and unrelated-field
rollback, missing same-TX proof, overflow and unchanged CharacterRevision.

The foreign Node fixture registers once and reuses its genuine proof for both operations.
The revocation fixture uses the actual async native `RuntimeScopeAssignmentWriter::open`,
reuses its Root's existing `item-transfer-writer` registration, supplies an independent
operation3 exact-scope grant for the authenticated fixture DB session and reads back actual
REVOKED state/generation. Neither opaque authority construction nor immutable recovery-row
mutation is used. Negative recovery DATA reaches the same production transaction prelude
before a historical receipt; it does not claim a manufactured stale Root capability.

The early independent architecture source review found no confirmed blocker on its exact
core hashes. Final coherent independent review and cheap-check evidence are recorded outside
the tree in `guild-1-text-native-4-evidence`; source-only review and formatter/governance
results never imply native compilation or PG success. Heavy qualification remains HOLD
behind Ground then VIP Remove until root grants the shared-lock resource slot. No Cargo,
Rust compile/codegen, PostgreSQL bootstrap/execution or large source hydration was run for
this successor during this resume. Previous strict production unused-private warnings
remain unqualified; no warning exemptions or fake production callers are introduced.

CP retains module/Cargo/canonical-PG registration, numbered migrations and production
privileges, normative event/retention admission and runtime consumers. Formation activation,
Premium/vice/deadline jobs, rank/leader/resign edits, disband and Bank/Hall closure remain
outside this bounded increment. No commit, push or remote mutation is authorized for this
worker; root alone may publish a qualified coherent successor to the existing draft, and
CP retains ready/merge/queue authority.


### Native attempt8 and narrow observation repair (2026-10-16)

The first actual native qualification attempt used source checkpointf4d37d76 and the
reviewed recipev8 under Root's exclusive resource grant. Configured production Clippy
and the disabled tracked wrapper passed. Strict production exited1 for eight unused
private/dead-code diagnostics and remains RED. The enabled native wrapper then exited1
with six E0599 errors: the new text-test tail called `Harness::root_revision()`, which
the real native Harness does not provide. The run stopped there. No native codegen or
any of the36 test bodies executed; no runtime/PG success is claimed. The immutable
attempt8 logs, exact source baseline and retired-new-metadata proof remain retained.

Checkpoint2 repairs only that new-tail observation: a private `text_character_revision`
query reads the real `game_character_roots.character_revision::text` for the fixture's
native CHARACTER through the existing pool, using the same SQL/binding already present
in the inherited Found test. Six call sites now use that read. Expected values, equality
and rollback assertions are unchanged; no assertion is removed. The complete66856-byte inherited owner/helper prefix and all29 inherited
bodies remain literal. Production core, shared Harness,
authority construction, grants and public APIs are unchanged. This is an observation
repair, with all11 pure and25 PG bodies still UNEXECUTED pending independent review and
a fresh Root grant. Checkpoint2 and its narrow diff are retained in
`guild-1-text-native-4-evidence/resume-2`; checkpointf4 and attempt8 remain historical.

### Native attempt9 and explicit async result types (2026-10-16)

Actual attempt9 used checkpoint2 and recipev9 under a fresh Root grant. Configured
production Clippy and the disabled tracked wrapper passed; strict production remained
RED for unused private/dead code. The enabled native wrapper then stopped with five
E0282/E0283 diagnostics at two new-tail async result bindings. No native codegen or
any of the36 test bodies ran. Attempt9 logs and checkpoint2 remain unchanged.

Checkpoint3 adds only `TestResult<()>` to those two `outcome` bindings, resolving the
error conversion type explicitly. The async bodies, cleanup, assertions and test names
are unchanged. Reversing the two annotations restores the complete checkpoint2 test
file byte-for-byte. The complete66856-byte inherited owner/helper prefix, all29 inherited
bodies, production core and shared Harness remain unchanged. All11 pure and25 PG bodies
remain UNEXECUTED on checkpoint3. Formatter and source review do not establish runtime
success; a fresh Root execution route and grant are required. Evidence is retained in
`guild-1-text-native-4-evidence/resume-3`; no retry is authorized by this repair.

### Final native component qualification (2026-10-16)

Final result on source checkpoint3 (`c14b75ac`): **11 pure +25 PostgreSQL bodies PASS**,
with no ignored, measured or filtered bodies. These36 comprise all29 inherited bodies
and seven new text bodies. The complete66856-byte inherited owner/helper prefix remains
literal in authored source; its22 inherited owner bodies plus seven unchanged domain
bodies are the preserved29. The current qualified command inventory is Found, Invite,
Revoke, Accept, Leave, Exclude, SetTitle and SetMessage.

The actual native Cargo command and all37 extern identities were captured from the
literal native package/dependencies on retained Rust1.94.0 in an isolated task-owned PC
context. Configured production, disabled tracked wrapper, enabled native wrapper and
selected-body Clippy checks passed. Native library codegen and both original-body drivers
passed. The11 pure bodies passed in the compile run. After the first PG run refused an
unsafe15432 URL before database creation, a verified task-local canonical5432 forward
exposed the same PostgreSQL17.6 fixture without changing the native URL guard. A separate
PG-only run of the exact retained artifact passed all25 bodies and emitted49 actual
`GUILD-NATIVE-PG server_version_num=170006` markers. No recompilation or source change
occurred between those runs; qualification is this explicit11+25 composition.

The native library and both driver hashes, all compiled source hashes, all37 extern
hashes and all five source-checkpoint3 files were unchanged before/after replay. Exact
whole-file reconstruction verifies all nine validation source/registration mappings:
only outside module/fixture mounts, test-attribute/header visibility changes and forwarding
functions were applied; all original bodies and assertions remain literal. Earlier
attempt8/9 compiler failures, the PC path failure and the initial wrong-port PG refusal
are retained as historical RED evidence, with their repaired successors separately bound.

Strict production remains **RED** for eight unused-private/dead-code diagnostics. The
configured checks above do not establish a warning-free production build. No new warning
exemption or manufactured production caller was introduced. This scoped original-Guild-body
qualification does not claim the whole Game-server test suite, latest-main integration
or a live production/runtime entry. CP still owns module/Cargo/canonical-PG registration,
numbered migrations, production privileges, admitted wire/event/retention contracts and
runtime consumers. The private SQL proposals remain uninstalled.

Actual receipts, original logs, source/body/context/extern proofs and the final document
bridge are retained in `guild-1-text-native-4-evidence/resume-3`. Independent native review
`3941ca3a` verifies the composed36 result and preserves the separate strict RED. This final
append changes only the task record; all four Rust files remain source-checkpoint3 bytes.
Root alone may publish the coherent successor to existing draft PR2136. No ready, merge
or queue action is requested.

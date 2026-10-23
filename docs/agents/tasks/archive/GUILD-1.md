# GUILD-1 — guild core tables and transactions (skeleton task record)

```yaml
task_id: GUILD-1
title: "GUILD-1: guild core tables and transactions"
mode: IMPLEMENT
status: completed
pr: 2302
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

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2302](https://github.com/Oteryn/Oteryn-Game/pull/2302). Fenced guild core, ranks/membership, disband participant guards and deadline jobs are merged; wire, chat, bank and halls remain separate cards. Evidence: `crates/gameplay/durability/src/durability/guild.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

## Goal

Guild, rank, member, invitation and account-leadership tables; found, invite, join, leave, exclude, rank, resign and disband transactions; formation and vice World jobs (GUILD0 §3, §4).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

GUILD-1 (hard, persistence and security review). GUILD-BANK-1, GUILD-CHAT-1 follow in Wave 4; GUILDHALL-1 and GUILD-WIRE-1 are Wave 5.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | Not a prerequisite after accepted SOCIAL-MAP §1.2/§2.2; no PARTY schema assumed |
| BANK-1 | MERGED (#1766); SOCIAL-MAP §1.2/§2.2 narrows the guild-core bank prerequisite to BANK-1 |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| ECON-RET-0 / GUILD-RET-0 | MERGED (#1748) |
| GUILD0 acceptance | ACCEPTED per plan §7 status list (file Status line still reads CANDIDATE: CP to confirm) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `crates/gameplay/domain/src/domain/guild.rs`
- `crates/gameplay/durability/src/durability/guild.rs`
- `crates/gameplay/durability/src/durability/guild_audit.rs`
- `apps/game-server/src/lifted_tests/durability_guild_tests.rs`
- `apps/game-server/src/node/guild_jobs.rs`
- `apps/game-server/tests/guild_postgres.rs`
- `docs/agents/tasks/archive/GUILD-1.md` (this record: status and notes only)

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

- durability/mod.rs and domain/mod.rs module registration
- node World job scheduler registration (formation/vice jobs) in the existing node job list
- GAME_EVENT_FOUNDATION_REGISTRY.json (guild event, retention `GUILD_ACTIVITY_RETENTION_V1`)
- no wire in this task (GUILD-WIRE-1, Wave 5)

## Inseparable bundles (never split across PRs)

- migration + guild writer + durability tests
- World jobs + their durable checkpoints

## Acceptance criteria

- [x] Every transaction of GUILD0 §4 has a durable test incl. concurrent invite/join and leader resign.
- [x] Disband leaves no orphan rank/member rows; formation job is restart-safe.
- [x] Premium gate via `premium_current(account)`.
- [x] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.

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

The only code write is the previously absent `crates/gameplay/domain/src/domain/guild.rs`:
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


### SetRank increment on published 7f6bdf04 — authored, UNVERIFIED

The next native command is `SetRank(target, expected_revision, level)` within the
existing Guild owner. GUILD0 §3.2 (lines149–173) requires both strict inequalities:
current actor rank < current target rank and current actor rank < proposed rank;
the proposed level must already exist in this Guild. This includes ordinary rank3
actors over rank4, without importing the leader/vice-only Exclude or text predicates.

Only a new move from a non-vice rank into level2 consumes the actual target Account's
Current/NotActivated Premium decision (§3.2 lines131–133; §3.3 lines185–192). Existing
vice-to-vice edits retain the exact leadership position without regrant; demotion deletes
only that Account/Guild/Character position. Global Account contention is serialized by
the native Account guard after Guild/member locks. No target root lock, caller Account,
caller role or caller Premium flag supplies authority. The real writer revalidates the
native target selector and current recovery/session/Node/member facts, applies expected
revision CAS and emits one closed immutable rank receipt/event in the same transaction.
NotActivated uses the actual coordinator's borrowed physical transaction and a savepoint;
callback refusal/error cannot commit partial Guild/member/position/event mutation.

GUILD0 §3.3 lines179–182 counts all current rank2 memberships for synchronous fourth-vice
FORMING→ACTIVE activation, including retained vices whose Premium later lapses. The actor,
leader and all other vices are not new Premium qualifications. ACTIVE below-four changes
set the first vice deficit once; further deficit changes preserve it and a timely cure
clears it. Formation/deficit deadline comparisons use the actual database transaction
clock, with equality permitted and strictly late cure refused (§3.2 lines169–173).
`premium_deficit_since` remains daily-job-owned and unchanged by this command. No daily
job, formation scheduler, transfer-leadership or shared-runtime path is implemented here.

The first coherent authored test set adds12 bodies (one pure and11 PostgreSQL families):
closed request/audit binding; target Premium/position transitions; both rank inequalities
and Guild/World scope; activation/first-deficit/cure; historical replay/conflicts/CAS/
overflow; two-root revision/occurrence races; global Account contention; physical-TX
savepoint/closed-SQL faults; current session/Node/recovery and target-selector recheck;
real Found→Invite→Accept feed-through; exact physical database-clock deadlines; and shared
Premium-latch ordering. The complete96638-byte inherited owner/helper prefix and all36
previously qualified bodies remain literal; the candidate total is48 (12 pure/36 PG).

All new native execution, ordinary Cargo compilation, Clippy, PostgreSQL and complete
candidate qualification remain **UNVERIFIED** at this source checkpoint. Early core
review47dce617 found no blocking semantic issue; it is not final source/native approval.
The separate ordinary Cargo probe uses immutable snapshots, never a live author clone.
Private SQL proposals stay uninstalled, and CP integration/production privileges/wire/
event-retention/runtime boundaries from the prior increment remain open. Root alone
publishes a reviewed candidate to existing draft PR2136; no ready/merge/queue action.

### SetRank — actual native48 qualification

The authored SetRank source at checkpoint `f205a399` passed ordinary native Cargo
compilation and all48 focused unit tests:12 pure and36 PostgreSQL, zero failures and
zero ignored. This reruns all36 inherited bodies with the12 new bodies; their complete
96638-byte owner/helper prefix and unchanged domain file remain literal. The command
inventory is now Found, Invite, Revoke, Accept, Leave, Exclude, SetTitle, SetMessage and
SetRank. Rank changes require both current-role inequalities, an existing proposed rank,
expected revision CAS and the actual target Account's Premium/position decision for a
new vice promotion. The tested owner commits rank/position/revision/receipt/event and
synchronous formation/vice-deficit transitions atomically.

Validation used the literal native package, manifests/dependencies and Rust1.94.0 with
`cargo test --locked --offline -p oteryn-game-server --lib guild::tests:: -- --nocapture --test-threads=1`.
Four validation-only cfg(test) declarations in three outside registration roots mount
the actual Guild/domain/audit and native PostgreSQL fixture modules. No body dispatcher,
test-attribute rewrite, API shim, copied auth helper or new product lint exemption was introduced.
The actual ELF list equals the48 expected native names;3194 other library tests were
filtered and are not claimed as executed. All36 PG functions emitted actual native
`GUILD-NATIVE-PG server_version_num=170006` markers,80 markers total. The full408356656-byte
native test ELF `57af7b93` and the18813-file physical source inventory were unchanged
before/after; the inventory is not a claim that every file was consumed by the compiler.

The corrected strict native-unit Clippy check passed with zero diagnostics. It uses
ordinary Cargo `--lib --profile test --no-deps` with the literal workspace lint policy
and `-D warnings`; the actual primary compiler is `--test` with `profile.test=true`.
This qualifies the native unit component, not a non-test production Guild graph.
The first current strict Clippy run returned101 for ten `expect_used` diagnostics solely
in the unchanged shared item-transfer PostgreSQL fixture, with zero authored-file
diagnostics. The corrected validation fixture mount restores the exact inherited native
allow block from ownHEAD `tests/guild_postgres.rs`; it changes no product/helper/test
body or workspace lint policy. The initial RED remains retained separately.
The earlier text-increment strict-production eight-dead-code RED is historical evidence,
not a current production check. This component is registered only for validation under
cfg(test); non-test production compilation/Clippy would omit Guild and is not claimed.
CP still owns production module registration, SQL installation/privileges, admitted
wire/event/retention contracts, runtime adoption, daily jobs and latest-main integration.
The private SQL proposals remain uninstalled; no live gameplay entry or full-suite pass
is inferred from these focused native results.

The retained compile proof is `reviews/guild-rank-pc/f205-native-normal-cargo-compile-actual.json`
(`2859e660`); runtime48 proof is `f205-native-normal-cargo-runtime48-actual.json` (`178fd56d`)
and raw runtime log is `fcd7adf8`. Independent actual-native review `6b1d0ff7` verifies
the focused48 result and its context/resource/source/PG proofs. Source reviews `0ab78391` and `744ae94e` independently
verify current authority, same physical transaction, target Premium, lifecycle/deadlines,
closed SQL/audit and all12 meaningful new test families. All four Rust files remain
source1 bytes; this final append updates only the evidence record. Root alone publishes
the reviewed successor to existing draft PR2136; no ready, merge or queue action.

Current strict-unit Clippy proof: `f205-unit-clippy-strict-original-fixture-scope-GREEN-actual.json` (`f8d8afa9`);
independent binding: `guild48-current-clippy-native-resource-review.json` (`c75cc304`).


### Resign increment on published6128 — authored, UNVERIFIED

GUILD0 §3.2 lines155–159 requires the current leader to choose a current vice of the same
Guild, check the successor through the real native BANK junior decision, and immediately
swap levels1 and2. Both actual Accounts consume the existing native batch Premium gate:
§3.2 lines131–133 requires Premium after delivery for every move into a new level1 or2.
Both exact Account/Guild/Character leadership positions already exist and stay byte-identical;
the table stores no rank. The writer locks them by AccountId after current Guild/member
locks and demotes the old leader before promoting the successor, preserving the immediate
unique-leader index and final deferred snapshot proof. Expected revision is this private
proposal's precondition, reusing existing CAS; no Resign wire field/codec is allocated.

The real Root recovery/session/Node/current-role/target-selector checks run again at the
writer boundary. Historical immutable outcomes replay before fresh roles/Premium, after
recovery. Genuine NotActivated shares the batch gate's physical transaction and a savepoint;
delivered Current decisions use the existing Root transaction. One immutable receipt/event
binds all16 closed Resign audit fields to the same operation/outbox; old1..9 event encodings
remain unchanged. A swap keeps vice count, lifecycle, formation/deficit times, names/titles,
positions and CharacterRevisions unchanged. No property, bank, shared SQL writer or job is added.

The current native BANK port is `fee_payer_is_junior`; its actual starter-island source
returns NoStarterIsland for every Character until DAWNPORT. No invented Junior flag/provider
or fabricated positive Junior PG case is used. GUILD0 §7.3 resolves a Guildhall OWNER from
the current level1 member; the actual current House ACL still reads Character ownership,
so its Guildhall adoption is CP/GUILDHALL work. No fake Hall binding or no-Hall eligibility
guard is added, and no live Guildhall access/ACL behavior is claimed by this owner increment.

The authored12 new bodies (one pure/11 PG) cover both-account Premium/current roles,
legacy-entry rejection, state/CAS/overflow/refusal/replay, real formation and follow-on leader
text authority, both physical transaction paths/ordered-swap rollback, all16 missing/mutated
Resign audit fields plus unknown-field rejection with matching forged NEW-row hashes,
current fences/actual assignment revocation/recovery-first, two-root races and batch-latch
ordering. The complete141302-byte inherited owner/helper prefix and all48 old bodies remain
literal. Candidate total is60 (13 pure/47 PG); all12 new bodies are UNCOMPILED/UNEXECUTED here.

Early core71399 passed ordinary native unit Cargo CHECK, not codegen or runtime. Independent
early transaction review6dbf5592 found legacy payloadless-entry and full audit-field binding
gaps; repaired core2b6e90bb0 passed exact-delta review6648b3ab. The immutable early source1
and actual compiler receipt remain separate from this full candidate. Integrated source,
ordinary Cargo/Clippy/native60 and final publication qualification remain UNVERIFIED.
Private SQL/module/runtime/wire/event-retention installation remains CP-owned; Root alone
publishes a reviewed successor to existing draft PR2136. No ready/merge/queue action.


### 2026-10-16 — authored native EditRanks batch (qualification pending)

This versioned five-path candidate retains the repaired Resign source2b and adds
EditRanks to the same native Guild owner. The prior Resign checkpoint/run remains
frozen. No current Resign60 or combined72 execution result is inferred here.

Accepted GUILD0 §§3.1–3.2 and GUILDS_V1 GI04/GI05 govern 3..20 contiguous levels,
current own-Guild leader authority, expected revision, distinct canonical name
keys and removed-level membership remapping to the new lowest level. The owner
uses the actual database game_character_name_key function. The cited contract
supplies no rank-specific lexical/length grammar; no CharacterName parser,
29-character rule or alternate normalizer is introduced. Nonempty/NUL checks
protect the existing SQL storage; the existing bounded audit envelope remains a
private structural limit rather than a per-name policy.

The real Root recovery/admission/session/node spine resolves current authority
and checks the locked Guild/leader/CAS. One complete CharacterId-ordered roster
lock precedes mutations. Rank-name uniqueness is named, DEFERRABLE INITIALLY
IMMEDIATE, and deferred only by the new operation so valid name cycles work.
Retained levels stay present; removed-tail members move before definitions are
deleted, preserving the immediate membership FK. Levels1/2, global leadership
positions, titles/join times, Character revisions, lifecycle and deficit times
remain unchanged. No Premium, junior or property writer is required for this
rank-definition edit. Numeric House ACL rows are not rewritten.

A private action11 binds the exact ordered name vector and expected revision to
one immutable receipt, one revision increment and one closed audit event in the
same physical transaction. Closed SQL guards enforce the exact final contiguous
rank set and full event binding; the payloadless legacy route refuses before
lookup. Old action1..10 identities and audit proposals retain their meaning.

All 188,289 bytes of the repaired inherited owner/test-helper source remain
literal, including all60 prior bodies. Twelve additional meaningful test bodies
(1 pure/11 PostgreSQL) cover names/cycles/grow/shrink/remap, real authority and
Revoke/current leader after Resign, CAS/replay/two-Root races, state/deadlines,
closed SQL/event tampering and rollback. Current totals are72 (14 pure/58 PG),
with126 source-derived expected PostgreSQL17.6 fixture markers; all new combined
qualification is UNVERIFIED pending actual ordinary native Cargo execution.

Resource's implementation and VIP's test proposals were read by the sole final
writer before application. Independent proposal/full-source/native/exact-C
reviews remain required. Production module/schema installation, privileges,
wire/events, public projections/relay/runtime adoption, jobs and Guildhall
integration remain CP gates; no ready/merge request is made.


### 2026-10-16 — native Decline authored; combined78 unverified

Accepted GUILDS_V1 §5 Decline permits the genuinely invited current actor to
remove only its own selected Guild invitation; absence returns NotInvited.
Existence is sufficient even when expired. The same Root/current-session/node,
World, locked Guild state and occurrence/replay spine applies. No Revoke
leadership gate, Accept cleanup of other invitations, Premium/Bank/Hall authority
or new wire allocation is introduced. One exact deletion, Guild revision and
closed immutable receipt/event commit in the same physical transaction. No rank,
leadership position, Character revision, lifecycle or deficit time is changed.

Six new meaningful bodies (1 pure/5 PostgreSQL;7 fixture calls) cover actual
selected/foreign invitations, expiry, current authority at both boundaries and
actual Revoke, replay/races, closed state, direct SQL23514 and rollback/event
forgery. The whole repaired226,597-byte72-body/helper prefix remains literal;
inside it the entire188,289-byte predecessor60 source remains literal. The one
new EditRanks session-negative repair aligns CommandRef with the changed test
fence session, retaining the same CommandId, so it reaches current authority
instead of an input-binding error. The actual strict72 nonminimal_bool RED is
retained; its one predicate was replaced by the equivalent is_none_or expression
without a lint exemption. Decline proposal parse RED and both fixture repairs
are preserved in versioned evidence.

Separately, repaired Resign source2b actually passed all60 tests, including all47
PostgreSQL families and111 actual server_version_num170006 markers; current
strict native-unit Clippy passed0. Independent actual review efabfca9 binds its
raw proof6b1925c3 and losslessly retained ELF82f0675e/gzipbbdc72b5. This earlier
qualification does not qualify the new combined78 source. Current authored totals
are78 (15 pure/63 PG), with133 expected markers derived from source only. Final
ordinary native Cargo/strict Clippy/PG execution and independent actual review
remain UNVERIFIED for this candidate. Genuine Disband remains blocked on absent
GuildBank/Hall release/closure ports; no fake partial implementation is claimed.


### 2026-10-16 — final native Resign/EditRanks/Decline qualification

Source4 `f894316d` actually passed current strict native-unit Clippy with
-D warnings, ordinary Cargo --lib test compilation and exact native --list78.
All78 selected bodies passed exactly once (15 pure/63 PostgreSQL families),
with133 actual GUILD-NATIVE-PG server_version_num170006 markers. Execution was78
exact one-body invocations of the Cargo-emitted native ELF, at most2 concurrent;
each has its own actual 1-passed footer. Their exhaustive disjoint union is78;
no aggregate78 libtest footer or sum of filtered counts is invented. The real
global list has3,272 names;3,194 distinct other names were excluded. This is
focused native unit-context qualification, not a full global suite run.

Actual proof: reviews/guild-resign-pc/batch78-final-native-actual.json
(`d8dac9df`); compiler independent review `dc015dcc`; final composed-source review
`33693550`; final independent actual review `c957fc41`.
All18,813 physical source-file hashes stayed equal before/after (this does not
claim every file was consumed), as did54 actual compiler extern paths and the
native ELF. The four real cfg(test) Guild/audit/shared-fixture registrations
reside in three validation-only roots; the shared fixture retains its exact
inherited lint scope. No product registration, copied owner/decoder, API shim,
new lint suppression or test-body substitution was used. Rust/Cargo/Clippy1.94
producer versions, real --test/profile.test=true and argv are retained.

The qualified native ELF is389,983,952 bytes, SHA256
`f83b02d2cafef2227bcb1fe2e1b05aee5d1730ca26de6a5c3fe1c33e92d6aa03`.
Its persistent61,594,453-byte gzip has SHA256
`d8d5bf124d638caceba3b094a53cf01e842264d1969ea273125bfa8207c0d3dc`;
whole decompression reproduces the exact qualified bytes. Prospective activated
V3 RAM budget `93fcbbab` and retention addendum `c8cd5e2b` govern this run;
all144 runtime resource observations passed with no new OOM events. Actual
PostgreSQL memory observations and both fresh17.6 endpoint checks are retained.
The earlier C-floor stop is a separate failed resource attempt, not relabeled.

History remains explicit: original Resign0449 returned59/60 because a fixture
seeded historical91 before90. Its corrected source2b independently passed60
(`6b1925c3`/review `efabfca9`,111 markers, strict0, retained82f0675e ELF).
Combined72 source5dfd stopped strict Clippy101 on one nonminimal_bool finding,
before ELF/runtime72; source3ca0 and repaired intermediate72f635 were never
executed separately. The final78 includes the equivalent predicate and the
current-session negative-case repair. Decline proposal V2 parse RED and its
corrected V3 remain retained. Transport/collector sampling failures happened
outside native tests; no hidden native-test rerun is claimed.

This append preserves the entire54,904-byte authored card and all four Rust
files from source4. The complete226,597-byte repaired72/helper prefix and its
entire188,289-byte inherited60 prefix remain literal. The native inventory is
Found, Invite, Revoke, Accept, Leave, Exclude, SetTitle, SetMessage, SetRank,
Resign, EditRanks and Decline. Independent final doc/exact-C binding
remains a publication prerequisite and is recorded separately by the control
plane; this card does not grant publication or readiness authority.

Production module/schema installation, privileges, wire/events, public
projections/relay/runtime adoption, jobs and Guildhall integration remain CP
gates. Rank-specific lexical/length policy is unallocated; no CharacterName or
29-character rank grammar is invented. The BANK junior port currently reports
NoStarterIsland until DAWNPORT. Disband remains blocked on genuine GuildBank/
Hall release and closure ports. No non-test production graph, all-targets/full
workspace, live Guildhall transition, draft-ready or merge claim is made.


### 2026-10-16 — accepted deadline start/checkpoint increment (unregistered)

CP gap review comment6216375759 asks for formation/vice jobs and activation.
The original SetRank writer already activates FORMING→ACTIVE at four actual
vices, gates late cures, and maintains the first deficit timestamp; its existing
`guild_rank_native_formation_and_first_deficit_cure` test is preserved literally.
No second activation writer has been added.

The new `node/guild_jobs.rs` pass discovers at most100 ordered candidates without
Guild locks, then processes each in its own native semantic transaction. Current
reconciled Root recovery and admission relations are rechecked in every boundary
(GUILDS_V1 §3; GUILD0 §4.1). Current node incarnation is an additional process check,
not an invented World ownership grant. No session fence or Character root is used.
The operation key precedes the Guild lock; the locked World/state/database deadline
is rechecked before writing. Strict formation and14-day comparisons use database
transaction time and overflow-safe arithmetic.

GUILD0 §3.4 start is retained as (Guild,step1) plus exact private event DATA in the
same physical transaction as FORMING/ACTIVE→DISBANDING and one revision increment.
This freezes ordinary commands while real Hall/bid release and Bank payout remain
pending. Restart reads checkpoints; concurrent starts have a typed AlreadyStarted
outcome. Closed SQL guards bind revisions, state, database time and whole payload,
reject missing/forged publication, and refuse updates/deletes/truncation of retained
checkpoint/event evidence. There is deliberately no terminal DISBANDED writer or
claim that Bank/Hall closure has occurred.

Seven new bodies (one pure, six PostgreSQL) cover strict boundaries and overflow,
actual Root restart/new node, pending checkpoint reload, foreign World/no-longer-due,
publication-fault rollback/retry, concurrent start, current authority and stale
recovery, bounded100+1 passes and late candidate recheck. The inherited guild_tests
prefix remains byte-identical. Native execution is pending the lead's cloud
validation slot; source parsing/rustfmt and diff checks alone do not qualify it.

All production module/scheduler installation, numbered migration, public event
registry/retention and publication remain CP slot gates. Terminal Disband,
Hall release, Bank payout, bounded member/invitation cleanup and final no-orphan
closure remain blocked on CP composition of #2136/#2139/#2153. This increment
neither supplies fake closure participants nor marks this partial task complete.


### 2026-10-16 — independent review repair: explicit player Disband start

Accepted GUILD0 §3.2 requires the current leader's confirmed Disband start even
while Hall/Bank closure remains pending. The native payload-bearing private
`GuildDisbandRequest` now enters the existing Root/recovery/admission/occurrence/
current session/node/Character root/Guild/sorted member lock spine. Only the real
current same-Guild leader may start. Exact confirmation bytes are bound to replay
and compared to the current locked name. Name equality is a bounded private
confirmation choice consistent with proposed GUILDS_V1 §5; that document's heading
is CANDIDATE, so this does not claim its independent acceptance or mint a wire ID.

The same transaction stores private action13's immutable operation and exact
command event, reason3's pending disband checkpoint, state DISBANDING and one
Guild revision increment. It emits no duplicate job event. The SQL precondition
requires the actual leader and exact name; deferred guards bind the whole event,
checkpoint and state. Payloadless legacy calls refuse before lookup. Historical
replay retains its typed outcome without reacquiring current authority; altered
confirmation conflicts. Real Hall/bid release and Bank payout remain pending and
no member/rank/leadership rows are removed or terminalized here.

Seven further meaningful bodies (one pure/six PostgreSQL) cover bounded/closed
confirmation DATA; FORMING and ACTIVE starts, exact-name rejection and replay;
vice/member/other-Guild leader refusal; current fences/node/Root provenance;
forged-event whole-transaction rollback/retry; player/job race; and actual Resign
followed by old-leader refusal/new-leader success. Current source-derived total
is92 Guild bodies (78 inherited +14 new); runtime qualification remains pending.
Native review/validation must use this repaired source, superseding the earlier
held deadline-only manifest. All inherited78 bodies remain literal.

### 2026-10-16 — D1098 terminal controller and genuine owner ports (partial)

CP6219643346/D1098 assigns the complete DISBANDING→DISBANDED controller
to this PR in durability/guild.rs and node/guild_jobs.rs. This successor
implements Hall release/recheck → Bank payout/retained claims → bounded
member/invitation cleanup → final leader/leadership/rank removal and state4.
Immutable per-Guild ordinal steps and exact private event DATA bind each
revision and original payout UUID; no phase skip or second step in one
physical transaction is admitted. At most100 members plus invitations are
deleted per step. Leadership remains held until the final step; the snapshot
exception requires a genuine retained cleanup record for that removed vice.
Deferred guards require actual Hall and Bank owner predicates and no orphans.

The private transaction capability is minted from actual Root-bound recovery,
admission, current node and its current Channel assignment in the World,
then bound to the exact PostgreSQL xid and locked Guild revision. Consumers
recheck those facts; the complete prelocked Bank identity is also bound.
Hall/Bank occurrences precede Guild and sorted dependent-row locks. Retained
outcomes cannot mint current authority. Native per-Guild transactions preserve
earlier commits, and bounded keyset discovery avoids starvation by waiting rows.

The actual Hall bid-refund and Bank payout/current retained-claim ports are
their owners' implementations, not surrogate closure witnesses. HOUSE-1b
#2141 already has native bounded Inbox movement, but drained disposition
retains ownership, slot and content fence, with Character-only ownership.
The exact remaining C prerequisite is Guild GUILD_DISBAND start/resume/final
property/slot/ACL/fence release and current closure binding under the accepted
lock order. Until C supplies that genuine port, Hall returns typed unavailable;
the controller can commit a real bid refund and wait, but cannot pay out,
clean membership or terminalize. No successful fake predicate/test is added.

Six additional native17.6 bodies in owned guild.rs cover real wait/retry/Root
reopen, foreign/replaced xid and current Root/node/World rejection, stale
recovery after retained start, SQL phase/deletion/false-closure rejection,
bounded keyset/batch admission and concurrent two-Root waiting. The published
guild_tests.rs and its92 bodies remain byte-identical. These six bodies are
authored, not claimed executed. Lead's earlier combined native --lib type
check passed on its separately retained early snapshot; final source-bound
native compilation/runtime/Clippy and independent review remain qualification.
Owned-source rustfmt and whitespace checks pass; no Cargo was run by this author.

Stable source SHA256: guild.rs
6ad77f920264238af348819dd5a4d3facf401bd247939afb51fcdc8b77fa2034;
node/guild_jobs.rs
a74804564b68013238d48ac75dab9170aae19c508f1ddab99dd691bd569e1803.
Literal private SQL SHA256: GUILD_TABLES_PROPOSAL
6d91fc5e5dbe597c72633f46dc328f30525a17498be1acbeabcc75829247343e;
GUILD_GUARDS_PROPOSAL
343728dac2958c6229532edc98f560f747e98aae1ad445dba349a65345104d9a.
These are uninstalled owner proposals, not a numbered migration or production
grant. Production registration/scheduler/security/public events/retention,
remaining Premium/rank decisions and real terminal composition qualification
remain CP gates. D1098 resolves controller ownership; it does not complete
the whole task or authorize ready/merge/production publication.

### 2026-10-16 — exact native qualification after D1098 repairs

Guild Rust candidate ea8690495ba0d52a305d8c8174c93d7dece70910
passed98/98 selected bodies: all prior92 plus the six new terminal-boundary
cases, exactly one body per invocation, zero failed/ignored. PostgreSQL cases
used the genuine17.6 route (server_version_num170006). Evidence is retained
separately in evidence/guild-terminal-sql-repair; independent execution review
is D1102-execution-review.md. Retained native ELF SHA256:
5c0f35c7e979023f5a8d6d8250a47e5da15b96af858d1a3b0da2bb0ad3f3b7aa;
exact compiler-source archive SHA256:
e91d07f37751afc1e1b72d529c795c36f08f7b64c136e9c6d6d8461dd34455f4.
This supersedes the earlier authored-only qualification statements for these
six bodies; earlier failing generations remain retained as separate evidence.

The repairs use the existing concurrent native fixture join and parenthesize
the two PL/pgSQL IF/CASE expressions without weakening their predicates.
Qualified guild.rs SHA256 is
33cfa4d52600ecbb4f08bc39b47d0c6f0a9754713b19090d110cfdff091d362b;
the corrected guard SQL SHA256 is
387ba1c48521bb09a738acd6505084b65a64ec719dc4098171af5c9fb9cbb596.
Tables and node source hashes above remain unchanged. This documentation-only
successor leaves all qualified Rust/compiler-input bytes unchanged.

The98 passes qualify actual starts, fences, waits, financial boundaries and
negative terminal guards; they do not prove successful C property/slot closure
or end-to-end DISBANDED execution. That genuine C prerequisite remains open.
Whole all-targets Clippy remains RED on uninstalled caller APIs/context;
lead's final strict unit-context check is a separate disposition, not implied
by these passes. Production installation, public events and whole-scope
completion remain pending; no ready/merge/production claim is made.

### 2026-10-16 — CHAT current-membership owner DATA preparation

AUTHORING reopened from clean published `caf3c375c02c97db1bb667f384529a110bcfe7d9`
for the accepted GUILD0 §8 / GUILDS_V1 §7 owner-read gap. Only guild.rs,
guild_tests.rs and this task record change. Native Root methods now read current
send/login GuildId, rank, MOTD and actual Guild state, or read the recipient
Character against the authenticated payload GuildId. They take the real
Root-bound reconciled Character authority, current node and full Character
session/lease/scope fence. The same native semantic transaction asserts recovery,
locks admission relations, proves current session/node assignment, locks the
live same-Account/World Character root, then Guild, then member. Unlocked Guild
discovery is rechecked after the Guild lock; a concurrent changed binding fails
closed rather than producing stale/rerouted membership. No occurrence, mutation,
public protocol, storage schema, Premium role surrogate or foreign writer is added.

These results are committed read DATA only, never retained current authority or
queue/transport/socket custody. Absence supplies no membership; DB/authority
errors return errors. CHAT owns counted failed-read drops and its future delivery
composition. The precise delivery linearization point and DISBANDING chat policy
remain unresolved CP decisions. Returning actual Guild state does not choose
that policy. The Root-owned transaction pattern is preserved; no arbitrary
caller-TX authority API or locks across socket I/O are introduced.

Six appended native PostgreSQL17.6 bodies cover committed MOTD/rank changes via
actual text/resign writers; leave/accept G-to-H movement without old-payload
rerouting; actual exclusion; both read methods against session, lease, scope,
Character/session binding, World/Channel and genuine alternate node mismatches;
Root provenance, stale recovery and actual DB-read errors; and actual DISBANDING
state observation. All existing guild_tests.rs bytes are an unchanged prefix;
prior98 qualified bodies/evidence remain historical unchanged evidence, not an
execution claim for this successor. No Cargo, native runtime or heavy check ran
in this authoring batch. The six new bodies and exact-source native Clippy need
qualification after independent review.

Pinned Rust1.94 rustfmt (edition2024, skip_children), whitespace, governance
validator (22 documents/9 lanes) and124 governance unit tests pass. Source is
released stable for independent review before heavy qualification. SHA256:
- guild.rs: `efeff4b4daf0b3ee7a054f241911d49196af2bba79260c70642b3c96b42dca82`
- guild_tests.rs: `f4a345b49754d50a90d9d9127caa55b8fd0dc1ebed5545ae581a40e68ac2a91e`

This is a local source freeze for lead review, not a new published remote
candidate. No commit/push, installation, ready/merge or whole-task completion
claim is made. Accepted review repairs must reopen AUTHORING and replace hashes.

### 2026-10-16 — D1167 actual C participant caller checkpoint

AUTHORING reopened from published `ce4ee5833118879e37cad67b6b7edef39fcc5289`.
CP D1167 comment6225337534 assigns this controller, cursor, private events,
grants and the genuine context-borrowing getters to #2136. The exact C source
is #2137@`50bc410809bdb0f909160134b487fa7ed76e0c8e`: verified source SHA256
`house/guildhall_release.rs`20903ec4c576f4dda92a7a6d13c55b109d781be0b2a0a6f9b8756d56e7bab5a1,
`durability/house_ownership.rs`026722307f6035cc962deced5b911c35d8fa2092dfb1a1f86a79425358f9778c.
Those actual files, rather than historical mapping proposals, define this batch.

The Root controller now calls C's actual opaque reservation/advance/assert APIs.
It allocates work IDs with the existing native UUIDv7 allocator before an attempt;
D retains the stable operation and C next-step as DATA in an append-only private
cursor/event journal. Each physical transaction allocates a fresh reservation ID,
takes occurrence/recovery/admission/current node+World evidence, then asks C to
pin actual sorted recipient Character roots FOR KEY SHARE at position2 BEFORE
any Guild row or D cursor insert/FK check. It then constructs the genuine locked
Guild spine and rechecks the discovered D cursor. The getters borrow the actual
Root-created recovery record and secret-bearing native node proof; no matching
current facts are reconstructed from retained work.

C Progress writes its D cursor/private event in the same physical TX and STOPS;
no bid refund, Bank payout, membership cleanup or phase2 runs. C Released yields
an opaque physical-TX proof revalidated before financial and bounded cleanup/
terminal consumers. Every later TX reserves afresh and obtains a fresh proof.
All errors explicitly roll back the complete physical TX; retries return to the
same lock order without a NOWAIT exception. The existing public job request
fields remain byte-compatible with Bank's existing literals.

D's proposed SQL guards bind raw cursor/event writes to actual same-TX protected
C reservation/authority and disposition-step evidence; Released observations
require C's exact-reservation terminal predicate. The journal is immutable and
monotonic with its whole canonical private payload; deferred verification rejects
lost C evidence or Progress paired with a terminal D step. Runtime receives
bounded D journal grants, never new grants to C storage/secret material. This is
private preparation DATA, not an admitted public EventEnvelope or duplicate C
item/custody store. Existing cleanup bounds and typed owner prerequisites remain.

NOT_FULLY_QUALIFIED (D1122): pinned1.94 rustfmt and git diff --check PASS;
governance22 documents/9 lanes PASS. Native compile, Clippy and PostgreSQL are
NOTRUN for this successor; no previous104/98 PASS transfers. The local branch
lacks the foreign C module/reexports and genuine native house_release_secret()
getter (C owns that shared accessor after #2163); original d77 fence ref remains
unavailable. No stub, copied C implementation, foreign edit or fabricated
positive fixture substitutes for those dependencies. The real C12/House17
PG matrices and exact rebased composition remain required before qualification.
Existing native test bodies and retained prior evidence are unchanged.

Local stable source release for independent review before any heavy execution:
- guild.rs `4285cadf338b2aac96d8a2255ac9a360b90324788a4c3b9d102d49d3d043cd06`
- node/guild_jobs.rs `754bbb7d89184c729faabb747fc204baae109a05ed67924a304814fbee4f2190`

No commit/push, rebase/merge, foreign C/E/Bank/shared-file write, @codex trigger,
ready transition, CI restart or fullqualification claim is made. Accepted review
repairs reopen AUTHORING and supersede these source hashes.

### 2026-10-17 — CP bounded checkpoint: main merge and composition probe

The branch merges main `f094e032`, which includes #2197 NPC-ECON-1 and the CHAT-2B re-land #2227. Owned files are unchanged by the merge. The `house_release_secret()`
prerequisite is now met on main by #2163's `NodeIncarnationProof::sql_possession_secret()`.
The modules stay unregistered and the harness stays `#![cfg(any())]`, so CI does not compile them.

Composition probe on the PC (local only, not pushed; D1420 lib.rs touch, D1429 `slot-run.sh`). The setup:
- this head, overlaid with #2137 `c03dc6c1`, #2139 `65a834fe` and #2153 `4b435e05`;
- registration lines only.

Results:
- `cargo check --lib`: 0 errors.
- `cargo clippy --lib -- -D warnings`: PASS.
- `guild_postgres` harness: builds and lists 104 guild cases.
- PG run (shared PG 17.6 via `in-pg-netns.sh`, filters `durability::guild:: domain::guild:: node::guild_jobs::`):
  101 passed, 3 failed. The same counts held on main `1d632319`.

The 3 failures are in `disband_terminal_tests`: `real_owner_wait`, `pending_cursor` and `two_current_roots`.
Each fails with `Unavailable(Database)` at the House port, because their `install()` loads
no House release SQL (`game_house_guild_reserve`). That SQL in turn requires:
- the HOUSE-1b #2141 guild fence ABI (`game_house_disposition_guild_fence_abi()`, its migration 0121);
- the item/entry hooks;
- the BED-1 #2147 revision hook.

These cases stay unqualified until that stack composes. Nothing here is skipped or rewritten.

`durability/guild.rs` and the Bank/Hall/House modules import each other, so only the full set of
four compiles. Activation (registration, migration from the first free number at or above 0124,
harness enablement) belongs in one slot commit, after #2141 and #2147. The proposed owner of that slot is #2153 GUILDHALL-1.
The composition order is in the #2136 status comment.

## Completion + wiring (post-checkpoint)

Imported from closed draft #2230 and rebased onto the ADR-0022 layout (`docs/architecture/ADR-0022-gameplay-crate-layering.md`); every existing path below was verified on origin/main `59785d76`. The checkpoint merged unfinished; nothing in the running game calls it. The completion PR must not merge until the mechanic is playable end to end through the real dispatch path, CI is green and Codex review is clean. Dispatch chain: `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (command `else if` chain) and the `GATED` table in `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` (highest capability id on main: 30). Wire modules live in `crates/protocol-oteryn/src/`. Migration numbers: next free number at allocation time (only one migration PR in the queue); 0130 MAP-PICKUP-1, 0131 BAGS-USE-1 and 0132 (provisional TIMED-RT-1c) are reserved.

Structure rules (binding acceptance criteria):
- [ ] One mechanic per module file in its ADR-0022 layer (pure rules L3, PostgreSQL in `durability`, runtime L4, dispatch L5).
- [ ] New tests in their own child files, not appended to existing files.
- [ ] Edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines (`connection.rs` is ~8,260 lines).

### Present on main (#2136, then #2270/#2280)
- The unregistered Guild sources from #2136 (`domain/guild.rs`, `durability/guild.rs`, `durability/guild_audit.rs`) were **removed** as dead code by the ADR-0022 restructure #2280 (`d25623a36`); recover them from `d25623a36^` (`crates/gameplay/domain/src/domain/guild.rs`, `crates/gameplay/durability/src/durability/guild.rs`, `.../guild_audit.rs`) as reference only.
- Still present and unregistered: `apps/game-server/src/node/guild_jobs.rs` (`run_guild_deadline_pass`, `run_guild_disband_pass`; no `mod guild_jobs` in `apps/game-server/src/node/mod.rs`), lifted tests `apps/game-server/src/lifted_tests/durability_guild_tests.rs` and `durability_guild_disband_terminal_tests.rs`, and `apps/game-server/tests/guild_postgres.rs` (`#![cfg(any())]`, so CI compiles none of it). No guild migration, wire or job registration.
- Last checkpoint (2026-10-17): Guild, Bank, Hall and House modules imported each other and compiled only together; with #2137, #2139 and #2153 overlaid, 101 of 104 PG cases passed; 3 `disband_terminal_tests` failed because House release SQL (`game_house_guild_reserve`) was missing.

### Checklist
- [ ] **Re-implement in layers**: pure guild rules in `crates/gameplay/social-rules/src/` (new `guild.rs`, L3), PostgreSQL in `crates/gameplay/durability/src/durability/guild.rs` + `guild_audit.rs` (new), runtime orchestration in `crates/gameplay/social/src/` (new); move or replace `node/guild_jobs.rs`; mount PG cases from `apps/game-server/tests/character_authority_postgres.rs` (PG-COVERAGE-1 pattern) or remove `#![cfg(any())]` from `guild_postgres.rs`.
- [ ] **Migration**: guild, rank, member, invitation, account-leadership, deadline/disband checkpoint tables, plus the House guild-fence ABI the HOUSE-1b card requires — next free number at allocation time (only one migration PR in the queue). Game-role privileges.
- [ ] **Runtime job hookup**: register the deadline and disband passes as a World job in `apps/game-server/src/node/serve.rs`, with `NodeIncarnationProof` and reconciled authority; restart-safe checkpoints.
- [ ] **Command transactions reachable from a session**: found, invite, join, leave, exclude, set rank, resign, edit ranks, disband, title/message invoked from a new `crates/gameplay/dispatch/src/gameplay_transport/guild.rs` handler; premium gate via `premium_current` (`crates/gameplay/social/src/premium/mod.rs`).
- [ ] **Protocol**: new `crates/protocol-oteryn/src/guild.rs` with `CAPABILITY_GUILD_V1` (next free id above 30), commands, state domain; registries (`PROTOCOL_OTERYN_V1_REGISTRY.json`, `GAME_EVENT_FOUNDATION_REGISTRY.json` guild event with retention `GUILD_ACTIVITY_RETENTION_V1`, error codes). GUILD-WIRE-1 card: `docs/agents/tasks/GUILD-WIRE-1.md`.
- [ ] **Dispatch**: wiring-only branch in `connection.rs`, `GATED` row in `capabilities.rs`.
- [ ] **Consumers**: guild chat membership (GUILD-CHAT-1, draft #2142), guild-mate checks in `crates/gameplay/combat-rules/src/pvp/legality.rs`, guild fields in `crates/gameplay/native-admission-source/src/public_projections.rs`, GUILDHALL bid funding, sealed payout ports.
- [ ] **Tests**: all 104 PG cases pass (no skips, including the 3 House-port cases); concurrent invite/join and leader resign; disband leaves no orphan rows; **end-to-end through `connection.rs` dispatch**: found, invite, join, rank change, resign/disband, capability-off refusal; deadline pass driven by the real job registration after restart.

### Blockers and dependencies (re-verified on origin/main `59785d76`)
- BLOCKING — re-implementation pending (drafts closing): HOUSE-1b #2141 (guild fence ABI), BED-1 #2147 (revision hook), HOUSE-1a #2137, GUILD-BANK-1 #2139, GUILDHALL-1 #2153 (proposed owner of the activation slot), GUILD-WIRE-1 #2159. Cards: `docs/agents/tasks/<ID>.md`.
- GUILD-CHAT-1 #2142: open draft, not merged.

## Re-land prerequisite audit, 2026-10-22

Local admission baseline is `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`;
the worktree was clean. Bound META 3.1.0 was read at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. This packet authorizes local
AUTHORING only, migration **0137**, and prerequisite implementation in this
lane. No publication, credentials, external writes or optional skills are used.

### Module layout planned before implementation

- `domain/src/domain/guild.rs`: shared `GuildId`, rank and lifecycle values needed
  by the waiting Bank, Chat, Wire, Hall and ACL owners.
- `social-rules/src/guild.rs` and `guild/*_tests.rs`: recovered pure eligibility,
  limits and strict deadline predicates; reuse `CharacterName` and its key.
- `durability/src/durability/guild.rs`: facade/registrations for mechanic children
  `guild/{found,membership,text,rank,resign,edit_ranks,disband,deadlines}.rs`.
  Recover the existing transaction implementations and guards from
  `d25623a36^`, preserving current recovery/session/Node checks and receipt replay.
  Keep audit encoding in `guild_audit.rs`, with mechanic children as needed.
- `social/src/guild.rs` and `guild/{deadlines,disband}.rs`: runtime orchestration;
  replace the old node job path and update all callers, rather than duplicate it.
- `dispatch/src/gameplay_transport/guild.rs` and `guild/*_tests.rs`: session
  composition and real-dispatch tests. Shared files receive wiring only.
- Migration `0132_guild_core.sql`: installed tables/guards and bounded role grants.
  Mount recovered cases exactly once through the canonical PostgreSQL target;
  split mechanic tests out of the old large owner without copying its fixtures.

### Proven prerequisite gaps and owner decision

`git show d25623a36^` recovers the 4,100-line durable owner, 360-line audit
encoder and 307-line domain source. The current durable module list registers
none of them. The recovered terminal controller imports absent `guild_bank`,
`guildhall` and `house_ownership` ports; restoring only Guild cannot compile or
prove terminal closure. These gaps fall under the integrator's option B, not
an automatic stop for missing prerequisites.

The retained owner and terminal test files contain 97 tests; the recovered
domain contains the other seven of the historical 104. The 306-line
`guild_postgres.rs` is disabled by `cfg(any())` and mounts the old crate graph.
Its compilation cannot qualify those cases. Reuse candidates are the existing
`item_transfer_postgres_cases` Harness, `bank_postgres_cases`, Premium coordinator
and `house_test_fixture`; none has been copied, wrapped or changed in this audit.

The unresolved step is protocol allocation/offer authority, not prerequisite
coding: accepted GUILD0 §9 reserves the capability, two commands and domain on
the coordination issue; GUILD-WIRE-1's accepted-decisions section assigns global
numbers to CP, and its slot-only section says **"capability not offered until CP
decides"**. This checkout has no Guild reservation. Registry maxima are capability
30, command 30 and domain 22; 31 / (31,32) / 23 are only possible next-free
candidates, not reservations against other lanes. The current packet requests
real dispatch but supplies only a migration reservation. A batched question asks
whether this lane may allocate next-free IDs and offer GUILD_V1, or CP will supply
the reserved IDs and offer decision. No new capability is exposed pending that
answer. The integrator prerequisite implementation remains pending; no new module
or successful prerequisite implementation is claimed.

### Validation and file-size record

Only this task record changes. No enum, struct field, trait method, public
function, fixture, migration or Rust registration is added. Cargo filters have
not been run; the disabled Guild target is not counted as a pass. Implementation,
all 104 recovered cases, new real-dispatch/PG suites, touched-crate Clippy and
rustfmt remain unfinished. The card grows by fewer than 150 lines; no file grows
by more than 200 lines. Whole-diff review uses the owner's
`~/oteryn-cache/CODEX-CHECKLIST.md`: no code/test changes or applicable code hits.
`python3 tools/agents/validate_governance.py` passed (22 documents, 9 lanes),
`python3 -m unittest discover -s tools/agents/tests -q` passed all 124 tests,
and `git diff --check` passed. These document checks do not qualify Guild.

## Integrator decision 2026-10-22

The allocating integrator holds CP authority and confirms this lane implements
the **guild CORE**: migration 0137, all core command transactions, formation and
vice World jobs, and the existing `guild_postgres` target. Protocol identifiers
and offering GUILD_V1 remain exclusively GUILD-WIRE-1; none are allocated here.
The scheduler is the production composition boundary for this core. Historical
wire-dispatch and House/Bank participant requirements are follow-up scope, not
authority to recreate their missing owners. The shared domain/durability surface
remains available to Bank, Chat, Wire, Hall and ACL children.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

The core implementation is locally qualified. Preserve the prerequisite audit above. The planned
layout is shared values in `domain/guild.rs`, pure rules in `social-rules/guild.rs`,
mechanic children under `durability/guild/` and `durability/guild_audit/`, and a
World-job owner in `social/guild.rs`, wired from the node scheduler. Recover the
existing transaction, audit and test bodies rather than copying helpers. The
canonical PostgreSQL target will use the production crate graph and the existing
item-transfer harness once. Any test-only visibility bridge stays in its own file.

### Core implementation and reuse, 2026-10-22

The prerequisite is implemented in the modules listed below. The node serve loop now calls the social Guild World scheduler; each pass uses the existing native Root/recovery/admission boundaries, bounded candidate pages and durable cleanup steps. The shared `GuildDisbandTransaction` and named SQL closure predicate are the participant seam. Core cleanup fails closed when an uncomposed Bank/Hall/House owner is installed; no financial/property participant is fabricated. GUILD_V1 remains unoffered, with no protocol IDs allocated. The explicit CP core-only decision governs the production boundary: command and membership APIs remain for the named children; their connection-dispatch E2E belongs to GUILD-WIRE-1.

Reuse: recover the existing Guild transactions/audit bodies from `d25623a36^`; reuse `CharacterName`, native session/recovery/admission/World proofs, `premium_current` through the existing Premium coordinator/latch, `ChatText`, native Account guards and Bank junior predicate. Reuse the existing item-transfer PostgreSQL Harness by **moving** its fixture body to `item_transfer_postgres_harness.rs` and re-exporting it from its original owner; all 21 original test bodies/registrations remain unchanged, and Guild does not register them again. Move the existing node cancellation race helper rather than copying it; extract the original same-transaction event-failure injector into `event_failure.rs` and reuse it for command/cleanup rollback tests. Reuse the existing Premium producer through `guild_premium_test_source.rs`, mounted once. Test-only request/writer field visibility lives in `guild/{request,rank,resign,edit_ranks}_data.rs`; the shipping fields remain private. Existing fixture-only `committed`/`character_revision` methods gain crate visibility for their original case owner.

Whole-diff checklist review re-read the owner checklist, audited 45 type/trait match/constructor/implementation references across `apps/`, `crates/`, `tools/`, and mechanically verified every retained core assertion after path/format changes. All 91 core tests and six terminal tests are registered once; the latter exercise core's uncomposed-owner boundary. Fixes cover shared replay reuse, the deferred terminal-state closure check, table-specific OLD-record checks in the disband trigger, and the native lock-observer fixture's full key/two-second production-deadline budget. No production deadline, guard, test assertion or capability gate is relaxed. Historical Guild snapshots rebuild the exact migration sections in their isolated databases before installing guards, as before; the added installed-migration command/scheduler path never uses that setup.

### Complete changed-file inventory (directory / filenames)

- `./`: `Cargo.lock`, `workspace-boundaries.toml`.
- `apps/game-server/migrations/`: `0132_guild_core.sql`.
- `apps/game-server/src/lifted_tests/`: `durability_guild_disband_terminal_tests.rs`, `durability_guild_tests.rs`.
- `apps/game-server/src/lifted_tests/durability_guild_disband_terminal_tests/`: `disband_terminal_tests.rs`, `terminal_support.rs`.
- `apps/game-server/src/lifted_tests/durability_guild_tests/`: `chat_tests.rs`, `deadline_tests.rs`, `decline_tests.rs`, `disband_tests.rs`, `edit_ranks_tests.rs`, `event_failure.rs`, `found_tests.rs`, `membership_tests.rs`, `rank_tests.rs`, `resign_tests.rs`, `support.rs`, `terminal_core_tests.rs`, `text_tests.rs`, `world_job_tests.rs`.
- `apps/game-server/src/node/`: `cancellation.rs`, `guild_jobs.rs`, `guild_scheduler.rs`, `mod.rs`, `serve.rs`.
- `apps/game-server/tests/`: `guild_postgres.rs`.
- `apps/game-server/tests/support/`: `guild_premium_test_source.rs`, `item_transfer_postgres_cases.rs`, `item_transfer_postgres_harness.rs`.
- `crates/gameplay/domain/src/domain/`: `guild.rs`, `mod.rs`.
- `crates/gameplay/durability/`: `Cargo.toml`.
- `crates/gameplay/durability/src/durability/`: `core_modules.rs`, `guild.rs`, `guild_audit.rs`.
- `crates/gameplay/durability/src/durability/guild/`: `chat_membership.rs`, `deadlines.rs`, `disband.rs`, `disband_mutation.rs`, `disband_transaction.rs`, `edit_ranks.rs`, `edit_ranks_data.rs`, `edit_ranks_mutation.rs`, `found.rs`, `journal.rs`, `membership.rs`, `membership_mutation.rs`, `operation.rs`, `rank.rs`, `rank_data.rs`, `rank_mutation.rs`, `receipt.rs`, `request.rs`, `request_data.rs`, `resign.rs`, `resign_data.rs`, `resign_mutation.rs`, `text.rs`, `text_mutation.rs`.
- `crates/gameplay/durability/src/durability/guild_audit/`: `deadlines.rs`, `decline.rs`, `disband.rs`, `edit_ranks.rs`, `membership.rs`, `rank.rs`, `resign.rs`, `text.rs`.
- `crates/gameplay/social-rules/`: `Cargo.toml`.
- `crates/gameplay/social-rules/src/`: `guild.rs`, `lib.rs`.
- `crates/gameplay/social-rules/src/guild/`: `rules_tests.rs`.
- `crates/gameplay/social/src/`: `guild.rs`, `lib.rs`.
- `crates/gameplay/social/src/guild/`: `deadlines.rs`, `disband.rs`.
- `docs/agents/tasks/`: `GUILD-1.md`.

### Files growing by more than 200 lines

- `apps/game-server/migrations/`: `0132_guild_core.sql (+680)`.
- `apps/game-server/src/lifted_tests/durability_guild_disband_terminal_tests/`: `disband_terminal_tests.rs (+260)`.
- `apps/game-server/src/lifted_tests/durability_guild_tests/`: `chat_tests.rs (+253)`, `deadline_tests.rs (+331)`, `decline_tests.rs (+471)`, `disband_tests.rs (+273)`, `edit_ranks_tests.rs (+769)`, `found_tests.rs (+425)`, `membership_tests.rs (+615)`, `rank_tests.rs (+963)`, `resign_tests.rs (+955)`, `support.rs (+771)`, `text_tests.rs (+632)`, `world_job_tests.rs (+202)`.
- `apps/game-server/tests/support/`: `item_transfer_postgres_harness.rs (+758)`.
- `crates/gameplay/durability/src/durability/guild/`: `found.rs (+287)`, `membership_mutation.rs (+309)`, `operation.rs (+208)`, `rank_mutation.rs (+215)`, `request.rs (+225)`.

All listed growth is in new files. Migration 0137 owns the inseparable durable schema/guards/grants; the five large durable mechanic files own transactional rules and immutable request binding at ADR-0022 L4. Large server test children retain the existing real-Root/Premium/PG cases at their composition owner. The large support files are moved fixtures, not copied harnesses; World-job tests exercise the actual installed migration and scheduler. Existing production shared slots receive registrations/composition only, with the cancellation body extracted rather than extended.

### Final local validation

Final `guild_postgres` batch: **102/102 passed**, zero ignored/filtered, PostgreSQL **17.6** (542.14s). All-target strict Clippy passed for `oteryn-gameplay-{domain,social-rules,durability,social}` and `oteryn-game-server`; `cargo +1.95.0 fmt --all -- --check` passed. Domain/social-rules unit suites passed **117 + 47**, plus **one** documentation test. Architecture boundary validation passed; governance passed (22 documents, 9 lanes); all **124** agent tests passed. The original item-transfer harness regression passed **1/1**; focused terminal/World/event-fixture repairs passed **3/3 + 2/2 + 1/1** before the complete clean batch.

Exact coverage commands (queued wrapper supplies the database route):
- `cargo +1.95.0 test --locked -p oteryn-game-server --test guild_postgres durability_guild_tests --quiet -- --test-threads=4`
- `cargo +1.95.0 test --locked -p oteryn-gameplay-domain -p oteryn-gameplay-social-rules --quiet`
- `cargo +1.95.0 test --locked -p oteryn-game-server --test item_transfer_postgres slot_then_entries_newest_first_replay_and_no_revision_advance --quiet -- --test-threads=1`

Core implementation/qualification is complete locally; status stays `in_progress` for CP integration. No candidate is frozen/published, no review is triggered, and no commit/push is performed. Wire IDs/offer/connection dispatch, Premium daily observation and financial/property participant integration remain with their accepted follow-up allocations; this lane exposes no substitute capability or owner.


### Continuation: final guards and bounded cleanup, 2026-10-22

Continues WIP `7706e5e80c43c5b841ebf00af1d26c2571f260b9` on local branch
`w4/guild-1r`, AUTHORING only. The earlier prerequisite audit and CP core-only
integration decision remain applicable; status stays `in_progress` for CP integration.

Layout selected before this continuation's edits: move durable cleanup from
`guild/disband.rs` into the mechanic child `guild/disband_cleanup.rs`; retain
shared values in domain and pure rules in social-rules. Keep SQL guards/indexes
in provisional migration 0137. Mount `disband_cleanup_tests.rs` and
`event_guard_tests.rs` once from the existing PostgreSQL owner. No new public
function, enum variant, struct field or trait method is introduced.

Cleanup now locks at most the next 100 members or invitations, removes matching
vice leadership in the same member batch, and leaves only the leader and at most
20 rank definitions for the final step. Guild serialization and the existing
physical-transaction/recovery/World proof remain authoritative. SQL guards bind
the exact batch accounts and require their deletion; the former exception for
orphan leadership during DISBANDING is removed. Matching cleanup indexes support
ordered guild scans. New deadline starts also reuse the existing World-assignment
proof before mutation; checkpoint replay remains read-only.

The event guard reconstructs the complete envelope before row mutation, including
source ranks, invitation-removal counts and every action-specific field. It
replaces the former partial/duplicated payload checks. Membership success now
requires the actual source membership/invitation and current Guild revision, so
an already-empty final snapshot cannot fabricate a Leave/Revoke success.

Reuse: move the original cleanup implementation rather than retain a second path;
reuse `assert_guild_job_world`, the physical transaction proof and World scheduler;
reuse the native Harness, historical Character/member/invitation fixtures,
`GuildAuditProposal` serializer, and existing event-failure injector. Move the
original Resign payload/hash injector into its existing `event_failure.rs` owner
and call it from both the original Resign test and the new guard tests; its body
is not copied. No new harness, test-only authority constructor or duplicate test
registration is added. The installed-migration scheduler test still runs alongside
these isolated historical-fixture cases.

Validation uses `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0` throughout:
- `test --locked -p oteryn-game-server --test guild_postgres durability_guild_tests --quiet -- --test-threads=4`: **106/106 passed**, zero ignored/filtered, 846.59s. The database route is required; PostgreSQL **17.6** is asserted as `server_version_num=170006` by the real fixture. This includes the installed-migration scheduler and all four new cleanup/envelope tests.
- Domain unit target: **117/117 passed** in the initial four-package unit invocation. That invocation then reported two `Database(Pool)` errors in existing terminal-replacement tests (SQLx pool timeout/closed classification), with 382 other durability tests passing. No assertions, tests or timeouts were changed.
- `test --locked -p oteryn-gameplay-durability --lib --quiet -- --test-threads=4`: the complete unchanged unit target passed **384/384**, 62.32s, including both earlier pool-error cases.
- `test --locked -p oteryn-gameplay-social-rules -p oteryn-gameplay-social --lib --quiet -- --test-threads=4`: **47/47 + 103/103 passed**. Total affected unit-target evidence: **651 passing tests**.
- `clippy --locked -p oteryn-gameplay-domain -p oteryn-gameplay-social-rules -p oteryn-gameplay-durability -p oteryn-gameplay-social -p oteryn-game-server --all-targets --quiet -- -D warnings`: passed.
- `fmt --all -- --check`: passed. Whole continuation diff and new files were reviewed against the owner's `~/oteryn-cache/CODEX-CHECKLIST.md`; `git diff --check` passed. Workspace symbol/caller searches cover `apps/`, `crates/`, `tools/`; each new test source has exactly one mount.

Changed files in this continuation:
- `apps/game-server/migrations/0132_guild_core.sql`.
- `crates/gameplay/durability/src/durability/guild.rs` and `guild/{deadlines,disband,disband_transaction,disband_cleanup}.rs`.
- `apps/game-server/src/lifted_tests/durability_guild_tests.rs` and `durability_guild_tests/{event_failure,resign_tests,disband_cleanup_tests,event_guard_tests}.rs`.
- This task record, updated last.

File-size record relative to WIP HEAD: **no file grows by more than 200 lines**;
new production cleanup is 112 lines and its test children are 169 and 128 lines.
Migration 0137 is 704 lines (+24 net); existing production modules grow by at most
four net lines, with only registration in the shared facade. Historical WIP growth
and its layer justification remain in the inventory above. No implementation or
local validation remains unfinished in this core allocation. CP review/publication/
integration and the separately allocated Wire/Bank/Hall/Premium children remain
outside this local-only continuation; no protocol IDs or GUILD_V1 offer are added.

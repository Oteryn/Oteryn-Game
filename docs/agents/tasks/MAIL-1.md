# MAIL-1 — mail and parcels (skeleton task record)

```yaml
task_id: MAIL-1
title: "MAIL-1: mail and parcels"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row MAIL-1 (row 88); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (MAIL-CONTENT-1, MAIL-TEXT-1, MAIL-1, MAIL-WIRE-1, MAIL-PARCEL-1, MAIL-SYSTEM-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/mail-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2133](https://github.com/Oteryn/Oteryn-Game/pull/2133). Native Inbox reward bridge, recipient resolver and direct-backpack text storage/cleanup are merged (migration 0101). Evidence: `crates/gameplay/durability/src/durability/mail.rs`.

Remaining: Atomic mail posting/value transfer, recipient/posting limits, qualified content and real text/mail command composition remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Goal

Letters and parcels between characters of one World, delivered to the CharacterInbox: mail operations, letter posting, stamp transform, mail event, Inbox mail ceiling and posting rate (MAIL-1), with its content, text store, wire, parcel and system-letter children.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

MAIL-CONTENT-1 (content) -> MAIL-TEXT-1 (hard) -> MAIL-1 (hard) -> MAIL-WIRE-1 (Reg) / MAIL-PARCEL-1 / MAIL-SYSTEM-1 (after HOUSE-1b, Wave 5). WRITE-WIRE-1 completes with this family.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| MAIL-CONTENT-1, MAIL-TEXT-1 (own children) | BLOCKING (not started) |
| INBOX-1a | MERGED (#1785) |
| ITEM-MOVE-1 | MERGED |
| MAP-LOAD-1 | MERGED (archive MAP-LOAD-1) |
| MAP-WIRE-2 | MERGED |
| BAGS-1, BAGS-DEPOT-1 (MAIL-PARCEL-1) | BLOCKING |
| HOUSE-1b (MAIL-SYSTEM-1) | BLOCKING (Wave 5) |
| acceptance of MAIL0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `crates/gameplay/domain/src/domain/mail.rs`
- `crates/gameplay/durability/src/durability/mail.rs`
- `crates/gameplay/durability/src/durability/mail/tests.rs`
- `crates/gameplay/durability/src/durability/item_text.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mail.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mail_tests.rs`
- `crates/protocol-oteryn/src/mail.rs`
- `crates/protocol-oteryn/src/mail_tests.rs`
- `apps/game-server/tests/mail_postgres.rs`
- `apps/game-server/tests/support/character_inbox_postgres_cases.rs` — D1390 only: the two named read-only function entries and explanatory comment in the existing `inbox_rows_are_immutable_and_no_other_writer_touches_them` inventory; no other fixture, query or assertion change.
- content authoring inputs for parcel/letter/label/stamped definitions and mailbox bindings (source files only, not regenerated outputs)
- `docs/agents/tasks/MAIL-1.md` (this record: status and notes only)

### Explicit D1152/D1318/D1356 ownership amendment

The [D1356 allocation](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098)
supersedes the historical slot-only restriction below for the exact hunks in
this checkpoint. It assigns the complete existing migration **0101**, not a new
migration number; production activation remains after #2197. Owned additions:

- `apps/game-server/migrations/0101_mail.sql` — existing TEXT schema plus the
  complete D1318 sink and fresh reward-mint/receipt/audit guards and grants.
- `crates/gameplay/durability/src/durability/character_inbox.rs` and
  `character_inbox_reward_claim.rs` — actual native seam and bridge exports.
- `crates/gameplay/durability/src/durability/character_inbox_reward_claim.sql` — remove
  the former private SQL copy after moving its full body into migration 0101.
- `crates/gameplay/durability/src/durability/character_inbox/reward_claim/mint.rs` and
  `character_inbox_reward_claim_mint_tests.rs` — native bridge and source tests.
- `apps/game-server/tests/support/mail_postgres_cases.rs`,
  `character_inbox_reward_claim_postgres_cases.rs` and
  `character_inbox_reward_claim_mint_postgres_cases.rs` — existing canonical
  mount, exact installed migration checksum, and twenty meaningful PG cases.

The same D1356 commit includes only these **eight granted shared file scopes**:

- `crates/gameplay/durability/src/durability/reward_claim_mint_audit.rs` — optional Inbox
  field 5 and its native encode/decode dispatch within existing operation tag 4.
- `crates/gameplay/durability/src/durability/reward_claim_mint_audit/inbox.rs` and
  `reward_claim_inbox_mint_audit_tests.rs` — that field's native codec and tests.
- `crates/gameplay/durability/src/durability/reward_claim_mint.rs` — only `inbox: None`
  at existing CHEST struct construction sites; no legacy behavior change.
- `crates/gameplay/durability/src/durability/item_mint_audit.rs` — only the real Inbox
  outbox writer leaf in the existing insert-site inventory; retain TRADE's line
  when the second PR integrates main.
- `docs/contracts/game-events/v1/native_one_item_transaction.proto` — optional
  field 5 and two messages; outer operation tags and tuple numbers unchanged.
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` — one additive tag-4
  Inbox note, without broad registry edits.
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` — eight Inbox rows explicitly
  **CANDIDATE** until actual native prost/JSON measurements are recorded.

The already-owned `item_text.rs` hunk only relocates its exact storage DDL into
0101 and removes private fixture schema execution. All prior TEXT table, key,
check and index semantics are preserved. Other shared wiring, source generation
and publication remain with the coordinator/CP.

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

- durability/character_inbox.rs (INBOX-1a) is the delivery target; extend via new functions, no schema change outside the MAIL migration
- connection.rs command dispatch chain (l.2735-2766 pattern)
- capabilities.rs `GATED` l.165 (`MAIL_V1`)
- crates/protocol-oteryn/src/lib.rs `pub mod` list
- GAME_EVENT_FOUNDATION_REGISTRY.json (mail event type)

## Inseparable bundles (never split across PRs)

- content (MAIL-CONTENT-1) + regenerate_content.py + pins by one author
- migration + mail writer + durability tests (MAIL-1)
- wire + registry + capability + handler + tests (MAIL-WIRE-1)
- stamp transform = BURN of label/parcel + MINT into Inbox + ledger in one transaction

## Acceptance criteria

- [ ] Posting is idempotent; a crash between debit and Inbox write leaves neither.
- [ ] Unknown recipient -> `UNKNOWN_RECIPIENT`; junior parcel -> `RECIPIENT_CANNOT_RECEIVE_PARCELS`; refused item stays with sender.
- [ ] Inbox mail ceiling and posting rate enforced from RESOURCE_LIMITS_REGISTRY rows.
- [ ] Text store rejects oversize/unsafe text (MAIL-TEXT-1 security review).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server mail
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md §3-§10, §14
- docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md §5 (CharacterInbox)
- crates/gameplay/durability/src/durability/character_inbox.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local MAIL-TEXT/address and recipient preparation — 2026-10-15

Historical snapshot: this section records the earlier bounded subset. Its
content-model and allocation statements are not current activation authority;
the native writer and later actual evidence are recorded below.

Existing draft #2133, `skeleton/mail-1`, unchanged authoring base
`06f4e8ee2479eb5bffce507ba50f9a1c10b4b718`. The active allocation permits only
the six paths named in `mail-1-allocation.json`; historical broader paths and
statuses above do not grant wiring/content/migration rights. No commit/push.

Accepted MAIL-0 §§4–6/§11 and WRITE-0 §4–6 were read from protected main
`3ef900c160c9713092a86bfcd1a8377c7587c898`, not inferred from the clone's stale
CANDIDATE heading. MAIL-0's writable-text amendment is effective with accepted
WRITE-0; a future store must retain a separate never-deleted revision row so
deletion cannot reuse a revision. No unsupported text schema is assumed here.

### Actual residue and useful owned component

- Native `character_inbox.rs` provides existing Inbox reads and typed errors;
  migration 0076 admits deliveries only through caller-owned SECURITY DEFINER
  writers with registered causes. MAIL causes, transfer+stamp transform and
  audit/outbox shapes are not present. A new fake delivery/MINT/BURN writer or
  ad hoc cause is not a usable substitute: accepted letter posting preserves
  item identity with TRANSFER+TRANSFORM and has no value/MINT/BURN line.
- No native content/runtime model presently carries `readable`, `writable`,
  `write_policy`, `max_characters` or an admitted letter/label/stamp mapping.
  WRITE-CONTENT-1/MAIL-CONTENT-1 owns that qualification. A client/request
  boolean cannot become current writable-item authority. Therefore no dead
  item-text writer or speculative persistence table is added merely because
  `item_text.rs` is allocated.
- `domain/mail.rs` supplies the real accepted bounded text/address value:
  letter 1,999/7,996 and label 79/316 scalar/byte bounds, LF only among control
  characters, empty text retained as the eventual delete request, exact body
  preservation and redacted Debug. It trims only the first LF-delimited line,
  ignores town/body and reuses canonical `CharacterName` parsing/comparison key.
  Empty/missing address is `NO_ADDRESS`; malformed names are privacy-safe
  `UNKNOWN_RECIPIENT`. A text class is policy data, not an admitted Item definition.
- `durability/mail.rs` uses actual existing reservation/root tables. It first
  resolves a same-World live name hint, then independently rechecks the current
  recipient root's World, lifecycle and addressed name under `FOR SHARE` in the
  delivering transaction. A hint is never proof of the current root. Foreign,
  missing, dead, renamed and held-only names expose no distinguishing result.
  No bank same-Account/junior/online/Channel restriction is imported into letters.

### Exact remaining content/CP boundaries

The eventual writer must independently derive the sender's World from current
native session/recovery/item authority; these query helpers are not an unfenced
public recipient-lookup endpoint. Existing
`item_transfer::character_item_fence_is_current` already checks CommandRef session,
reconnect/lease/scope generations, current node incarnation, admission guards and
the sender root without changing CharacterRevision. Reuse it, together with the
current recovery seal/admission lock order; never reconstruct current facts from
the prepared text, hint or receipt. Sender/recipient root locks must be acquired
by CharacterId (one for self-mail), before items/container/Inbox counters. The
recipient lock must remain held through delivery in the same transaction.

Qualified writable/stamped/max-character/rewrite/current-definition facts and
each location's current reach/fence remain prerequisites for text persistence.
MAIL-TEXT/WRITE must implement durable non-reused revisions, two writes/second,
CommandRef replay/conflict, deletion, current ownership/definition recheck and
privacy-safe reached-item reads. The accepted named table shape is permitted,
but is not enough to authorize writing an Item with missing current content facts.

Full MAIL posting still needs the accepted private operation schema, closed mail
causes/audit/outbox and privileged Inbox writer, stamp rule, sorted root/item locks,
mail ceiling/rate, rollback/idempotency/reconciliation, mailbox reach/binding,
content/wire/resource registries and actor/transport composition. No migrations,
registries, shared registrations, protocol, content regeneration, Inbox schema,
new audit causes or world truth are changed in this branch. Full task acceptance
remains unchecked; the detached subset is not mail delivery or text persistence.

### Actual local validation and scope

- **8/8 passed**, 3,179 unrelated library tests filtered: five text/address
  tests (letter/label max/max+1, four-byte scalars, combining scalars, control
  refusal, first-line/canonical key, missing/invalid address and private Debug)
  and three real isolated **PostgreSQL 17.6** cases (same-World/held-name privacy,
  independent stale rename/World/root-removal facts and actual recipient
  `FOR SHARE` versus a second connection's `DELETE`, SQLSTATE `55P03`).
- Unit tests ran against an exact native snapshot with only domain/durability
  mail module registration additions **outside this branch**. Upstream
  `sqlx::test`, existing SQLx features, actual native migrations and the existing
  genuine bestiary fixture were used. No fake current-fence or writer model.
  Migration-owner fixture seeding/corruption is explicit and local; this is not
  production role/authority, full sender-fence or full-mail E2E qualification.
  CP must route these PostgreSQL cases to the real PG lane when registering
  modules; the explicit validation script pins its local test `DATABASE_URL`.
- Library Clippy `-D warnings`, snapshot/branch workspace format checks and
  owned Rust formatting passed. The existing vendored Tokio missing-doc warning
  remains in raw Cargo logs; no lint or dependency feature was changed.
  One shared Rust lock, debug0, incremental0, jobs2; library-only Cargo commands,
  no implicit server/migrate executable link. **83** actual source/native
  migration inputs match before/after, and branch shared-file hashes are unchanged.
- Governance and **124/124** agent unit tests passed; whole owned diff reviewed,
  whitespace and exact scope checks passed for four allocated paths.
- Earlier actual attempts are retained separately: the integration-root-only
  shared PostgreSQL helper did not compile in a library context; a subsequent
  6-pass/2-fail run incorrectly used lifecycle2 in fixtures. Native 0005 permits
  only lifecycle1, so the corrected fixture tests actual root removal instead.
  No invented lifecycle was added and product source did not change. Neither
  setup/fixture failure is a claimed product defect or behavioral RED predecessor.
- Durable evidence `/workspace/oteryn-prep/mail-1-evidence/`: raw logs, exact
  accepted sources, source/snapshot bindings, prior fixture sources and commands.
  SHA-256: domain mail
  `d0577b8542b664c420a9f959b949863178447535b66b91eafa38e0d732a34605`;
  durability mail `a7eef6fe053412c55a8a3bc6f2f176545c1f126bceb2dc46befa4e1154cf937a`;
  native tests `81ec66289639e87967f15e91aecc5c367ecbbb22508ee95a30840ea584337def`.

No independent exact-head review or architect/integration acceptance is claimed.
The prepared subset is ready for CP review; full task criteria remain unchecked.

## Native direct-backpack MAIL-TEXT increment — 2026-10-16

The owner has explicitly assigned exclusive authorship of the family branches to
this lead, superseding the old local six-path allocation prerequisite for the
specific `.rs` paths in the current packet. Work remains AUTHORING; no candidate
freeze, publication, migration number, shared-file or integration permission is
inferred. The source base is `ad504f33da022abb7ce2f770dfee1f20eb7885e7`.

`durability/item_text.rs` now implements a bounded native rewrite writer for
letters and labels in the writer's direct main backpack. It uses the actual
reconciled recovery seal, admission relation locks, current node/session/item
fence and Character root lock. An independently queried latest
`game_content_activations.server_artifact_digest`, protected by the existing
content-activation lock, must match the loader-decoded `NativeGameplayState`.
Writable/readable/max-length facts come from its qualified production profile;
unknown, conflicting or present write-once targets fail closed. After an Item
row lock, a fresh query checks current definition, World and direct ownership.

The native transaction stores CommandRef replay/conflict bindings without text,
retains a separate revision tombstone after deletion, refuses unsigned revision
overflow, preserves unchanged text and enforces two changing writes in a rolling
second across sessions using the Character root serialization. Its timestamp is
sampled from PostgreSQL after blocking locks; the request supplies no timestamp
or writable/current-authority boolean. No item value, audit event or
CharacterRevision changes. Raw SQLx errors use the existing native classification
which discards database detail; the request has no text-bearing Debug output.

`ITEM_TEXT_SCHEMA_PROPOSAL` is an unnumbered slot input, not a delivered migration
or runtime DDL path. Grants, cleanup, numbered migration, shared registrations and
wire wiring remain undelivered. This is not full MAIL/WRITE acceptance: other
locations, write-once transforms, reachable reads, reporting/moderation and mail
posting are outside this increment.

Activation remains blocked on qualified native letter/label content (the active
413-profile artifact has neither), its explicit rewrite policy and a real
player-trade owner exclusion/serialization port for offered containers under
WRITE-0 §5.1. No fake trade table or client-supplied flag replaces it. CP also
needs to resolve registration of native-content consumers in standalone durability
PostgreSQL targets: their existing `content_shim.rs` lacks `native_gameplay`.
The source is disconnected from shared module/wire composition until these
conditions are satisfied; it grants no early activation permission.

Three pure tests cover delete/rewrite revision non-reuse, overflow/unchanged
behavior and replay-binding substitutions. Three SQLx cases use actual native
migrations plus the explicit schema proposal for storage deletion/reload, rate
refusal/no-op and rollback/retry. Fixture trigger bypass is explicit; these are
storage-only cases and fabricate no current content or session authority.
The rate-refusal fixture sets its two stored receipt times to PostgreSQL
`infinity` so CI delays cannot age them out; it exercises conservative refusal
and state preservation, not wall-clock expiry. The separate deletion/rewrite
case explicitly ages its fixture receipts. Neither changes the production clock.
At this initial authoring checkpoint all six new cases were **unexecuted**:
this container had no Cargo/compiler and no global PC test slot had been acquired. The lead supplied an isolated official
rustfmt 1.94 matching the repository toolchain; the owned source's formatting and
`--check` passed, as did whitespace checks including the new file. No build, PG
qualification, frozen-head CI or paid review is claimed. Existing eight-test
evidence above covers only the earlier subset and does not qualify this writer.

## Actual native qualification and targeted lint disposition — 2026-10-16

The lead ran the ordinary native library pipeline on frozen local preparation
`5f2fef3b0dbf371c7eb5ff13d0f3b846ae0e0fe0`, with only the independently reviewed
private module-registration mount
`c408b248c74f3bc66467f022cdd55ffe8e0ff9c78498782ec5abfe1503ca9ed0`.
Actual source proof covered all 18,812 files and the runner reported unchanged
source after execution. The actual PostgreSQL probe returned `170006`.

- Native library `cargo check`, complete library unit-test compilation and
  workspace formatting passed with the installed Rust 1.95.0 toolchain.
- Exact ordinary source inventories listed and ran **14/14 passed, zero failed,
  zero ignored**: five domain cases; three native recipient PostgreSQL cases;
  three item-text pure cases and three item-text PostgreSQL storage cases.
  These are **8 pure + 6 PostgreSQL** cases, not fourteen full-writer cases.
- Strict library and all-target Clippy both exited **101**. Actual diagnostics
  were eleven production dead-code errors, with five test-build dead-code
  errors for the intentionally unwired item-text producer. The upstream Tokio
  documentation warning remains unrelated. The runner therefore recorded
  `NOT_QUALIFIED_GATE_FAILURE`; full-package tests were not run.

Evidence is retained at `/workspace/oteryn-family-b/actual-mail-native-1/`:
`summary.json`, `tests-0.log`, `tests-1.log`, `tests-2.log`, `clippy-lib.log` and
`clippy-all-targets.log`. This evidence qualifies the exact old `5f2fef3b` native
subset only; it does not qualify a later source head, full current-authority
writer/replay/trade admission, production activation or full MAIL acceptance.

The lead explicitly returned this checkout to AUTHORING for a narrow successor.
`item_text.rs` adds only two item-level, self-enforcing `expect(dead_code)`
dispositions, following the repository's existing pending-consumer entrypoint
pattern with a tighter expectation:

- The schema proposal expectation applies only outside tests; actual storage
  tests use it, and production must not execute proposal DDL. The reason names
  the actual CP migration slot.
- The actual `commit_item_text_rewrite` entrypoint expectation names pending CP
  wire composition, admitted content, migration and real offered-trade exclusion.
  Its genuine private call graph is preserved; no fake caller, blanket module
  allowance or public API widening is introduced. An admitted real caller must
  remove the now-unfulfilled expectation during integration.

Removing precisely those two new attributes reproduces the old source bytes,
including schema, every function body, test body and authority check. No source
behavior or test inventory changes. Focused official rustfmt and whitespace
checks passed locally. No Cargo/compiler/PG run was performed by this author;
at that checkpoint the successor's native strict-Clippy result was pending
actual recheck and independent review. Existing frozen commit, PC snapshots, refs
and publication state were unchanged by that authoring repair.

## SAME3bc actual continuation evidence and docs-only bridge — 2026-10-16

The lead froze the targeted lint successor as
`3bc9949ef9af75badcf6ba99eafdd71802e2b672`. Its actual native execution is now
complete for the selected subset: **14/14 passed, zero failed, zero ignored**,
comprising **8 pure + 6 PostgreSQL** cases, with actual canonical server probe
`170006`. This supersedes the historical unexecuted/pending statements above.

- `actual-mail-successor-interrupted-1/`: actual library check, complete library
  test compilation and five domain tests passed before interruption; this is
  retained stage evidence, not a completed pipeline claim.
- `actual-mail-successor-nonpg-2/`: workspace `cargo fmt --all --check` and
  strict library `cargo clippy ... --lib -- -D warnings` exited **0**. The two
  targeted expectations suffice for this actual library lint run; no payload or
  blanket module allowance was needed.
- `actual-mail-successor-pg-3/`: canonical PG lock/version proof and exact native
  inventories ran the missing three recipient cases plus six item-text cases
  (three pure, three storage PG), all passed. The five prior domain cases and
  check/no-run results were retained without rerunning them. Source/compiler/ELF
  proofs bind the same candidate; the prior test ELF remained byte-identical.
  The continuations report unchanged source and successful own-child cleanup.

The broader gates remain **failed**: strict all-target Clippy and ordinary
full-package `cargo test --locked --offline -p oteryn-game-server --no-run`
both exited **101**. Actual diagnostics are **E0432/E0433**, at
`crates/gameplay/durability/src/durability/item_text.rs:20–21`: integration crates that
locally include Durability cannot resolve `crate::content` or its
`native_gameplay` module. Logs identify `runtime_scope_assignment_postgres`,
`wp5_s3b_composition`, `durability_postgres`, `character_authority_postgres`, and
(full-package compilation) `native_admission_source_postgres`. These are actual
shared-composition failures, not unresolved dead-code lint or pending focused PG
execution. Full-package runtime tests were not run after compilation failed.

CP owns shared module registration and integration-root/Content composition.
The existing narrow `tests/support/content_shim.rs` does not supply native
Content; the other affected roots have no Content root. A legitimate native
composition or agreed bounded leaf text-policy ABI with an actual Content-owner
adapter needs a shared slot and review. No new shim, cfg exclusion, fake caller,
unapproved ABI or type-visibility expansion is supplied by this increment.

Evidence directories above are under `/workspace/oteryn-family-b/`.
Their interrupted results/non-PG summary/PG summary SHA256 values respectively:
`c15442e4116e42279607741fceeead60c66d612706364e098578d8ceac56b29f`,
`d59e12c294936733bb0b15369c32f3ee1db00e62dd75e1792a238355b9b424a7`,
`36566d50f21bd184ec96379380ff9ec0eaff22a567fbe5062ba8793420cde9b5`.

This final authoring bridge changes documentation only. Rust source
`item_text.rs` remains SHA256
`5bf55209bc89fd6498d19142d7b3aa97aa09e47b36b3eff7c66c313bd6ed5049`;
the actual native results bind frozen `3bc9949`, not a later docs-only commit.
No later-head CI, full current-authority writer/replay/trade qualification,
production activation, full MAIL acceptance, ready/merge or publication is
claimed. Content admission, real trade-offer serialization, numbered
migration/grants/cleanup and wire composition remain activation gates.

## D1113 accepted leaf composition repair — 2026-10-16 AUTHORING

CP explicitly accepted the leaf ABI in #2131 comment `6222415946`, archived by
this lead at `/workspace/oteryn-family-b/coordination-D1113-D1114.json`, against
MAIL #2133 `85bb3aeeafb5a1bd3568f7d081e789c2074eef05`. This supersedes the
previous unresolved choice of Content composition; it does not close the other
activation gates or authorize a readiness/merge claim.

`durability/item_text.rs` now imports no Content type. Its closed
`MailTextPolicy` has private exact-definition, generation digest, class,
max-length and CharacterInventory/direct-main-backpack fields. The documented
sole producer is `NativeGameplayState::mail_text_policy` in the newly allocated
`content/mail_text_policy.rs`. The adapter looks up the actual native generation
member and retains the same exact definition, materializability, inventory
admission, Known(true) readable/writeable, class-compatible max_text_length and
NotApplicable write_once_target checks. Unknown/conflicting/unsupported facts
still refuse. No caller-supplied current-authority boolean or Content shim is
introduced.

The writer consumes the leaf policy, independently compares its native source
digest to current DB activation under the existing lock and checks request
identity against its exact definition. Recovery/node/session/admission fences,
root serialization, item lock plus fresh direct-backpack custody read,
CommandRef receipt replay/conflict, rate/revision storage and privacy handling
remain in place. The policy conveys no current custody/activation/trade proof.
The unnumbered schema proposal and all original fourteen test bodies are unchanged.

The only granted shared edit is `mod mail_text_policy;` in `content/mod.rs`.
The existing three MAIL domain/durability registrations still need the ordinary
private validation mount/CP slot; no other shared registration, test root,
visibility, migration, wire, registry, pin or Content authoring file is changed.
The new adapter has one targeted pending-consumer dead-code expectation, naming
real wire/content/migration/trade work; no fake caller or blanket allowance.

Four additional tests in `content::mail_text_policy::tests` cover:

- `text_profile_keeps_letter_and_label_limits`;
- `text_profile_requires_exact_definition_and_inventory_admission`;
- `text_profile_refuses_unadmitted_mail_definitions`;
- `text_profile_refuses_unknown_conflicting_and_write_once_facts`.

Their explicit decoded-field unit inputs do not construct a NativeGameplayState,
MailTextPolicy or admitted/current Content authority. They exercise the exact
adapter validator's positive field grammar and single-invariant refusals, not
live writable letter admission or the full fenced writer.

Only focused rustfmt parsing/formatting and whitespace/source-diff inspection
were performed during this repair. No Cargo, test, PG or PC job was run; the
required fresh original 14 cases plus these four adapter cases and strict
all-target Clippy/full-package no-run remain **unexecuted for this successor**.
Earlier native results remain bound to their historical heads. Root owns the
new freeze, independent review, real native validation and eventual publication;
this author made no commit/push. Activation still requires admitted Content,
real offered-trade serialization, numbered migration/grants/cleanup and actual
transport composition. Full MAIL/WRITE acceptance remains open.

### CP D1160/D1122: publish actual MAIL registration (2026-10-16)

Status NOT_FULLY_QUALIFIED. CP comment6224912485 identifies GitHub CI E0432 for
`domain::mail`/`durability::item_text` and grants minimal module registrations without
a shared slot. Add only `pub mod mail;` in domain/mod.rs and `pub(crate) mod item_text;`
plus `pub mod mail;` in durability/mod.rs; content::mail_text_policy is already registered.
Both shared files now exactly match the previously tested private mount SHA256 values
(023d18b7...f8fe62 and9d29956e...9f0f8d), checked in D1160-MAIL-SOURCE-BRIDGE.json.
The effective source bytes retain prior c8ade87f native18/full-package PASS; this does
not relabel that run as exact-new-head CI. New-head Cargo/PG/CI: NOT_RUN before checkpoint;
published predecessor CI: RED (E0432). Rustfmt/diff checks required before the one existing
DRAFT push under D1122. No new runner, test suppression, visibility shim or production
activation. Content/trade/migration/transport activation gates remain assigned to CP.

## Documentation-only successor and predecessor CI — 2026-10-17

Status remains **NOT_FULLY_QUALIFIED**. The predecessor GitHub read confirmed draft #2133 at
`e13b2b4cb4eac29dc0c08e87360234371d26d0b6`, branch `skeleton/mail-1`.

CP D1310 permits this documentation-only successor checkpoint on the existing
draft branch. This increment changes only this task record; Rust implementation
and test sources remain unchanged from the predecessor above. **New successor
Cargo/PG/CI checks: NOT_RUN.** The recovered results below remain bound to the
stated predecessor SHA and are not new-commit qualification. Publication permission
does not grant runtime activation, readiness, merge or shared integration.

The following evidence supersedes historical pending/RED statements only for the
named validation scope; it does not grant runtime or shared-integration authority.

The ordinary module registrations and six PostgreSQL wrappers are present. The
wrappers live in `tests/support/mail_postgres_cases.rs`, mounted by
`tests/character_authority_postgres.rs`. The owned text/recipient/Content adapter
source was inspected in this continuation; no demonstrated product defect or
reason to change its behavior was found.

GitHub Actions Rust job
[117222211787](https://github.com/Oteryn/Oteryn-Game/actions/runs/38974887773/job/117222211787)
explicitly checked out the SHA above. Strict all-target Clippy passed. Its library
run reported **3,162 passed / 0 failed / 29 ignored**, and the operations binary
**6 passed**. All **12 MAIL pure/adapter cases** appear as `ok`: five domain
text/address cases, three item-text revision/binding cases, and four native
Content-profile validator cases. The exact-head check collection contains
**16 success / one skipped** (World bundle), with no pending or failed check
at the read.

Canonical PostgreSQL on that implementation predecessor remains **NOT_RUN in this continuation**.
The inspected Rust job ran unit/bin targets, and the S3-B job ran its selected
composition case; neither log executed `character_authority_postgres`.
The six MAIL SQLx wrappers therefore need their own recovered or fresh canonical
PG result before qualification. Historical 18-case/full-package evidence remains
bound to its recorded source and does not become a later-head result.

Focused canonical command, after configuring the allocated disposable PG service:

```bash
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --test-threads=1
```

It requires `DATABASE_URL` for the SQLx administrative test pool, PostgreSQL 17.6
(`server_version_num=170006`), native migrations, and the fixture-only privileges
used by the existing cases. The expected inventory is exactly **six** cases:
three recipient-resolution/root-lock tests and three text-storage tests.
They do not exercise full current-authority text writing or mail delivery.

Activation still needs current qualified Content, actual offered-trade exclusion,
the numbered migration/grants/cleanup and admitted transport composition. The
larger family still needs posting/stamping/Inbox/audit/wire and its dependent
parcel/system-letter integrations. Historical dependency table rows must be
reconciled against current protected main before the final integration slot.
No ready, merge, full-family completion or new paid review is claimed.

## D1318 same-transaction reward Inbox writer — 2026-10-17

Status remains **NOT_FULLY_QUALIFIED**. This implementation checkpoint is based
on `25241039efba1f5a9dbd0ab3d3290986e40743c6`; that predecessor was the
documentation-only successor described above. This increment changes Rust and
test source, so no predecessor CI or historical PostgreSQL result qualifies it.
**Candidate Cargo, Clippy, unit tests and native PostgreSQL: NOT_RUN.** Rustfmt
1.94 parsed and formatted the five changed Rust leaves successfully.

CP [D1318](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232842478)
assigns the Inbox writer to MAIL #2133, extending
[D1152](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224947337).
The implementation reuses the existing `reward_claim_mint` typed cause. It does
not add a new audit Cause or treat a caller plan/boolean as native authority.

`DurabilityRoot::with_reward_claim_inbox_transaction` opens the native type-2
transaction inside a real bounded semantic pass. It checks this Root's recovery
proof, admission locks, and the actual Character/session/lease/scope/node fence.
The source owner borrows the same transaction for claim, fresh mint, binding
and audit writes. The private-constructible Inbox context returns actual
Character/World/ordinal positions. MAIL commits only after every planned output
has been delivered, the callback succeeds, physical transaction identity still
matches and all deferred constraints pass. A caught delivery refusal poisons
the context; an omission or refusal rolls back source and sink together.

The complete frozen plan contains the full unsigned CommandRef, Character,
World, Channel, TransactionId, source binding and ordered item identities. An
indexed CommandRef lookup detects a changed plan even if every item identity
changes on retry. Exact replay returns the original positions without calling
the source again or reacquiring a current session fence. Reconciliation still
requires this Root's actual current recovery proof. The output bound is the
DAILY source's 20 items, not an Inbox capacity restriction.

The adjacent `character_inbox_reward_claim.sql` is **uninstalled candidate
material**, included for exact-source native tests and the eventual protected
migration slot. It grants runtime EXECUTE only on narrow lookup/delivery
functions, preserves the denial of the raw Inbox function and Inbox tables,
locks the same full-u64 reward-mint CommandRef key, and checks fresh item
identity, same-transaction audit presence and complete Inbox placement. It
does not replace the shared MINT receipt or exact audit-payload guards.

The real upstream fresh-mint/Inbox bridge is still required implementation.
The existing `reward_claim_mint` writer and audit encoder accept a CHEST claim,
one item and a backpack destination; its receipt requires a backpack parent.
They cannot be followed by a second Inbox commit and called atomic. The
post-#2197 integration must add the actual Inbox destination/receipt/audit
encoding and its registered resource accounting, then qualify it with the
source owner's claim and binding writes in this same context. No such bridge,
shared guard change, migration number, production grant activation or DAILY
claim implementation is claimed by this checkpoint.

Two pure tests cover plan refusal and every replay-binding component. Eight
new canonical native cases cover complete two-item delivery and full-u64
replay, rollback with ordinal reuse, independently isolated omission and caught
refusal, wrong World/old mint/other location, real session/node/Root and recovery
refusals, SQL partial-plan/missing-audit refusal and runtime grant boundaries.
They use the existing native Root/harness and execute the Inbox code normally;
an explicitly test-owned seed function and nonproduction audit witness stand
in for the upstream bridge. These are **sink/composition tests**, not full mint
or DAILY qualification. Missing admin routing is an error, never early success.

The canonical support mount adds only the new child module to the existing six
MAIL wrappers; it does not create a standalone native test root. Expected
inventory: **8 new cases**, or **14** with the unchanged MAIL wrappers. Under
the D1329 approved PostgreSQL 17.6 namespace and its existing resource routing:

```bash
cargo +1.94.0 test --locked --offline -p oteryn-game-server --lib character_inbox::reward_claim::tests::
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases::reward_claim_inbox:: -- --list
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases::reward_claim_inbox:: -- --test-threads=1 --nocapture
```

Independent bounded source review corrected the callback's owned/static
lifetime contract and isolated the caught-refusal test. It did not compile or
execute the source. Publication, protected integration, exact-source native
qualification, ready and merge remain separately controlled by the coordinator.

## CI routing repair after D1318 — 2026-10-17

This repair is bound to published head
`941a94c1a7d04cf9a779d846d1e9d5d2973c0c06`. It supersedes the earlier
missing-admin-as-error statement and the earlier six-wrapper
`DATABASE_URL` prerequisite. The source writer, SQL candidate, test bodies,
assertions and expected inventory remain unchanged.

Actual exact-head evidence for **941a94c1**, distinct from this successor:

- Strict Rust job `117383203472` failed on ten `expect_used` diagnostics in
  the pure test leaf and duplicate loading of the canonical bestiary harness.
- Integration-rest job `117382981285`, without canonical PostgreSQL routing,
  finished **1496 passed / 14 failed / 0 ignored**. All MAIL failures were the
  two inconsistent environment prerequisites, before their PostgreSQL bodies.
- PostgreSQL job `117382981119` checked out the exact 941a94c1 head, installed
  Rust **1.95.0**, and used the pinned PostgreSQL **17.6** service.
  `character_authority_postgres` finished **1504 passed / 6 failed / 0 ignored**.
  **All eight new native Inbox sink/composition cases executed and passed.**
  Its only failures were the six historical MAIL wrappers requiring the
  absent `DATABASE_URL`, while the canonical admin URL was configured.
  This is actual sink/composition evidence with the disclosed upstream fixture;
  it does not qualify the unimplemented full fresh-mint/DAILY bridge.

The pure test leaf now has the existing canonical test-local
`clippy::expect_used` allowance. The new native leaf imports
`crate::bestiary_postgres_harness` instead of loading that file again.
The six historical wrappers use the canonical root's existing `Database`,
which validates the routed admin endpoint, enforces `server_version_num=170006`,
applies the full migration set and cleans up each private database.
Their unchanged native exercises receive that migrated pool.

All fourteen cases use the canonical `configured_admin()` routing:
an absent `OTERYN_TEST_POSTGRES_ADMIN_URL` emits **PRE-ROUTING / NONCANONICAL**
and returns without a PostgreSQL qualification claim. A present but unsafe URL,
connection failure, wrong PostgreSQL version or failing migration is an error.
With the configured PostgreSQL 17.6 service, all fourteen bodies execute with
their existing assertions. No test was ignored or removed; no new harness,
alternate endpoint, shared root edit or production grant was introduced.

**Successor Cargo, Clippy, unit and native PostgreSQL: NOT_RUN.**
Rustfmt 1.94 parsing/check and exact-parent patch/whitespace checks are the
local preparation gates; predecessor results do not qualify the successor.
The coordinator runs the exact successor's strict CI and the configured
canonical filter, retaining its inventory and complete output:

```bash
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --list
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --test-threads=1 --nocapture
```

Expected inventory is **14**: six unchanged recipient/text exercises and eight
Inbox sink/composition cases. Without routed PostgreSQL, the same filter is
only noncanonical regression execution. Status remains **NOT_FULLY_QUALIFIED**;
the actual fresh-mint receipt/audit bridge, protected activation and source-owner
composition remain unfinished work, with no ready or merge claim.

## D1356 — installed reward-mint/Inbox composition checkpoint (2026-10-17)

**Status: NOT_FULLY_QUALIFIED. This successor's Cargo, strict Clippy, native
PostgreSQL and actual prost/JSON size measurements are NOT_RUN at author handoff.**
The exact base is MAIL `dcac535eb5141ebd8bee6159e9af100459314dad`, tree
`f623359e4290b44afbb1c26ac6a2c3587a547488`, with parents
`00c4767bd3a5c8793e0678f5a6209f9fecb0b865` and protected main
`f010472bcb6794a2cbc56dfaf755bd122c10225e`. All retained preimages were fetched
at this exact base and Git-blob verified. The historical local proposal remains
frozen separately; this checkpoint implements the subsequently granted
[D1356 composition](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098).

### Actual writer and standing migration

`DurabilityRoot::prepare_reward_claim_inbox_mint` validates real recovery,
admission and the session/node/Character fence, then durably freezes the source
plan, resolved mint facts, native EventId/time and full unsigned CommandRef.
The opaque candidate binds original source hash, complete mint-spec hash and
full output plan. CHEST and Inbox take the same advisory CommandRef lock and
cannot reserve one another's command.

`commit_reward_claim_inbox_mint` first reconciles an exact immutable replay.
Otherwise it durably charges the three-attempt work budget, enters the real
fenced D1318 transaction, creates fresh ItemInstances and optional keys, obtains
permanent Inbox ordinals, writes the immutable receipt and registered
`reward_claim_mint` audit, and invokes the source owner's claim/line callback
with that same transaction. Refusal or incomplete proof rolls all value back;
the charged work survives. Committed replay performs no mint, source callback or
new charge. Callers reconcile the prepared candidate's fully bound plan.

Standing `apps/game-server/migrations/0101_mail.sql` now contains the complete
existing TEXT schema, D1318 narrow sink, fresh-mint reservation/receipt/lines,
all proof functions, shared guard amendments and grants. The three original
mint/size/KEY guard bodies are preserved below their new Inbox proof arms.
Two keyed outputs remove only the old unique minted-transaction index from
`game_item_keys`; its item primary key, immutability and exact same-transaction
receipt/quantity-one proof remain. Private reservation/receipt/line tables
have no runtime read/write grants; pinned definer helpers expose the narrow
operations and boolean proof required by native triggers.

There is no remaining private runtime SQL schema copy. All twenty canonical
MAIL cases require the actual SQLx migrator's **0101 checksum**, including the
six existing recipient/text exercises. Their assertions are preserved. The
TEXT schema literal moves unchanged from its owned Rust leaf into the migration;
the historical recipient/text identity fixture still seeds its own input and
is not fresh-mint qualification.

### Meaningful native inventory

The six new bridge cases test twenty actual outputs (including two distinct
keys), native audit decoding, exact physical transaction stamps and full-u64
replay; source-refusal/caught-refusal rollback and retry; real fence refusals;
persistent three-attempt work; runtime/private grants and bidirectional
CHEST/Inbox exclusion; and independently omitted actual receipt or audit.

The eight prior sink cases now run against that same complete installed schema:

- Real public mint + source + full-plan commit and bound-plan replay once.
- Source refusal after actual value/proof writes, zero value footprint, and a
  retry that retains ordinals while charging the next work unit.
- Real prepare/charge/begin with one omitted delivery, then a separate full
  bridge attempt that catches an unplanned delivery after all planned writes.
- A genuinely committed native Ground MINT supplies the old-mint and existing
  Ground refusal inputs. The wrong-World probe explicitly targets the narrow
  SQL **Character-root World** guard while the real Root fence stays current;
  it makes no claim about the later item-World branch. Each refusal preserves
  the entire preexisting footprint.
- Independent session, connection, lease, scope, node and Root authority faults,
  with a valid source-entry control and no callback on the rejected paths.
- A privileged test-only PostgreSQL recovery-generation fault against the
  externally held real seal, without disabling any trigger, blocks delivery
  and reconciliation before source entry.
- Real partial-plan finish and a complete native receipt with omitted outbox,
  each reaching its specific SQLSTATE/message. The missing-audit test forces
  only its named receipt constraint so another guard cannot mask the fault.
- Actual narrow EXECUTE grants and private journal/reservation/receipt/line
  access refusals under the runtime role.

The fresh-value cases use the actual Root holder under `oteryn_game_runtime`
before preparation, work charge and value writes. Only source claim/line tables
and the explicitly named Combat-death input for the native Ground refusal
fixture are test-owned. No fake item/audit seed, replica bypass, duplicate
harness or weakening of value guards remains in these fourteen Inbox cases.
Every configured case emits its completion marker only after its body and
database cleanup succeed. Missing admin routing emits PRE-ROUTING /
NONCANONICAL and cannot emit such a marker or qualify PostgreSQL behavior.

### Codec, caps and qualification boundary

The native codec adds optional Inbox field 5 inside existing tag 4, preserving
legacy CHEST fields/bytes and all outer operation and type-2 tuple numbers. Its
eight resource rows remain **CANDIDATE**: 20 fresh outputs/placements/participants,
41 bridge effect units, one event, 32,768 payload bytes, 34,816 envelope bytes
and a 65,536-byte frozen JSON spec. The source owner accounts for its additional
claim/line effects. This is a source output bound, not an Inbox capacity rule.

Three source tests cover complete binding, independent input refusals and
maximum JSON escaping. Four audit tests cover tuple round trips, independent
malformed fields, exact envelope identity and finite bounds. Their maximum
cases print the actual native prost payload/envelope and canonical JSON sizes;
**no measured size or executed result is claimed here**.

The coordinator qualifies the exact published successor with the repo-pinned
toolchain and current authorized runner/resource route. Core command vectors:

```bash
cargo +1.94.0 test --locked --offline -p oteryn-game-server --lib character_inbox::reward_claim::mint::tests:: -- --nocapture
cargo +1.94.0 test --locked --offline -p oteryn-game-server --lib reward_claim_mint_audit:: -- --nocapture
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --list
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --test-threads=1 --nocapture
```

The configured canonical inventory is **20 PG cases: 6 recipient/text + 8
adapted sink/refusal + 6 fresh-mint bridge**. Rustfmt 1.94 parsing/check, exact
preimage matching and patch/whitespace checks are source preparation gates only.
The earlier exact `00c4767b` strict CI (3,164 passed, 29 preexisting ignored,
6 ops passed) and canonical PG (1,510 passed, including its old 14 MAIL cases)
remain predecessor evidence; they do not qualify this bridge or migration.

Production activation remains after #2197 under D1152/D1318/D1356. The complete
MAIL value writer is now concrete; actual DAILY/source claim acquisition and
resolved Content caller integration must still use its real same-transaction
callback. No source-specific caller authority, successful production activation,
MAIL transport, parcel, posting or system-letter completion is inferred. CP
retains ready/merge authority and the second shared-audit PR integrates current
main before its slot commit.

## Migration0101 PL/pgSQL parse repair (2026-10-17)

This bounded successor is based on exact MAIL
`9579944b48fb24fe36163a7befa4592ed8a68d1e`, tree
`74ea66dfc5d755166614434e50906d68650b019c`. It changes only two SQL condition
lines and appends this task evidence; all Rust, prior test bodies, value guards,
grants, caps and callback/activation boundaries remain unchanged.

The [CP excerpt](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234971256)
for [job117409572690](https://github.com/Oteryn/Oteryn-Game/actions/runs/39037287703/job/117409572690)
reports **1,143 passed / 135 failed** and the repeated root error
`ExecuteMigration(...,101)`, PostgreSQL `42601`, `syntax error at end of input`,
`Original(20084)`. Those 135 failures share the migration parse error; this
is not successful execution of the 20 MAIL bodies.

The verified published SQL blob `b5df21651e71ba7d0493d8e05413a20680b06304`
is complete: **60,007 bytes / 60,006 characters**. Position20084 is line366,
column84, immediately before the inner `THEN` of an unparenthesized SQL CASE
inside a PL/pgSQL IF condition. The same pattern occurs on line391. It is not
the physical end of the migration or the final REVOKE/GRANT block.

The [PostgreSQL17.6 grammar](https://github.com/postgres/postgres/blob/REL_17_6/src/pl/plpgsql/src/pl_gram.y)
reads an IF expression until `THEN` at parenthesis depth zero. Consequently it
ends this expression at the CASE's inner `THEN`, and the SQL parser receives
an incomplete CASE. Both CASE expressions are now parenthesized. This changes
only the PL/pgSQL delimiter grouping; the 128/512-byte definition bounds and
all positive/refusal conditions remain the same. Removing exactly those four
parentheses reproduces the complete published SQL byte for byte.

**Candidate SQL parse, Cargo, Clippy and native PostgreSQL: NOT_RUN at author
handoff.** Exact preimage, two-line/four-character delta and patch/whitespace
checks pass. The coordinator qualifies the complete migration through the
actual native migration path before making any runtime claim. Resource rows
remain CANDIDATE and production activation remains after #2197. The separate
owned TEXT ACL/retirement proposal is not included in this repair.

## Owned TEXT runtime ACL and retirement port checkpoint (2026-10-17)

**Status: NOT_FULLY_QUALIFIED. This candidate's Cargo, strict Clippy and native
PostgreSQL are NOT_RUN at author handoff.** Its exact base is MAIL
`4322022f2738789519f3fd1b6425f2cec77317f3`, tree
`0d4c42017e90aef0dc6d4ea60f38c4dd11042978`, sole parent
`9579944b48fb24fe36163a7befa4592ed8a68d1e`. This is a separate four-file owned
successor. It uses the existing TEXT leaf, canonical MAIL test mount, task
record and standing migration **0101** under the existing grants/cleanup
allocation; it introduces no new migration number or shared registration.

The predecessor `9579944b` had successful strict Clippy/unit and integration-rest
metadata, but its [PostgreSQL job](https://github.com/Oteryn/Oteryn-Game/actions/runs/39037287703/job/117409572690)
failed migration 101 with SQLSTATE `42601` at character 20084. The exact failure
was recovered in [CP comment 6234971256](https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6234971256).
The [published parse repair](https://github.com/Oteryn/Oteryn-Game/pull/2133#issuecomment-6235286577)
is now the exact base above. This rebind preserves both pairs of CASE parentheses
and the complete newly appended repair evidence. The two Rust leaves and the
TEXT SQL hunk are identical to the separately reviewed historical TEXT proposal.
Independent bounded TEXT review was SOURCE PASS; neither that review nor the
predecessor's CI qualifies this new candidate. Cargo, Clippy, SQL parsing and
native PostgreSQL for this TEXT successor remain **NOT_RUN** at handoff.

### Native storage privileges and proven retirement

The existing TEXT writer had installed tables but lacked runtime privileges.
0101 now grants only its actual storage operations: SELECT/INSERT plus
`last_revision` UPDATE on the revision table; SELECT/INSERT/DELETE plus the
four mutable text columns on the text table; and SELECT/INSERT on CommandRef
receipts. Runtime cannot directly delete a revision, change its item key, or
update/delete a receipt. The original TEXT table/key/check/index definitions
are unchanged, and the complete D1318 sink and D1356 fresh-mint/guard/grant
suffix, including the published CASE parse repair, remains byte-identical
to the exact `4322022f` base.

`cleanup_retired_item_text` is a same-transaction port over a caller-held native
SQL transaction, typed WorldId and ItemInstanceId. Its pinned definer helper
locks the actual item row, requires that exact World and stored lifecycle 2,
and removes only the text and revision rows. A live, missing or foreign-World
item is ineligible. The return value reports deletions; it is no authority
token. Runtime receives only this narrow cleanup EXECUTE grant; the control
role and PUBLIC receive none. The function pins the actual migration schema
with the same search-path pattern as the existing native helpers.

Cleanup leaves the ItemInstance, its value receipts/audit and all TEXT command
receipts intact. It never retires an item, changes custody, advances a Character
revision, starts a second transaction or invents a receipt retention period.
A repeated cleanup of the same retired item is a no-op result. The caller owns
commit/rollback and eventual real World-job authority; scheduling, selection
of a daily batch and the scheduler's fence are not installed by this leaf.

### Four added native cases and preserved prior assertions

The canonical MAIL inventory becomes **24**: the prior 20 cases plus four TEXT
cases below. All prior 20 test bodies and the existing TEXT rewrite/persistence
production bodies are preserved. The three old TEXT assertion bodies still
exercise the same storage helper; its shared storage transaction now explicitly
uses the real runtime role. Their historical privileged identity/rate fixtures
remain storage fixtures, not claims of Content or ingress qualification.

- Runtime storage writes succeed, while direct revision/key and receipt
  mutation fail with PostgreSQL `42501`; the control role cannot execute cleanup,
  and a live item's empty-text revision and receipts survive.
- A genuinely native-minted, transferred and ItemUse-retired item can be cleaned
  only in its actual World; live, missing and foreign-World controls preserve
  the entire TEXT footprint.
- Two real transactions serialize on the retained item row lock (`55P03` for
  the blocked second call). Rollback restores both metadata rows and the exact
  original receipt; committed cleanup and its retry preserve that receipt.
- Empty text retains its revision while the item is live. After actual native
  retirement, cleanup removes only that tombstone and preserves both exact
  command receipts.

The retirement cases reuse the canonical target's already-mounted native
MINT/TRANSFER harness. MINT, backpack custody and last-unit ItemUse execute
their actual Root, fences, receipts and value audit under the runtime role.
The item is not seeded retired and no value trigger is bypassed. The existing
Combat-death/backpack inputs are disclosed test fixtures; attaching TEXT
metadata through the real storage half to that test item does not claim an
admitted writable-item Content definition or an installed text command caller.

All cases require the exact installed SQLx migration101 checksum. Each new
configured retirement case emits its body-plus-cleanup marker only after both
succeed. Absent admin routing remains PRE-ROUTING / NONCANONICAL, emits no such
marker, and cannot qualify PostgreSQL. The coordinator retains the exact
inventory and output under the current approved runner/resource route:

```bash
cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --list
cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --test-threads=1 --nocapture
```

### Actual remaining owner composition

[D1369](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234490484)
assigns common SESSION-INGRESS-1 only after A2/#2187. TEXT activation still needs
that common owner serialization guard retained through the complete native
transaction and the actual TRADE owner's offered-item check. The existing
writer-only TRADE storage does not supply this shared caller. This checkpoint
adds no caller boolean, duplicate reservation owner, transport capability or
shared scheduler hook. Qualified Content admission, text/parcel posting,
stamp transfer/transform, actual DAILY claim acquisition and full MAIL runtime
composition retain their concrete owner gates; the owned cleanup and storage
ports are now available for those callers. Ready/merge remain CP decisions.

## TEXT and installed-Inbox fixture completion after 4322022f — 2026-10-17

Status **NOT_FULLY_QUALIFIED / candidate native execution NOT_RUN**. This complete
six-file successor composes the previously reviewed four-file TEXT packet with
the exact [D1390 shared inventory grant](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235491923)
and one owned negative-fixture repair. Its sole predecessor is
`4322022f2738789519f3fd1b6425f2cec77317f3`. The published two CASE parenthesis
pairs and the entire D1318/D1356 migration suffix remain byte-identical.

Actual predecessor evidence is [PG run39042229146](https://github.com/Oteryn/Oteryn-Game/actions/runs/39042229146/job/117423864377):
the full installed migrator proceeded, `durability_root` finished **1278 PASS**,
and `character_authority` finished **1610 PASS / 2 FAIL**, with no ignored cases.
Within that run the exact twenty MAIL cases were **19 PASS / 1 FAIL**. The fresh
twenty-keyed-output commit/audit/replay, source rollback, omitted proof, current
fences, runtime grants and durable attempt-budget cases passed. The log captures
no passing-case resource measurements; no payload/JSON number is promoted from
CANDIDATE on that basis. The full failed run is not relabeled success.

D1390 adds exactly `game_character_inbox_reward_claim_mint_proven` and
`game_reward_claim_inbox_mint_is_complete` to the existing closed Inbox function
inventory. Both complete function bodies were checked as direct read-only proof
readers, without DML or dynamic execution; their pinned definer/ACL and trigger
routes remain unchanged. The query, filters, sorting and exact-list assertion
are preserved. The current MAIL branch has six names. TRADE's separately owned
addition and these two belong to the **same existing list**; ordinary integration
with migration0100 must retain the seven-name union, without importing a function
that does not yet exist on this branch or creating a second test.

The other actual failure was the exact `Delivery(ItemInAnotherLocation)` assertion
in `native_inbox_refuses_wrong_root_world_old_mint_and_real_ground_atomically`.
Its real, previously committed Ground MINT intentionally also lacks a fresh
same-transaction Inbox mint receipt/audit. Source review shows that delivery is
inserted before the Inbox location, allowing the newly installed deferred delivery
proofs to refuse before the later location guard. The old assertion did not print
the alternate result, so its exact CI error variant remains unobserved.

Only that deliberate malformed-output callback now sets the **existing named**
`game_character_inbox_location_exclusive` constraint to IMMEDIATE before deliver.
This exercises the genuine Ground/Inbox collision at INSERT. All receipt, audit,
location and transaction guards stay enabled, the exact OTI03 typed assertion is
retained, and its failure now prints the actual result. Existing rollback,
Ground/item/receipt/root-revision checks and the other seven sink test bodies are
unchanged. This isolates the location invariant; it does not claim that OTI03 has
priority over every other refusal during ordinary production ALL evaluation.

The TEXT SQL grants, retired-item cleanup port and four real test bodies are byte-identical
to the reviewed TEXT source packet. The existing owned migration check adds one
read-only observation of `current_database()` and `server_version_num` before each
body, inside its unchanged cleanup boundary. It prints only the actual database
name and version, enabling the native runner to verify and clean its observed own
databases without guessing names or exposing a connection URL. No other wrapper,
case body or shared harness changes. If shared creation fails before returning a
pool, no database identity has been observed and no guessed DROP is authorized.
There are now **24 canonical MAIL PG cases**:
the existing twenty (one isolated refusal fixture corrected) and four TEXT ACL/
retirement/serialization/tombstone cases. Both new native execution and all
changed-head CI remain separate pending gates. Root and independent TRADE source
review passed on the exact TEXT and both fixture deltas; no SQL/storage permission
or runtime error mapping is weakened by these fixture changes. Independent MAIL
review also passed on the single database-observation hunk and all three existing
result/cleanup routes, with no native execution claim.

The concrete [DAILY source-owner handoff](https://github.com/Oteryn/Oteryn-Game/pull/2151#issuecomment-6235666199)
binds the unchanged published prepare/commit/reconcile API. It records callback
ordering after mint, separate source/CommandRef idempotency, mandatory rollback of
a competing fresh claim, exact recovery bindings and the narrow runtime proof ACL.
DAILY source writes, zero-item claims and an actual PENDING delivery producer remain
with their allocated owners. This checkpoint claims no full MAIL client flow or
production activation; those prerequisites remain as recorded above.

## D1377c owned TEXT history and exact118 lint repair — 2026-10-17

Status **NOT_FULLY_QUALIFIED / source candidate, native execution NOT_RUN**.
This three-file successor is based on the exact published draft
`118f1c7966607e122730aede14833a92f58e9c48`, tree
`986cda20eb93cb103060b6ec87625563c9f94fa2`. It changes only the already-owned
`durability/item_text.rs`, canonical `tests/support/mail_postgres_cases.rs` and
this task record. Migration0101, every shared owner/caller and the complete
published six-file checkpoint remain intact. Prior evidence above is historical;
its passing jobs do not qualify this successor.

[D1377c](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234906472)
requires a recovery-qualified, occurrence-locked receipt reader before the real
TEXT consumer can advance. `ItemTextWriteRequest::freeze_receipt_binding`
retains the original CommandRef, Character identity and existing SHA256 binding
of the item, typed definition and exact private text. Its fields are private;
it exposes neither text/fingerprint diagnostics nor an outcome constructor.
The value is immutable lookup DATA, not session, Character, item or Content
authority. A future consumer must retain it before awaiting the corresponding
write and across an uncertain COMMIT, rather than reconstructing it from new
client input.

`DurabilityRoot::reconcile_item_text_rewrite` consumes the real
`ReconciledCharacterAuthority` for that exact Root, rechecks the stored recovery
admission, takes the existing admission relation locks and the identical
`oteryn:item-text:` advisory transaction lock over GameSessionId bytes followed
by big-endian CommandId. Writer and reader now share one private lock function
and one private receipt lookup. The subsequent lookup statement compares the
original Character and request binding before returning any revision/change
result. Runtime already has the necessary SELECT grant in installed0101; no
new SQL, grant, registry or shared API is introduced.

The reader requires actual `READ COMMITTED` isolation because the existing Root
transaction opener preserves the connection's default. A repeatable pre-lock
snapshot cannot prove absence after a blocked writer commits. The reader refuses
such a snapshot without interpreting history. Under the ordinary profile,
`Some(outcome)` is a committed historical receipt, `ConflictingCommand` is a
stored different binding, and `None` is absence established after waiting for
the original transaction. Cancellation, unavailable storage, stale recovery or
a foreign Root capability never become absence. None of these historical
results grants permission for another physical write; any retry still needs
fresh native admission and the actual future TEXT/TRADE serialization owner.

The writer's complete admission/content/custody tail and private persistence
function are unchanged. The historical lookup extraction preserves its SQL,
key bytes, binding comparison and replay result. Exact118's
[strict job117434933385 / run39045981860](https://github.com/Oteryn/Oteryn-Game/actions/runs/39045981860/job/117434933385)
reported `clippy::nonminimal_bool` on the retired-cleanup outcome check at line68.
The same successor replaces only that Boolean expression with its equivalent
minimal form. All eight input combinations retain the same rejection; no lint
suppression or mirror test is added. Candidate Clippy remains **NOT_RUN**.

### Five real native history cases

The complete MAIL PG inventory becomes **29**: the existing24 plus five below.
The entire prior canonical MAIL test leaf is a byte-identical prefix; all three
existing TEXT unit bodies, every old storage assertion and all prior Inbox
bodies remain unchanged.

- `text_history_distinguishes_absent_unchanged_and_conflicting_original_bindings`
  distinguishes no receipt from a committed unchanged empty revision0, reads an
  exact changed receipt, refuses different private text/item/definition/Character
  bindings, and preserves the exact stored footprint through every read.
- `text_history_waits_for_the_original_command_commit_before_returning_receipt`
  refuses an actual nondefault Root snapshot, restores READ COMMITTED, then
  observes the real reader waiting on this exact transaction's advisory lock
  through PostgreSQL. Its physical COMMIT becomes one historical receipt and
  replay leaves that receipt unchanged. This coordinates actual COMMIT; it does
  not claim to inject a lost network acknowledgement.
- `text_history_waits_for_rollback_and_keeps_the_exact_request_retryable` observes
  the same native wait, resolves actual rollback as absence, and checks that a
  later identical storage attempt yields the one receipt under the retained
  original binding.
- `text_history_survives_real_retirement_with_fresh_authority_on_a_new_root`
  performs actual native last-unit ItemUse and owned retired cleanup, then
  rejects an old Root capability on a newly opened real Root and reads the
  retained receipt only after that new Root issues its own recovery capability.
- `text_history_stale_database_recovery_exposes_neither_receipt_conflict_nor_absence`
  uses an explicit administrative recovery-mismatch fault while the real external
  seal remains held; all three possible historical branches remain unavailable.
  It fabricates no item value or TEXT receipt and disables no guard.

Each new case uses the existing canonical MINT/TRANSFER harness and genuine
Root/runtime role, exact PostgreSQL17.6 and installed migration101 checksum.
The item is actually minted and placed in the direct backpack. TEXT metadata
uses the real private persistence code under the exact CommandRef/root/item
locks; an owned test-only staging helper retains that transaction before its
COMMIT or rollback. The existing backpack/death fixtures and the writable
metadata attachment are explicitly storage fixtures, not admitted letter
Content or an installed TEXT writer/transport path. No receipt/value rows are
seeded by the new tests. Completion is printed only after the entire body and
canonical database cleanup succeed; absent routing emits no success marker.

Coordinator-only commands after binding the real successor and approved route:

```bash
cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases::text_history_ -- --list
cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases::text_history_ -- --test-threads=1 --nocapture
cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres mail_postgres_cases:: -- --test-threads=1 --nocapture
```

Local source checks cover exact118 preimages, identical preserved writer/test
bodies, current Root/recovery API binding, Rustfmt1.94 parse/check and isolated
patch application/whitespace. No Cargo, Clippy, unit/PG execution, new CI or
performance qualification has been performed for this successor. Root handles
independent review, final current-head readback and any authorized publication.
D1377/SESSION and the actual #2132 offered-item consumer remain required before
physical TEXT activation; this reader installs no runtime consumer or peer hook.

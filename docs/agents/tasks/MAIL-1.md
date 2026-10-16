# MAIL-1 — mail and parcels (skeleton task record)

```yaml
task_id: MAIL-1
title: "MAIL-1: mail and parcels"
mode: IMPLEMENT
status: skeleton
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

- `apps/game-server/src/domain/mail.rs`
- `apps/game-server/src/durability/mail.rs`
- `apps/game-server/src/durability/mail_tests.rs`
- `apps/game-server/src/durability/item_text.rs`
- `apps/game-server/src/gameplay_transport/mail.rs`
- `apps/game-server/src/gameplay_transport/mail_tests.rs`
- `crates/protocol-oteryn/src/mail.rs`
- `crates/protocol-oteryn/src/mail_tests.rs`
- `apps/game-server/tests/mail_postgres.rs`
- content authoring inputs for parcel/letter/label/stamped definitions and mailbox bindings (source files only, not regenerated outputs)
- `docs/agents/tasks/MAIL-1.md` (this record: status and notes only)

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
- apps/game-server/src/durability/character_inbox.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local MAIL-TEXT/address and recipient preparation — 2026-10-15

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
All six new cases are **unexecuted**: this container has no Cargo/compiler, and
no global PC test slot was acquired. The lead supplied an isolated official
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
the successor's native strict-Clippy result remains pending actual recheck and
independent review. Existing frozen commit, PC snapshots, refs and publication
state are unchanged by this authoring repair.

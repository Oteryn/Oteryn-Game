# APP-1 — durable appearance storage and native fenced selection (partial)

```yaml
task_id: APP-1
title: "Durable selected appearance and AccountUnlock set"
mode: IMPLEMENT
status: partial
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2170
wave: 4
branch: skeleton/app-1
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
observed_main_sha: 24fc332f256638e3b784bff1b3c85b18e2665a76
allocation: "D1112 / CP issuecomment-6221235486; supersedes D1092 slot restriction"
plan: docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md
plan_row: "§4 Epic 6 APP-1; §7 supporting brief"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY
```

## Authority and base

CHARACTER_APPEARANCE is ACCEPTED through protected #2209, read at
`e2da9805d296a27b06b32335bf67394509d4910b`. Its physical storage belongs to APP-1;
account-progress/#707 §4.3/§4.6 governs earned facts. Store remains unavailable.

[CP D1112](https://github.com/Oteryn/Oteryn-Game/pull/2170#issuecomment-6221235486)
requires module registration and migration 0093 **now with the appearance recovery hunk**.
The previous D1092 requirement to withhold those files is superseded. The true branch fork
is still `5742f30f`; protected main was freshly observed at `24fc332f` on 2026-10-16.
#2163 remains open/unmerged at that read. Resolve its eventual module-line conflict by
merging main after #2163, never by rebase; this repair does not claim a newer ancestry base.

## owned_paths

- `apps/game-server/src/durability/character_appearance.rs`
- `apps/game-server/src/durability/character_appearance_stored.rs` (D1112 production-reader rename)
- `apps/game-server/src/durability/character_appearance_tests.rs` (focused pure tests only)
- `apps/game-server/tests/character_appearance_postgres.rs`
- `apps/game-server/tests/character_appearance_native_postgres.rs` (D1092 nine native cases)
- `apps/game-server/tests/support/bestiary_postgres_harness.rs` (D1092 additive helper only)
- `apps/game-server/src/durability/character_authority.rs` (appearance recovery hunk only)
- `apps/game-server/src/durability/mod.rs` (D1112 appearance registration only)
- `apps/game-server/migrations/0093_character_appearance.sql` (D1112 installed migration)
- `docs/agents/tasks/APP-1.md`
- `apps/game-server/Cargo.toml` (target registration; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))
- `apps/game-server/tests/support/character_appearance_postgres_cases.rs` (cases included from CI-run targets; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))
- `apps/game-server/tests/support/character_appearance_native_postgres_cases.rs` (added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))
- `apps/game-server/tests/character_authority_postgres.rs` (CI-run target include; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))

Other shared registries, protocol/runtime wiring, pins, content regeneration and grant-owner
ports remain outside this task. One writer, one batched push; CP owns ready, queue and merge.

## Current implementation

The unconditional native writer uses the existing semantic pass/transaction, exact root-object
recovery authority, recovery fence, admission locks, occurrence serialization, complete native
gameplay/root fence, locked progression context, exact prior selection and pending-respawn guard.
Root and progression advance exactly +1 with the immutable receipt and selected projection in
one transaction. Current interpretation is independently checked by the native owner.

CP explicitly accepts replay before the gameplay fence: an exact binding returns historical
`AlreadyCommitted`; a changed binding is `ConflictingOccurrence`. Neither result grants actor
or publication authority. The appearance revision bump stays; stale XP/death writers must retry.

`read_current_character_appearance` checks recovery/admission/gameplay/root/state in one semantic
transaction. Its progression SELECT is a plain read; the existing native gameplay fence retains
its root lock. Selected revisions ahead of the root and pending respawn fail closed. Historical
read/reconcile and the current DB read return data, not loaded-content, sex, actor or condition
capabilities. APP-3 must independently recheck those current owners immediately before publication.
`Unavailable` remains unresolved until reconciliation rather than a definite rejection.

The one production reader/recovery implementation is `character_appearance_stored.rs`.
`character_appearance_tests.rs` now contains only the unchanged three pure decoder/validator
bodies. No duplicate native IDs, validator, fence implementation or cfg-hidden writer is added.

Migration 0093 is the sole DDL source. It installs selected appearance, immutable selection/grant
receipts and earned facts, root/projection/receipt FKs, shape constraints, immutable/deferred
appearance-chain guards, all existing 0073 global progression clauses plus appearance arms,
fixed function search paths and least-privilege runtime/control grants. The appearance recovery
union and verifier call are registered in the same candidate. There is no staging SQL literal,
`PRIVATE_SCHEMA`, private SQL include or qualification overlay.

## Acceptance and outstanding qualification

- [x] D1112 source assembly: native module, installed 0093 and recovery integration together.
- [x] One migration source; four explicit installed-migrator PostgreSQL storage cases.
- [x] Production reader renamed; pure decoder tests remain separate unit tests.
- [ ] Exact clean-head qualification: nine native + four installed-storage + five unchanged
  Bestiary PostgreSQL17.6 cases, strict affected Clippy and CI. The old private PASS is insufficient.
- [ ] Production outfit/addon/mount/colour validation with admitted content, sex and earned facts.
- [ ] Actual client relog/restart/session-generation flow through the admitted runtime caller.
- [ ] Complete earned grant consumer and production observation/publication integration.

The native writer implements the tested starter subset and rejects earned/Store/mount authority
until its real resolver exists. Occurrence concurrency, typed losing writers, stale-fence no-write,
restart reload, cross-family revision chains and recovery/rollback fault cases remain in the nine
native bodies; their new clean-head execution is required before claiming this repair qualified.

The storage target has four real PG bodies, each using the unchanged native constructor through
`Harness::create_appearance`: installed ledger/version/checksum, native root/bootstrap/session and
actual root/state/receipt constraints. They cover unknown-key/provenance roundtrip and reconnect,
receipt/global revision rollback, immutable first account provenance/isolation, native validation
and privileged-corruption recovery refusal. Test-admin INSERTs are storage fixtures, not event
or grant authority. The three focused pure tests are additional unit inventory, not three PG cases.

## Validation and evidence

Required focused checks: workspace fmt; strict Clippy for the three affected integration targets;
actual PostgreSQL17.6 execution of exactly 9+4+5 selected cases with inventory/filtered counts;
governance validation and its tests. CI owns broader package/workspace qualification.
The next action is clean-head qualification and independent D1112 review, then guarded publication
on existing draft #2170. No ready/merge/new PR or manual paid-review trigger is authorized here.

Historical `e32`, `8295`, `a392` and published `29ef` evidence remains immutable and separate.
The previous 29ef **private overlay** 18-case PASS included registration and 0093 and therefore
never qualified the published head. Its exact-head CI failed E0433 because registration was
absent; D1112 repairs that source assembly. No old result is relabelled as new clean-head evidence.

## Remaining owners

The granting owner must expose the accepted transaction-bound cosmetic consumer on
`FencedGrantingCharacter`; APP-2 must supply admitted content/key/provenance compatibility;
Character creation owns sex. The pure `AppearanceContent`/`Sex` fixture inputs prove none of
those authorities. APP-3 and the runtime owner must integrate actual admission/current actor,
connection/scope/content/conditions and publication. Native storage reload is not full client
relog. Legal same-store recovery replacement cannot coexist with the native retained seal;
the issuance substitution test is privileged DB corruption, not a fabricated successor workflow.
No APP-1-specific deadline or ambiguous-COMMIT injection mechanism is claimed.

# APP-3 continuation validation — 2026-10-22

Historical checkpoint; run 3e routing, migration numbering, current filters and
final mounts supersede it: [ROUTING_3E_2026-10-22.md](ROUTING_3E_2026-10-22.md).

Layout planned before coding: durability `character_appearance/command_replay.rs`
owns stable native CommandRef association and receipt reconciliation; dispatch
`appearance_change/command_replay.rs` translates canonical wire intent. Session
`appearance_state/item_dictionary.rs` owns pinned client dictionary validation.
Native admission, replay, receipt substitution, observer visibility, temporary
Outfit and temporary Item qualification each have their own child file. Shared roots receive wiring only.

AUTHORING / in_progress, branch `w2/app-3`, starting HEAD `64fdc0693`.
No Git mutation, publication, credential access, dependency download, external
write, optional skill or paid review. The earlier continuation left migration numbering and the GUILD-1 fixture
unchanged; run 3e renumbers the migration to 0134. The owner decision remains verbatim in APP-3.md.

## Implemented

- A server-native CommandRef deterministically names one appearance occurrence.
  The APP-1 receipt binds the full canonical Change digest, Character, original
  root, prior/next selection and immutable creation sex. The writer and reconciler
  share the complete decoded-field matcher; a digest alone is insufficient.
  Restart reconciliation
  returns the historical outcome before attempting current admission; changed
  reuse refuses. It grants no current actor authority. Settlement removes only
  that matching pending command; terminal session retirement keeps full cleanup.
- Session admission accepts an optional verified-artifact Item reference set.
  It checks artifact generation, sorted unique/nonzero refs and both capability
  and membership on snapshots/deltas. A bad delta leaves the presentation state
  unchanged. Native/dev clients currently supply None and fail closed on Item
  looks until their artifact loader supplies the dictionary; no guessed outfit.
- Two real admitted players use native visibility and the fenced appearance
  reader. The peer sees the committed look with its own private catalogue root.
  Native cases also exercise rejected contexts/options, stale admitted fences,
  a base commit beneath Outfit/Item transforms, owner clear/expiry, retirement,
  and exact/changed replay after rebuilding the actual durability root.
- The observer fixture reads current global publications, keeps the Runtime row,
  and refreshes signing trust through the existing CAS publisher. It uses the
  exact published signing provenance, rather than reconstructing a publication
  at a different timestamp. Grant issuance follows publication work; all existing
  five-second evidence and ten-second grant/deadline checks remain intact.
- CI follows merged CI-PG-LIB-1 (`origin/main` 9e5770362): the rest PostgreSQL leg
  runs durability, dispatch and game-server libraries. The hard-requirement APP-3
  PG case is routed out of non-database library shards into that service leg.
  Two fixture threads bound migration load; internal competing-writer tests
  retain their concurrency. Protection, game-gate and MQ remain unchanged; the
  exact workflow pins and regression expectations move with the workflow body.

## Passing local evidence

Restart recheck PASS: ten-package Clippy, explicit Rustfmt on 30 changed files,
dispatch 3, filtered standalone targets 12/8, domain/protocol/session 6/5/4,
replay identity 1, Python routing/policy/governance and 124 governance tests.
All 15 selected PostgreSQL cases executed; no early-return pass or broad PG rerun.
Every cargo invocation used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`.
No Cargo manifest/lock changed. All-target Clippy with `--locked --quiet --
-D warnings` passed for protocol-oteryn, gameplay-domain, gameplay-content,
gameplay-durability, gameplay-dispatch, session, game-server, client, dev-client
and synthetic-client-harness (each package prefixed `oteryn-`). Rustfmt passed;
the include-only observer wrapper was checked explicitly with Rustfmt 1.95.0.
Vendored Tokio emits its pre-existing dependency documentation warning.

Exact focused cargo test arguments after that wrapper/toolchain prefix (the
current owner PG scope supersedes the earlier broad fixture invocations):

```sh
test --locked -p oteryn-gameplay-dispatch --lib appearance_change --quiet -- --test-threads=1
test --locked -p oteryn-game-server --test character_appearance_postgres --test character_appearance_native_postgres appearance --quiet -- --test-threads=2
test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-domain -p oteryn-session --lib appearance --quiet
test --locked -p oteryn-gameplay-content cosmetics --quiet
test --locked -p oteryn-gameplay-durability character_bootstrap_intent --quiet
test --locked -p oteryn-gameplay-durability --lib appearance_command_replay --quiet
test --locked -p oteryn-session --quiet
test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch -p oteryn-game-server --lib capabilities --quiet
```

Preserved earlier counts: dispatch 3 (2 actual connection/framing cases + 1 native PG case);
standalone PG targets 413 and 417 including their inherited unit tests;
authority appearance filter 17; domain/protocol/session appearance 6/5/4;
cosmetics 22; bootstrap 4; replay identity 1; complete session 68; isolated
monk-release boundary 1; capabilities 2/60/1; broader native fixtures 30/30.
The native PG case includes all its mechanic children. Receipt substitution was
RED with the partial reconciler (false Committed), then GREEN for five stored
field substitutions through real dispatch after sharing the writer's matcher.

**The PostgreSQL cases really ran; none of these appearance PG results is an
early-return pass.** The wrapper supplied the admin environment; native harnesses
created isolated databases, asserted PostgreSQL 17.6, applied the real migrator
and executed their row/receipt/fence assertions. The APP-3 dispatch and authored
earned case explicitly fail if the environment is absent. The 9 APP-1 native,
4 installed-storage, 1 authored-earned and 1 APP-3 native PG case all executed.
The two GUILD probes selected zero cases because the existing Guild integration
wrapper is `cfg(any())`; they are not PG qualification and were not changed.

Python: appearance routing 2 PASS; repository policy and its full PR/MQ routing
regressions PASS; governance validator PASS; governance unit tests 124 PASS.
Whole continuation diff/checklists and workspace constructor/caller audit PASS;
`git diff --check` PASS. No tests/assertions were removed, ignored or loosened.

## Qualification limit / unfinished

The bounded native fixture run (`--test-threads=2`) passed **30/30** on PostgreSQL.
Earlier 16-thread runs returned deadline/commit-unknown outcomes; a serial run
was 27/30, and a four-thread run was 29/30 with a monk-release timing failure.
`boundary_release_retries_not_expired_and_saves_new_remaining_serene` passed
alone and in the final bounded run; any shared-load recurrence is CI-only. No deadline,
assertion or expected outcome was loosened. CI uses the validated two-thread
fixture budget; its concurrency cases still perform their internal races.

Independent protocol/authority review, frozen-head CI and MQ remain required
and were not attempted under the local-only instruction. Acceptance is not
claimed complete. The dormant Guild fixture stays with GUILD-1; it is not an
active APP-3 library-CI dependency. The Item loader/rendering follow-up stays at
its documented admission seam, with unavailability refusing Item looks.

## Reuse and caller audit

- APP-1 receipt loader/decoder, canonical semantic binding (moved, not copied),
  recovery seal, current reader, writer and sequencer. No new receipt table.
- Existing CommandRef/CommandId, SHA-256 and strict canonical appearance codec.
- Existing sequencer Harness/database/migrator/seeder/bootstrap binding; an
  APP-3-owned `appearance_fixture.rs` wrapper selects Historical/Own/Observer
  inputs. Default historical fixture behavior is preserved.
- Existing fresh Source constructor/request body now have private input seams;
  `observer_admission.rs` is the minimal test-only wrapper, included once.
  Existing guard load/publish, signed grant verification and fresh commit are
  exercised; no fixture/harness body was copied.
- Existing Dependencies owner/runtime/initialize body, now parameterized for a
  second player. Existing native spell condition staging and owner clear paths,
  qualified spell appearances and pinned Item index.
- Existing session snapshot fixture body moved to `appearance_fixture.rs` and
  shared by old/new tests; same golden bytes, peer, framing and domain router.
- Existing CI-PG-LIB-1 step and existing reviewed workflow/pin validators.

`rg apps/ crates/ tools/` covered every new field/variant/method and its
constructor/impl/caller: transport_binding, Admission.appearance_items,
AppearanceState.items, dictionary fields, three test CreationFixture variants,
Source input seams and both new production replay APIs. The unit replay module
is mounted once from the durability lib root, so path-loaded integration owners
cannot register it again. All new production public methods have native dispatch
callers and the real-dispatch PG qualification. Public test wrappers are listed
above and expose only the existing private fixture implementation.

## File sizes

The restart audit also explicitly formats/checks all 30 changed Rust files:
`cargo fmt --all` cannot reach the durability children behind `include!`.
Two durability formatting repairs remove the whole-diff whitespace hit.
No file grows by more than 200 lines against the continuation HEAD; no existing
source file grows by 150 lines. Cumulative task growth from 9f814e72a includes:

| File | Lines added net | Layer reason |
|---|---:|---|
| native_gameplay/native_cosmetics.rs | 390 | Existing content admission body extracted once. |
| gameplay_transport/appearance_change.rs | 299 | One native command composition mechanic; durable/pure rules stay below it. |
| gameplay_transport/appearance_change_postgres_tests.rs | 242 | Native driver and wiring to mechanic-specific children, reusing the native fixture. |
| connection/visibility_tests/appearance_change_tests.rs | 238 | One appearance transport qualification module. |
| session/src/domain_dispatch.rs | 233 | Existing presentation-domain router extracted from the large lib root. |
| docs/agents/tasks/APP-3.md | see final card | Retained prerequisite history, verbatim decision and validation; no runtime code. |
| APP-3/VALIDATION_2026-10-22.md | 221 | Historical validation/manifest evidence, no runtime code. |

## Complete continuation changed-file manifest

- `.github/workflows/merge-gate.yml`
- `.github/workflows/merge-group-gate.yml`
- `.github/workflows/pr-postgres-harness.yml`
- `apps/client/src/lib.rs`
- `apps/game-server/tests/support/character_appearance_native_postgres_cases.rs`
- `apps/game-server/tests/support/character_appearance_postgres_cases/earned_unlocks_tests.rs`
- `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`
- `apps/game-server/tests/support/character_revision_sequencer_postgres_cases/appearance_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/command_replay.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/command_replay_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/native_admission_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/observer_admission.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/observer_visibility_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/receipt_substitution_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/temporary_appearance_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests/temporary_item_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_fresh_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/character_appearance.rs`
- `crates/gameplay/durability/src/durability/character_appearance/command_replay.rs`
- `crates/gameplay/durability/src/durability/character_appearance/command_replay/command_replay_tests.rs`
- `crates/gameplay/durability/src/lib.rs`
- `crates/session/src/appearance_state.rs`
- `crates/session/src/appearance_state/item_dictionary.rs`
- `crates/session/src/appearance_state/mapping_reducer.rs`
- `crates/session/src/appearance_state/mapping_tests.rs`
- `crates/session/src/lib.rs`
- `crates/session/src/tests/appearance_session_tests.rs`
- `crates/session/src/tests/appearance_session_tests/appearance_fixture.rs`
- `crates/session/src/tests/appearance_session_tests/item_dictionary_tests.rs`
- `docs/agents/tasks/APP-3.md`
- `docs/agents/tasks/APP-3/CONTINUATION_2026-10-22.md`
- `tools/dev-client/src/lib.rs`
- `tools/repository/test_appearance_postgres_routing.py`
- `tools/repository/test_validate_merge_group_pg_sim.py`
- `tools/repository/validate_repository_policy_core.py`

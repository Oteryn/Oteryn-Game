# APP-3 authoring validation — 2026-10-22

Layout: domain owns pure earned-fact interpretation; content owns authored grant
admission; durability owns creation sex, transaction-bound earned reads and the
appearance revision writer; dispatch owns runtime/transport composition;
protocol owns the codec; session owns the connection-local presentation reducer.
Each mechanic has its own child module and child tests. The original admission,
bootstrap, integrity, movement and session-dispatch bodies were moved, not copied.

State: **in_progress / AUTHORING**, baseline
`9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`. No branch change, freeze, PR,
commit, push, credential access, dependency download or external write.
The candidate is not acceptance-complete or ready for integration.

## Validation evidence

All commands use the local installed Rust 1.95.0; the advertised queued wrapper
is unavailable. No Cargo manifest/lock changed, so Clippy used nine named
packages rather than workspace scope. `cargo fmt --all -- --check` passed.
Clippy with `--locked --all-targets -- -D warnings` passed for dispatch,
durability, content, domain, protocol-oteryn, session, game-server,
synthetic-client-harness and client. Vendored Tokio emits its existing
missing-documentation dependency warning; no repository crate lint failed.

Exact passing cargo commands/filters:

```sh
cargo +1.95.0 test --locked -p oteryn-protocol-oteryn appearance
cargo +1.95.0 test --locked -p oteryn-gameplay-domain appearance
cargo +1.95.0 test --locked -p oteryn-gameplay-content cosmetics
cargo +1.95.0 test --locked -p oteryn-gameplay-durability character_bootstrap_intent
cargo +1.95.0 test --locked -p oteryn-gameplay-durability character_revision_sequencer
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch connection::visibility_tests::appearance_change_tests
cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch capabilities
cargo +1.95.0 test --locked -p oteryn-session
```

Counts respectively: 5, 6, 22, 4, 11, 2, 60 and 67. The session suite includes
`appearance_result_first_and_observation_first_converge_through_session_dispatch`.
The two transport cases exercise actual admitted connection dispatch/framing,
stationary mapping-before-VIS2 ordering, hidden-player exclusion, rejection,
Unresolved cursor preservation and capability downgrade with scripted owners.
They do not substitute for the native DB qualification below.

Python cosmetics authoring: 23 tests / 37 subtests passed. Governance validator
and its 124 unit tests passed. `git diff --check` passed.
The owner and repository checklists were re-read against tracked diffs and new
files. Self-review fixed stale admitted-fence adoption, Unresolved cursor
advancement, nested transient error classification, native temporary palette
validation, and staging of visibility/item/appearance candidates before cursor
publication. No assertions were removed or relaxed. Existing tests moved to
child modules retain their registrations once per owning test target.

## Unfinished and blockers

- `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent and no PostgreSQL executable or
  advertised queued wrapper was found. Existing skip-guard results are not PG
  evidence. The new PG cases fail explicitly without that environment.
- Exact outstanding native filters:
  `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch appearance_change`
  (includes the native real-dispatch PostgreSQL case),
  `cargo +1.95.0 test --locked -p oteryn-game-server --test character_appearance_postgres appearance_earned_authoring_drives_fenced_writer_without_revision_equality`,
  `cargo +1.95.0 test --locked -p oteryn-game-server --test character_appearance_native_postgres`,
  and `cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres`.
- GUILD-1 exclusively owns `durability_guild_tests.rs`; its version-2 bootstrap
  fixture needs version 3 plus `requested_sex: Male`. APP-3 restored its initial
  edit on discovering that ownership. Its writer must align the fixture, or the
  owner must explicitly permit the narrow edit.
- The complete native negative matrix remains unqualified, especially real
  two-observer publication, competing writers, owner retirement, temporary
  transform clear/expiry, and ambiguous commit across process restart. The
  retained occurrence association currently lives in the owner memory; a durable
  selected row/receipt alone is not proof of CommandRef replay after restart.
- Item Look checks capability 4 and the server's pinned Item index. The session
  reducer has no client-artifact Item dictionary resolver; that required boundary
  remains unfinished. No guessed fallback is installed.
- New PG cases deliberately require the DB environment, including the library
  native-dispatch case. Existing non-PG library CI legs do not supply it; final
  PG target routing must be resolved without adding skips or weakening tests.
- Authenticated operator bootstrap v3 carries sex. Existing Character Commands
  V1 creation carries no sex and remains unavailable for appearance. Platform
  producer rollout and any sex-bearing user-creation contract are not changed
  in another repository by this lane.
- Required independent protocol/authority review, exact-head CI and merge-queue
  evidence do not exist for this uncommitted authoring candidate.

## Reuse and wrapper inventory

- Existing APP-1 writer, receipts/reconciler, fenced current reader, account
  unlock reader, recovery/admission/session fences and revision sequencer.
- Existing domain selection validator and immutable `Sex`; no replacement
  entitlement or palette policy.
- Existing APP-2 native cosmetics compiler/promotion/source-pin admission and
  `NativeCosmeticsView`; extracted original implementations into children.
- Existing private `cosmetics_test_packet` and activated-content fixture bodies
  moved into test-only `native_gameplay/test_support/cosmetics_fixture.rs`.
  `decode_cosmetics` is the minimal wrapper over the production decoder;
  `activated_with_optional_cosmetics` parameterizes the existing activation
  fixture, with its previous caller forwarding `None`.
- Existing Bestiary/PostgreSQL harness, `seed_unlocks`, monk-save `Harness`,
  current-session preparation and `Dependencies` production-owner composition;
  only the optional content pin/room input was added to that fixture.
- Existing private familiar fence helper moved into `character_gameplay_fence`;
  every old caller uses it and the old body is deleted.
- Existing visibility interest query, item handles, frame codec and movement
  follow-up owner; staged clones preserve the original publication boundary.
- Existing condition stores, spell-appearance registry qualification, owner time,
  runtime lease/control facts and pinned Item index for effective public looks.
- Existing strict charm wire primitives and existing session command/domain
  dispatch; the independent byte fixture is shared by codec/session tests.
- Existing connection test `Channel`, runtime runner, session/admission helper,
  frame reader/writer and `first`; existing session peer/admission/spatial helpers.
- Python tests compose the existing `CosmeticsAuthoring` fixture instance and its
  mutation helper; no inheritance duplicates old test registration.
- Historical standalone ability fixture now re-exports canonical domain
  appearance instead of loading that source a second time with `#[path]`.

Constructor/match/implementation searches covered `apps/`, `crates/`, `tools/`
for the new cosmetic identity, grant index/document field, creation sex fields,
current appearance state, appearance protocol messages, session event/update,
composed owner command store and trait extensions. Three owner constructors
were updated; all account-unlock reader callers were updated. Synthetic-client
exhaustive matching was updated; renderer consumers retain their existing
metadata catch-all because rendering is APP-4. The Guild fixture remains the
explicitly reported ownership exception.

## Files growing by more than 200 lines against HEAD

| File | Growth | Layer justification |
|---|---:|---|
| `native_gameplay/native_cosmetics.rs` | +390 | Existing source/promotion admission body extracted from its content owner; no second decoder. |
| `gameplay_transport/appearance_change.rs` | +284 | One native command mechanic: original intent retention, current owner checks and sequencing composition; pure and durable rules remain in their owners. |
| `connection/visibility_tests/appearance_change_tests.rs` | +238 | One new mechanic's real transport tests, mounted once under the existing connection owner and reusing its harness. |
| `session/src/domain_dispatch.rs` | +233 | Existing session domain router extracted from the large root; only appearance routing/entity-reference validation was added. |
| `docs/agents/tasks/APP-3.md` | +246 (183 → 429) | Task evidence retains the two previous prerequisite audits and verbatim owner decision; no runtime logic. |
| `APP-3/VALIDATION_2026-10-22.md` | +221 | Required complete manifest, test evidence, reuse and size audit in the task-record layer; no runtime logic. |

No pre-existing source file grows by over 150 lines. No original file above
1500 lines gains a new mechanic body; shared slots contain wiring/extractions.

## Complete changed-file manifest

- `apps/game-server/migrations/0134_character_sex.sql`
- `apps/game-server/src/lifted_tests/durability_vip_tests.rs`
- `apps/game-server/src/lifted_tests/party_shared_xp_postgres_tests.rs`
- `apps/game-server/tests/ability_engine.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/bestiary_postgres_harness.rs`
- `apps/game-server/tests/support/character_appearance_postgres_cases.rs`
- `apps/game-server/tests/support/character_appearance_postgres_cases/earned_unlocks_tests.rs`
- `crates/gameplay/content/src/content/native_gameplay.rs`
- `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader.rs`
- `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs`
- `crates/gameplay/content/src/content/native_gameplay/cosmetics_view.rs`
- `crates/gameplay/content/src/content/native_gameplay/native_cosmetics.rs`
- `crates/gameplay/content/src/content/native_gameplay/test_support/cosmetics_fixture.rs`
- `crates/gameplay/content/src/content/native_gameplay/tests/grant_mappings_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/catalogue.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/observation.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/offer.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/appearance_change_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/character_gameplay_fence.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/appearance_change.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/appearance_change/mapping_refresh.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/moved_step.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/visibility_tests/appearance_change_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/familiar_cast.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/familiar_cast_dispatch.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_view.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_spatial.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/world_spatial/appearance_visibility.rs`
- `crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs`
- `crates/gameplay/domain/src/domain/appearance.rs`
- `crates/gameplay/domain/src/domain/appearance/earned_cosmetics.rs`
- `crates/gameplay/durability/src/character_bootstrap_intent.rs`
- `crates/gameplay/durability/src/character_bootstrap_intent/tests.rs`
- `crates/gameplay/durability/src/character_bootstrap_intent/tests/character_sex_tests.rs`
- `crates/gameplay/durability/src/durability/character_appearance.rs`
- `crates/gameplay/durability/src/durability/character_appearance/character_sex.rs`
- `crates/gameplay/durability/src/durability/character_appearance/current_appearance.rs`
- `crates/gameplay/durability/src/durability/character_appearance/earned_unlocks.rs`
- `crates/gameplay/durability/src/durability/character_appearance/stored.rs`
- `crates/gameplay/durability/src/durability/character_authority.rs`
- `crates/gameplay/durability/src/durability/character_authority/bootstrap_character.rs`
- `crates/gameplay/durability/src/durability/character_authority/creation_integrity.rs`
- `crates/gameplay/durability/src/durability/character_progression.rs`
- `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`
- `crates/gameplay/durability/src/durability/character_revision_sequencer/appearance_revision.rs`
- `crates/protocol-oteryn/src/appearance.rs`
- `crates/protocol-oteryn/src/appearance/catalogue.rs`
- `crates/protocol-oteryn/src/appearance/mappings.rs`
- `crates/protocol-oteryn/src/appearance/selection.rs`
- `crates/protocol-oteryn/src/appearance/wire.rs`
- `crates/protocol-oteryn/src/appearance_tests.rs`
- `crates/protocol-oteryn/src/appearance_tests/mappings_tests.rs`
- `crates/protocol-oteryn/src/capability_tables.rs`
- `crates/protocol-oteryn/src/lib.rs`
- `crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin`
- `crates/session/src/appearance_state.rs`
- `crates/session/src/appearance_state/mapping_reducer.rs`
- `crates/session/src/appearance_state/mapping_tests.rs`
- `crates/session/src/domain_dispatch.rs`
- `crates/session/src/gated_command.rs`
- `crates/session/src/lib.rs`
- `crates/session/src/tests/appearance_session_tests.rs`
- `docs/agents/tasks/APP-3.md`
- `docs/agents/tasks/APP-3/VALIDATION_2026-10-22.md`
- `docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md`
- `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`
- `docs/contracts/protocol-oteryn/v1/appearance_v1.proto`
- `tools/content-schema/cosmetics-authoring/README.md`
- `tools/content-schema/cosmetics-authoring/cosmetics.schema.json`
- `tools/content-schema/cosmetics-authoring/cosmetics_authoring.py`
- `tools/content-schema/cosmetics-authoring/test_grant_mappings.py`
- `tools/synthetic-client-harness/src/live/model.rs`

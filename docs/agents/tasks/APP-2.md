# APP-2 — outfit and mount definitions (skeleton task record)

```yaml
task_id: APP-2
title: "APP-2: outfit and mount definitions"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-2 (rows 126, 127); §7 supporting briefs row APP-2"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-2
needs_migration: false
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2173](https://github.com/Oteryn/Oteryn-Game/pull/2173). Canonical cosmetics authoring and candidate compiler are merged. Evidence: `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader.rs`.

Remaining: Active content/profile/sex/unlock admission and complete production outfit/mount population remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Goal

Outfit and mount definitions with look types, starter flags and palette in the content pipeline.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-2 (impl, content).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| content pipeline | MERGED |
| #1025 loader follow-up | verify |
| acceptance of CHARACTER_APPEARANCE | ACCEPTED on main; D1093 grants schema/materializer and native ABI authoring |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/cosmetics/outfits/ and content/cosmetics/mounts/ authoring inputs (existing index.json present: extend)`
- `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader.rs`
- `crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/tests.rs`
- `docs/agents/tasks/APP-2.md` (this record: status and notes only)

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

- content/cosmetics/outfits/index.json, content/cosmetics/mounts/index.json (existing)
- tools/content-migration/regenerate_content.py

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author

## Acceptance criteria

- [ ] Every outfit/mount has look types per sex, starter flag and palette.
- [ ] Loader rejects malformed rows; regenerate_content.py --check clean.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §4.1, §5
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Private candidate document/source-binding design — before source work (2026-10-15)

CP allocated exactly the two cosmetics loader files and this card on `skeleton/app-2`,
base `afc0be27023b6ba7f937740577111df9a0e7f7cc`, existing draft #2173. The owning
CHARACTER_APPEARANCE decision is ACCEPTED on main `ac9f172836f00226b68088aa04db95ccf1ab994e`.
This is a candidate compiler, not active-world admission or a cosmetic grant/writer.

1. **Existing reader output, not another DAT parser.** Reuse the actual
   `tools/content-census/stage_appearance_tables.py` Outfit source observation format:
   `imports/cipsoft-appearances/outfits/manifest.json` + its ten existing shards. The historical
   manifest binds the 15.30 DAT source, each exact shard and each Outfit record's digest.
   `client-assets::AppearanceIndex` is also real but is not a game-server dependency; no Cargo
   dependency is added and no object-only item membership is misused for Outfit looks. Current
   source observations are reference data only and grant no ProductionKey/gameplay authority.
2. **Explicit proposed binding, no installed production pin.** The compiler receives an independent
   proposed expected document digest/content revision, source-manifest digest and DAT digest.
   It checks exact bytes, manifest format/family/source identity, shard names/hashes/counts and
   duplicate/out-of-range source IDs. No hardcoded new active pin, source generation, trust root
   or registry is installed. A proposed expected binding is data, not current authority. A later
   CP owner must obtain it from the independently admitted active artifact/world/content source.
3. **Private document `OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1`.** Required fields: content revision,
   source manifest/DAT digests; a content palette (identity/revision, explicit RGB entries and
   provenance); Outfit definitions (identity/revision, male/female look refs with exact source-
   record digests, addon count, starter flags and explicit starter-addon masks per sex, premium
   flag, unlock-source citations and provenance); Mount definitions (identity/revision, look ref,
   recorded speed bonus, premium flag and provenance); exactly one fallback per sex (outfit key,
   four colour indices). Fallback has no addons/mount by shape. No serde defaults fill missing
   facts. Unknown/duplicate members and malformed/partial input fail closed. Source citations
   record authoring evidence; their truth/currentness is not certified by syntactic validation.
4. **Semantic compiler.** Native ProductionKey/definition references and existing
   `domain::appearance::{AppearanceSelection, Sex, AppearanceContent, validate_selection}` are
   reused. Look refs must belong to the Outfit category and match their actual staged record
   digest. Duplicate keys, invalid addon masks/counts, empty/oversized palette, non-starter
   fallback, unknown fallback outfit and out-of-palette fallback colours reject. The compiled
   candidate exposes a read-only AppearanceContent view, explicit sex look lookup and native
   fallback selection. Premium is retained metadata, not a new gate (D61); Store/unlocks/speed
   effects/current sex/active generation remain outside this compiler. Mount speed is retained
   authored metadata, not an applied profile rule; final profile agreement is a CP prerequisite.
5. **Bounds and qualification.** Private compiler input limits:8MiB per JSON document and32MiB
   total staged source bytes; ID representation matches the existing native reader's u16 range,
   palette size fits the native AppearanceContent u16 API, addon count fits native u8 bits.
   These are defensive parse/allocation bounds, not registered runtime resource rows. Current
   Outfit `READY_UNPOPULATED` and Mount declarations with empty `fields` remain rejected. Tests
   may use explicitly synthetic authored documents against real pinned historical observations;
   that proves compiler behavior only, never population or current-world admission.

Shared CP work remains: concrete authoring/source facts (starter/sex/fallback/palette/unlocks),
source/active artifact producer and compatibility/provenance admission, profile speed agreement,
loader registration, full permanent-cosmetics population, regeneration/pins, Character sex input,
APP-1 writer and APP-3/4 observation. The original acceptance boxes remain open. No content,
production pin, registry, migration, wiring, dependency or remote action is authorized here.

### Prepared result and exact CP seams

The two owned sources implement the above private format, deterministic typed diagnostics,
exact manifest/shard/record binding and read-only candidate view. No production source-pin
constant was installed: all expected document/manifest/DAT/version bindings are explicit proposed
data. Native `ProductionKey` and `DefinitionRevisionRef` enforce structural identities; the
Reference runtime's `DefinitionFamily` has no Outfit/Mount variant, so none is invented.
`AppearanceSelection`, `Sex`, `AppearanceContent` and `validate_selection` are actual native types
and code, including fallback validation with no earned unlocks. Palette values, starter flags,
look mappings, source citations and speed metadata in positive authored tests are explicitly
RAW SYNTHETIC TEST DATA, not actual cosmetic content, AccountUnlock or world admission evidence.

`tools/content-census/stage_appearance_tables.py` was actually replayed in memory on the exact
existing 15.30 DAT from this base: all1,480 Outfit observations equal the ten existing shards.
No outputs, content, dependencies, active pins or source/registry files were changed. The current
Outfit index and252 Mount records remain unpopulated and fail this candidate format; no fallback,
starter, sex, look, RGB palette, unlock source or source-value population was guessed.

Native public-API source-bound qualification: **15 tests PASS,0 failed/ignored**, plus Clippy
metadata on the same actual owned source/tests. A controlled mutant omitting the shard-byte digest
comparison is caught by the exact negative test; unchanged original source then passes that
focused case (1pass/14filtered). This is diagnostic-mutation RED→GREEN, not a pre-existing
production defect. Initial wrong native reference-constructor integration failed compilation
before any tests; its actual log/source hashes are preserved separately and carry no pass claim.
Workspace fmt, governance124 tests (22documents/9lanes), diff/scope and hash binding are recorded
in `/workspace/oteryn-prep/app-2-evidence`. No full native Cargo/all-target/server/E2E is claimed.

Concrete integration requirements:

- Normalize fully sourced permanent Outfit/Mount authoring into this private document. Existing
  ProjectV2Declaration Outfit/Mount records have generic `fields`; their missing facts are not
  inferred from numeric membership. Bind authoring citations to actual source facts at the owning
  content admission boundary; syntax/hash proposals here are not a truth/currentness oracle.
- Extend the actual `content/native_gameplay::{PinnedGameplayBytes,NativeGameplayInput,
  compile_native_gameplay}`/`ActiveGeneration::native_gameplay` owner to retain the permanent
  cosmetics bytes/source observations under independently qualified artifact identity. The exact
  data field/ABI delta belongs CP; no guessed production API is added in this task.
- The actual `ContentActivationController::activate<A: AuthorizedContentGeneration>` boundary has
  sealed authorization, target/current identities and activation sequence. That current authority
  must independently bind the candidate to the admitted World/artifact/content generation; a
  matching document digest or constructed AppearanceContent object cannot replace it. Channel and
  World are not interchanged. Candidate compilation performs no actor or session mutation.
- Bind current Character sex, compatible earned AccountUnlock provenance and future Store
  entitlement lifecycle through their real owners. This compiler mints none. Mount speed metadata
  must agree with the admitted profile/ruleset fact before any speed effect; it applies no bonus.
- Register module/load path and populate/regenerate content/pins only in the CP slot; integrate
  APP-1 durable writer and APP-3/4 committed observation later. No readiness checkbox is closed.

## D1093 authoring successor — 2026-10-16

CP [6219643346](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346)
assigns APP-2 the canonical cosmetics schema/materializer and the `native_gameplay.rs`
cosmetics ABI/compiler hunk plus the `activation.rs` caller. This supersedes the earlier
schema/ABI allocation blocker above; historical compiler evidence remains historical.
The ten source conflicts submitted in
[6219674132](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219674132)
remain unresolved. No source value, starter, sex/look or fallback mapping is promoted here.

The tree-first schema/tool under `tools/content-schema/cosmetics-authoring/` requires complete
typed records, explicit per-field value-bound promotion evidence and exact historical source
bytes. `world_project_v2_to_tree.py` calls that producer before derived writes when a canonical
authoring input exists; absent input creates no catalogue and a stale orphan output refuses.
The immutable historical identity-only WorldProject packets and the existing cosmetics shards
are unchanged. The real producer's future output joins its manifest/lock set in the CP slot.

`NativeGameplayInput.cosmetics` is the mandatory `PinnedGameplayBytes` assigned by CP.
Historical provisioning V1–V5 supplies an explicit checked unavailable packet; V1–V8 artifact
bytes remain identical and expose no permanent cosmetics. Provisioning V6 requires authored
cosmetics. Binary V9 appends one framed section to the V8 layout. Its decoded immutable view
and sorted key iterators are owned by `NativeGameplayState`, with the same independently
qualified outer digest, and use the existing source-bound compiler. The activation caller
checks the mandatory pin and retains independent issuance and sealed generation activation.
`UNKNOWN`, OTS-only and observations-only promotion classes fail closed. The per-field decision
references are admission data, not self-issued authority or proof of source truth.

Current cheap qualification: 14 Python tests PASS; targeted Rust formatting and diff checks
PASS; governance validator and its 124 tests PASS. Five new native integration cases are authored (legacy byte preservation/pin rejection;
V9 framing/decoded metadata; unknown/promotion/source refusal; canonical-schema boundary parity;
actual activation/stale issuance).
They are not yet claimed compiled or executed. The synthetic fixture is explicitly test-only.

The first actual native V2 qualification attempt stopped before tests with E0631 in the
activation pin check: `ContentError` was passed directly to the existing `ProjectError`
qualification variant. The successor preserves that error through the existing
`From<ContentError> for ProjectError` conversion. The failed attempt remains failed evidence;
native execution and production lint must be rerun against the successor's exact source.

Seven external test constructors require the mechanical new unavailable-pin initializer in six
other-family files; these hunks remain private validation assembly pending their exact CP scope.
No compiling/publish-ready whole-head claim is made while that cascade remains unresolved.
Native qualification/review, real content population, APP-1/APP-3/APP-4 integration and the shared
index/manifest/lock slot after #2129/#2197 remain open. No ready/merge authority is claimed.


## D1117 canonical initializer checkpoint — 2026-10-16

CP 6224930201 grants exactly seven cosmetics initializers in six native test-fixture files.
They now explicitly call the existing checked `NativeGameplayInput::unavailable_cosmetics()`;
no production catalogue, content identity or authority is supplied by these fixtures.
The former private constructor overlay is no longer needed for this successor.

Qualification state at authoring freeze: **NOT_FULLY_QUALIFIED**.
- Historical fa06 private native41 / Python14 / real-generator3 PASS remain separately bound
  to their original source and explicit private overlay; they are not this head's qualification.
- Current successor native compile/focused cosmetics tests/strict affected Clippy: NOT_RUN.
- Full package tests, CI and integrated admission/runtime: NOT_RUN.
- Previous fa06 CI RED seven missing initializers is addressed by source; fresh execution
  must establish the result, and the historical missing presentation-module CI failure
  remains historical, not a repaired or passing claim.

D1122 permits one coherent existing-draft checkpoint after fmt/diff checks, without ready,
manual review trigger or CI restarts. Actual results belong to exact-SHA evidence and PR
status. The ten live-source conflicts remain UNKNOWN; lock/manifest waits for #2129/#2197.

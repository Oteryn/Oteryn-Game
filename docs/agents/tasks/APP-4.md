# APP-4 — client outfit window and other-actor rendering (consolidated task card)

```yaml
task_id: APP-4
title: "APP-4: client outfit window and other-actor rendering"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
class: "impl (client)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-4 (rows 126, 127); §7 supporting briefs row APP-4"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md   # §3, §4.1-4.7, §5
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: false   # wire ids belong to APP-3; this task adds none
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. No outfit window/composer from this card is retained on main. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: APP-3 integration and the authorized outfit catalogue/window/rendering work remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2176, branch skeleton/app-4
(card, plus a partial static compositor and preview in `crates/client-runtime/src/appearance_rendering*.rs`
and `apps/client/src/outfit_preview*.rs`; the draft code is described below as reference behaviour only)

## Goal

Client outfit window and rendering of other actors' looks. The window lists the server-authorized
choices and previews a draft; other actors (and the own actor) render their committed, server-observed
look. The client presents; the server owns selection, unlocks, sex, fallback and temporary conditions.
Inseparable bundle: client window + renderer + tests.

## Dependencies (re-verified against origin/main 864beba6d, 2026-10-22)

| Dependency | State |
|---|---|
| APP-3 (appearance command, actor appearance state, wire codec; sibling in `docs/agents/tasks/APP-3.md`) | NOT_STARTED re-implementation card (draft #2175 closing): **BLOCKING**. Defines catalogue/change/base/temporary observation, `appearance_ref` resolution, sequencing and capability negotiation; this task adds no codec, id or registry entry |
| APP-1 durable appearance writer, recovery | MERGED partial (#2170), status `partial` in `docs/agents/tasks/APP-1.md`; fenced selection/grant writer and receipt integration not complete |
| APP-2 canonical cosmetics authoring, native ABI | MERGED partial (#2173), NOT_FULLY_QUALIFIED in `docs/agents/tasks/APP-2.md`; no admitted populated permanent cosmetics, current sex, activation or fallback truth |
| client owner (#1942) | open draft, untouched by this card: **BLOCKING**; owns the window/panel shell, focus/hit-test capture, input registration and `world.rs` adoption. CP must pin its exact head and file ownership |
| Temporary Outfit/Item condition candidate (`condition_snapshots`, `crates/protocol-oteryn/src/world_spatial_entities.rs` actor detail) | qualified producer exists; no allocated routing/capability and no Session consumer: reuse and admit through APP-3, do not build another lane |
| CHARACTER_APPEARANCE acceptance | ACCEPTED (ACCEPT_BATCH §1, owner item 5, #2209); child integration gates remain |
| Pinned client asset source | native assets are the pinned 15.30 manifest; the 15.32 normalized contract is semantic evidence only |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be drafted
earlier but must not assume unmerged schemas.

## Accepted decisions

- **CHARACTER_APPEARANCE §4.1-§4.7:** the server owns the per-Character selected look and account-scoped
  earned unlocks. Starters are implicit active-content choices (no synthetic AccountUnlock); earned
  provenance must be compatible; Store stays unavailable without its own entitlement path; the server
  validates the four colour indices, each addon and the optional stored mount. Client rendering and preview
  are presentation, never authorization. A rejected change changes nothing. A world-incompatible selection
  uses the server's explicit content/sex fallback and preserves the stored selection.
- **Mount-speed amendment:** the client never computes the mount speed bonus, infers an active mount from a
  stored mount key, or invents mounted-pattern semantics.
- **D61 (APP-3):** a premium metadata flag adds no V1 gate; the UI consumes the server-authorized choices and invents no eligibility.
- No session, account, sex, authority, unlock provenance or grant receipt is taken from or fabricated by the client.
- Unknown look, mask, palette, dimension or sprite produces unresolved presentation (a factual marker), never a
  nearby id, a default sex, look 128 or a client-authored fallback. Visibility remains server-owned: a marker fallback
  never reveals a hidden actor. Preview data is kept out of observed state.
- Not decided here: numeric ids, capability offer, default hotkey, registry allocation, payload bounds, error enum
  (APP-3 / registry); the UX policy "Apply disabled until the preview is resolvable" is a proposal for #1942 review.

## Reference behaviour from the draft (re-implement, do not copy)

**Static compositor (draft `appearance_rendering.rs`, 293 lines, unregistered, 9 runtime tests authored).**
`PinnedOutfitAssets::open` consumes the native `AssetStore::open_pinned`, `Catalog`, `AppearanceIndex` and
`SpriteSheets` (independent exact manifest pin required); `compose` returns composed RGBA pixels and a native
`AtlasImage` with source bottom-right anchoring and displacement, plus a variant key. It reuses the native outfit
category, sheet layout/cell decode/hash checks and bounded sheet LRU; it adds no DAT/BMP/LZMA decoder, HSI resolver,
sprite cache or fallback look. The four colours resolve from a supplied 133-entry content palette and indices
(supplied palette is data, not authority). Rendering: exact opaque mask roles (yellow head, red body, green legs, blue
feet), RGB multiplication by role colour/255, base alpha-over then mask per row, ascending enabled addon rows;
cardinal widths 2/3 only; unsupported layer/addon grammar, unknown look and invalid palette fail explicitly; sprite 0
keeps the native empty-cell meaning. It renders only phase 0 / z 0 of the first frame group the loader exposes; no
other group, animation, moving, mount or effect support. The cache identity includes palette, direction, addons
(a sprite-only `CellKey` cannot distinguish colour variants).
**Preview (draft `outfit_preview.rs`, 28 lines):** calls the same compositor, returns a fresh resolved or unresolved
frame; a failure never returns the previous look; stores no selected or observed state.
**Draft status:** component-qualified on Linux x86_64 (runtime 9 + preview 2 passed, zero failed/ignored; strict Clippy
passed) with real pinned 15.30 assets, 4 synthetic semantic controls and 6 native asset/preview cases. Not qualified:
`apps/client` integration, module/dependency registration, scene or other-actor adoption, outfit UI, animation, GPU,
performance, admitted palette/sex/unlock/fallback authority.

**Design requirements from the card (not implemented):**
1. **Own catalogue.** Opening requests the server-authorized outfits, allowed addon subsets and stored mounts with
   content/palette binding and freshness; sex-specific look references come from the server; implicit starters and
   compatible earned choices only (not every DAT outfit or historical unlock); RGB entries and fallbacks from admitted
   content, never APP-2 fixtures or Citizen 128 defaults.
2. **Change request/result.** One semantic request (outfit key, four palette indices, addon bits, optional stored mount) under
   APP-3's catalogue/revision binding; the result correlates the command and distinguishes rejection, committed and unresolved.
3. **Base vs displayed.** Committed base selection stays distinct from the effective temporary/fallback display; a bare
   `appearance_ref` or acknowledgement is insufficient; a temporary override is never stitched onto a newer base, expired by a
   local timer, or cleared by an omitted packet; Item transforms use item presentation.
4. **Window states.** Closed -> Loading (no invented rows or enabled Apply) -> Editing (separate draft; shared composer for
   preview) -> PreviewUnavailable -> Submitting (exactly one pending request; scene shows committed observations only) ->
   Rejected/NeedsRefresh (draft kept where compatible) -> CommittedAwaitingObservation (observation-before-result and duplicates
   converge once) -> OutcomeUnknown/Ended (no automatic resubmit; session end drops catalogue, actors, pending state and drafts).
5. **Flow.** APP-1 change/grant + APP-2 content/sex -> APP-3 catalogue/result/actor projection -> Session validated domain store ->
   PlayLink task -> PlayView appearance store/window -> one shared composition path for draft preview and observed actor draws ->
   existing renderer; commands return through the same session-command owner (UI starts no runtime/network writer).
6. **Scene.** Retain a separate all-actor appearance map keyed by exact `EntityRef`/generation (do not turn players into attack
   targets; Creature-only attack targeting stays); preserve map-under -> lifted actor -> map-over -> target order and floor culling;
   qualify 256 snapshot entities with distinct outfits against the 81,920-quad batch and 4096 px atlas limits; no new budget.

## Owned paths (ADR-0022 layout)

ADR-0022 governs `crates/gameplay/*` only; the client paths keep their crates. Mapping used: the draft's
`crates/client-runtime/src/appearance_rendering*.rs` (a Tokio-only crate with no Scene, actor store or render backend) moves to
`crates/client-assets` beside `appearance.rs` (the pixel source), as the draft's own card proposed; the app-side window and actor
presentation stay in `apps/client/src`. Both placements need CP/#1942 approval before code.

- `crates/client-assets/src/outfit_composition.rs` (new, the static compositor; one mechanic)
- `crates/client-assets/src/outfit_composition_tests.rs` (new)
- `apps/client/src/outfit_preview.rs`, `apps/client/src/outfit_preview_tests.rs` (new)
- `apps/client/src/outfit_window.rs`, `apps/client/src/outfit_window_tests.rs` (new, pure window model: catalogue, independent draft, one pending command)
- `apps/client/src/appearance_scene.rs`, `apps/client/src/appearance_scene_tests.rs` (new, per-`EntityRef` presentation store)
- `docs/agents/tasks/APP-4.md`

Slot-only (final commit at the queue turn): `crates/client-assets/src/lib.rs` export; `apps/client/src/{lib.rs,windows_shell.rs,input.rs,play.rs,scene.rs,world.rs}`
registration, focus/hit-test capture, semantic command/events and per-actor draws (owned with #1942); renderer resource-key/upload and
Windows draw-pass for composed variants (`crates/renderer/src/{batch.rs,resources.rs,windows.rs}`); Session consumer (`crates/session/src/lib.rs`)
and APP-3 protocol codec wiring (`crates/protocol-oteryn/src/lib.rs`, registry). No migration, no content regeneration.

## Acceptance criteria

- [ ] The outfit window lists only server-authorized looks; the preview matches the committed look rendered by the same composer.
- [ ] Other actors render with their committed look; the own actor too.
- [ ] A rejected change leaves the scene unchanged; observation-before-result, duplicates and an unresolved outcome converge without a second write or automatic resubmit.
- [ ] Temporary Outfit/Item overrides and server fallback shown distinctly from the committed base; the client never clears or expires them locally.
- [ ] Unknown pixels render a factual marker and never reveal a hidden actor; no fabricated look, sex or palette.
- [ ] Cache identity includes content/palette/direction/addons; same sprite with different colours does not collide; no unbounded atlas/cache.
- [ ] Modal input is captured (no click-through gameplay commands); resize/device-loss recovery on the real Windows shell.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change (a new session drops old catalogue, drafts, EntityRef generations).
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file; new tests in their own child files; `lib.rs`/`mod.rs`/`world.rs`/`scene.rs` edits wiring-only.

Planned qualification matrix (none run by this card): catalogue authorization (starter, earned, locked, unknown sex, unpopulated),
palette min/max/max+1 and mask/addon negatives, commit/reject ordering, reconnect (no old catalogue, result or generation reuse),
reference binding refusal, temporary/current display, server fallback, source/pixel cache (manifest binding, cross-version mismatch,
same sprite different palette/phase, eviction while pinned), capacity (max/max+1, page exhaustion, multi-batch split) and real
#1942 shell input. Linux model tests alone do not prove the UI.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-client-assets outfit_composition
cargo test -p oteryn-client outfit_preview
cargo test -p oteryn-client outfit_window
cargo test -p oteryn-client appearance_scene
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
python3 tools/agents/validate_governance.py
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md` §3, §4, §5; Game-owned `OTERYN_ATLAS_ANIMATED_APPEARANCES_V1` outfit grammar
- `docs/agents/tasks/APP-1.md`, `APP-2.md`, `APP-3.md`
- `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4, §5 (Wave 4), §7

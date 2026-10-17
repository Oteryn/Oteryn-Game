# APP-4 — client outfit window and other-actor rendering (skeleton task record)

```yaml
task_id: APP-4
title: "APP-4: client outfit window and other-actor rendering"
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-4 (rows 126, 127); §7 supporting briefs row APP-4"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-4
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Client outfit window and rendering of other actors' looks.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-4 (impl, client).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| APP-3 | BLOCKING |
| client owner (#1942) | BLOCKING (#1942 open draft) |
| CHARACTER_APPEARANCE acceptance | ACCEPTED: ACCEPT_BATCH §1, owner item 5 (#2209); child integration gates remain |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/client/ outfit window module (new files only)`
- `crates/client-runtime/ appearance rendering (new files only)`
- `docs/agents/tasks/APP-4.md` (this record: status and notes only)

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

- client panel registry in apps/client (coordinate with #1942 owner)
- crates/protocol-oteryn appearance codec from APP-3

## Inseparable bundles (never split across PRs)

- client window + renderer + tests

## Acceptance criteria

- [ ] Outfit window lists unlocked looks only; preview matches committed look.
- [ ] Other actors render with their committed look.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §5
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Concrete client owner/interface packet — 2026-10-15

**REPORT_ONLY_CLIENT_OWNER_PACKET**, exact base
`eaa7e0217cd2f3fcf87c879f114f1d2acb63e12d`, `skeleton/app-4`, draft #2176.
The adjacent allocation authorizes **this card only**. All filenames, interfaces
and UI states below are proposals for CP/#1942 coordination, not allocated code,
registered wire, implemented APIs or runtime delivery. No external owner message
was sent. Effective CHARACTER_APPEARANCE was read at protected main
`ac9f172836f00226b68088aa04db95ccf1ab994e`.

### Accepted behavior and exact dependency floor

CHARACTER_APPEARANCE §4.1-§4.7 makes the server owner of the per-Character selected
look and account-scoped earned unlocks. Starters are implicit active-content
choices, not synthetic AccountUnlock entries; earned provenance must be compatible;
Store stays unavailable without its own usable-entitlement delivery path. Four
colour indices, each addon and the optional stored mount are validated by the
server. Client rendering/preview is presentation, never selection authorization.
Rejected selection changes nothing. A world-incompatible selection uses the
server's explicit content/sex fallback and preserves the stored selection.

The mount-speed amendment is present in the accepted brief; client code must not
compute its +10, infer active mount from stored mount_key, or invent mounted
pattern semantics. Effective mount activation/observation and movement facts must
arrive from their actual server owners. Likewise a premium metadata flag does not
let the UI invent an eligibility gate: it consumes the current server-authorized
choices under the effective accepted policy.

APP-1 and APP-2 are **REVIEWED_PARTIAL local preparations**, as recorded in their
exported RESULT.json files, not assumed merged/current runtime prerequisites:
APP-1 provides private projection/unlock readers and schema tests, without the
fenced selection/grant writer or receipt-family integration; APP-2 provides a
private candidate compiler, without admitted populated permanent cosmetics,
current sex, current-world activation or source truth. Its positive authored test
values are synthetic, not a client catalogue. APP-3 has no selected-look change /
observation contract on this floor. Its packet and #1942 ownership remain gates.

### Actual source trace (not proposed interfaces)

| Existing owner | Observed behavior / exact residue |
|---|---|
| `crates/client-assets/src/appearance.rs` | AppearanceIndex::load verifies the catalogue-named DAT through AssetStore; outfit metadata and sprite IDs are already parsed. resolve_outfit draws frame-group0/phase0, base layer only, y=0, no mask/addons/mount. Reuse this parser/index. |
| `apps/client/src/world.rs` | Pins local 15.30 manifest; PLAYER_LOOK_TYPE=128 is resolved for four directions, packed into one static atlas and held as World.player. This is a hardcoded own-player presentation, not a content-declared fallback or committed selection. Builtin mode uses marker cells. |
| `apps/client/src/scene.rs` | Scene::centered_with/rebuild_sprites draws map-under, per-tile glyph, World.player for own tile, map-over, target last. Others have glyphs; there is no per-actor appearance path. The batch assertion budgets one own outfit plus tile glyphs. |
| `apps/client/src/play.rs` | PlayView owns session-thread-delivered view state. rebuild maps WorldSpatialEntity into Targetable and loses Actor detail; only Creature gets its targeting EntityRef. PlayEvent/Pushed coalesce position/entities/markers. Appearance must retain a separate all-actor map, not turn every player into an attack target. |
| `apps/client/src/windows_shell.rs`, `input.rs` | Application owns window/renderer, PlayView/PlayLink and normalized input. Arrow/spell/mouse paths currently route straight to gameplay. No outfit panel is registered. A modal needs focus/hit-test capture before those routes. Cyclopedia is an existing pure-model pattern, not an installed outfit UI toolkit. |
| `crates/client-runtime/src/lib.rs` | Owns Tokio creation/spawn/cancellation/shutdown only; no Scene, actor store or render backend exists there. The historical “client-runtime appearance rendering” path needs an owner decision against this real composition, not a speculative second runtime. |
| `crates/session/src/lib.rs`, protocol world_spatial_entities | Session stores EntityRef(identity,generation), own_identity and sequenced entity observations. Actor detail already carries direction/appearance_ref/health; server world_spatial currently emits PLACEHOLDER_APPEARANCE_REF=0. No committed-reference dictionary or catalogue resolution contract is supplied. appearance_ref is not a DAT look type. |
| condition snapshot candidate / server condition_snapshots | A real candidate codec and independently qualified producer already carry temporary Outfit/Item, content/map/frame, owner time, player revision and visibility/light fields. It declares no allocated numeric routing/capability; SessionEvent has no such consumer. appearance_atomic is a source temporary-condition transaction, not the durable outfit-selection owner. Reuse/qualify this work rather than build another temporary appearance lane. |
| `crates/renderer/src/batch.rs` | AtlasPage, SpriteFrame and QuadBatches already provide uploads, frame-pinned eviction, per-entry placeholder and batch splitting. CellKey identifies sprite/cell only, not colour/palette/content variants. Native assets are pinned 15.30; the normalized Atlas appearance contract explicitly names 15.32. Neither numeric IDs nor pixels may be silently transferred between those source bindings. |

### Proposed files and concrete owner deltas (none written)

| Proposed new file | Responsibility / consumed interface |
|---|---|
| `apps/client/src/outfit_window.rs`, `outfit_window_tests.rs` | Pure window model: authoritative catalogue/current selection, independent editable draft, one pending command and feedback; produces semantic UI actions and preview requests. Shell/widget ownership stays #1942. |
| `apps/client/src/appearance_scene.rs`, `appearance_scene_tests.rs` | PlayView-facing actor presentation store keyed by exact EntityRef and admitted session/content binding; maps accepted observations into common preview/scene render requests. Does not decode wire or validate unlock authority. |
| `crates/client-assets/src/outfit_composition.rs`, `outfit_composition_tests.rs` | Proposed asset-owner extension using existing AppearanceIndex::outfit metadata and SpriteSheets::cell_rgba. Resolves layers/addons/masks from the exact admitted source plus supplied palette; no DAT parser, hardcoded cosmetic facts or second eligibility validator. This path is outside the historical APP-4 file lane: CP must explicitly allocate/coordinate it with the existing asset owner before code. |

Selected routing proposal puts presentation state beside actual PlayView/Scene and
pixel resolution beside the existing asset index. It adds no appearance module to
the Tokio-only client-runtime crate. CP/#1942 must approve this concrete file split
and amend the historical lane before implementation; card-only authority grants
none of these paths.

Required existing-file integrations, each owner-coordinated at CP slot:
`apps/client/src/{lib.rs,windows_shell.rs,input.rs,play.rs,scene.rs,world.rs}` for
registration, widget/focus ownership, semantic command/events and per-actor draws;
`crates/client-assets/src/lib.rs` for composer export; renderer resource-key/upload
and Windows draw-pass owner for composed variants; Session + APP-3 protocol/registry
owner for admitted codecs, command and observations. Exact routing cannot be fixed
by adding only pub mod lines. No numeric ID, capability offer, default hotkey or
registry allocation is proposed here.

### Proposed server/client semantic interfaces (wire shape belongs APP-3)

1. **Own catalogue request/snapshot.** Opening requests the current server-authorized
   outfits, allowed addon subsets and permitted stored mounts, with current selected
   base look, content/palette binding and sufficient session/Character freshness.
   Server resolves sex-specific look references; client never guesses sex. Include
   implicit starters and compatible earned choices, not every indexed DAT outfit
   or every historical unlock. Source RGB entries, names and fallback values come
   from admitted content, never APP-2 test fixtures or inferred Citizen 128 defaults.
2. **Change selection request/result.** One semantic request carries outfit key,
   four palette indices, addon bits and optional stored mount key under APP-3's
   exact current catalogue/revision binding and existing command envelope. Client
   supplies no authorization, AccountId grant or new CharacterRevision authority.
   Result must correlate the submitted command and distinguish terminal rejection,
   committed outcome and unresolved transport outcome; exact error enum, payload
   bounds, replay and numeric codec IDs remain APP-3/registry decisions.
3. **Committed/base and displayed observation.** Own client receives the same
   authoritative actor appearance as other observers. APP-3 must bind opaque
   appearance_ref resolution to exact EntityRef generation, visible actor,
   admitted session/scope/content, definition/palette provenance and sequenced
   state. Keep committed base selection distinct from effective temporary/fallback
   display. A bare reference, an acknowledgement or an authoring key is insufficient.
4. **Temporary condition integration.** Qualify/admit the existing candidate and
   publish its current effective Outfit/Item or explicit clear with ordering
   coherent with base/visibility state. A prepared candidate payload is not a live
   observation. The client must not stitch an old override onto a newer base, use
   owner_time as its own authoritative clock, expire a condition locally or infer
   that an omitted packet means clear. Item transforms use item presentation.

Concrete flow: fenced APP-1 change/grant + admitted APP-2 content/sex → APP-3
committed catalogue/result/actor projection → Session's existing validated domain
store/events → existing PlayLink task → PlayView appearance store/window → one
shared composition path for draft preview and observed actor draws → existing
renderer. Requests travel back through the same chronological session-command
owner; UI code starts no independent runtime/network writer. APP-3 must define
initial join/current snapshot, refresh, revision/correlation and reconnect behavior
before these proposed interface names become code.

### Proposed window states and failure/reconnect semantics

| State | Required transition / visible behavior |
|---|---|
| Closed | No request or mutation until explicitly opened through #1942's assigned input. |
| Loading | Request current catalogue; no invented rows, chosen sex, palette, fallback or enabled Apply. Missing negotiated surface becomes Unavailable. |
| Editing | Copy authoritative base selection into a separate draft only once the complete compatible view is ready. List authorized choices; preview uses draft and the same composer as observed looks. Cancel discards draft and sends no mutation. |
| PreviewUnavailable | Show explicit unresolved preview/diagnostic, preserve actual committed scene state. Proposal: Apply remains disabled until preview is resolvable; this is client readiness, not new server eligibility. #1942 reviews that UX policy. |
| Submitting | Exactly one pending request/correlation; keep scene on committed observations. Disable duplicate Apply; neither preview nor result acknowledgement paints an unobserved committed look. |
| Rejected / NeedsRefresh | Server state remains unchanged. Preserve draft for correction where still compatible; stale catalogue/content/entitlement feedback invalidates Apply and obtains a new authoritative view. Client does not invent server rejection codes. |
| CommittedAwaitingObservation | Wait for the result's declared committed observation/revision using APP-3's settlement/liveness contract. Observation-before-result and duplicate observations converge once; no second cosmetic write or duplicate success action. No copied combat timeout becomes a new appearance resource bound. |
| OutcomeUnknown / Ended | No automatic fresh command or guessed rejection. Reconcile only through APP-3's exact binding rules; session end uses existing return-to-login, drops actor/catalogue/pending view state and drafts, and waits for a fresh authoritative session snapshot. No cross-account/Character draft or old-generation packet reuse. |

A server temporary override changes the effective scene, not durable selection or
the draft; show base-preview versus currently displayed override distinctly. Only
a newer qualified server clear restores base presentation. Compatibility fallback
is server-selected and never persisted by the client. Unknown pixels for an
otherwise server-visible actor retain a factual marker; visibility/invisibility
itself remains server-owned, so marker fallback cannot reveal a hidden actor.

### Renderer/asset contract and resource qualification

Use the existing verified AssetStore/Catalog/AppearanceIndex/SpriteSheets path.
The composer must preserve exact source category, direction/frame selection,
base layer and enabled addon rows; tint masks consume the admitted palette's RGB
values. The normalized Game-owned appearance contract is a semantic reference,
not permission to substitute 15.32 bytes into the pinned 15.30 client. Source-owner
qualification must establish the actual supported source mapping. Unknown look,
mask, palette, dimensions or sprite produces unresolved presentation, never a
nearby ID or a client-authored fallback. Static-only support must be labelled;
current phase0/group0 behavior is not animation or mount support.

Preview and scene share composition but keep preview data out of observed state.
A cache identity must include actual asset/content/palette binding, resolved
outfit/temporary variant, direction/phase, colours and addons; mount visual state
only once admitted. Existing sprite-only CellKey cannot distinguish two colour
variants. Renderer owner must supply an actual typed variant/upload seam; never
mint fake sprite IDs, reuse another actor's tint or add an unbounded atlas/cache.
Reuse AtlasPage frame-pinned residency/placeholder and SpriteFrame/QuadBatches;
adoption into the current World/Windows single-atlas draw path needs #1942 review.

Current protocol max is 256 snapshot entities; World caps 256 draws per tile/own
outfit; renderer caps 81,920 quads per batch and 4096 px atlas dimension. The old Scene
one-own-outfit assertion does not bound 256 distinct actor outfits plus map draws.
Qualify the real multi-batch/page path and current total memory/visibility limits;
this packet allocates no new budget or silently increases an existing one.
Preserve map-under → lifted actor → map-over → target order, floor culling,
position/frame binding and Creature-only attack targeting.

### Qualification matrix (planned, none executed by this card)

| Invariant / actual consumer | Required positive and negative evidence |
|---|---|
| Catalogue authorization / APP-1+3 → window | Real starter without unlock, compatible earned addon/outfit, unavailable Store path; locked/incompatible/unknown-sex/unpopulated catalogue supplies no fabricated choices. APP-1 private reader tests cannot qualify this writer path. |
| Palette/addon/preview / composer+model | Actual palette min/max and max+1, missing RGB/mask/sprite, allowed/forbidden addon bit, two actors with identical sprite IDs but different colour tuples; exact source comparisons, not synthetic data called canonical. |
| Commit and reject / Session → PlayView | Result-before-observation and reverse order converge once; rejection changes neither own nor others' committed look; stale content invalidates draft readiness; two competing real server changes give the typed losing result. |
| Unknown/reconnect / command+admission owner | Ambiguous outcome never auto-resubmits as a new command; fresh session/Character/content cannot consume old catalogue, result, EntityRef generation or pending draft. Real relog restores committed state via actual APP-1 writer, not a fixture reload claim. |
| Reference/binding / domain decoder+store | Missing/foreign appearance_ref mapping, wrong World/scope/content/frame, stale sequence and leave/reappear generation refuse or refresh through APP-3 rules; numeric equality alone never grants a look. |
| Temporary/current display / condition owner | Current qualified Outfit and Item, clear, expiry publication, stale override/base mismatch; no local timer guesses or durable selection rewrite. Server-hidden actors never acquire a factual marker from stale display data. |
| Fallback / actual active-content owner | Server's explicit per-sex fallback with exact colours renders while stored incompatible selection is preserved; unknown local pixels remain unresolved and cannot pick128/default sex/adjacent look. |
| Source/pixel cache / assets+renderer | Verified manifest and supported DAT binding, cross-version mismatch, duplicate directions/unsupported dimensions, same-sprite different palette/content/phase, cache eviction while current-frame cells are pinned. No second parser or generated fake IDs. |
| Capacity/layout / Scene+Windows renderer | Existing max/max+1 entity and entry bounds; crowded view, map+many layered outfits, page exhaustion, multi-batch split and placeholders; factual visibility/selection survives resource fallback. |
| UI input / real #1942 shell | Mouse/keyboard focus, modal arrow/cast/walk capture, cancel, resize/device-loss recovery and real Windows draw/hit-test; no click-through gameplay commands. Linux model tests alone do not prove this UI. |

Before implementation CP must pin #1942's exact head/file ownership, admit APP-3
catalogue/change/base/temporary observation and sequencing contracts, finish real
APP-1 writer/grants and APP-2 active content/sex/palette/fallback inputs, then
allocate the proposed app/assets/renderer paths and their existing-file hooks.
No new runtime, codec, content facts, capability or appearance authority is created
by this packet. All original family acceptance boxes remain open.

### Local documentation qualification

Source/hash manifest, actual governance commands/results, whole-diff self-review
and exact card-only scope check are retained in
`/workspace/oteryn-prep/app-4-evidence/index.md`. No Rust, renderer, Windows,
protocol roundtrip, server writer, relog or runtime appearance qualification is
claimed from this documentation change.


## Native verified-asset compositor successor, 2026-10-16 (AUTHORING, UNQUALIFIED)

This five-path successor starts at actual draft `e075762135c932d8f848076325c86b61a46278af`.
The narrower card-only preparation above is historical; root now leases the four exact new
Rust files and this record. Existing client/server/asset files and shared wiring stay untouched.
Accepted CHARACTER_APPEARANCE §§3, 4.1–4.6 and its cited Game-owned
`OTERYN_ATLAS_ANIMATED_APPEARANCES_V1` outfit grammar bind this bounded increment.

`PinnedOutfitAssets::open` consumes actual native `AssetStore::open_pinned`, `Catalog`,
`AppearanceIndex` and `SpriteSheets`; `compose` returns actual composed pixel bytes and native
`AtlasImage`, with source bottom-right anchoring and displacement. It uses the native public
outfit category, native sheet layout/cell decode/hash checks and existing bounded sheet LRU.
It adds no DAT/BMP/LZMA decoder, asset parser, HSI resolver, sprite cache or fallback look.
The four palette RGB values resolve from a supplied133-entry content palette and indices;
this component grants no catalogue/current-content/unlock authority.

Rendering follows exact opaque mask roles (yellow head, red body, green legs, blue feet),
RGB multiplication by role colour/255, base alpha-over then mask per row, and ascending enabled
addon rows. Cardinal widths2/3, unsupported layer/addon grammar, unknown look and invalid
palette fail explicitly. The pixels use actual15.30 native assets; the15.32 export contract is
semantic evidence, never an alternative asset source. Sprite0 keeps the native empty-cell
meaning. Existing per-entry and native renderer output bounds remain in force.

The component intentionally renders only phase0/z0 of the first frame group retained by the
existing native loader (its documented group0 surface). That loader does not expose fixed
frame-group identity or timing; no other-group, moving, animation, mount or effect support
is qualified. Native group identity/source mapping still needs owning integration review.
`outfit_preview.rs` directly calls this same compositor and returns a fresh resolved/unresolved
frame; failure cannot return a previous look. It stores no selected or observed actor state.

Ten original bodies are authored: four explicitly synthetic pixel/palette/grammar controls and
six native asset/preview bodies. The latter require the actual pinned15.30 manifest/assets,
exercise native decoders and sheet pixels, compare white-control output to the native base
resolver, independently inspect exact mask pixels, vary addons and test explicit failures.
A supplied white/RGB test palette is synthetic DATA, never claimed an admitted game palette.
All ten bodies are UNEXECUTED; no build/runtime/pixel-parity/performance/GPU pass is claimed.

Outside-tree ordinary test-module mounting with actual native dependencies may qualify this
private component after root heavyGO. `client-runtime` needs its native asset/renderer Cargo
dependencies and module export; those shared changes remain CP-owned, as do the client module
registration, scene/window adoption and APP-3/Session wire ownership. No existing `world.rs`,
Scene, Cargo, lib/mod, codec, registry, content or pin file is edited. Full outfit UI, other-actor
appearance observation, server unlock/sex/fallback selection and family acceptance stay open.
No commit, push or remote mutation is authorized for this worker. Guild text remains frozen.


## Actual native component qualification, 2026-10-16 (partial APP-4)

The preceding UNQUALIFIED authoring section records the earlier state. The exact
four Rust files from source checkpoint
`c41e283a62d2a3e788d698cf74379cb97b4f4b2815e9882a04ab5b4922aa0dbb`
now have actual component evidence. Their bytes did not change during qualification.
This append preserves the complete 26,557-byte previously qualified card prefix;
it changes no compiled input or acceptance scope.

Owner-authorized isolated Ubuntu-24.04 WSL/Linux x86_64 execution used pinned
Rust/Cargo/Clippy 1.94.0, jobs4, debug0 and incremental0. Actual literal native
client-assets, renderer and client-runtime packages/manifests/features/lock were
used. The outside-tree runtime module mount adds the two actual built native
externs; preview compiles its verbatim source as an ordinary component test root.
The original ten bodies are unchanged: four synthetic semantic controls and six
native asset/preview cases, with the unchanged native runtime baseline as case11.
Exact test discovery and execution passed runtime9 + preview2, with zero failed,
ignored or filtered cases. Production component Clippy with `-D warnings` passed;
preview component strict Clippy and runtime unit configured Clippy also passed.
The capture-only Cargo101 is the expected recorded compiler-interception sentinel,
not a test or compilation failure.

All675 exact ownHEAD native input records (5,935,066 bytes), the four authored Rust
files and five canonical 15.30 fixtures are hash-bound. The physical inventory of
636 Rust files is labelled physical input inventory, not proof all were consumed.
Captured native Cargo roots/argv/environment, all48 actual Cargo artifact records,
standalone metadata/rlib/test binaries, exact test lists, source/fixture and direct
extern before/after hashes are retained. Unit/preview metadata files were actually
zero bytes; their compile exit/logs and executed native test binaries provide the
qualification evidence. Compiler scratch stayed inside the isolated task root.
The 8GB whole-root cap, physical C-drive 20GB floor, memory and new-OOM guards passed;
no measured peak-memory or performance qualification is inferred.

Two earlier setup failures remain recorded separately: original649 discovery
omitted26 real manifest-declared test roots; the next offline attempt lacked a
locked sibling-workspace rustls index version. Neither reached product compilation
or tests. Exact ownHEAD input restoration and isolated locked registry resolution
repaired setup without changing native manifests, lock, product code or test bodies.
Full online and offline dependency metadata then passed; all297 cached archive
checksums match the unchanged lock. The successful third attempt ended exit0.

Compact actual evidence is retained at
`/workspace/oteryn-prep/reviews/app4-pc/actual-native-3/`; its manifest SHA256 is
`65fbfd3be4e4a8de476813a12c2562809a02db8d5f45abcb4ce5c4fadb42bc6c`.
Full raw build JSON, source/cache evidence and binaries remain in the isolated
owner-computer evidence directory, with hashes in that manifest. Historical setup
failures and all earlier frozen evidence are preserved.

This qualifies the limited static first-loader-group phase0/z0 compositor and
preview component with real pinned assets. It does not qualify full apps/client
Cargo/application integration, module/dependency registration, scene/other-actor
adoption, outfit UI, animation, GPU/performance, or admitted current-content,
palette, sex, unlock and server-selected fallback authority. Those owning CP/client
and APP-1/2/3 integration gates and family acceptance boxes remain open. This worker
performs no commit, push, publication, ready transition, merge or queue operation.

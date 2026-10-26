# CLIENT-STRUCTURE-TARGET-1 — native-client target structure

**Status:** design proposal; no code move is authorized by this document.
**Evidence baseline:** `main` at `3582f6621b8ab8a1d4e35eb68194d443d54e3ae4`; draft PR #1942 branch `feat/client-completion-20261009` at `fdc20be476e6d38913d254209ccfa710743d6c59` (fetched 2026-10-26).
**Scope:** native client and its client-facing crates. Server code, `.github/workflows/**`, `.github/repository-policy.json`, `tools/repository/**`, `vendor/**`, and Platform device-session activation are excluded. No third-party dependency is proposed.

## 1. Findings and design rule

The current five hot sources total 178,627 bytes: `play.rs` 53,450; `lib.rs` 45,972; `world.rs` 33,572; `input.rs` 26,175; and `cyclopedia.rs` 19,458. Draft #1942 does not relieve the hotspot: its versions of `play.rs` and `lib.rs` grow to 135,325 and 61,067 bytes, while new `game_ui.rs` is 207,783 bytes. It also introduces new central catalogues and dispatch matches in `panel_catalog.rs`, `panel_icons.rs`, `panel_views.rs`, `settings_catalog.rs`, `preferences_pages.rs`, `hotkeys.rs`, `client_panels.rs`, and `game_ui.rs`. This is the same merge topology the server later paid to remove.

**Rule:** adding one mechanic or panel must normally add or change files only below one feature directory and add one self-contained registration fragment. Aggregators discover, validate, sort, and generate code; authors never edit an ordered master list or an exhaustive cross-feature match.

### Inventory conventions

The appendices inventory every blob below `apps/client` on each evidence head. Sizes are Git blob bytes, not filesystem allocation. “Central ownership” means an ordered catalogue, registration list, or cross-feature dispatch match whose extension would make unrelated feature lanes edit the same file. Ordinary local matches (error rendering, state transitions, parsing) are marked “none.” Draft files unchanged from main are repeated so each head is complete.

## 2. Current crate dependency graph

Direct internal edges from the current manifests are:

```text
apps/client (thin in name only today)
 ├─ client-assets
 ├─ client-runtime ── diagnostics, foundation
 ├─ input-actions
 ├─ input-platform ── input-actions
 ├─ platform-client ── diagnostics, foundation, platform-contracts
 ├─ renderer ── foundation
 ├─ session ── protocol-oteryn
 ├─ session-tcp ── session ── protocol-oteryn
 └─ diagnostics, foundation, identity, platform-contracts, world-bundle

client-domain ── foundation       (not reached by apps/client today)
client-simulation ── client-domain (not reached by apps/client today)
client-assets ── no internal crate
input-actions ── no internal crate
protocol-oteryn ── no internal crate
```

This graph exposes two problems: the nominal app directly owns session, transport, rendering, input, assets, Platform admission, world loading, and UI; and the intended pure `client-domain`/`client-simulation` seam is disconnected from the runnable client.

## 3. Lessons from the server restructurings

| Work | What forced it | Client-equivalent risk and required response |
|---|---|---|
| CAPS-SPLIT-1 (#2361) | Hand-maintained runtime capability facts and offered/gated tables made every capability lane edit shared lists. One JSON fragment per capability plus `build.rs` validation/generated tables removed the write hotspot. | Panel, handler, action, setting, page, and asset metadata must be one fragment per feature; generated views may be globally sorted, but the source of truth must not be a shared Rust array. |
| CAPS-SPLIT-2 | Even after runtime facts split, a single ordered golden capability snapshot remained a conflict surface. One golden file per capability retained independent verification. | Do not create a single golden UI catalogue. Put expected registration/codec/settings snapshots beside each feature, then have tests scan and compare set equality. |
| CAPS-SPLIT-3 / DISPATCH-REG-1 (#2384) | `connection.rs` had grown beyond 8,700 lines; 26 command bodies across 25 mechanic modules still initially required ordered predicates in the connection. Per-mechanic registration fragments were needed to remove that last shared router edit. | `play.rs`/`game_ui.rs` must not dispatch every `ServerEvent`, action, panel, or command. Generate handler tables from feature fragments; a command addition changes its feature and fragment, not the shell. |
| REGISTRY-SPLIT-1 | Protocol and resource-limit registries (25 capabilities, 19 commands, 21 state domains, and 591 limits at the time) were ordered monoliths. Directory-scanned fragments preserved validation and deterministic aggregation without shared edits. | Use directory scans for client registration sources. Validate unique IDs, known server mechanic/capability IDs, schema, paths, and deterministic order. Never use a “temporary” master list. |
| DISPATCH-WIRING-1 (#2314) | Gameplay transport accumulated fourteen independently owned mechanic states and duplicated giant test-owner constructors. `ChannelSessionStates` and a composed test builder isolated state and construction while retaining lifecycle behavior. | Put feature state in each feature module; compose `ClientState` through generated registrations/typed slots. Central shell tests use a builder and explicit overrides rather than constructing every feature field. |
| SERVE-PHASE-1 (#2380) | One 2,183-line connection method mixed setup, snapshots, poll/wake, refresh handling, frame handling, execution, and result encoding. It became a 48-line orchestrator with twelve phase files (all under 700 lines). | Split the client loop into named phases (`admit`, `receive`, `reduce`, `render_snapshot`, `send`, `shutdown`). The app shell orchestrates; it does not contain protocol decoding or feature behavior. |
| DUR-ONEOF-1 (#2383) | Twelve operation additions and three fee-cause additions repeatedly replaced the same prost tag line and inserted enum variants at the same locations. One-tag-per-line marked blocks plus per-variant fixtures made additions insert-only and wire parity testable. | Generated client protocol dispatch must use stable numeric IDs and one-entry fragments/fixtures. Avoid handwritten exhaustive wire enums/matches in app code; keep byte fixtures per command/feature. |
| CAST-SPLIT-1 (#2381) | A 1,788-line cast file received 26 commits across 18 tasks; state, gates, preparation, and tests collided. It was separated into state, gate, and prepare while retaining the public facade. | Every client feature uses fixed concern files. Protocol mapping, state reduction, input, view, settings, and tests cannot accumulate in one mechanic file. |

The lesson is not merely “smaller files.” The server only eliminated contention when **ownership and registration sources** became disjoint. A small central match is still a future hot file.

Evidence for this comparison is the repository task record for each named work item:
`docs/agents/tasks/archive/CAPS-SPLIT-1.md`, `docs/agents/tasks/CAPS-SPLIT-2.md`,
`docs/agents/tasks/CAPS-SPLIT-3.md`, `docs/agents/tasks/archive/REGISTRY-SPLIT-1.md`,
`docs/agents/tasks/archive/DISPATCH-WIRING-1.md`, `docs/agents/tasks/SERVE-PHASE-1.md`,
`docs/agents/tasks/DUR-ONEOF-1.md`, and `docs/agents/tasks/CAST-SPLIT-1.md`, together with
their recorded pull requests and the merged follow-on commits identified above. The records distinguish
measured conflict evidence from inferred risk; this design carries forward only the structural lessons.

## 4. Target architecture

### 4.1 Crate layers and allowed edges

```text
Layer 0  protocol-oteryn, foundation, input-actions
             ↑
Layer 1  client-domain (pure state/types/reducers; no IO, windowing, GPU, async runtime)
             ↑
Layer 2  client-simulation (deterministic prediction/replay over domain)
             ↑
Layer 3  client-runtime (feature orchestration, generated protocol adapters)
         session ← session-tcp (transport adapter)
         platform-client (pre-game admission only)
             ↑
Layer 4  renderer (render data + GPU backend), input-platform, client-assets
             ↑
Layer 5  client-ui (native panels/views; new workspace crate, using existing deps only)
             ↑
Layer 6  apps/client (process, OS window, composition root, shutdown only)
```

Allowed directions are upward only as drawn from consumers to lower layers:

* `client-domain` may depend only on `foundation`, `protocol-oteryn` **generated value types only if unavoidable**, and `input-actions`; recommendation: keep protocol types out and map them in runtime.
* `client-simulation` depends on `client-domain` only (plus foundation test utilities).
* `session` owns protocol framing; `session-tcp` implements transport and may depend on `session`. Neither imports UI, renderer, assets, or app.
* `client-runtime` may depend on domain, simulation, session, protocol, diagnostics, and foundation. It exposes UI-neutral snapshots/intents.
* `renderer` consumes domain/render snapshots, never UI, runtime/session, Platform, or app. `client-assets` is similarly IO-format focused and cannot import feature UI.
* `client-ui` may depend on domain/runtime public facades, renderer abstractions, input-actions, and assets; it never opens sockets or calls Platform.
* `apps/client` may depend on all composition components but contains no mechanic state, protocol switch, panel implementation, or settings catalogue.

**Enforcement:** extend the existing `oteryn-architecture-check` package (not repository-policy or workflow files) with `client_layering.rs`. Parse `cargo metadata --no-deps`, reject forbidden direct internal edges and cycles, and scan Layer 0/1 sources for banned IO/runtime/window/GPU imports (`std::net`, `std::fs`, `tokio`, `reqwest`, `wgpu`, OS APIs). A compile-fail fixture verifies each forbidden edge class. Run as `cargo test -p oteryn-architecture-check --test client_layering` and as part of the existing architecture-check invocation.

### 4.2 Feature unit

Canonical location, using the server capability/mechanic slug and numeric ID:

```text
crates/client-ui/src/features/<capability-id>-<mechanic>/
  mod.rs                 # private facade/re-exports; no registry edits
  state.rs               # UI-local projection; domain truth stays client-domain
  protocol.rs            # generated-binding ↔ domain intent/event mapping
  view.rs                # panel/HUD rendering
  input.rs               # actions/key defaults and intent construction
  settings.rs            # typed settings and migration hooks
  tests/
    state.rs
    protocol.rs
    view.rs
    input.rs
    settings.rs
  registrations/
    handlers.json
    panels.json
    keybindings.json
    settings-pages.json
    cyclopedia-pages.json
    assets.json
  fixtures/              # per-command bytes and per-feature registration golden
```

Each fixed file is optional only when its registration explicitly says `not_applicable` with a reason; do not create empty placeholders. A feature may split a concern into a same-named directory once it nears the file limit. Cross-cutting shell features use reserved non-server IDs such as `local-login`, `local-window`, and `local-preferences`; gameplay features must use the exact protocol capability/mechanic ID. Shared visual primitives live in `client-ui/src/components`, not in a pseudo-feature catalogue.

Domain state follows the same slug under `crates/client-domain/src/features/<id>-<mechanic>/`; runtime adapters follow it under `crates/client-runtime/src/features/...`. This repeats the ownership boundary across layers without allowing UI state to become domain truth.

### 4.3 Registrations: additive sources, generated consumers

A client build helper scans every `features/*/registrations/*.json`, with no manifest of filenames. It validates:

1. directory ID equals the fragment's mechanic/capability ID and exists in protocol-oteryn's generated capability facts;
2. registration IDs and command IDs are unique; referenced Rust entry symbols and asset paths exist;
3. handler direction, required capability, panel/page parent, key chord, setting key/type/version, and asset digest are valid;
4. every protocol command declared client-consumable has exactly one handler or an explicit unsupported fragment;
5. deterministic output is sorted by numeric capability ID and stable registration ID, never filesystem enumeration order.

It emits private code to `OUT_DIR`: handler lookup, panel factory lookup, keybinding defaults, settings/page descriptors, cyclopedia routes, and asset lookup. Runtime lookup should use generated numeric tables or a generated perfect `match`; the important property is that the match is **output**, never hand-edited source. Generated output is not committed.

The six registration kinds are independently fragmentable. A feature that owns three panels has three panel fragments, not an array inside one feature file. Set-equality tests compare discovered fragments, generated entries, and per-feature goldens. Duplicate and missing-entry negative fixtures run in the architecture-check package.

### 4.4 Protocol binding flow

`protocol-oteryn` and the command-type fragments (currently
`docs/contracts/protocol-oteryn/v1/registry/command_types/<id>-<slug>.json`, the repository's
`docs/contracts/command_types` concept) remain authoritative. Extend the existing build generation to
emit a client-facing descriptor for each command: numeric ID, direction, capability, payload/result
binding, and stability. `client-runtime` consumes those descriptors and generates typed handler
registration glue. Handwritten mapping remains only in the owning feature's `protocol.rs` when wire
and domain shapes differ.

Definition of done for a new server command: the client patch changes only:

1. `features/<same-id>-<mechanic>/protocol.rs` (and its feature tests/fixtures), and
2. one `registrations/handlers.json` fragment.

The server contract/protocol work is a separate authoritative patch. No edit to app `lib.rs`, `play.rs`, a handler enum, global match, global golden, or ordered command list is permitted. A repository test synthesizes a fake command fragment and proves generation succeeds without modifying existing files.

### 4.5 File-size and test guards

| Class | Warning | Hard limit | Escape hatch |
|---|---:|---:|---|
| Production Rust | 400 lines | 600 lines | owner-approved architecture note with expiry and split issue |
| Test Rust | 700 lines | 1,000 lines | split by behavior before merge; generated fixtures excluded |
| `mod.rs` / app shell | 200 lines | 300 lines | none for cross-feature registration |
| Registration fragment | 100 lines | 200 lines | split one entry per file |
| Generated Rust | n/a | n/a | generator source is still limited; generated file measured/reported |

`oteryn-architecture-check/tests/client_structure.rs` walks the client-owned paths, counts physical lines, rejects central registration patterns in non-generated sources, verifies required feature layout, and reports the largest files. Grandfather the five current hot files and #1942 extraction inputs with a checked-in ratchet containing exact byte/line ceilings: they may shrink or disappear, never grow. Remove each exception in the PR that completes its split.

Test layout:

```text
feature/tests/*                 unit reducer/mapping/view/input/settings tests
feature/fixtures/*              per-feature protocol bytes and catalogue goldens
crates/client-runtime/tests/    multi-feature handler/ordering/session integration
crates/client-ui/tests/         generated catalogue set equality and headless UI smoke
apps/client/tests/              composition/startup/shutdown only
architecture-check/tests/       dependency, layout, size, registration negative cases
```

### 4.6 Ownership map

| Lane | Exclusive paths | Must not edit |
|---|---|---|
| One gameplay mechanic | `client-{domain,runtime,ui}/src/features/<id>-<slug>/**` | another feature, app shell, generator |
| One local panel | `client-ui/src/features/local-<panel>/**` | panel catalogue or panel dispatcher (none exists) |
| Preferences | owning feature's `settings.rs` + its settings-page fragment | central settings list/page match |
| Graphics | `client-ui/.../local-graphics/**`, renderer public seam if separately owned | session/runtime protocol |
| Chat | matching chat capability directories in domain/runtime/UI | global game UI/play loop |
| Protocol generation | `protocol-oteryn` generator and descriptor schema | mechanic mapping bodies |
| Registration infrastructure | build helper/schema/tests only | feature fragments except fixtures |
| Renderer | `crates/renderer/**` | UI catalogue/runtime/session |
| Session/transport | `crates/session/**`, `crates/session-tcp/**` | UI/render/domain |
| App shell | `apps/client/src/{main,lib,composition,window,shutdown}.rs` | feature behavior |

If a PR needs two rows, split it or state a temporary two-owner sequence; never let parallel lanes share the same head. Generated outputs are regenerated by the infrastructure owner or CI, not merged by mechanic lanes.

## 5. Behavior-preserving migration plan

Every step below is a small PR: move/copy tests first, preserve public re-exports, compare test inventories before/after, and make no visual/protocol behavior change.

1. **Install guards and empty composition seams.** Add dependency/layout/size tests, fragment schemas/scanner, and ratchet current hot files. Create `client-ui` with no feature behavior. This is the only infrastructure-first PR.
2. **Split `world.rs`.** Move immutable asset/index decoding to `client-assets`; world coordinates/cells to `client-domain`; deterministic viewport/scene projection to `client-simulation`; renderer upload/draw preparation to `renderer`; tutorial-window tests to the matching tutorial feature. Keep `world` as a deprecated facade, then delete it when callers move.
3. **Split `input.rs`.** Move normalized action vocabulary to `input-actions`, platform event translation to `input-platform`, movement timing/state to domain/simulation, and click/key intent mapping into each feature `input.rs`. Replace the default binding list with per-feature keybinding fragments.
4. **Split `cyclopedia.rs`.** Establish one feature directory per matching server capability (bestiary/charms, map, and later pages). Move state/reducers, protocol mapping, page view, and settings independently. Cyclopedia navigation is generated from page fragments, not a page enum match.
5. **Split `play.rs`.** First extract the mailbox/session-loop phases without behavior change; then move event reducers and command constructors mechanic by mechanic. Generate handler dispatch. End with a sub-300-line runtime orchestrator; the app must not retain a compatibility mega-facade.
6. **Split `lib.rs`.** Move release parsing to a build-info module, Platform admission to `platform-client` facade, session connection/retry into client-runtime/session, and test harness construction into test support. `apps/client/lib.rs` ends as composition exports only, under 200 lines.
7. **Delete ratchets and facades.** Only after downstream call sites and test inventories match; add a negative architecture fixture showing a new central catalogue fails.

Recommended move order deliberately handles `world` and `input` before `play`, because `play` currently composes both; `cyclopedia` supplies the first real proof of feature-local registration; `lib` is last so public compatibility re-exports can keep intermediate PRs small.

### 5.1 Decomposing draft PR #1942

Do not merge #1942 as one patch and do not port its deletion of newer protocol work. Rebase concepts, not the stale branch. Land in this order:

1. **Independent visual primitives/assets:** provenance/licence docs, digest fragments, login backdrop, cursor, layout primitives, icon decoding. Split assets one fragment per consumer; no global `PANELS` icon list.
2. **Panels, independently:** one PR per local panel or server mechanic panel, each in its target feature directory with panel fragment and headless tests. Convert `panel_catalog`, `panel_icons`, `panel_views`, and `client_panels` into generated infrastructure and feature views rather than landing them as four new centres.
3. **Preferences storage/export:** settings envelope, migration, device store, remembered-device UI, import/export, and audit tooling. Then one PR per settings page/feature. Do not land the 80 KB catalogue or 80 KB page dispatcher; express each entry/page as fragments and feature settings.
4. **Graphics settings:** renderer-backed toggles, map draw/view, minimap, streamed scene, actor HUD, then graphics preferences. Each renderer seam lands before its UI consumer and carries its own measurement/smoke test.
5. **Chat:** chat output component first; chat domain state and capability-specific handlers next; chat input/hotkeys/view last. Avoid putting channel/action routing back into `game_ui.rs`.
6. **Action bar and hotkeys:** action state, view, and settings as separate feature PRs after keybinding fragment generation exists. Convert the draft's 49/38-entry default arrays into per-action fragments.
7. **Login and shell composition:** login screen/chrome/window shell only after the independent pieces compile behind the thin composition root.
8. **Protocol-dependent wiring:** streamed map/session events, combat, cyclopedia, panels with live server data, chat intents/receipts, and action commands last, one capability at a time, against current `protocol-oteryn` generated descriptors.

The draft's giant `game_ui.rs`, expanded `play.rs`, and settings/preferences files are extraction sources, not target files. Preserve their tests by relocating them with the owning feature; do not accept their size as a temporary new baseline.

**Authentication (owner decision 26):** real authentication is not active yet. Platform prepares its login/device-session contract in parallel; the client uses a stub adapter of the same shape and wires the real one at step 7. Platform keeps device-session authority, and no draft activation path from #1942 lands.

## 6. Owner decisions (accepted 2026-10-26)

The owner accepted this design with the changes below (owner decision 26 in
`OTERYN_GAME_NIGHT_W1_OWNER_DECISIONS_2026-10-25.md`). Where a row changes an earlier section, this
table wins.

| Decision | Accepted rule |
|---|---|
| Create `client-ui` crate or keep UI below the app? | **Create it.** Thin app shell, enforced by `oteryn-architecture-check`. |
| Can `client-domain` import protocol types? | **No; map in runtime.** Only generated stable identifiers, after an ADR. |
| Registration format | **JSON fragments + generated, uncommitted Rust.** |
| Feature directory naming | **Changed:** the zero-padded server capability/mechanic number plus the canonical slug, with **no version suffix** (`0057-cyclopedia-map`, not `0057-cyclopedia-map-v1`). Protocol versions live in the contract, so a new version never renames or moves a directory. `local-*` only for non-server UI. |
| Handler dispatch strategy | **Generated numeric lookup/match, private and uncommitted.** |
| Settings ownership and unknown keys | **Feature-owned typed settings; unknown keys round-trip in the envelope.** |
| Keybinding conflicts | **Changed:** the build fails only when two *default* keybinding fragments collide (a developer error), with the small explicit precedence field for context-specific overlaps. A conflict the *player* creates by rebinding never blocks saving: the settings page shows it at runtime and lets the player resolve it, as in Global. |
| Size thresholds | **400 warning / 600 production hard limit and the ratchet, immediately.** |
| Commit generated registration output? | **No.** Deterministic regeneration and per-feature goldens. |
| Draft PR #1942 | **Changed:** #1942 is not owned by this work and is never modified. Its tests and working behaviors are ported into small PRs in the §5.1 order; its structure and stale protocol deletions are not. |
| Authentication | **Changed:** real authentication is not active. Platform prepares the login/device-session contract in parallel with the structural work. Until it lands the client uses a local stub adapter with the same shape, and the real adapter is wired at §5.1 step 7 (login and shell composition), replacing only the adapter. Platform keeps device-session authority. |
| Mechanic coverage | **New:** every server mechanic must work in the client. An `unsupported` handler fragment (§4.3 rule 4) is only a tracked, temporary state: the architecture check reports the list, and the client is complete only when it is empty and each mechanic has a headless test against the generated descriptors. |

## Appendix A — complete `main` client-file inventory
| File | Bytes | Responsibility | Central ownership |
|---|---:|---|---|
| `apps/client/Cargo.toml` | 2,327 | Client package manifest and direct dependency root. | none |
| `apps/client/REFERENCE-PREFERENCE-KEYS.json` | 6,703 | Reference preference data or provenance metadata. | none |
| `apps/client/SETTINGS.md` | 13,055 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/REFERENCE-AUDIT.md` | 14,880 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/src/play.rs` | 53,450 | Session loop, play state, spatial/combat/chat events, commands, and tests. | cross-mechanic input/event/session dispatch |
| `apps/client/src/main.rs` | 2,218 | Process entry and Windows/native startup. | none |
| `apps/client/src/lib.rs` | 45,972 | Build identity, bootstrap, Platform admission, connection retry, and library tests. | module/export surface; admission-class/retry routing |
| `apps/client/src/scene.rs` | 14,929 | Tile scene selection and scene model. | none |
| `apps/client/src/spell.rs` | 9,080 | Spell shortcuts/action-bar state and disposition UI data. | none |
| `apps/client/src/input.rs` | 26,175 | Normalized movement/click input, timing, and default bindings. | default bindings/action routing |
| `apps/client/src/cyclopedia.rs` | 19,458 | Bestiary/charm state projection and tests. | charm-kind/page behavior matches |
| `apps/client/src/windows_shell.rs` | 15,331 | Native window/event/render shell. | none |
| `apps/client/src/combat_input.rs` | 4,728 | Combat command pump and disposition text. | none |
| `apps/client/src/world.rs` | 33,572 | Pinned world assets, placement/sector decode, atlas creation, and tests. | none |
| `apps/client/src/win_mutex.rs` | 7,135 | Windows single-instance mutex. | none |
| `apps/client/src/bin/oteryn-launcher.rs` | 10,672 | Windows release launcher and installed-release selection. | none |
| `apps/client/src/world/tutorial_window_tests.rs` | 508 | World tutorial-window tests. | none |
| `apps/client/installer/client.env.dev` | 444 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/ci-installer.ps1` | 17,063 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/oteryn-client.iss` | 23,401 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/sign.ps1` | 625 | Windows installer, signing, or environment support. | none |

## Appendix B — complete draft PR #1942 client-file inventory

| File | Bytes | Responsibility | Central ownership |
|---|---:|---|---|
| `apps/client/Cargo.toml` | 3,174 | Client package manifest and direct dependency root. | none |
| `apps/client/FONT-PROVENANCE.json` | 1,856 | Reference preference data or provenance metadata. | none |
| `apps/client/REFERENCE-AUDIT.md` | 17,862 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/REFERENCE-PREFERENCE-KEYS.json` | 6,703 | Reference preference data or provenance metadata. | none |
| `apps/client/SETTINGS.md` | 18,672 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/THIRD-PARTY-FONTS.txt` | 20,545 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-EXECUTION-PLAN.md` | 23,182 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-FIDELITY.md` | 89,792 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-LOCAL-OPTION-INDEX.md` | 30,621 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-COVERAGE.md` | 11,959 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-EXTRACTED.md` | 14,847 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-MODULES.md` | 15,172 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-OBSERVATIONS.md` | 7,948 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-OTCLIENT.md` | 8,059 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/UI-REFERENCE-SETTINGS.md` | 7,993 | Client reference, settings, provenance, or design documentation. | none |
| `apps/client/assets/Cinzel-OFL.txt` | 4,382 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/LICENSE.txt` | 880 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/README.md` | 478 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/anvil.svg` | 471 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/book-open.svg` | 378 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/chart-no-axes-column.svg` | 334 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/chart-no-axes-combined.svg` | 413 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/circle-help.svg` | 322 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/clipboard-list.svg` | 452 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/columns-2.svg` | 285 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/compass.svg` | 351 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/contact-round.svg` | 382 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/crosshair.svg` | 412 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/crown.svg` | 464 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/gem.svg` | 308 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/gift.svg` | 434 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/globe.svg` | 331 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/library.svg` | 304 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/list-checks.svg` | 336 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/manifest.json` | 4,425 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/messages-square.svg` | 348 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/orbit.svg` | 412 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/panels-top-left.svg` | 307 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/panels.webp` | 9,060 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/paw-print.svg` | 419 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/scan-eye.svg` | 508 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/scroll-text.svg` | 419 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/shield-alert.svg` | 435 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/skull.svg` | 423 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/sword.svg` | 393 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/swords.svg` | 573 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/trophy.svg` | 514 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/icons/users-round.svg` | 336 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/login-background.webp` | 361,200 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/assets/login-brand.ttf` | 125,468 | Shipped bitmap/font/icon asset or asset licence/manifest. | none |
| `apps/client/installer/ci-installer.ps1` | 17,373 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/client.env.dev` | 444 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/oteryn-client.iss` | 23,490 | Windows installer, signing, or environment support. | none |
| `apps/client/installer/sign.ps1` | 625 | Windows installer, signing, or environment support. | none |
| `apps/client/src/action_bar.rs` | 59,207 | Action-bar model, slots, persistence, commands, and tests. | none |
| `apps/client/src/action_bar_ui.rs` | 61,382 | Action-bar rendering, drag/drop, tooltips, and UI tests. | none |
| `apps/client/src/actor_hud.rs` | 15,728 | Actor HUD bars and HUD preference projection. | none |
| `apps/client/src/bin/oteryn-launcher.rs` | 10,672 | Windows release launcher and installed-release selection. | none |
| `apps/client/src/chat_output.rs` | 6,766 | Selectable/copyable chat output widget. | none |
| `apps/client/src/client_chrome.rs` | 8,074 | Client frame/chrome layout primitives. | none |
| `apps/client/src/client_panels.rs` | 47,571 | Panel window manager and generic field renderer. | panel destination/field/column matches |
| `apps/client/src/combat_input.rs` | 5,479 | Combat command pump and disposition text. | none |
| `apps/client/src/cyclopedia.rs` | 19,458 | Bestiary/charm state projection and tests. | charm-kind/page matches |
| `apps/client/src/device_store.rs` | 14,223 | Device-local persistence backend. | none |
| `apps/client/src/game_ui.rs` | 207,783 | Whole in-game UI composition, chat/action routing, equipment, panels, and tests. | global UI/action/chat/panel matches |
| `apps/client/src/hotkeys.rs` | 70,126 | Profiles, defaults, validation, resolution, migration, and tests. | 49/38-entry default action registries and action validation |
| `apps/client/src/input.rs` | 36,522 | Normalized movement/click input, timing, and default bindings. | default binding list/action routing |
| `apps/client/src/input_map_tests.rs` | 5,884 | Map click/input mapping tests. | none |
| `apps/client/src/layout.rs` | 13,248 | Window/panel layout model and persistence. | none |
| `apps/client/src/lib.rs` | 61,067 | Build identity, bootstrap, Platform admission, connection retry, and library tests. | module/export surface and admission/retry mapping |
| `apps/client/src/login_backdrop.rs` | 4,270 | Procedural/fallback login backdrop. | none |
| `apps/client/src/login_screen.rs` | 45,034 | Login UI and async bootstrap presentation. | none |
| `apps/client/src/main.rs` | 4,634 | Process entry and Windows/native startup. | startup-mode dispatch |
| `apps/client/src/map_draw.rs` | 14,667 | Map item resolution and draw ordering. | none |
| `apps/client/src/map_draw/tests.rs` | 12,452 | Tests for containing module. | none |
| `apps/client/src/map_view.rs` | 156 | Map-view facade. | none |
| `apps/client/src/minimap.rs` | 10,724 | Minimap state and raster generation. | none |
| `apps/client/src/mouse_cursor.rs` | 4,926 | Cursor presentation and tests. | none |
| `apps/client/src/options_export.rs` | 21,693 | Options archive export/import bounds. | none |
| `apps/client/src/panel_catalog.rs` | 33,616 | Panel definitions, tabs, fields, shortcuts, and lookup. | PANELS array and `(panel, tab)` field match |
| `apps/client/src/panel_icons.rs` | 7,588 | Panel/equipment icon catalogue and rendering. | PANELS icon array and equipment icon match |
| `apps/client/src/panel_views.rs` | 88,434 | Cross-panel view dispatcher and all panel bodies. | panel/page dispatcher matches |
| `apps/client/src/play.rs` | 135,325 | Session loop, play state, spatial/combat/chat events, commands, and tests. | cross-mechanic command/event dispatch |
| `apps/client/src/preferences_browser.rs` | 136,694 | Preferences navigation/browser UI. | none |
| `apps/client/src/preferences_display_pages.rs` | 34,079 | Graphics/display preference pages. | none |
| `apps/client/src/preferences_hotkey_pages.rs` | 118,651 | Hotkey preference pages. | none |
| `apps/client/src/preferences_hud_pages.rs` | 67,056 | HUD preference pages. | none |
| `apps/client/src/preferences_overview.rs` | 10,390 | Preference overview page. | none |
| `apps/client/src/preferences_pages.rs` | 80,720 | Cross-section settings page dispatcher and common pages. | settings-section page match |
| `apps/client/src/preferences_slider.rs` | 12,208 | Reusable preference slider. | none |
| `apps/client/src/reference_dialogs.rs` | 34,732 | Reference-compatible local dialogs. | none |
| `apps/client/src/remembered_device.rs` | 28,010 | Remembered-device state/UI. | none |
| `apps/client/src/scene.rs` | 16,977 | Tile scene selection and scene model. | none |
| `apps/client/src/settings.rs` | 69,803 | Settings envelope, defaults, migration, typed access. | none |
| `apps/client/src/settings_audit.rs` | 66,028 | Settings audit CLI/reporting. | none |
| `apps/client/src/settings_catalog.rs` | 80,291 | All settings metadata, sections, choices, consumer lookup. | 20 section IDs, SETTINGS_SECTIONS array, runtime-consumer match |
| `apps/client/src/settings_semantic_contract.rs` | 22,136 | Settings semantic validation contract. | none |
| `apps/client/src/settings_ui.rs` | 82,521 | Settings editor state/UI and tests. | settings section/page assembly |
| `apps/client/src/spell.rs` | 9,080 | Spell shortcuts/action-bar state and disposition UI data. | none |
| `apps/client/src/streamed_scene.rs` | 4,183 | Streamed scene state. | none |
| `apps/client/src/streamed_scene/tests.rs` | 2,876 | Tests for containing module. | none |
| `apps/client/src/win_mutex.rs` | 7,135 | Windows single-instance mutex. | none |
| `apps/client/src/windows_shell.rs` | 64,150 | Native window/event/render shell. | global native event dispatch |
| `apps/client/src/world.rs` | 32,952 | Pinned world assets, placement/sector decode, atlas creation, and tests. | none |

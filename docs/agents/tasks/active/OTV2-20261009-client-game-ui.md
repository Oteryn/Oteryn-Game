# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

## Authority and publication state

Draft PR #1942 remains in AUTHORING. Root is the sole branch publisher. Do not
freeze or qualify a candidate until all owned changes are committed, merged
normally with current `origin/main`, validated and reviewed as one exact SHA.
Before publication, fresh-read the remote branch head and use the guarded
publication-integrity helper with that exact predecessor and an external recovery
bundle. Never force, reset, rebase, amend or publish reference assets/private
account data. No production deployment, account mutation or database reset is in
scope.

The owner requires the Tibia Global client to be matched 1:1 and explicitly does
not count a control that merely serializes a value as implemented behavior.
Reference facts are retained in `apps/client/UI-REFERENCE-COVERAGE.md`,
`UI-FIDELITY.md`, `UI-REFERENCE-OBSERVATIONS.md` and
`UI-REFERENCE-SETTINGS.md`. Original captures remain private compatibility
evidence. Missing engine/server mechanisms must remain visibly unavailable and
must not be claimed as complete.

## Current authoring batch — 2026-10-11

- Production Settings navigation is restricted to the exact 20 observed pages
  and order. Quick/Account/Transparency shell entries, seven catalogue-only
  pages and Oteryn-only controls embedded in reference pages are hidden while old
  values remain loadable.
- Compiled semantic coverage routes those same 20 production pages, checks a
  page-specific marker set plus targeted labels/choices/dependencies, outer-page
  scroll reachability, Help action order and render-time non-mutation. It is not
  yet an exhaustive per-row snapshot. A Windows-only `--settings-audit` mode
  captures every page through the real DX12 renderer and emits a
  geometry/readback manifest. Audit frames expand all three bounded nested lists
  into the driven outer scroll, require full marker visibility and horizontal
  fit, then reopen, decode and hash every capture before writing the manifest.
- General Hotkeys contains all 120 observed English actions, exact untouched
  Chat On/Off defaults and two chords. Real consumers cover movement, Stop All,
  Options/Hotkeys, profiles, fullscreen/FPS, minimap zoom/pan/center, action-row
  visibility, admitted battle targets, supported chat actions and selected real
  panels; unsupported actions remain disabled. The official Open Options -
  Custom Hotkeys action now opens that exact page. The key-capture modal exposes
  only the observed OK/Clear/Cancel actions, and Assign Text exposes only
  OK/Apply/Cancel.
- Chat Off / Chat On / temporary Chat On* now own resolver context and composer
  focus. Enter and Numpad Enter activate temporary chat from Off and send from
  On; explicit On persists after send. Claimed chords are removed before egui so
  they cannot replay into text. Chat On no longer reclaims focus from unrelated
  text editors. Custom Text shares the same validation/send path.
- The observed mouse preset choices and Classic-only `Loot: Right / SHIFT+Right /
  Left` dependency are composed but disabled until an actual input consumer
  exists.
- The foreground FPS control uses the observed integer 1–200 range. No-limit
  retains its prior numeric value across Apply/restart and the native redraw
  scheduler consumes the enabled/disabled result.
- Action bars have nine independent 50-slot rows and two Chat On/Off chords.
  Global lock migration, the observed single trailing lock per nonempty edge
  group, clear and primary-button move/swap primitives are covered against
  in-memory session assignments. Production still has no authoritative
  spell/object assignment source, so assignment menus, binding/item-amount/
  tooltip controls, clear controls and both lock surfaces stay disabled and are
  not counted as reachable behavior. The exact six observed context-category
  labels are present and unsupported categories are disabled instead of
  fabricated.
- Manual screenshot hotkey captures the real renderer, optionally crops the game
  viewport, writes lossless non-overwriting PNG and reports success/failure. The
  17 automatic event triggers remain disabled without an event pipeline.
- Help exposes the observed local-data order. Options export and confirmed reset
  are real; the combined options/minimap import, minimap export and unsupported
  support destinations stay disabled without an honest format/route.
- Settings migration preserves legacy action rows, sound keys and prior wide FPS
  values. Exact untouched v2 single-Stop maps upgrade to the 49/38 reference
  matrices while customized and explicitly cleared maps stay unchanged.
  Profile/movement edits are atomic and action-bar conflicts reserve only active
  legacy movement fallbacks.

## Acceptance evidence and next actions

Native Windows scissor rounding was repaired in `08495a50`; the exact SHA passed
MSVC tests/strict Clippy and the DX12 release page audit (20 pages, 78 captures,
all markers/footer buttons and no overflow/draft mutation). Private captures are
under `C:\Users\barte\Downloads\oteryn-settings-audit-08495a50`.
Integral sprite scaling with a shared physical transform for rendering, picking,
HUD overlays and screenshot crops was published in `1b3a3be5`, then passed native
Windows tests (190/163/5), strict Clippy and the 20-page/78-capture DX12 audit.
Its private manifest SHA-256 is
`b0bcedda748b9622147e63000a33057dac9736fe7378beb165f7acf8826db460`.
Keyboard delay was published in `5bf89893` and passed native Windows tests
(192/163/5), strict Clippy and the 20-page/78-capture DX12 audit; the private
manifest SHA-256 is
`a6b2866c47a4a84e9442e7a406520fd220520312e46824bbdfb4819443e200e0`.
The modifier/explicit-zero repair `597fda90` passed native Windows tests, strict
Clippy and the DX12 audit. The current AUTHORING batch supplies the independently
drawn custom arrow and its Big variant, while restoring the OS pointer under
Native and retaining specialized OS cursor semantics. The reference's Animated
cursor is a contextual walk/use/view/container/attack/NPC/quick-loot family;
its unavailable consumers stay disabled. Fresh native qualification remains
required for this successor.

The current client tree passes 193 client-library, 161 native-binary
and 5 launcher tests, strict client all-target Clippy, rustfmt and
`git diff --check`. These checks must be repeated on any repaired final
candidate. The native audit must then be built and run from a fresh exact-SHA
checkout on the authorized owner Windows machine; verify all 20 manifest entries
and captures before launching that same binary for the owner. The existing dirty
checkout at `C:\Users\barte\Downloads\oteryn-client-main-20261007` must not be
modified or cleaned.

After guarded publication, fresh-read PR/check state and repair every red check in
one AUTHORING batch. CI on an older head never qualifies a successor.

## Known completion gaps

- Spell/Object Custom Hotkeys need authoritative spell/inventory models and the
  observed picker compositions; the existing incomplete editors must not be
  enabled.
- Many General actions and panel interiors still lack genuine consumers or
  authoritative projections. Diagonal movement remains visible but disabled
  because the accepted movement wire has only four cardinal directions; the FPS
  toggle does not yet supply the label's lag indicator.
- Mouse preset/loot semantics, most Console and Game Window filters, graphics
  engine/AA, non-ambient effects, all audio families,
  Misc/Gameplay options and automatic screenshots remain pending.
- Action-bar production assignment sourcing remains absent; Clear/profile-remove
  confirmation semantics and edge-master behavior are not reference-proven.
- HUD condition projection/ordering, native visual comparison, populated module
  states and platform-specific graphics/audio choices remain open.
- Graphics Engine and Sound Device need real platform/runtime enumeration; their
  recorded static rows stay disabled and empty rather than hardcoding one probe
  machine. Factory-default state still lacks independent reference evidence.
- Secure remembered sign-in remains OFF/unwired. Actual sign-in stays system-
  browser PKCE. `protocol-oteryn`, multichannel WorldId/ChannelId separation and
  session-generation fencing remain invariant.

Presence, cardinality, saved intent, generated mockups and static reachability do
not establish 1:1 behavior. Continue in evidence-backed, reviewable increments.

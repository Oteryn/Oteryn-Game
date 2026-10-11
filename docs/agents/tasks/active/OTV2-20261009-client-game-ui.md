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
- The foreground FPS control uses the source-corrected integer 10–200 editable
  range; legacy 1–9 values remain loadable and executable. No-limit
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

Owner renewed implementation scope for all nine settings lanes: Graphics,
Effects, Audio, Controls/General Hotkeys, Action Bars/Custom Hotkeys,
Interface/HUD, Console/Game Window, Misc/Gameplay and Screenshots/Help.
Whole-scene AA/Smooth Retro source head `28565f4fbd1c92adc84ff2c71dddf7ad08bc3b3d`
passed Linux/MSVC checks, physical DX12 pixel readback and all 20 pages/78 captures.
Full counts, manifest and recovery evidence are in UI-FIDELITY.md. The owner's
running binary was retained; release uses a separate build directory.
Source `9e2d0cbc` adds real Unicode Select All, expiry OR gating and upstream OS
clipboard instead of in-app fallback. Linux/MSVC tests, Clippy, World pin and
20-page/78-capture DX12 audit pass; evidence is in UI-FIDELITY.md. External
clipboard transfer and nine-lane contract gaps remain open. No lane is 100%.

Earlier exact-head qualifications (source contracts `993ec2c4`, sliders/volume
`e6941338`, half-native scaling `985fe6b4`) and their manifests/recovery bundles
are retained in UI-FIDELITY.md. Reference QML contracts and private source hashes
are in UI-REFERENCE-EXTRACTED.md. Physical slider/trackpad parity, live lighting
and playback remain unqualified. Qt qsb 6.4.2 rejects serialization v9 even with
exit zero; this is not evidence of decoded shaders. Use installed-original
evidence, never publish original assets/account data, and qualify each new head.

MAP-WIRE-1 preparation reuses the accepted capability-18/domain-17 codec through
Session re-exports. The bounded map model atomically replaces/applies snapshots
and deltas, drops departed tiles/handles, enforces whole-view limits, and derives
targets solely from server origin metadata. Bad updates invalidate drawing and
targets until a valid snapshot. Eight targeted tests cover binding, reset/floor,
roof visibility, window changes, budgets and received-only stacks/ground speed.
This is preparation only: capability 18 remains unadvertised. No session resync,
streamed drawing, live targeting or ground-speed switch is claimed. Complete
MAP-CLIENT-1 requires those consumers and the joint client/server speed switch
under the accepted packet, followed by exact-head review and native qualification.
Preparation source `fe3eec72` passed Linux client 205/171/5, MSVC client 204/175/5,
Session 58 tests on each platform, strict Clippy, formatting, boundaries and
governance/60 tests. Exact-source DX12 audit passes 20 pages/78 verified captures/
zero invalid entries at 125% DPI. Manifest/recovery hashes are in UI-FIDELITY.md.
The owner's PID 12896 remains responsive. Protected-base audit passes, not full
CI. Next: complete accepted MAP-CLIENT-1 runtime consumers, retaining the joint
speed-switch requirement and not enabling speculative light/sound projections.

Historical qualification remains recorded, with exact SHAs, manifests and private
capture paths, in `apps/client/UI-FIDELITY.md`: scissor rounding `08495a50`, integral
scaling `1b3a3be5`, repeat scheduler `5bf89893`/`597fda90` and custom pointer
`5b784e60` passed their MSVC/Clippy/DX12 chains. These are not qualifications for a
new source head. Contextual Animated cursor consumers remain unavailable.

Earlier receipt-clock/Clear head `1b2928fb` passed MSVC tests (193/166/5), strict
Clippy and the 20-page/78-capture DX12 audit; full evidence and manifest remain in
`apps/client/UI-FIDELITY.md`. Accepted ChatLine receipt metadata supplies genuine
timestamps via upstream `time` 0.3.55. Clear uses local per-tab sequence cutoffs;
later identical messages survive, server/session history is unchanged, and an
unavailable clock never produces invented time.
The root lockfile is a World compiler input. Its changed digest reproduced the
red World bundle job locally; the AUTHORING repair refreshes only the pin's
`inputs_digest`, preserving world content, identity, production=false and the
bundle digest. Qualification uses the unchanged double-build gate.

Repeat selected local checks and exact-SHA MSVC/DX12 qualification on each new
source head; earlier evidence never qualifies it. Verify all 20 pages/captures.
Never modify or clean the owner's dirty checkout
`C:\Users\barte\Downloads\oteryn-client-main-20261007`.

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
  engine selection/AA visual parity, non-ambient effects, all audio families,
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

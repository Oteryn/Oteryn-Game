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

Slider/volume semantics and private source hashes are in UI-REFERENCE-EXTRACTED.md;
the following exact-head evidence supersedes the authoring checkpoint. Pixel-wheel,
native interactive parity and playback remain unqualified. Keep strict geometry
gates and qualify every new client head independently.

Slider/volume source head `e69413380a2e44aa763b603844fd60fb7d7d6ed8` passed
Linux 194/169/5 and MSVC 193/173/5 tests, strict Clippy and the 20-page/78-capture
audit. Full manifests, private paths and recovery evidence remain in UI-FIDELITY.md.
Next: physical/native interactive slider parity (including pixel/trackpad units),
remaining source-derived Interface dependencies and real graphics/audio runtime
capabilities/consumers. Do not repeat completed source inventory or count disabled
audio as playback. Qualify any new client source head independently.

Graphics source contracts and hashes are in UI-REFERENCE-EXTRACTED.md.
The bounded repair retains a half-native viewport when
integral scaling cannot fit 1x, matching the original minimum without HUD overflow.
Tests verify each tile's pointer transform at half-size and 125% DPI. Exact source
head `985fe6b499979ac8420079a418731b1e373c8088` passed Linux 195/169/5 and MSVC
194/173/5 tests, strict Clippy and 20-page/78-capture DX12 audit. Full manifest and
recovery evidence are in UI-FIDELITY.md. AA resampling is now implemented above;
lighting lacks real source/environment projections. Stock
Qt qsb 6.4.2 rejects QSB serialization v9 (with exit zero): compatible upstream
decoding remains pending, not a reason to invent a parser or enable controls.

Use installed-original evidence before implementation. Original source, assets
and account data are never published. UI-REFERENCE-EXTRACTED.md retains the QML
contracts, source digests, resource counts and backend gaps.

Source-contract repair `993ec2c4a19aab5099e8b82dff9d8a1ece753e75` was published
through the guarded exact-candidate path. Recovery bundle:
`/workspace/oteryn-client-publication-993ec2c4.bundle`, SHA-256
`885787491f248dfae27b4a77adca4a886c069f510fa01cdbaea3a137aae5be7b`.
It passed Linux tests (194/165/5), strict Clippy and formatting; governance
validation and all 60 governance tests passed. A fresh Windows detached checkout
passed MSVC tests (193/169/5), strict Clippy and DX12 release audit with 20 pages,
78 captures and zero invalid entries. Changed default-state Controls, Graphics,
Effects and Sound captures were visually inspected. Private directory:
`C:\Users\barte\Downloads\oteryn-settings-audit-993ec2c4`; manifest SHA-256
`a5384dbff955c770564d1938c8ef1ee6a3183d8ab503a92ff7397a0ccdcb6197`.
This is not warning-state visual parity or playback acceptance. The slider and
volume-filter follow-up is recorded above; real renderer/audio capabilities and
full native interactive parity remain pending.

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

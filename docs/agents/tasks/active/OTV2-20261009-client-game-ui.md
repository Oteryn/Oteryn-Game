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

The 20 observed production pages/order, semantic markers, scroll/geometry and
non-mutation checks are implemented; DX12 audit captures and hashes each page.
General Hotkeys retains 120 observed actions and exact untouched Chat On/Off
defaults, two chords and real supported consumers. Unsupported actions, mouse
presets/loot and assignment controls stay disabled. Chat On/Off/temporary focus,
shared text sending, 10–200 editable foreground FPS with preserved legacy values,
nine 50-slot action rows, screenshot PNG capture, options export/reset and atomic
settings migrations are implemented in the qualified batches documented in
UI-FIDELITY.md. Saved values alone do not establish a runtime feature. Original
automatic screenshots, spell/object assignment sources, import/minimap formats,
many panels and remaining engine contracts are still absent.

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

Next AUTHORING batch connects the accepted map domain to Session admission and
typed pushed events, without advertising 18 yet. Bundle joins require domain 17
instead of 2, validate generation and dependencies 4/6, and reject mixed object
domains. Binding mismatch emits the existing FND ResyncRequest, invalidates map
state and ends this unusable session; recovery requires a new admission snapshot.
The missing protocol encoder implements the existing foundation.proto request
only, without allocating/changing a wire type. Dev-client exhaustive error mapping
and fixture harness builders follow the added Session types. The fixture harness
does not advertise/test 18; bundle E2E remains pending. These consumer/public-API
changes require independent review on the final frozen candidate.
Session source `e6357687` independently passes MSVC/DX12 20-page/78-capture
qualification; counts/manifests/recovery are in UI-FIDELITY.md. Current draw batch
reuses pinned assets, SpriteFrame/AtlasPage, floor/stack phase ordering, actor
overlap and neighbour anchors; five tests include a real pinned sheet cell.
Linux client 210/171/5, Session 62, renderer 39, assets 2/13 and strict Clippy pass.
Next: actual GPU page uploads/multiple draws and the play-view consumer, not
another assertion that preparation alone constitutes a live feature.

Draw source `108d99f8` now passes MSVC seven-package strict Clippy/tests and the
exact-source 20-page/78-capture DX12 audit; full evidence is in UI-FIDELITY.md.
The current renderer batch adds up to four ordered GPU instance buffers, bounded
atomic page-cell uploads and CPU backing for texture recreation. Physical DX12
later-batch/overwrite/recreation readback is added but not yet native-qualified.
Continue with the Session-owned full map store, bounded play mailbox and actual
scene consumer; do not stop after renderer preparation. Capability 18 stays off
until the accepted consumer and joint speed-switch requirements pass.

Historical qualification remains recorded, with exact SHAs, manifests and private
capture paths, in `apps/client/UI-FIDELITY.md`: scissor rounding `08495a50`, integral
scaling `1b3a3be5`, repeat scheduler `5bf89893`/`597fda90` and custom pointer
`5b784e60` passed their MSVC/Clippy/DX12 chains. These are not qualifications for a
new source head. Contextual Animated cursor consumers remain unavailable.

Receipt-clock/Clear `1b2928fb` and the lockfile/World-pin CI repair remain fully
qualified and documented in UI-FIDELITY.md. Never invent timestamps or mutate
server history. Lockfile changes require refreshing only the World input digest
and repeating the unchanged double-build gate, preserving content and identity.

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

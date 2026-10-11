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

Help ZIP export AUTHORING uses upstream window-owned `rfd`, `zip` and atomic
`tempfile` replacement, reusing the existing validated Oteryn preferences.
Linux 209/175/5, strict Clippy, formatting, boundaries and governance pass.
World input pin `5e2707ac` double-builds the unchanged bundle (`f8b11ebc`).
Fresh MSVC/DX12 qualification and actual picker interaction remain pending;
unknown original ZIP contents/minimap import are not claimed as implemented.

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

## Map consumer continuation — AUTHORING

Source `c302e2555c83aeeb51481d324663603304420140` is committed/published:
Session admits and maintains the complete bounded domain-17 map; mismatched
bindings request existing FND resync and invalidate drawing/targets. Pinned
appearance/sheet resolution, floor/stack phases, bounded four-batch GPU uploads
and texture recreation reuse the existing renderer. Play coalesces complete map
projections, renders them through the Windows page/capture path and never anchors
them to the local start area. Native secondary USE clicks pick the highest visible
floor/top received entry and derive authority only from its origin. Session checks
queued targets again; stale inputs send no frame or command ID. Duplex tests
verify exact base-key/revision and item-handle payloads. Actor visuals without an
accepted binding remain placeholders; this does not enable all mouse/loot presets.

Linux client 209/171/5, Session 72, dev adapter 34, harness 36 and strict Clippy
pass. MSVC client 208/175/5, Session 72, protocol 214, dev adapter 34, harness 36,
renderer 43 and assets 2/13 pass with strict seven-package Clippy. Exact-source
DX12 audit verifies 20 pages/78 capture hashes/zero invalid entries at 125% DPI;
PID 12896 is preserved/responsive. Source/manifests/recovery for all intermediate
batches are retained in UI-FIDELITY.md. Same-head protected-base audit passes,
not full repository CI, independent review or bundle E2E.

Capability 18 stays unadvertised. The joint ground-speed switch needs an owning
contract: admitted actor/entity/vitals and step-result codecs expose no effective
actor speed or step-duration/deadline, while production speed depends on level,
conditions and equipment. Never substitute 110/150 or switch only the server.
Finish unblocked settings/reference work, then route missing speed authority with
the existing spell/condition/light/audio/event projection gaps. Floor/teleport
snapshot recovery, map drag, bundle E2E and frozen-head independent review remain
open. Animation/light/minimap are outside the accepted MAP-CLIENT-1 packet.

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

- Original Help source and a live SaveFile dialog confirm ZIP export/import
  filters. AUTHORING replaces fixed-path JSON with native ZIP selection and
  atomic Oteryn-only export, with cancellation/error/round-trip regressions.
  ZIP contents remain unknown: the probe produced no archive and the original
  reference process disappeared; do not restart/sign in or copy global identity
  configuration. Schema-only metadata and private evidence are recorded in
  UI-REFERENCE-EXTRACTED.md. Continue offline/independently authored work.

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

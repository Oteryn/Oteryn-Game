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

## Completion continuation — 2026-10-13

Copy consumes the real composer via ordered upstream egui Copy events; hidden,
read-only, modal and unrelated focus cannot claim it. Linux 210/188/5, strict
Clippy and formatting pass. Native regression is added but PC access reports
offline; old Windows evidence is not inherited. Exact-version BSL exceptions
with provenance fix the clipboard CI cause; local cargo-deny passes all checks.
Full remote CI is not yet proven. REA 6.4.0/Ghidra are available with scoped
configuration; ELF/pwntools is unavailable, not an analysis success. Staging
world/channel exist; gameplay TLS 1.3/ALPN, hostname and supplied CA verify;
no account operation or admission is performed. Main
`0db5c53b` adds no missing active presentation/speed projections. Do not allocate
them or enable unoffered caps. Details/remaining acceptance are in UI-FIDELITY.md.
Oteryn-only ZIP import now stages bounded/validated draft, never auto-Apply.
Duplicate/interrupted central records and local CRC/size mismatches are refused;
real Cancel/Apply/persistence tests pass. Linux 210/194/5, strict Clippy/fmt pass.
Original/minimap formats, native picker/DX12 and missing server inputs remain open.

## Native login launcher — 2026-10-12

GUI uses PKCE, owned characters, native ticket/Gateway grant and server admission.
`tools/qualification/login_local/launch-client-local.ps1` validates the private
env file without evaluation/credential comments, passes four public values only
to the child and selects the copied certificate without installing trust.
World/Character selection remains the authenticated GUI's responsibility.
No automatic login, deployment, account mutation or production claim.
Linux/Windows PowerShell 22 checks and client/Platform login tests 10/13 pass. Data
is in PR comment 6125867648 and NAS `/volume1/oteryn/preprod-ca-public/`;
Root/leaf are valid. Historical zero-world/TLS-reset probes are superseded above;
no missing-data request. Game admission still needs actual acceptance.
Historical Windows hashes, captures and public launch paths are in UI-FIDELITY.md.
Existing installations remain intact; no automatic login or replacement.
No login/restart. Windows fixes: exact expanded IPv6 loopback and closed-file
validation on refusal; no weaker TLS or additional insecure origins.

## Main reconciliation — 2026-10-13

Normal merges preserve accepted maina7a0b374 producers and STEP-PACE-WIRE-0
59f9fb12. Bundle c49538eb/start/identity/topology and inputs86d18a80 stay intact.
Windows9c build exposed the ZIP picker's nonexistent open_file; use upstream
rfd0.17.2 pick_file. Real duplex tests cover receipt/legacy decoding and poison
invalid or unselected receipts. Decoder acceptance is not a pacing deadline:
emission must await STEP-PACE-CLIENT-1. Defaults stay [4,6,7,13,17]. Windows5d:
216/201/5+Session76, release/Clippy and DX12 20pages/78captures pass; PID37764.
CP6145246782 confirms emission OFF/order; wrong CC link corrected6145306643.

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

Qualified Help ZIP `7898fdf7`, immutable output selection `34f59214` and
footer/page Reset `a95a92dc` evidence, exact SHAs, Linux/MSVC counts, DX12
20-page/78-hash manifests and recovery remain in UI-FIDELITY.md. Their code
is preserved; earlier evidence does not qualify later heads. Help reuses
window-owned upstream `rfd`/`zip`/`tempfile`; output selection is per-channel
and immutable. Ordinary Reset affects implemented controls of the current page
only, preserving unavailable intent, bindings, locks and other pages. Original
factory/profile defaults, physical picker/clipboard, original ZIP/minimap
interoperability, live bundle E2E and final CI/review remain open.

Owner renewed implementation scope for all nine settings lanes: Graphics,
Effects, Audio, Controls/General Hotkeys, Action Bars/Custom Hotkeys,
Interface/HUD, Console/Game Window, Misc/Gameplay and Screenshots/Help.
Earlier AA/Smooth Retro `28565f4f`, Unicode/expiry/OS clipboard `9e2d0cbc`,
source contracts `993ec2c4`, sliders `e6941338` and half-native scaling `985fe6b4`
qualifications, manifests and recovery remain in UI-FIDELITY.md; private QML
hashes remain in UI-REFERENCE-EXTRACTED.md. No lane is 100%. Physical clipboard,
slider/trackpad, lighting and playback remain unqualified. Qt qsb 6.4.2 rejects
serialization v9 despite exit zero; no shader decode acceptance follows.
Preserve the owner's running binary; use separate release build directories,
never publish original assets/account data and qualify each new source head.

## Map consumer continuation — AUTHORING

Owner continuation retains this single client writer; CP6146799516 withdraws
the competing CP allocation, not this owner task. Architect6146789689 supplies
the complete MAP/receipt/native criteria. No server/protocol/deployment writes.
Current map fixes: same-floor received terrain clicks actually start walking;
shared projected-floor picking invalidates covered queued USE (raw duplex proves
no frame/ID); malformed origins request existing resync and fail closed; D627
east/north walkers stay under both endpoint tops, including resident-atlas quad
order. Explicit default/--assets selection injects one pinned cached provider;
missing/mismatched catalogues log once. Actual received Item-ground uploads match
decoded pixels, survive reset and disappear on provider replacement. Production
bootstrap now advertises [4,6,7,13,17,18]; raw selected/declined tests cover map17
versus legacy2. Linux client222/196/5+Session79, strict all-target Clippy/fmt pass.
Map7335c8b9 passes MSVC221/201/5+Session79 and DX12 20pages/78hashes/0invalid;
hashes/failed-release-ID attempt in UI-FIDELITY.md. PID37764 remains untouched.
Receipt consumer continues in OTV2-20261013-step-pace-client-1; no E2E claim.
Unknown actor visuals remain placeholders; no fabricated binding/mouse presets.

CP decision 6143183755 accepts authoritative step_wait_ms (existing cap13).
#2034 owns STEP-PACE-WIRE-0; server emission precedes map18 activation, receipt
consumption and native E2E. This batch changes no protocol/registry or offered
capability. PlayLink now gates actual queued/undrained steps and latches end;
strict bootstrap1007/1116 reaches update-required without login retry. Independent
wire and delayed-task regressions pass: client217/194/5, Session74. Canonical
1007 is SessionFatal, N8/1116 TransportFatal. Design resolved, runtime proof
pending. PC offline; Windows/E2E/CI/review remain open. Details: UI-FIDELITY.md.
Battle window consumes admitted HP, with real egui replacement/filter/empty
regressions; names and original sorting/visual parity remain unqualified.

Historical scissor `08495a50`, integral scaling `1b3a3be5`, repeat
`5bf89893`/`597fda90` and pointer `5b784e60` MSVC/Clippy/DX12 evidence remains
in UI-FIDELITY.md, not inherited by successors. Animated cursor stays unavailable.

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

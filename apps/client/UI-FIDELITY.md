# Native UI fidelity acceptance

Draft #1942 is AUTHORING. Presence of a panel or persisted preference does not
establish visual parity or working gameplay. Original reference captures and
assets remain private; the product ships independently authored code/artwork.

Latest local-input qualification:71bbdf89d1ab5c913c95adced7cd35827a2b0cbd
passes Linux230/214/5 and exact MSVC229/221/5+Session82,strict/fmt/release,
plus native DX12 at125%DPI20pages/78rehashed captures/0invalid. Unfocused
binding capture, IME/history ownership, profile confirmation and PL/EN import
failure/retry are actual consumer tests, not saved-field completion. Full hashes
and paths: docs/agents/tasks/active/OTV2-20261014-client-six-local-steps.md.
Physical OS interactions, version-matched original resources, legal audio bank
and post-B2/B3 action-bar activation remain unqualified. Original installed SHA
now differs from historical REA evidence; old binary addresses are not reused.
71bb CI subsequently fails four Rustls0.23.45 advisories; the0.23.46 upstream
patch/lock successor requires fresh qualification and independent security review.

## Reference audit correction

Launcher correction2026-10-16 removes a fixed zero panel height which clipped
the owner-character list to64px and hid the second row. Actual shared panel/list
regression is RED before the fix, GREEN afterward: fully visible second names
and real selection clicks, PL/EN at900x620/1280x720. A one-row native screenshot
was not proof that Platform omitted characters. Fresh native successor acceptance
remains pending; this launcher repair does not establish gameplay or1:1 parity.
The options ZIP dependency moves from yanked9.0.1 to upstream9.0.4 without
relaxing the existing bounded archive/import rules or supply-chain checks.

Playable continuation2026-10-16 connects the observed Logout action to the real
profile/chat-context resolver and typed LOGOUT_V1 consumer. Rebinding replaces
the old defaults; extra modifiers, key repeats, dialogs and inactive windows do
not request logout. Main's hard-coded key probe is not a parallel native route.
Accepted/refused/unselected logout, map/receipt coexistence and a committed join's
initial HUD projections have deterministic regressions; live successor acceptance
is pending. No vitals, items or chat success is manufactured when the server
does not supply them. This does not close the other settings/gameplay gaps.

[Live observations](UI-REFERENCE-OBSERVATIONS.md) establish **35 HUD conditions**
and the later [settings inventory](UI-REFERENCE-SETTINGS.md) establishes **50 slots
in each of all nine action bars**. The draft now carries those cardinalities and
the observed target-marking membership. Dedicated composition and persistence
still do not establish runtime behavior; only the consumers named below count as
implemented.

Installed-original 15.33 source contracts and recovered-resource boundaries are
recorded in [UI-REFERENCE-EXTRACTED.md](UI-REFERENCE-EXTRACTED.md). They correct the
old 1–200 FPS observation: editable bounds are **10–200**. Legacy 1–9 values remain
loadable and are not rewritten on page opening. Controls now distinguish critical
custom delays below 50 ms from warning delays below 250 ms; the default master
hides both. Clouds/audio zero values carry Off markers. Anthem/item children
follow their positive group volume, but all audio remains disabled without a
backend; these dependency repairs are not playback acceptance.

The next source-backed batch adds slider arrows, Shift × 10 / Ctrl × 100 (combined
× 1000), normal line-wheel range/10 changes, and held-arrow repetition after the
reference's 350 + 60 ms timers. Repeats ignore click modifiers and cancel on release,
disable or lost focus. Battle and UI filters now also require positive group
volume while preserving selections; missing audio consumers remain disabled.
Trackpad/pixel-wheel equivalence and native interactive acceptance remain open.

Current AUTHORING adds a Windows whole-scene resampling consumer: None uses
nearest image sampling, Antialiasing smooth sampling, and Smooth Retro composes
at ceil(scale) native multiples before smooth downsampling. At native size the
extra pass is bypassed. Atlas filtering stays nearest and HUD/text stay outside
the pass. Mode/size/format changes replace cached resources; source dimensions
are checked against device limits. The exact-head DX12 regression reads actual
rendered pixels to distinguish the modes; qualification is recorded below.
These changes do not qualify full lighting, engine switching or visual 1:1 parity.

## Page matrix

Supported hotkey/profile authoring now refuses over256-byte Unicode text,
control characters and full256-action profiles before changing the draft.
Profile naming and auto-switch use the persisted trim/to_lowercase rule;
same-frame edits are revalidated before confirmation. Legacy unsupported
Spell/Object bindings remain loadable but cannot claim a physical key in either
the resolver or native router. Actual modal button transactions and retained
input/data regressions are part of this batch. Exactfabf441a native component
qualification is recorded below; previous4ff evidence is not inherited. See
`docs/agents/tasks/active/OTV2-20261013-client-hotkey-authoring.md`.

| Reference page | Dedicated composition | Actual consumer / remaining acceptance |
| --- | --- | --- |
| Basic | Yes | Display controls/action-row visibility work; own-resource bars/arcs have a vitals renderer behind explicit HUD selections. Audio stays disabled because no backend/content exists |
| Controls | Yes | Four cardinal movement bindings work; diagonal rows remain disabled because the accepted movement wire has no diagonal intent. Keyboard Delay has the observed 0–1000 ms slider and explicit default 250 ms master; applied selections drive a client-owned held-key scheduler independently of OS repeat and frame-rate limits, using the existing paced session step queue. Releases, lost focus/device, modal/text editors, chat-context/profile changes and Stop All cancel repeats. The custom delay is retained under the default master. The exact three mouse presets and the Classic-only `Loot: Right / SHIFT+Right / Left` selector are composed with the observed dependency, but stay disabled because their input consumers are pending; persistence alone is not counted. Fresh native qualification for the repeat scheduler remains pending |
| General Hotkeys | Partial: dedicated profile/action table | All 120 official English labels have stable IDs; Polish labels are Oteryn-authored. Profile management, both chat contexts, delayed search, two bindings and cross-family conflict validation work. Real consumers cover cardinal movement/explicit clearing, Stop All, profile cycling/character auto-switch, admitted battle targets, Options/Custom Hotkeys, existing Cyclopedia/dialog/panel navigation, fullscreen/FPS, minimap and action-row visibility, channel/mode/send operations and composer Select All/ordered Copy. Escape remains reserved for Stop All. Dialogs open idempotently; windows toggle. Existing local panel compositions are not server-backed data, and unknown interiors remain unavailable. Diagonals, unsupported inventory actions and most other observed actions remain disabled without exact consumers. Physical clipboard and original link-warning acceptance remain open |
| Action Bar Hotkeys | Dedicated visible 9 × 50 table; editing disabled | Both stored chord columns remain visible in the distinct Chat On/Off contexts, and legacy 12-slot/default migration remains loadable and validated. The runtime router executes a slot only when that session has an executable assignment and no longer claims an empty slot's key. Production has no authoritative spell/object assignment source, so every binding cell stays disabled and the internal assignment/migration primitives are not counted as reachable activation behavior. A fresh native Windows acceptance for this candidate remains pending. |
| Custom Hotkeys | Partial: observed editor inventory | Text assignments fill the real composer or send through the typed chat intent. Spell/object assignment and legacy bindings are disabled because no authoritative live picker/assignment exists; their persisted presence is not counted as behavior |
| Interface | Yes | Tile-under-pointer highlighting works. Explicit Native off draws an independently authored Oteryn arrow, Native on restores the OS pointer, and Big doubles the custom arrow around the same pointer hotspot. Specialized text/resize cursors retain their OS semantics; contextual Animated cursor states, native/Big combination acceptance, link-copy warning, loot-colour and expiry remain pending. Fresh native qualification of the custom arrow is required. Oteryn language, contrast, motion and scale controls are intentionally absent from this reference page |
| HUD | Partial: groups/arcs and all 35 observed conditions | Exact master labels, the two Harmony radio positions and 35 locally drawn condition symbols are present. Own name and harmony use admitted/session data; own health/mana bars/arcs and other-actor health bars consume real projections. Condition controls and ordering are visible but disabled because the protocol exposes no condition projection; native live-session visual acceptance remains pending |
| Console | Yes | Per-tab read-only, bounded/non-overwriting Save Window, receipt-time timestamps and sequence-cutoff Clear Window work without mutating session history or inventing server timestamps. AUTHORING adds immutable received-output selection/copy through upstream egui, with per-channel selection and hidden-output focus release; native qualification is pending. Chat Off / Chat On / temporary Chat On* own focus and send behavior. Select All selects the real Unicode composer. The upstream Windows clipboard backend is compiled; external clipboard acceptance is still required. Participants, secondary console, private tabs and event filters remain pending. Legacy text size is retained but has no reference-page control |
| Game Window | Yes | Frame & Highlight / Frame Only / Highlight Only / None drive the real target frame and scene overlay. Text/combat/effect projections remain disabled |
| Action Bars | Partial | Three masters and nine independent 50-slot rows drive the live layout; visible stored hotkey labels affect that renderer. Global lock migration, a single trailing lock per nonempty edge group, clear and primary-button move/swap primitives are tested against in-memory session assignments. Production still has no authoritative spell/object assignment source, so an admitted client starts empty; assignment menus, binding/item-amount/tooltip controls, clear controls and both lock surfaces remain disabled. Those primitives are not counted as reachable behavior. |
| Shortcuts | Yes, two lists | Add/remove/order edits for destinations with real local compositions affect draft only; Apply updates the live registry and Cancel discards. Clean profiles and Defaults omit Lenshelp; a Lenshelp entry retained from an existing profile remains visible and removable but disabled, and cannot be added while its destination is unverified. |
| Graphics | Yes | Fullscreen/VSync, the source-corrected integer 10–200 editable limit (legacy 1–9 retained), retained disabled value under No Frame Rate Limit, live frame-rate readout and the optional FPS overlay work. Integral-multiple scaling fits whole physical multiples of the 32-pixel sprite cells, with the source-confirmed half-native minimum when no whole multiple fits. Rendering, picking, HUD and capture share its transform; windows too small for that minimum expose recovery. Windows None/Antialiasing/Smooth Retro now drive whole-scene image sampling, with a physical DX12 pixel regression. Engine selection remains disabled; original-client live image parity and full lighting composition remain pending |
| Effects | Yes, lighting + opacity groups | The Light master gates its retained Ambient, Level Separator and combined Clouds & Indoor children; ambient intensity drives the bounded scene overlay. Level separation, the combined attenuation row and spell-effect controls stay disabled pending projections/renderers |
| Sound | Yes, device + five volume groups | Food/move-item controls belong in item group; all controls are disabled because no audio backend/content exists |
| Battle Sounds | Yes, own/others/creatures | Nested spell categories are present but disabled because no mixer/events exist |
| UI Sounds | Yes, UI/social + console categories | Filters are present but disabled because no audio playback exists |
| Miscellaneous | Yes, seven rows | Confirmation and session selections require explicit consumers; secure remembered sign-in stays OFF/unwired |
| Gameplay | Yes | Inspection/chase/loot intent prepared; engine contracts/consumers pending |
| Screenshots | Yes, two trigger columns | The folder action creates and opens the local Oteryn screenshot directory. The screenshot hotkey performs a real non-overwriting PNG renderer capture, and Only Capture Game Window selects the actual scene crop. Seventeen automatic triggers stay disabled because their event pipeline does not exist |
| Help | Partial | Info is local; Compendium opens the existing local composition idempotently during play; confirmed reset works. Native window-owned ZIP export is atomic; import accepts only the bounded flat Oteryn options archive and stages validated/migrated preferences into draft. Apply/Ok saves normally, Cancel discards and errors preserve current/draft/files. The reference combined Import Options/Minimap row explicitly supports Oteryn options only; original-client/minimap archives are refused, not partially guessed. Export Minimap remains disabled. Native picker interaction and original interoperability remain unqualified. Client Help/Lenshelp, Rule Violations, Manual and FAQ stay visibly disabled because no verified Oteryn destination exists |

## Compiled-page verification — 2026-10-11

- Production navigation exposes exactly the 20 audited reference pages in their
  observed order. Supplementary Quick/Account/Transparency navigation, seven
  catalogue-only pages and Oteryn-only controls embedded in reference pages are
  no longer exposed; their old values remain loadable for compatibility.
- The settings backing surface is opaque. An authorized same-dialog comparison
  over unrelated login and in-world backgrounds found zero background bleed in
  a 430 × 250 empty-content sample; legacy transparency values still load but
  do not alter the reference settings surface.
- A binary-target `egui` regression renders all 20 audited reference pages at
  900 × 620 and requires a page-specific marker plus Reset/Ok/Apply/Cancel. It
  is a reachability guard; page-family tests and typed setting contracts provide
  additional behavior coverage, but there is not yet an exhaustive per-row
  semantic snapshot.
- A native Windows audit mode opens and scrolls those same production-routed pages,
  reads completed egui geometry and writes captures plus a manifest per page.
  Audit-only frames expand the HUD-condition and both shortcut lists so the
  driven outer scroll reaches every row; ordinary UI keeps the bounded nested
  scrolls. Full marker visibility, horizontal fit, file type, decoded dimensions
  and recomputed SHA-256 are fail-closed. Its implementation is compiled, but a
  fresh run against each changed client candidate remains required before
  publication evidence can be recorded. Native scissor bounds use egui-wgpu's
  physical-pixel rounding, including the observed 125%-DPI marker case; an actual
  one-pixel glyph cut still fails the regression.
- PL/EN composition tests render the dedicated page families without mutating a
  default draft merely by opening them.
- Exact commit `08495a500fdc92653a31be398dfcb1863f60ef86` passed native
  Windows MSVC locked tests and strict all-target Clippy, followed by a DX12
  release audit: 20 pages, 78 captures, no missing markers/footer buttons,
  horizontal overflow or render-time draft mutations. Private output is
  `C:\Users\barte\Downloads\oteryn-settings-audit-08495a50`.
  First/last captures for all 20 pages were visually inspected from the repaired
  diagnostic run before the exact committed run. Later client changes require
  fresh native qualification; this evidence does not qualify their behavior.
- Integral-scaling successor `1b3a3be5032053df29147ee8cdba6381aa24dcf8`
  passed 190 Windows library, 163 binary and 5 launcher tests plus strict Clippy.
  Its DX12 release audit again passed 20 pages/78 captures with no invalid page;
  the changed Graphics page was visually inspected. Private manifest SHA-256:
  `b0bcedda748b9622147e63000a33057dac9736fe7378beb165f7acf8826db460`.
  Keyboard-repeat head `5bf89893125d74d133edd687c13f034b2c2a41b8` then passed
  native MSVC tests (192 library/163 binary/5 launcher), strict Clippy and the
  20-page/78-capture DX12 audit. The changed Controls page was visually inspected.
  Private manifest SHA-256:
  `a6b2866c47a4a84e9442e7a406520fd220520312e46824bbdfb4819443e200e0`.
  The modifier-cancellation/explicit-zero repair `597fda90` passed native tests,
  strict Clippy and the 20-page/78-capture audit. The custom-pointer successor
  `5b784e609e7fa0f80bff5fb1ecb4082656dead0b` also completed its MSVC/DX12
  qualification chain and passed all 20 pages/78 captures with no invalid page.
  Its private manifest SHA-256 is
  `40cc3e17a964666a2ea8a4ea7c4ce331d988109d062f63b6cd0dd3d29316e5eb`.
  Page-audit evidence does not qualify contextual cursor behavior or live chat.
- Receipt-clock/Clear successor `1b2928fbfb8d14bf566f06df334c2f312a2fb6f0`
  passed native MSVC tests (193 library/166 binary/5 launcher), including actual
  Windows local-clock availability, strict Clippy and the 20-page/78-capture
  DX12 audit with no invalid page. The changed Console capture was visually
  inspected. Its private manifest SHA-256 is
  `3bfb69468bb3505fdb0a85bf535c6f781bfe267bcc4c0f7d00cf2d39452a9a3f`.
  The lockfile delta invalidated the World bundle input pin; the repair updates
  only that input digest and must preserve the existing bundle digest.
- Installed-original contract repair `993ec2c4a19aab5099e8b82dff9d8a1ece753e75`
  passed Linux locked tests (194 library/165 binary/5 launcher), strict Clippy,
  formatting and governance checks. Native MSVC tests passed (193/169/5),
  including the FPS legacy-retention and delay-boundary regressions. Its DX12
  release audit passed 20 pages/78 captures with no invalid page; default-state
  Controls/Graphics/Effects/Sound captures were visually inspected. Private
  directory: `C:\Users\barte\Downloads\oteryn-settings-audit-993ec2c4`.
  Manifest SHA-256:
  `a5384dbff955c770564d1938c8ef1ee6a3183d8ab503a92ff7397a0ccdcb6197`.
  The captures do not establish populated audio, warning-state visual parity or
  unavailable renderer/lighting functionality.
- Slider/volume successor `e69413380a2e44aa763b603844fd60fb7d7d6ed8` passed
  Linux locked tests (194/169/5), strict Clippy, formatting and governance checks.
  Fresh MSVC tests passed (193/173/5), including arrow timing/modifiers, normal
  line-wheel steps and retained volume-dependent filters. Its DX12 audit passed
  all 20 pages/78 captures with no invalid entry. Changed default-state HUD,
  Graphics, Battle Sounds and UI Sounds captures were visually inspected.
  Private directory: `C:\Users\barte\Downloads\oteryn-settings-audit-e6941338`;
  manifest SHA-256:
  `47bc86e9582d5c0310ffc97da661f6c64ac4f4f8a8ccc7ac84f9404dadd01dc3`.
  The strict compact HUD/footer geometry gates were retained and passed after
  repairing arrow sizing. This is not physical mouse/trackpad parity or playback
  acceptance, and later client changes still require fresh qualification.
  Preserved recovery bundle: `/workspace/oteryn-client-publication-e6941338.bundle`,
  SHA-256 `5d0b1057e135a5f2373a05e3e9308830801864f618266d714c7789c5aeb04438`.
- This establishes native compiled-page reachability, not exhaustive reference
  comparison or working behavior for disabled rows. Native live-session visual
  acceptance and every row whose consumer remains pending are still open and
  must not be reported as 1:1 complete.

Half-native scaling source head `985fe6b499979ac8420079a418731b1e373c8088`
passed Linux locked tests (195/169/5), strict Clippy, formatting, governance and
all 60 governance tests. Fresh MSVC tests passed (194/173/5), including half-size
tile picking and 125%-DPI bounds; strict Clippy and the DX12 release audit passed
20 pages/78 captures with zero invalid entries. The Graphics capture was visually
inspected. Private directory:
`C:\Users\barte\Downloads\oteryn-settings-audit-985fe6b4`; manifest SHA-256
`11066619eb47a8730cc0fdf0662cd1f539927c1d73385f96e6d3c1ac43e93425`.
The page audit does not exercise an admitted half-size live scene or qualify AA,
lighting, physical input or disabled rows. Exact source-head recovery bundle:
`/workspace/oteryn-client-publication-985fe6b4.bundle`, SHA-256
`7ebcb54fdae881dd392d071136b4b4b2670e4545055086625991716fce6f1fd3`.

## Whole-scene sampling qualification

AA source head `28565f4fbd1c92adc84ff2c71dddf7ad08bc3b3d` passed Linux client
tests (196/169/5), renderer tests (39), strict Clippy, formatting and governance.
Fresh MSVC client tests passed (195/173/5), renderer tests (40), including actual
DX12 readback distinguishing nearest, smooth and ceil-scale Retro sampling.
Strict Clippy and the DX12 release audit passed 20 pages/78 captures with zero
invalid entries; changed Graphics and default Basic captures were inspected.
Private directory: `C:\Users\barte\Downloads\oteryn-settings-audit-28565f4f`;
manifest SHA-256 `4fdeb86cc3bdccb90fd585ca189548cec393bead78ae36e1cb80abca4c91b7b6`.
Release binary lives in the separate `oteryn-client-ff934b4e\target-settings-aa`
directory; the owner's running `985fe6b4` binary was not closed or overwritten.
Recovery bundle: `/workspace/oteryn-client-publication-28565f4f.bundle`, SHA-256
`1f6569da680a33c2dbce9885d79fbd9ec1eb4dd4070315f8df35ce6510a45bcc`.
This proves a working sampling consumer, not complete original-client visual
parity, real lightmap composition or every physical Apply/Cancel interaction.

## Continued nine-lane work

The next bounded batch connects Chat Text: Select All to the real composer using
upstream egui selection state, rather than retaining an unconsumed binding. Tests
select the complete Unicode draft and replace it through the real TextEdit,
preserve the recipient, reject unavailable/read-only contexts and verify Ctrl+A
is bound only in the observed default Chat On map. Interface now applies the
source-confirmed inventory OR container expiry dependency; all expiry controls
remain disabled because no authoritative item-expiry projection exists.
Exact source successor `9e2d0cbcf0b9059683119609c55187c8bc11726a` passed native
MSVC client tests (196/175/5), renderer tests (40, including DX12 pixel readback)
and strict all-target Clippy. A fresh separate-directory DX12 release audit
passed 20 pages/78 captures/zero invalid entries; Interface, Console and General
Hotkeys captures 01/02 were inspected. Private audit directory:
`C:\Users\barte\Downloads\oteryn-settings-audit-9e2d0cbc`; manifest SHA-256
`970077ae12e479faf501a1f8b2f283032426672103be28709555a2541e2b9b45`.
Recovery bundle `/workspace/oteryn-client-publication-9e2d0cbc.bundle`, SHA-256
`7ec2823c9617e653f5b6fd7d1452c865496f659a4736831f3fbc1aedb45d212f`.
The prior owner-launched client (PID 12896) remained responsive at login.

The same batch corrects the Windows `egui-winit` feature selection: explicit
`clipboard` enables the mature upstream OS backend instead of its fallback string.
No native clipboard contents were inspected or overwritten during this audit;
external clipboard transfer remains an interactive acceptance item. Dependency
resolution changes Cargo.lock. The World input pin was refreshed to
`8bc3d815169e801cfb374e2f02161a8a3262ff2806324ba91a3e2e8b6ca1cd84`;
pin-check rebuilt the unchanged 25,086,645-byte World bundle twice, retaining
digest `f8b11ebcd96a4140f71dcbb5944519434d3a8e6b0b0f2ecf9fecb9c74f67b6c8`.
Linux client tests pass 197/171/5, strict Clippy, formatting and governance
(60 tests) and exact-candidate impact-routing regressions pass. The added
clipboard dependencies compile under MSVC; system clipboard transfer and
exhaustive physical input/reference parity remain unqualified.

No lane is claimed 100% complete. Remaining activation requirements are:

| Lane | Missing input or consumer |
| --- | --- |
| Graphics | Runtime engine capabilities/switching and original live image/gamma comparison |
| Effects | Floor/light/environment and spell-effect projections; real compositing |
| Audio | Licensed playable assets, device/mixer backend and authoritative sound events; the pinned 15.30 manifest contains no sound files |
| Controls | Mouse/loot consumers; diagonal movement requires an accepted contract beyond the four cardinal wire directions |
| Action bars | Authoritative learned-spell/object assignment sources, cooldown/count metadata and reachable pickers |
| Interface/HUD | Contextual cursor states, safe link-copy warning, loot values, expiry and condition projections |
| Console/Game Window | Accepted filter/participant/private/secondary routing and clipboard semantics |
| Misc/Gameplay | Supported purchase/container/inspection/chase/loot operations and accepted remembered-sign-in lifetime policy |
| Screenshots/Help | Accepted automatic event pipeline, supported destinations and original-compatible minimap/import formats |

The existing session event inventory has spatial/entities/objects/vitals/combat,
chat/rooms, inventory and container projections. It does not supply the missing
learned-spell, cooldown, condition, light/environment or screenshot-trigger
catalogues. Original reference metadata is not authoritative Oteryn session data.
Changing those server/protocol/identity contracts requires the owning contract
and review; this client authoring task cannot silently allocate them.

## MAP-WIRE-1 preparation — 2026-10-11

The existing accepted server map codec is reused via Session re-exports, not
reimplemented. The bounded client model stages snapshot/delta updates atomically,
removes tiles and handles leaving the window, retains server floor/roof visibility
and rejects accumulated handle/size overflow. Targets come only from current
`BaseOrdinal` revision/bundle keys or item handles; display-only and hidden entries
have no target. Refused updates invalidate all drawing/targets until a valid
snapshot. No missing ground speed or truncated stack entry is invented.

Eight focused tests cover these properties. Linux client tests pass 205/171/5,
Session tests pass 58, strict all-target Clippy and formatting pass for both
packages, workspace boundaries pass, and governance validation/60 tests pass.
This is not a live map implementation
or settings-lane acceptance: capability 18 stays unadvertised, and transport
resync, streamed sprite rendering, real pointer targeting and the joint server/
client ground-speed switch remain pending. MAP-WIRE-1 does not allocate lighting,
weather, sound or spell/HUD projections; those controls remain unavailable.
Source head `fe3eec7283cbe61194999d9db6bde0935bd8d70f` independently passes
MSVC client tests 204/175/5, Session tests 58 and strict all-target Clippy for
both packages. A fresh DX12 release audit passes all 20 pages/78 captures with
zero invalid entries at 125% DPI; every capture digest was verified. Private
audit directory: `C:\Users\barte\Downloads\oteryn-settings-audit-fe3eec72`;
manifest SHA-256: `eef44c25cbece3e84c51b75b624b624b8611b21e3059f3beaf30ae91cbf04ff2`.
The owner's existing login client (PID 12896) remains responsive and unmodified.
Recovery bundle `/workspace/oteryn-client-publication-fe3eec72.bundle`, SHA-256
`1e86027a61e5e686928531b2452b21070225a2dc21b86fc1b105ac3596439987`.
The same-head protected-base audit passes; this is not full repository CI or
live MAP-CLIENT-1 acceptance. The reused codec's latest producer commit is
`0e834533323a032893b51b9bca748d3ba8d56aa6` (MAP-CUTOVER-1b, #1916),
and its bytes match the fetched `origin/main` codec.

Existing server spell definitions, learned-cast facts, cooldown deadlines,
condition store and presentation outbox should be reused when their owning
transport contracts are accepted. They are not absent implementations, but the
current client event stream does not expose their missing projections. The
spell/condition presentation publishers are explicitly unallocated candidates,
not active wire routes. The starter SpellBook has three spells, not the original
reference's full catalogue. Map capability 18 does not solve these independent
gaps or authorize activating them.

## MAP-CLIENT-1 session integration — AUTHORING

The Session consumer now recognizes selected capability 18 and strictly requires
its dependencies 4/6. The atomic join must carry map domain 17 instead of object
domain 2, with the admitted spatial content generation. Unselected, missing,
mixed or incorrectly typed map domains are refused. Pushed map deltas go through
the existing generation/sequence/revision discipline and a typed event; the current
map header/revision advance only after successful canonical decoding.

A binding mismatch sends the existing fenced, unsequenced FND ResyncRequest with
the previous applied sequence and map revision, drops retained map state/events,
and fails the session closed. Recovery uses a fresh admission snapshot; no new
mid-session snapshot route is invented. The added protocol encoder implements
foundation.proto without changing its schema or registry. Four duplex-stream
regressions cover actual admission, delta routing, dependency/domain rejection
and the emitted resync bytes. Protocol encoder tests exercise ingress validation,
zero/duplicate domains and the 256-domain bound. The dev adapter maps the new
error exhaustively; the fixture harness retains its existing capability set and
does not constitute a bundle-map E2E test.

Capability 18 remains unadvertised by the production client until drawing,
targeting and the joint speed switch are connected and qualified. Previous
Windows evidence does not qualify these new consumer changes. Final frozen-head
independent review remains required; this draft is still AUTHORING. Linux tests
pass: client 205/171/5, protocol 214, Session 62, dev adapter 34 and harness 36.
Strict all-target Clippy passes for all five packages, formatting and workspace
boundaries pass, and governance validation/60 tests pass. Windows qualification
completed for source `e63576875b6c8db741f521f414c7814cd52da3cf`: MSVC client
204/175/5, protocol 214, Session 62, dev adapter 34 and harness 36 tests pass,
as does strict all-target Clippy for all five packages. Fresh DX12 release audit
passes 20 pages/78 capture hashes/zero invalid entries at 125% DPI; the prior
owner client (PID 12896) remains responsive. Private audit:
`C:\Users\barte\Downloads\oteryn-settings-audit-e6357687`; manifest SHA-256
`5ea6657fc0dcbaf765c183c68ee6836148ee3f35092ec5334dcf9de3efacdaea`.
Recovery bundle `/workspace/oteryn-client-publication-e6357687.bundle`, SHA-256
`95a8047243f801f8d644e828885522e20fe0d7101efe7e97537c5db3b5452d38`.
Same-head protected-base audit passes; full repository CI and bundle E2E remain
unqualified. This evidence qualifies Session reception, not map drawing.

## Streamed map draw composition — AUTHORING

Map stacks now resolve through the existing hash-pinned 15.30 appearance/sheet
loader without reading a local map, bundle palette or placements. The draw
composer emits the existing bounded SpriteFrame/AtlasPage batches in ascending
floor/y/x order, shifts floors in perspective and preserves received stack order
inside ground/border/bottom/common/top phases. Actor cells enter after common
items and before top items, including actors on omitted tiles. A walking actor
uses the later source/destination tile phase with its supplied native offset.
This is presentation data, not a new movement authority or wire projection.

Missing/refused appearances emit a real placeholder quad without falsely raising
the oversized-entry counter. The view retains right/bottom neighbour anchors for
large up-left sprite cells. Whole-frame splitting and LRU residency reuse the
existing renderer pipeline rather than a new atlas/cache. Failed sheet uploads
must overwrite reused slots with placeholder pixels, never stale texture data.

Five regressions cover phases, floors/hidden actors, omitted/walking actors,
neighbour overlap and missing/unknown/real appearances. The real pinned
appearance 104 is sixteen position patterns of one cell each, not a sixteen-cell
object; its actual sheet cell is read and checked. Linux client tests 210/171/5,
Session 62, renderer 39 and asset tests 2/13 pass; strict Clippy passes for the
changed Rust packages. This is draw composition only. GPU sprite-page uploads,
the play-view consumer, picking and joint speed switch remain pending; capability
18 stays unadvertised. Prior Windows evidence does not qualify this batch.

Source `108d99f8239b99c6dfc807c460eca9944cba389f` subsequently passes native
MSVC client 209/175/5, Session 62, protocol 214, dev adapter 34, harness 36,
renderer 40 and assets 2/13 tests, plus strict all-target Clippy for all seven
packages. The exact-source DX12 audit passes all 20 pages, 78 independently
hashed captures and zero invalid entries at 125% DPI. Private audit directory:
`C:\Users\barte\Downloads\oteryn-settings-audit-108d99f8`; manifest SHA-256
`45e32a92c8c750aedf5581df9401aadfd8d006076cc313143833688837da3371`.
Recovery bundle `/workspace/oteryn-client-publication-108d99f8.bundle`, SHA-256
`e21387d64ca1e350ad218f04116c11e1336bdf69372c143de8132784fefdc275`.
The owner's running client remains responsive. Same-head protected-base audit
passes; this is not full-CI or live streamed-map qualification.

The next AUTHORING batch adds bounded GPU sprite-page consumption: up to four
ordered instance buffers, real subrect uploads and CPU backing retained across
surface acquisition/recreation. Invalid or duplicate cell uploads fail before
mutation, and ordinary uploads cannot overwrite the placeholder row. Static
scene composition, AA, UI and captures retain their existing paths. A physical
DX12 readback regression checks later-batch pixels, overwritten cells and a
recreated texture. It requires native execution before it counts as evidence;
the live play-view consumer is still pending and capability 18 remains off.

GPU source `8ae5a9d0c837aa311ddeb6d3193256284cedfbc6` passes MSVC strict Clippy
for all seven selected packages, their tests and the physical DX12 page readback
regression. The exact-source release audit passes 20 pages/78 independently
hashed captures/zero invalid entries at 125% DPI, without replacing the owner's
responsive PID 12896. Private audit `C:\Users\barte\Downloads\oteryn-settings-audit-8ae5a9d0`;
manifest SHA-256 `a04a84d7d70e1681ae485b2958e3b4641ad8759974377e2527102fde05c1f7b3`.
Recovery `/workspace/oteryn-client-publication-8ae5a9d0.bundle`, SHA-256
`c29f3298c6f7aae442c2860755391ca2b1ae449574227b8b87a63ecb3fb794cc`.

The bounded map store now belongs to Session, reusing the previous eight model
regressions rather than maintaining a second transport state implementation.
Admission and deltas enforce whole-view tile/handle budgets before revision
advance. A real Session regression verifies replacement, clearing and non-150
received ground speed. The play mailbox coalesces full validated map projections
at changed revisions instead of retaining/dropping arbitrary delta tails; an
undrained-mailbox regression covers latest-wins and before/after-step ordering.
PlayView holds the join/current map, but streamed scene drawing, picking and the
joint client/server speed switch are still pending. Capability 18 stays off.

Map-store source `8725984429e361aba5306263e1294721c79ce44b` passes MSVC client
202/175/5, Session 71, protocol 214, dev adapter 34, harness 36, renderer 43 and
assets 2/13 tests, plus strict all-target Clippy. The exact-source DX12 audit
passes 20 pages/78 independently hashed captures/zero invalid entries at 125%
DPI; PID 12896 stays responsive. Private audit:
`C:\Users\barte\Downloads\oteryn-settings-audit-87259844`; manifest SHA-256
`22e06f92c2fc98f0e31c913170b1783bc30211b20cccba33fac645eb850cbad0`.
Recovery `/workspace/oteryn-client-publication-87259844.bundle`, SHA-256
`1213b75eb9045c79b4adee6d4bae379e6de003fd480c8f2d0f6f1466b4e28eef`.

The next AUTHORING batch connects server-map composition to PlayView, Scene and
the real Windows GPU page/capture path. Local start-area anchoring and its
minimap image cannot replace a streamed map. The page retains latest pending
pixels across multiple unpresented rebuilds; a new renderer resets residency and
reuploads visible cells. Login return restores the static atlas. Missing assets
overwrite reused cells with owned placeholder pixels, and the target frame uses
the existing owned yellow outline, not a fabricated appearance reference.
Three regressions exercise retained uploads, recreation, placeholder pixels and
the play-view coordinate/atlas transition. Actors without an accepted visual
binding remain explicit placeholders. Floor-correct picking, movement presentation,
bundle E2E and the joint ground-speed switch remain pending; capability 18 stays
unadvertised. Settings captures do not qualify live admission/drawing by themselves.

Scene source `9d43a594b893c52098014921667f89c09031db96` passes MSVC strict
seven-package Clippy/tests and exact-source DX12 settings audit: 20 pages, 78
independently verified capture hashes, zero invalid entries, 125% DPI. Private
audit `C:\Users\barte\Downloads\oteryn-settings-audit-9d43a594`; manifest SHA-256
`40327b9c35bc88d697d5586137e0ac1c383ae4d3fa48b762d2a18173482a01a2`.
Recovery `/workspace/oteryn-client-publication-9d43a594.bundle`, SHA-256
`035532071ab1fa92227f52e8d6c4f421121e82b9fcc9f3ebd5bb9717ec1022ad`.
The owner's PID 12896 remains responsive. This still does not qualify bundle E2E.

Current AUTHORING adds floor/perspective-correct map picking, topmost received
origin-only USE targets and an actual secondary-click consumer using the existing
input router and accepted USE_INTENT encoders. A display-only top entry cannot
fall back to an actionable lower stack/floor. Visible actors are protected from
upper-floor picking, including battle cycling. Before sending a queued USE,
Session rechecks that the target still names a topmost visible current entry;
departed/replaced targets send no frame and reserve no command ID. Duplex tests
verify exact base-key/revision and item-handle wire payloads, not just serialization.
Input regressions cover hidden roofs, handles leaving the view and modal/focus
suppression. This does not enable all original mouse presets or loot semantics.

The joint ground-speed switch is not yet safe to activate: the accepted map
projection carries per-tile ground speed, but admitted actor/entity/vitals and
paced step-result codecs expose no effective actor speed or step-duration
deadline. Production speed depends on level, conditions and equipment, not a
constant 110. Reusing the shared checked-in speed table is correct only after
that missing authoritative input is supplied under an accepted owning contract.
Do not guess 110/150, allocate a new field here or change only the server source.
Capability 18 therefore remains unadvertised; existing server pacing stays intact.

Input source `c302e2555c83aeeb51481d324663603304420140` passes Linux client
209/171/5, Session 72, dev adapter 34 and harness 36 tests, strict Clippy,
formatting, workspace boundaries and governance. MSVC client 208/175/5, Session
72, protocol 214, dev adapter 34, harness 36, renderer 43 and assets 2/13 pass,
with seven-package strict Clippy. The exact-source DX12 audit verifies 20 pages,
78 capture hashes, zero invalid entries and 125% DPI. The owner's PID 12896 remains
responsive. Private audit `C:\Users\barte\Downloads\oteryn-settings-audit-c302e255`;
manifest SHA-256 `2ce3bf80c6b6c4527912e7a408d7c9eb0f42f626f88f58e161a6daa661bc1731`.
Recovery `/workspace/oteryn-client-publication-c302e255.bundle`, SHA-256
`7ca7ff19086db198b3ee6b82b17e220edc756e68a4a2cef451f2a2be78185763`.
Same-head protected-base audit passes, not full repository CI or bundle E2E.

## Help ZIP export continuation — AUTHORING

Source-first Help inspection confirms ZIP SaveFile/OpenFile filters, not the
archive contents. The client now dispatches export after the egui frame to an
upstream `rfd` picker owned by its Windows window, both at login and during play.
The independently authored ZIP contains only `oteryn-options.json`, using the
existing bounded, validated preference serializer. No account directory, cookies
or minimap are copied. Upstream `zip` and `tempfile` provide encoding and atomic
destination replacement after complete writing and sync. Cancelling preserves
the draft/message/current settings; errors preserve previous exports; the active
preference destination is refused, including before its first save.

Four new regressions cover the real export-button dispatch, ZIP readback/legacy
retention, failure preservation and active-file protection. Linux client tests
pass 209/175/5, strict Clippy and whole-workspace formatting pass, boundaries and
governance/60 tests pass. The dependency lock refresh changes the World input
pin to `5e2707acfa842f0d96590b573b1ca7d589b5c8c73a564b8b41d09e4b014a003d`;
release pin-check rebuilds the same 25,086,645-byte bundle twice, preserving
`f8b11ebcd96a4140f71dcbb5944519434d3a8e6b0b0f2ecf9fecb9c74f67b6c8`.
Source `7898fdf79a54d969d0114878f1d114cb26d73ee8` passes native MSVC client
208/179/5, Session 72, protocol 214, dev adapter 34, harness 36, renderer 43 and
assets 2/13 tests, plus strict seven-package Clippy. The exact-source DX12 audit
passes 20 pages/78 independently verified hashes/zero invalid entries at 125%
DPI; the Help capture was visually inspected, and PID 12896 remains responsive.
Private audit `C:\Users\barte\Downloads\oteryn-settings-audit-7898fdf7`;
manifest SHA-256 `547fff2e0bbe28de3842207612a4f3ea730fb0e061b15d70dea10179ea71065c`.
Recovery `/workspace/oteryn-client-publication-7898fdf7.bundle`, SHA-256
`1c257df208f77d3bc7759bf0f1f1dde6a622d8a54a4fdc51c912d82e4bb2fbac`.
Same-head protected-base audit passes, not full repository CI. Native physical
picker acceptance, original archive interoperability and the combined minimap
importer are not established by serialization or settings-page captures.

## Immutable chat-output selection — AUTHORING

Received chat rows now use upstream egui's immutable `&str` TextBuffer instead
of non-selectable labels. Primary dragging selects across Unicode lines and Copy
emits the actual selected text, while typing/paste/cut cannot edit received
history. Channel IDs keep selection state separate; a shortened/cleared view
cannot copy departed lines. Legacy font size and the existing receipt-clock/
sequence-cutoff projection are preserved. Output focus is not a gameplay/chat
composer route and is explicitly surrendered when chat is hidden.

Four regressions exercise pointer dragging, actual Copy commands, immutable
rendered text, channel isolation, stale selection and native input/focus gating.
Linux client tests pass 209/179/5, strict Clippy and formatting pass. Source
`34f5921499960120e726df4100211ebe614d5a5d` passes native MSVC client
208/183/5 and strict seven-package Clippy/tests, plus DX12 20 pages/78
independently verified hashes/zero invalid entries at 125% DPI. PID 12896 is
preserved/responsive. Private audit
`C:\Users\barte\Downloads\oteryn-settings-audit-34f59214`; manifest SHA-256
`8254099676642bad3d7009889970565b50dce6df7dd5337f942566c16ef296e4`.
Recovery `/workspace/oteryn-client-publication-34f59214.bundle`, SHA-256
`7aab52ae5d7834ad40fe5783f7ba0c56d453d051eac8172c564f7a8c6578c03b`.
Same-head protected-base audit passes, not full CI. Live-session physical
selection and external OS clipboard transfer remain unqualified.
This does not implement rich-text NPC keywords, external-link warnings, remapped
Chat Copy actions, private/secondary consoles or unsupported event filters.

## Options footer, keyboard and page Reset continuation — AUTHORING

Recovered OptionsMenuDialog/TibiaDialog source fixes visual and traversal order
to Ok / Apply / Cancel, makes outer Escape discard the draft, and leaves unfocused
Enter without an implicit save action. Popups, profile/action dialogs, Help reset
confirmation and binding capture retain first ownership of Escape; Stop All can
still capture Escape. Small-window recovery uses bounded/truncated footer labels.

TibiaOptionsPage routes ordinary Reset to the current page, not all options.
The local implementation restores only implemented controls to Oteryn defaults,
preserving unrelated pages, unavailable intent, assignments, bindings and locks.
Active-profile action-bar visibility mirrors remain consistent. Profile-table,
Help and wholly unavailable page resets are disabled, not counted as working.
Original factory values and profile-table reset semantics remain unverified.
Actual footer-click, draft/current isolation, resize and inner-dialog regressions
are retained. Linux client 209/184/5, strict Clippy, formatting, workspace
boundaries and governance validation/tests pass. Source
`a95a92dccf8deb8ae89ad75e0652dd02ba9e89e0` passes MSVC client 208/188/5,
Session 72, protocol 214, dev adapter 34, harness 36, renderer 43 and assets
2/13, plus strict seven-package all-target Clippy. Native process 36568 exits
zero (98.25 seconds). Exact-source DX12 audit verifies 20 pages/78 capture
hashes/zero invalid entries at 125% DPI, with owner PID 12896 responsive.
Graphics, General Hotkeys and Help captures were inspected: footer order is
correct and unverified table/Help page Reset remains disabled.
Private audit `C:\Users\barte\Downloads\oteryn-settings-audit-a95a92dc`;
manifest SHA-256 `016e6e18d3b470f3ad461bd61438f7caa1de4ca70b55edfbb1518f0f188123f0`.
Recovery `/workspace/oteryn-client-publication-a95a92dc.bundle`, SHA-256
`256d1f1ad4fdd669d489c0f97e27a2b33c331e226e36513043ee3b5756c21836`.
This is not 1:1 completion, physical keyboard/picker acceptance, live bundle E2E
or proof of every page's runtime consumers. Final independent review/full CI
remain open; protected-base audit alone cannot establish them.

## Latest-source Windows launch — 2026-10-12, AUTHORING

Owner requests the latest PR source, not the installed `gc177231dee65` client.
Exact source `6c3a884a7066a25ef3935efbdebf98fcb1a1f609` is fetched and checked
out in a separate Windows directory; tracked code and the existing installation
are not modified. A filtered shallow sparse checkout covers the workspace build
inputs after two owned unproductive downloads were stopped; partial directories
are preserved. Rust 1.95 builds the locked x86_64-pc-windows-msvc release.

Private directory:
`C:\Users\barte\Downloads\oteryn-client-6c3a884a-729b8d1e3a93483c995c272d78183a54`.
Binary SHA-256 `073bb054698dc8e9bd2eca226fec84a88e09b030680aac91cf649170bc05133d`.
The exact-source DX12 audit verifies all 20 pages, 78 independently hashed
captures, markers, unchanged drafts, footer controls and width bounds at 125%
DPI. Manifest SHA-256
`efbefe76af77b8b8998664017bdf03f0faf82cb846edf8fc7d1cd8a5442613a6`.

The preprod launcher starts that exact executable as PID 37844; its verified
path matches the build and its window `Oteryn - logowanie` responds. The existing
public-only connection folder now has `Start-Oteryn-latest.cmd` pointing to this
binary: `C:\Users\barte\Downloads\oteryn-preprod-login-20261012-6fb03369a0bf45e290ed32340e706a08`.
The old installation/starter remain intact. No automatic sign-in,
credential consumption or Game/Platform restart is performed. This proves the
latest application is running, not server admission, 1:1 completion, full CI or
final independent review. The prior empty directory/reset evidence remains an
unresolved backend acceptance gap, not missing connection data.

## Ordered composer Copy and supply-chain repair — 2026-10-13

General `chat.text.copy` now has an actual composer consumer, superseding the
earlier unsupported-Copy status. Both context maps and remapped bindings resolve;
the native shell claims it only for a focused, visible, nonmodal, writable
composer. Recipient fields, received output and other editors keep upstream
clipboard handling. It appends upstream `egui::Event::Copy` at the native key's
position in pending egui-winit input, not a previous-frame selection snapshot.
Actual Unicode/reversed selection and preceding text/selection/Copy/following
text produce the expected `CopyText` without sending chat, modifying selection
or stealing focus. Empty/stale selections, hidden/read-only chat and lost focus
are covered. Linux client tests pass 210/188/5; strict all-target Clippy and
formatting pass. Native claim/repeat/release regression is added but unexecuted.
External clipboard transfer, original link-warning semantics and physical
Windows acceptance remain unqualified: the bounded PC call reports offline.
Old MSVC/DX12 manifests do not qualify this source change.

The actual red CI cause at `6c3a884a` was BSL-1.0 rejection for the two locked
Windows clipboard adapters, not draft status. Exact-version exceptions and
verified provenance are documented in `docs/repository/LICENSING.md`, without
widening the global license allow-list. Local cargo-deny passes advisories,
bans, licenses and sources; fresh remote CI still needs its own result.

Read-only backend checks supersede the previous empty-directory/TLS-reset
observation: the public directory now lists Oteryn Staging and its channel;
the supplied private-LAN gameplay endpoint completes TLS 1.3 with the supplied
public CA and ALPN `oteryn-game/1`. This is not sign-in, ticket consumption,
admission, movement or chat acceptance. Main `0db5c53b` adds no active client
spell/cooldown/condition/light/sound/speed projections; unallocated candidates
and unoffered capabilities are not enabled by this client task.

## Bounded Oteryn options import — 2026-10-13, AUTHORING

The reference combined import row dispatches a parent-owned native OpenFile
picker after the egui frame, at login or during play. Only the existing exported
`oteryn-options.json` flat Stored ZIP is accepted, with an 8-MiB settings bound,
bounded archive input, exact entry name/type, regular-file checks and indexed
plus streaming CRC/local/central validation. Unknown/original/minimap formats,
paths, symlinks, encryption, other compression, duplicates and oversized or
invalid settings are refused. No archive entry is extracted into user folders.

Upstream zip 9.0.1 deduplicates indexed names and its stream visitor swallows
central parse errors. Actual duplicate/interrupted-record regressions establish
these limitations. A minimal guard checks upstream-parsed offsets and the exact
flat ZIP32 envelope written by our exporter; no ZIP parser/fork is introduced.
Both local CRC/size mismatch cases and actual corrupted payloads are covered.

Import parses through the existing settings migrations/validator in a private
temporary directory, then changes draft only. Picker cancellation/error preserves
draft/current/files. The real Cancel click discards; a child-process fixture
clicks real Apply and exercises ordinary persistence without changing the
parent environment or the user's preferences. Linux client 210/194/5, strict
Clippy, formatting and diff checks pass. Windows picker/clipboard, fresh DX12
pages and live admission remain unqualified while the PC is offline.

TLS acceptance additionally verifies the exact SNI hostname and previously
supplied public-CA fingerprint, not just chain trust; no trust store is changed.
REA Ghidra is available with command-scoped `GHIDRA_INSTALL_DIR` pointing to the
existing 12.1.4 installation. The ELF/pwntools route remains unconfigured; no
full auto-analysis timeout is repeated and no new original behavior is claimed.

## Main conflict reconciliation — 2026-10-13, AUTHORING

After the two published client batches, PR #1942 reports mergeability `dirty`.
Ordinary pull_request workflows do not run on conflicted heads; successful
pull_request_target authority audits did not establish full CI. A normal merge
of protected main `c754f847b9a69084afa2d401712b13b663885347` preserves every client
change. The sole textual conflict is the World input digest. Main's accepted
bundle/content/entry start are retained, and inputs are recalculated from the
merged Git index to `718013fb67a60b5359fd7e540a59860e19149a77ac908f2c5aeb48e26f8156b6`.
Release pin-check double-builds the unchanged `5c32d7ac` bundle, 25,097,259 bytes.

Staged comparison proves client, Session, protocol, Platform client and
Cargo/toolchain bytes unchanged from `11d92f90`. Existing focused local evidence
retains that limited scope; native Windows/E2E/final CI/review still need their
own qualification. New main producer work is reused without enabling unoffered
capabilities or inventing missing presentation/speed data. Earlier main/pin
qualifications remain historical evidence, not the current integration state.

## Accepted client-completion continuation — 2026-10-13, AUTHORING

Owner/CP [decision 6143183755](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6143183755)
supersedes the earlier unresolved speed-design question, not its missing runtime
proof. STEP-PACE-WIRE-0 is separately owned in #2034, followed by server emission,
map activation, receipt consumption and native E2E in that order. No new wire
field, protocol ID, capability activation or guessed speed is introduced here.

The actual PlayLink boundary now reserves one step atomically before consuming
view input. Repeated/direct calls cannot enqueue another step or overwrite the
single undrained outcome. Completion and unrelated pushes do not release it;
the shell must consume Stepped first. An ended link remains terminal after its
one Ended notification, and a closed/enqueue-racing channel fails closed without
reopening the gate. Pending keyboard/click direction and before/step/after
projection order are preserved. An actual delayed run_session fixture measures
one Stepper invocation until the outcome is polled, then permits the second.
This interim gate is not smooth animation or authoritative step timing.

Independent duplex tests exposed the formerly discarded capability mismatch.
Session now preserves strict bootstrap code 1007 as terminal update-required,
like 1116; native local TLS/login composition proves one admission and one
Gateway login, without automatic retry. Canonical 1007 is SESSION_FATAL, whereas
N8/1116 is TRANSPORT_FATAL. Crossed dispositions, non-zero correlations and
non-bootstrap envelopes are refused; the existing protocol registry/table is
unchanged. A first incorrect shared TransportFatal assumption failed the actual
Session regression and was repaired using the canonical server encoder and
independently encoded fixtures. Both queue and refusal defects have RED/GREEN
evidence, not just serialization tests.

Normal upstream composition additionally preserves protected main
`2daf1e4cc5d09fb2b0d373f4e8eb73bdde20fead`, including accepted bundle combat,
chat/NPC transport and the NPC-placement compiler repair. The sole conflict is
the World input fingerprint. The accepted `e6451f92` bundle, start/identity and
preprod digest stay intact; the merged inputs become `744b6cad`. A fresh release
pin-check double-builds the identical 25,089,885-byte bundle. Client, Session,
protocol code, Platform client, Cargo and toolchain bytes are identical to the
published guard source `1562c11e`; its component proof retains only that scope.
Repository policy (23 files/63 workflows), routing regressions and governance/60
pass. This is not a deploy, live NPC/E2E or new native-page qualification.

Linux client 217/194/5 and Session 74 tests pass, as do strict all-target Clippy,
formatting, boundaries, governance validation and all 60 governance tests.
The predecessor 1bd4a7a5 has green exact-head PR CI, including the repaired
supply-chain gate; those results do not qualify this successor. Fresh physical
Windows acceptance is unavailable: the bounded PC connectivity call reports
offline. Existing binaries, preferences and native captures are unchanged.

## Received Battle-panel health — 2026-10-13, AUTHORING

The reachable shortcut/hotkey Battle window now renders the admitted capability-6
Actor.health_percent instead of claiming health is missing. It reuses the existing
primary HUD's upstream 4-point progress-bar style, retains type/search filters,
and does not retain bars after actor removal. Names remain explicitly unavailable;
original populated-row geometry, colours and eight sort semantics are not claimed
complete. No fabricated name, default, clock or new capability is introduced.

Actual egui output through ClientPanels.open/show verifies 0/37/100 percent,
replacement to 81, removals, different player/creature/NPC fractions, search and
object exclusion. NPC fixtures obey the accepted codec's mandatory 100 percent.
A controlled removal of the consumer fails the warmed-window shape test;
restoring it passes. Headless paint evidence is not a native Windows/reference
comparison. The PC remains unavailable, and the wider 1:1/module gates stay open.

## Accepted NPC/consumable producer composition — 2026-10-13, AUTHORING

Normal composition of source02170282 with accepted protected maina7a0b374
preserves all client/Session/protocol/Platform-client/Cargo/toolchain bytes.
Main adds monster AI (#2036), consumable content (#2031) and read-only NPC trade
(#2035). The sole conflict is World inputs_digest; the qualified pin route yields
86d18a80ebe412b8857ee564067f2d646c86c8134a9e2d44515e73e0340b739d.
The release pin-check double-builds25,089,897 bytes with main's unchanged bundle
c49538eb1ed44e9c02a9a8229e80cf4a169c7afb11803d5120c57dc7eef48524.
Main's content revision, entry_start, identity and preprod topology are retained.
Fresh Linux client217/196/5 and Session74 tests, strict all-target Clippy and
formatting pass. Repository policy23files/63workflows and governance/60 pass.
These are source/component checks, not compiled Windows interaction evidence.
CI on021 had only protected-base audits because this source conflict prevented
ordinary PR runs; it is not evidence of a failed client test or a green candidate.

The new producer genuinely opens domain8 offers after command7 `trade` in an
active NPC conversation; ChatIntent::Say is not that command, and command8
purchase/sale remains rejected. Session has no typed NPC conversation/trade
stores or talk API yet. Merely advertising capability3 would ignore snapshots
and reject pushed domains7/8. Bundle admission also requires capability18, whose
activation remains ordered after STEP-PACE-WIRE-1 by accepted CP decision
6143183755. Defaults stay [4,6,7,13,17]; no invented item names, buy/sell behavior
or original NPC-window geometry is claimed. A subsequent allocated consumer must
route both domains, strict independent revisions and command7 before enabling3.

#2034 remains unmerged and has no integrated server emission; map/pacing/native
acceptance therefore remains open. Its decoder must reach client artifacts before
the server emits step_wait_ms: legacy cap13 clients fail closed on that extension.
This compatibility ordering is recorded for CP in comment6144441159, not a new
protocol ID or deployment authorization. Existing Windows evidence does not
qualify this source; physical PC/E2E and final same-head CI/review remain open.

## Windows picker repair and accepted receipt codec — 2026-10-13, AUTHORING

The PC connectivity check now succeeds. Exact source9c342779 is fetched into a
separate detached sparse worktree; the owner's dirty checkout and old installation
are not touched. The first physical attempt fails formatting before compilation
because inherited core.autocrlf writes CRLF. Preserve that directory and create
another owned worktree with worktree-only core.autocrlf=false/core.eol=lf; LF fmt
passes. The next real MSVC release build fails E0599: the import ZIP picker calls
FileDialog.open_file, which does not exist in pinned upstream rfd0.17.2. Replace
only that call with its supported pick_file API. Native sibling API inspection
finds no other mismatch; Linux tests did not compile this Windows-only call.
Failed attempts remain evidence, not successful Windows qualification.

Accepted STEP-PACE-WIRE-0 (#2034) is now merged at59f9fb12 and is normally composed
without editing its protocol/registry bytes. Current Session automatically uses
the strict extended cap13 decoder. Independent raw duplex fixtures exercise the
maximum111150 MOVED receipt with its actual claimed spatial delta, TOO_EARLY1,
legacy selected/unselected results and sequence/command state. Wrong capability,
BLOCKED/REJECTED receipts, explicit0, offbeat/over-max/duplicate/unknown fields and
over-bound payloads must poison Session and reject further steps locally.
No second positive step is sent by these decoder tests: StepOutcome still discards
wait and retrying still uses its historical backoff. Full receipt-time deadline,
resets, smooth map interpolation and native gameplay proof remain the separately
allocated consumer; syntactic decoder compatibility is not emission readiness.
The c49538eb bundle/pin and current default capabilities are unchanged.
Fresh Linux client217/196/5, Session76 and protocol216 tests, strict all-target
Clippy and formatting pass. Pin-check again double-builds25,089,897 identical
bytes; governance/60 passes. This does not replace corrected MSVC qualification.

New visible coordinator handoff6145031564 links the actual branch files, exact
source and four execution needs. Publication is verified, not CP acknowledgment.
CP acknowledgment6145246782 accepts default-OFF server emission and serial
WIRE-1→MAP-CLIENT-1→STEP-PACE-CLIENT-1→native, with NPC afterward. Its CC report
link mistakenly points to our original dependency request; fresh read proves no
CC/MAP-CLIENT-1s allocation there. Correction6145306643 requests the exact source,
without repeating owner design questions or opening a competing server writer.

Corrected source5d1dee8eea012f7cdf386ce57395f2c033847482 now builds the exact
Rust1.95 locked x86_64-pc-windows-msvc release and passes strict all-target client/
Session Clippy. The first unit attempt has eight missing-input failures because
the sparse checkout omits pinned asset manifests/placements. Add only the accepted
content/assets/files, content/world/placements and checksum manifest directories;
the same clean source then passes client216/201/5 and Session76. This is an explicit
fixture/environment repair, not erasure of the failed attempt or a code bypass.

Private source/build directory:
`C:\Users\barte\Downloads\oteryn-client-5d1dee8e-20261013-native`.
Release identity0.1.0+dev.ci.g5d1dee8eea01; executable SHA-256
90b89b9aeeae0ddd86f1ae6be355a6a15e6a7d8aaf1134006f29a9408934add9.
Actual DX12 audit passes20pages/78captures/zero invalid page entries at125% DPI,
with every capture hash independently checked; markers, widths, footer and draft
non-mutation are checked. Output:
`C:\Users\barte\Downloads\oteryn-settings-audit-5d1dee8e-20261013`;
manifest SHA-256d0c1ab2305d520ad86f15fee014dd1ddb52e911500040a75ab20a6038a5ab7d2.
Help/Graphics captures are visually inspected. This is not original visual parity,
physical picker/input acceptance, full runtime consumers or live gameplay E2E.

The existing public-only preprod connection files are reused through the exact
source's validated launcher, with four public child settings and no automatic
login. Exact executable5d is running asPID37764; path/hash, nonzero native window
handle, title`Oteryn - logowanie` and responsiveness are verified. Old installation,
source checkouts, original starters and preferences are preserved. No account,
Game/Platform restart, trust-store, deployment or production operation occurs.
Source/component evidence remains bound to5d; later documentation-only successors
retain it only for byte-identical code, not final head/CI/review qualification.

## Map activation fixes — 2026-10-13, AUTHORING

Owner continuation retains the existing client writer. CP6146799516 withdraws
its competing allocation; complete architect criteria are in6146789689. Map18
may be negotiated before receipt consumption while emission stays OFF and the
real one-flight guard/strict dual-shape cap13 decoder remain active. No server,
protocol, deployment or account mutation is included.

Ground clicks now produce real walk intents, including Item-family ground from
the pinned appearance flag. Roof clicks do not guess a lower-floor destination.
Shared projected-floor selection rejects covered queued USE: raw duplex proves
no emitted frame or reserved command ID. Invalid origins request existing FND
resync, invalidate the map and poison the old session; recovery still requires
fresh admission. D627 east/north quad order keeps walkers under both endpoint
tops, including a fully resident atlas, without duplicating entries.

Map assets use default content/assets/files or literal --assets <dir>; legacy
OTERYN_ASSET_DIR still selects the repository/manifest root. One injected cached
provider checks the independent manifest pin, logs missing/mismatched catalogue
once and preserves placeholders. Tests copy only required pinned catalogue,
appearances and sheets, never placements; actual PlayView upload bytes equal the
decoded sprite, survive reset and cannot leak after provider replacement.
Production advertisement is now [4,6,7,13,17,18]. Actual bootstrap/selected18
domain17 and declined18/legacydomain2 routes are tested; dependencies remain4/6.
Linux client222/196/5 and Session79, strict selected all-target Clippy/fmt pass.
Map source7335c8b99124ff9d121fa5f789319b9ecfa1e242 passes Rust1.95 MSVC
client221/201/5+Session79, strict all-target Clippy and release build. The first
audit correctly refuses the default local release ID; retain that failed attempt.
Rebuild the same source with release0.1.0+dev.ci.g7335c8b99124: binary SHA256
4e46bb4e14967e51bed4e0e8e7782604ef6c8bcf68b0c83b1c0d7753d632ad62.
DX12 then passes20pages/78independently hashed captures/0invalid at125%DPI.
Private build C:\Users\barte\Downloads\oteryn-client-7335c8b9-20261013-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-7335c8b9-20261013-identity,
manifest69f7c0d13a9c5b6b0209d0ff1b352b2d77c14d8dae57b2a1738196a9f5e78d48.
Owner's login PID37764 remains responsive; no login/installation replacement.
Recovery /workspace/oteryn-client-publication-7335c8b9.bundle SHA256
2f772ca31228244fd91444353f5ddad3f1c16819f6fedd4182b77ec412ba8d63.
This is component/page proof, not completed map/pacing/native E2E or rollout.

## Authoritative step-receipt consumer — 2026-10-13, AUTHORING

StepReceiptOutcome wraps the source-compatible legacy StepOutcome, carrying the
validated wait plus command/generation and monotonic receipt time. Session stamps
the result before awaiting its claimed delta. Native routing retains the receipt;
the real run_session bridge test makes legacy methods fail if accidentally used.
Receipt-bearing TOO_EARLY returns immediately, without old doubling backoff.
The receiptless path retains that exact existing compatibility behavior.

The native mailbox fences timing and checks the deadline before consuming view
input, including direct callers. One step stays reserved through outcome draining;
receipt-aware movement keeps only the latest buffered arrow. TOO_EARLY preserves
the goal, uses the remaining37ms exactly and cannot resurrect a previously drained
Stop. Native wake-up includes the deadline; command Notify interrupts only the
idle first-byte wait. A started frame must finish, validate and apply intact.
Own actor quads actually interpolate for MOVED100/max111150 from receipt time;
the authoritative position is never interpolated. Corrections/teleports, floor
changes and terminal/fresh-channel boundaries reset the visual/clock state.

Tests exercise10,000 real channel dispatches on a jittered fake clock (no sleeps),
zero early/burst sends, no early input consumption and simulated lateness<=20ms.
Raw golden100/max111150/37/legacy, withheld claimed delta, idle wake/partial frame,
actual animated quads, generation/command fencing and existing malformed/poison
cases are covered. Linux client227/196/5+Session82 and strict Clippy/fmt pass.
This simulated bound is not a native network-performance or gameplay E2E claim.
No new wire IDs/codecs/modules/server/deployment/account changes. Live resume or
mid-session snapshot replacement remains absent; fresh admission is not its proof.
Source4ff71216e45500962f831cff32276bca8b9ba0a7 normally merges acceptedmain
8c45d03c9f4cf8f5d2fcfa36f781396887cbd97c (#2042/#2043). This imports the approved
source manifest enabling the World Bundle, not a deployment or live-node proof.
World c49538eb/inputs86d18a80 remain unchanged. Exact Rust1.95 MSVC client226/201/5
and Session82, strict all-target Clippy, formatting and release build pass.
Release0.1.0+dev.ci.g4ff71216e455; binary SHA256
ad54ede88550993e3b86f9ccb9e8f4fe55260db5fcd1c0a55318828605f0b9ff.
DX12:20pages/78independently checked captures/0invalid at125%DPI; manifest
8dc5058714abd1d02756a8fe534c5800255adf480193537d5be716506e7b638a.
Private build C:\Users\barte\Downloads\oteryn-client-4ff71216-20261013-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-4ff71216-20261013.
Qualified binary now responds as login PID1372/window3150006, while old37764
remains responsive. Existing public launcher/CA and explicit asset root reused;
no automatic sign-in, account/trust/install/start-file mutation or restart.
Recovery /workspace/oteryn-client-publication-4ff71216.bundle SHA256
4804935674ca7e9d4c15a9fda78d2a48fdb91558a0cdd51983a39d77bc13a97b.
Source CI currently runs without failed checks; component/page proof is not
full original parity, native gameplay/performance E2E, final review or MQ proof.
The server switch stays OFF. Canonical lookup #1622 through6146799516 supplies
the complete criteria, but no dedicated QA setup or ON/OFF/harness authority;
#2041 remains open4f4fbadc. Client work continues under the owner task, not the
withdrawn competing CP allocation. Older binaries are preserved, not retired.

## Supported hotkey/profile batch — 2026-10-13, AUTHORING

One batch fixes text UTF-8/control limits, bounded custom-action/profile creation,
Unicode duplicate names, retained-name auto-switch matching and unavailable
legacy Spell/Object key claiming. Modal OK/Apply clicks test64/65emoji,
128/129two-byte letters, controls/blank input and255/256actions. Refusals retain
input and valid profile data; own case-only rename and Unicode byte bounds are
covered. Legacy payloads round-trip unchanged; both chat contexts refuse their
unavailable keys in the resolver and actual router. Real Text consumers remain.

Sourcefabf441a1a45a747bad9f82c1a0b994b517d8e95 passes Linux228/199/5, strict
all-target Clippy/fmt, governance60/diff. Exact Rust1.95 MSVC227/204/5+Session82,
strict all-target Clippy/fmt/release and DX1220pages/78independently rehashed
captures/0invalid at125%DPI pass. Release0.1.0+dev.ci.gfabf441a1a45;
binary SHA256a7f2fa3a04114cac439529c56aef92be3f8110c9bceca070f68748c3b615f83e;
manifest038cff0d364828db276a7dc4927b5da85de5f1eb58285042cbb71606513c2a3f.
Private build C:\Users\barte\Downloads\oteryn-client-fabf441a-20261013-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-fabf441a-20261013.

Retain outer build/audit PID6600 exit1: all build and audit guards completed,
then metadata lookup found previous1372 absent. Read-only source/log/manifest
readback independently verifies the same artifact; no build/audit was repeated.
The initial window fence used an ASCII hyphen and refused the source's U+2014
title. Source-correct codepoint fence then verifies currentPID38236 responding,
window8065072, exact path/hash and login title. Old37764 still responds; neither
old client/install/checkouts nor trust/account/deployment were changed. Public
launcher/CA and asset root are reused without automatic login. No personal
clipboard/input or gameplay proof follows from native unit/page tests.

Guarded recovery /workspace/oteryn-client-publication-fabf441a.bundle SHA256
5b1c45bb336d526e807c0b66a737f9e9a95e7f6988b1da573542951d9c46288e.
Same-head CI last read:7successful/6running/0failed. Still draft AUTHORING,
not100%, final CI/review/MQ, live ON/OFF E2E or a qualified main-channel release.
Visible execution request6149042654 has no supplied QA/ON-OFF/harness answer;
server2041 remains unmerged, latest head e4ac000cabc760f693847d8d9b80a0ae17d87dd6.
Earlier4f4fbadc evidence is historical, not proof of that successor producer.

## Supported settings transactions — 2026-10-13, AUTHORING

B1 continuation adds isolated PL/EN production-widget transactions reaching
actual redraw/repeat, cursor/HUD/Harmony, target-marking, receipt-time chat and
action-row viewport consumers. Cancel/reload, real staging-write failure and
visible recovery are covered; unavailable children remain inert. Immediate
language save now commits only after persistence and preserves unrelated draft
edits. Actual toolbar and routed matrix tests execute on exact Windows source
a65e7428fcbd92221997b87ffe22bc70451a7426: MSVC227/206/5+Session82, strict checks
and release pass; post-merge Linux228/200/5+82 and governance60 pass.
Release0.1.0+dev.ci.ga65e7428fcbd; binary SHA256
3d4de918e4aaa7271e775a76c84018f6a80cb98ae8bbdb1b17abfe38445a0213;
DX1220pages/78 independently rehashed captures/0invalid at125%DPI, manifest
7174e7b965670718f3bb991cd91b0649cfd1e3e32702158c8e49237239bc2078.
All20 initial captures were visually inspected; automated scrolling checks the
remaining captures. Exact responding loginPID3008/window3346350 is launched
without automatic sign-in. Private paths, recovery and remaining boundaries:
`docs/agents/tasks/active/OTV2-20261013-client-settings-transactions.md`.
Component/page tests are not physical clipboard/picker/trackpad or gameplay proof.
Same-a65 CI is13successful/4running/0failed, not final all-green. Producer2041
is now merged/main376, but no dedicated QA/ON-OFF/harness/retirement reply was
supplied to6149042654. Registered clock19 stays unoffered; no deployment follows.

## Existing item-menu safety before B2 — AUTHORING

The already active command9 context path is restricted to received open-container
entries. Connection/container/handle-stable upstream widget IDs and stale-popup
cleanup prevent replacement/reordering from selecting a neighbour. Busy and
closed receivers refuse enqueue visibly, with retry on a busy popup. No new
drag/preset/capability is activated. Three real-egui regressions use actual
PlayLink/run_session mailbox dispatch, not a substituted send closure; their
scripted Unavailable result is not wire/server/delta/live-gameplay proof.
Current packet: `docs/agents/tasks/active/OTV2-20261013-client-item-menu-safety.md`.
Changed-runtime native qualification remains pending; do not inherit a65 proof.

## Continuation — 2026-10-14, point6 excluded

Owner explicitly excludes HUD/graphics redesign from the current completion
scope. Accepted main71c014d4 is normally merged via32ee396e (after3295351a), preserving client
runtime. Remote00198640 has19/19 successful checks; new source must qualify
independently. Read-only mapping confirms existing NPC talk/trade producers,
unoffered item-use15 and acceptance-only2084/2085, not supplied live setup.

One existing-path stability repair refreshes admitted GameState before a step
completion becomes visible. The production mailbox regression fails without
the repair and passes with it, covering received changes and closed/emptied
containers while idle remains pending. It does not fabricate received inventory
or count a saved setting as a function. Published source6f217f16 passes Linux
client229/203/5+Session82 and native MSVC228/209/5+Session82, strict checks and
release. DX12 at125%DPI validates20pages/78 independently rehashed captures/
0invalid, not every original control or live gameplay. Release0.1.0+dev.ci.g6f217f1632ee;
binary b87ef4e040fdbe89f5a14d788db9baa2bc5954b0436ba042cdca14f641a7b500;
manifest fa864d07e2e8f8f06e659c265554ce8f82aa73221e8d87c05d5e90952b3c1426.
Source-bound loginPID11608/window461552 responds without automatic sign-in.
Attempt1 empty-index refusal and all older artifacts remain retained. Same-6f
CI is13success/4running/0failed, not final qualification. This docs-only
checkpoint waits locally rather than cancelling useful CI on unchanged runtime.
Packet: docs/agents/tasks/active/OTV2-20261014-client-step-projections.md.

Canonical #1622 still lacks the dedicated QA/node/ON-OFF/harness/retirement
setup needed for B2. The Windows PC is online; this is build access, not server
or account authority. Later gameplay capability activation remains gated.

## Local input continuation — 2026-10-14, AUTHORING

Owner requested the local settings/chat/Windows packet while E2E is blocked.
Official manual4.2.1 establishes Shift+Up/Down recall. A session-local64-line
memory bound records text only after real mailbox acceptance, not server Sent.
Exact focused/enabled chords recall without sending or changing recipient/tab;
returning past newest restores scratch and editing ends browsing. Unmodified
egui handles ordinary input segments in order and paints only the final text.
Actual GameUi.show/Send/mailbox tests cover Unicode/caret, boundaries, mixed
event order, hidden/read-only/modal/focus and busy/closed/invalid refusal. A
consumer bypass fails the painted Unicode test; restoration passes.

Two input defects are proved and repaired: unfocused queued Escape no longer
discards Options drafts; a closed chat receiver refuses without reserving text.
PL/EN controls/console/screenshots Reset/Cancel tests also survive tiny resize
and retain unavailable intent. These are real consumer/component checks, not
new protocol, fake echo, physical clipboard/picker or every original interaction.
Linux230/209/5+Session82, strict checks/fmt and governance60 pass; exact new
MSVC release/native20-page audit follows publication. Parent6f has green CI,
including game-gate; successor needs its own checks. Prior14d native evidence
is retained/published in the same packet, not relabelled as changed-code proof.
HUD/graphics remain excluded; B2 and later capability activation remain gated.
Packet: docs/agents/tasks/active/OTV2-20261014-client-local-input.md.

Published99a8a385 additionally passes exact MSVC229/215/5+Session82, strict
checks/release and DX1220pages/78rehashed captures/0invalid at125%DPI. All
named new consumer regressions execute. Binary8e48deb23e3d3e3185cd2d0635f96f445d4dda3f8f909ff3d9cdec784e33931e;
manifest26c62c859cbccf17033870372bf9464424ae08484b8c4f18781e1d5e4bdc8b75.
Source-bound login31460 responds without automatic sign-in. Packet above keeps
exact paths, native helper exit0, release, recovery and prior artifacts.

Ordinary CI99a8 is blocked by real source conflict, not a red test or queued
success. Accepted mainaf85035f moves World/placement data; normal reconciliation
resolves one input-pin conflict via the provided release compiler. Double-build
matches accepted digest1b04cc35/25,089,907bytes with merged inputs883be88a.
No server deployment/start/identity mutation or graphics redesign. Updated
placement fingerprint requires new native qualification; retained99 proof does
not qualify the successor or its final same-head CI/review/MQ/main-channel release.

Exact post-merge sourcec53099c9 passes Linux230/209/5+Session82 and native
MSVC229/215/5+Session82/strict/fmt/release, with all new consumer tests executed.
DX12 at125%DPI:20pages/78rehashed captures/0invalid; binary
4fd3e1b44171bacd68d9ddc46da46719a9c61c63dfbce4918e8bbe67b6a0d056;
manifestedc8624c1f84cab9cda342df40d313d2d37dc9d99fe609e96d3da6f988a57c5c.
Native helper exits0; source-bound login26640/window4393676 responds without
automatic sign-in. Fresh PR MERGEABLE, ordinary CI10success/7running/0failed;
game-gate/final review remain pending. Full release/path/recovery/retained-source
evidence is in the local-input packet. This docs-only successor waits locally
without repeating Windows or cancelling useful CI. No full1:1/physical OS/
live-gameplay/main-channel qualification is inferred from these component proofs.

## Local input focus ownership follow-up — 2026-10-14

Production widget/mailbox regression proves Enter→Shift+Up in one frame could
submit the historical line instead of the committed draft. Recall now rechecks
editor focus after each ordinary input segment; Shift+Up→Enter still submits
the intentionally recalled line. Four PL/EN/order cases check exact command
text and one enqueue. Twelve Copy/Cut/Paste cases check Unicode selection,
ordered recall, CR/LF normalization, final paint and scratch recovery without
fabricating received chat or accessing the owner's clipboard. All seven history
regressions pass; Linux230/211/5,strict all-target Clippy,fmt/governance60 pass.
Exact new MSVC/native qualification follows publication.
Physical OS clipboard/picker/IME parity remains open. Prior c530 proof and green
CI belong to that unchanged source, not this repaired candidate. Packet:
docs/agents/tasks/active/OTV2-20261014-client-local-input.md. No post-B2 activation
or HUD/graphics redesign is authorized by this local repair.

Source98319fd7531430dc604dd8e92eb80c7eb857ac5a additionally passes exact
Rust1.95 MSVC229/217/5+Session82,strict all-target Clippy/fmt/release. All seven
history regressions execute/pass. DX12 at125%DPI:20pages/78independently
rehashed captures/0invalid. Release0.1.0+dev.ci.g98319fd75314; binary
04208c8fee07c95a0096e501582af4844654123ab8673c857ec2a77eafbf14c8;
manifest394c1a5795ddaca7d9f36996ec128476566a7fc9ee52d74e7c6fdebaca9d0e3e.
Private build C:\Users\barte\Downloads\oteryn-client-98319fd7-20261014-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-98319fd7-20261014.
HelperPID13632 exits0 after129.98seconds. The compiled audit runs/exits without
automatic login,new personal-profile/clipboard operations or terminating older
clients. Guarded publication/readback and recovery98319fd7 pass; bundle SHA256
7422d9e606bd51d0c12c4898003a4ffe563cb07cd3b11444df780d9c00a96c60.
This exact input proof is preserved with the next real export-guard fix, not a
status-only push. A changed Options export source needs new native qualification.

Options destination follow-up narrows the Windows gap to absent preferences:
existing files already canonicalize safely, but lexical equality misses an
ASCII-case alias of a missing settings.json. A conservative same-parent/name
guard and Windows real-filesystem regression preserve bytes/absence while
allowing ordinary sibling ZIP export/import. Linux230/211/5,strict Clippy/fmt
and governance60 pass. An isolated Windows counterfactual and exact-source
qualification remain pending; physical picker interaction is not claimed.
Packet: docs/agents/tasks/active/OTV2-20261014-client-export-destination.md.

## Required evidence per page/panel

Normal-client690e22ad F10/focus proof (prelogin-focus.md) now includes actual
foreground loss, queued background keys, unsaved Advanced navigation retention
and explicit Escape discard. It does not establish physical keyboard/IME parity.
The next table-fit packet fixes a newly observed per-row gap that the20-page
audit did not prove: header/key-column alignment and full second-key hit targets.
Profiles Add/Apply is independently proved; later native Copy/Remove/key-capture
fixture filenames are rejected because they show the wrong screen, not success.
After e4's exact native table qualification, a new inspected normal-client probe
does open the second-key editor and proves F10 hides its parent instead of being
captured. Existing modal ownership now also fences the prelogin global toggle;
raw input remains with upstream egui. Six PL/EN binding/profile/reset cases pass
without changing draft/current, with inner Escape and ordinary-F10 recovery.
Published1954627d has fresh exact MSVC229/225/5+Platform16+Session84+TCP2,
strict/fmt/release and DX12/125%DPI20pages/78rehashed captures/0invalid.
A separately inspected normal195 client now paints actual F10 in the real
second-key editor without hiding Options; Escape dismisses only the editor,
then ordinary F10 hides/reopens the parent with the draft intact. No Apply or
active settings file. All6 PNGs hash/decode1125x775/0invalid; report SHA256
5a0ca6f15f627d9cd7649ee6c11b838f2c196cdf21029f1d912e182af2d6965e.
This is OS-injected native modal acceptance, not gameplay/E2E, physical keyboard,
native profile/reset confirmation coverage or whole-client1:1. Exact195 CI lacks
Merge gate/game-gate proof; the protected-base audit alone does not qualify it.

Latest native OS evidence is in OTV2-20261014-client-native-os-input: bounded
current-desktop permission supersedes older isolation requirements, and actual
Options clipboard/pickers/import+Apply/wheel tests pass on ab111236. Emoji
VK_PACKET typing fails while identical standard Win32 Edit input succeeds;
Paste is not proof of a typing fix. Latest F10 focus repair/source composition
is in OTV2-20261014-client-prelogin-focus; earlier native proofs are not inherited.

Six local steps, without E2E: current packet
docs/agents/tasks/active/OTV2-20261014-client-six-local-steps.md distinguishes
catalogue declarations (UI-LOCAL-OPTION-INDEX.md) from actual consumers. Initially
red unfocused key-capture and IME/history regressions are repaired above upstream
egui. Routed profile confirmation and PL/EN import failure/retry use real widgets
and local filesystem. Native bar Text remains unavailable, unlike Custom Hotkeys
Text. Fresh REA transport and physical OS isolation are still absent; audio
upstream choice is investigated, not playback. Linux230/214/5,strict Clippy/fmt
and governance60 pass; exact new native checks follow. Prior f8 component/page
proof is not inherited as changed-source qualification.

- Control type, label, grouping, order and dimensions compared with a named private
  capture; dropdown membership and defaults only when actually observed.
- Unknown settings remain unset; simply drawing a page must not persist defaults.
- Catalogue rows without a real runtime consumer are disabled. Previously saved
  values remain round-trippable for compatibility but cannot be edited or counted
  as working behavior.
- Draft edits survive navigation; Cancel discards; successful Apply updates live
  consumers and restart; failed save preserves the previous active configuration.
- Every actionable control is reachable at the default native window size and
  after resize. Scroll offsets are independent per page. Keyboard/modal focus
  cannot leak into gameplay.
- Actual compiled Windows capture and interactions, plus targeted regressions.
  A generated mockup, catalogue count or passing serialization test is insufficient.

The same acceptance applies to all 28 panel shortcuts, 22 prepared panel views,
20 Cyclopedia subpages and six dialogs. Their current presence is scaffolding;
server values, allocation, rewards, spell actions and commercial operations must
not be fabricated. Missing engine mechanisms remain explicit work rather than
an assertion of completion. Secure remembered sign-in remains unwired.

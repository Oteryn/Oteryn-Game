# Native UI fidelity acceptance

Draft #1942 is AUTHORING. Presence of a panel or persisted preference does not
establish visual parity or working gameplay. Original reference captures and
assets remain private; the product ships independently authored code/artwork.

## Reference audit correction

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

| Reference page | Dedicated composition | Actual consumer / remaining acceptance |
| --- | --- | --- |
| Basic | Yes | Display controls/action-row visibility work; own-resource bars/arcs have a vitals renderer behind explicit HUD selections. Audio stays disabled because no backend/content exists |
| Controls | Yes | Four cardinal movement bindings work; diagonal rows remain disabled because the accepted movement wire has no diagonal intent. Keyboard Delay has the observed 0–1000 ms slider and explicit default 250 ms master; applied selections drive a client-owned held-key scheduler independently of OS repeat and frame-rate limits, using the existing paced session step queue. Releases, lost focus/device, modal/text editors, chat-context/profile changes and Stop All cancel repeats. The custom delay is retained under the default master. The exact three mouse presets and the Classic-only `Loot: Right / SHIFT+Right / Left` selector are composed with the observed dependency, but stay disabled because their input consumers are pending; persistence alone is not counted. Fresh native qualification for the repeat scheduler remains pending |
| General Hotkeys | Partial: dedicated profile/action table | All 120 official English labels have stable IDs; Polish labels are Oteryn-authored. Profile management, both chat contexts, delayed search, two bindings and cross-family conflict validation work. Real consumers cover cardinal movement/explicit clearing, Stop All, profile cycling/character auto-switch, admitted battle targets, Options/Custom Hotkeys, existing Cyclopedia/dialog/panel navigation, fullscreen/FPS, minimap and action-row visibility, channel/mode/send operations and composer Select All. Escape remains reserved for Stop All. Dialogs open idempotently; windows toggle. Existing local panel compositions are not server-backed data, and unknown interiors remain unavailable. Diagonals, clipboard-copy assignments, unsupported inventory actions and most other observed actions remain disabled without exact consumers |
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
| Help | Partial | Info is local; Compendium opens the existing local composition idempotently during play; confirmed reset works. AUTHORING replaces fixed-path JSON export with an upstream native Windows ZIP SaveFile picker and atomic Oteryn-only archive writer. Cancel leaves state/files unchanged; invalid drafts and failed writes preserve previous exports. Native picker interaction and original archive interoperability remain unqualified. Client Help/Lenshelp, Rule Violations, Manual and FAQ stay visibly disabled because no verified Oteryn destination exists. Export Minimap and the combined Import Options/Minimap action stay disabled until a versioned minimap format and honest format dispatch exist |

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

## Required evidence per page/panel

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

# Installed-original resource evidence — 2026-10-11

Continuation 2026-10-13: the owner identified the installed REA launcher at
`/home/kasm-user/.local/bin/rea`, a symlink to `rea-agents/scripts/rea.mjs`.
Read-only `--help` confirms REA 6.4.0. Existing private Ghidra projects and
bounded native-handler observations are retained under
`/home/kasm-user/otclient-track-a/evidence-rea/`; its README records original
15.33.b21348 source SHA-256
`065f961b29904f3231b5c82be2f7748d88c7b74f3d324e12b5d5dd50b14afead`.
Those earlier facts are retained provenance, not newly verified behavior.
REA's ELF-layout route currently reports `provider_unavailable`: it requires a
caller-supplied Python with pinned pwntools/pyelftools/Unicorn. Earlier full REA
analysis timed out; it is not rerun here. Existing Qt/resource and focused
Ghidra evidence remains usable. No original process/account is started and no
proprietary code, assets, logs or account configuration are published.

Reference: authorized Linux installation, Tibia **15.33.b21348**. This is an
independently written semantic inventory, not redistributed original source or
assets. Initial offline inspection did not modify the installation, apply preferences,
read account configuration or execute gameplay actions. Qt's resource APIs read
the packaged RCC and the ELF's embedded resource arrays; Qt decompression reads
the shader packages. Temporary tools and original captures remain private.

## Reusable evidence, not an implementation claim

| Material recovered | What it establishes | What it does not establish |
| --- | --- | --- |
| 396 readable QML files among 408 embedded files | Actual page composition, properties, slider bounds, dependencies and UI handlers | C++ implementations, factory defaults or working Oteryn consumers |
| Graphics RCC: 1,063 files including 1,040 PNGs and 14 JPGs | Cursor geometry, reference artwork/layout vocabulary | Rights to ship the artwork |
| Four decompressible QSB packages | Recoverable packaged shader representations for simple/world-map-highlight vertex/fragment stages | Recovery of the complete world-lighting/AA renderer |
| Sound protobuf catalogue | 844 sound records, 592 numeric effects, 91 ambience streams, nine ambience-object streams, 25 music templates; zero missing referenced filenames | An Oteryn mixer, authoritative events or permission to redistribute sound files |
| Spell resource: 198 records | Reference picker fields, costs, cooldowns, vocation/level restrictions and icon indices | Authoritative Oteryn learned spells or executable spell assignments |

The sound catalogue follows the existing upstream schema at OTClient revision
`53c3878a1c5119c78e148adb32def53fea3c53f2`, `src/protobuf/sounds.proto`.
Use that schema rather than inventing a format. The catalogue has pitch/volume
ranges and simple/random effect references. A sampled OGG was Vorbis, 48 kHz,
stereo; this is not a claim about every recording.

Packaged `tutorial/clientoptions.json` is tutorial configuration, **not factory
defaults**. The 23 × 23 default cursor and 256 × 32 walk/attack strips establish
geometry, not animation cadence. Original artwork/audio/source stays outside
the public repository; Oteryn implementations remain independently authored.

## Help export continuation

`OptionsPageHelp.qml` uses SaveFile dialogs for Export All Options and Export
Minimap, and an OpenFile dialog for the combined Import Options/Minimap action.
All three offer ZIP and all-file filters. Their handlers delegate to the native
controller; the QML does not establish archive entry names, schemas or validation.
Private resource SHA-256:
`ab2d9f9856efc4bc71da6c2dad2652997f1df6b8f53e4383c9732437ef95c4e1`
(9,350 raw QFile bytes, excluding the diagnostic framing).

A non-applying live inspection independently confirmed the Help rows
and the ZIP save dialog. One attempted export reported an I/O error; no archive
was created. The original process later disappeared during this probe. Its cause
is unknown; no restart or account sign-in was attempted. No Apply, Reset,
preference edit or gameplay action was requested. Private captures remain under
`/tmp/oteryn-reference-help-preview-GTxHFx` on the authorized NAS.

A schema-only inspection of the installed global configuration found
`clientOptionsVersion: 9`, 192 keys in `options`, and a separate `hotkeyOptions`
object. Only key names/types and the version were inspected; private values were
not retained. Other root sections include identity-related data, so that file
must not be copied or treated as a credential-free export. Neither its version
nor its 192 keys establishes the ZIP format, factory defaults or visible-page
cardinality. At discovery, Oteryn's fixed-path JSON export was a working local
backup, **not** reference-compatible Help export. The AUTHORING successor uses
upstream `rfd` for the Windows ZIP SaveFile picker and `zip`/`tempfile` for atomic
export of the existing validated Oteryn preference representation. Its sole
entry is `oteryn-options.json`; this independently authored local format is not
claimed to match the unknown original archive layout. Original archive
interoperability, combined import and minimap dispatch remain unqualified.

## Exact settings contracts recovered

Options dialog continuation: `OptionsMenuDialog.qml` (11,389 raw bytes, SHA-256
`bbcc3cfe46242f6caed9f7bffd1a05fe4a8dd101529dbb7515579fada3cc37bd`)
has a right-aligned RowLayout in left-to-right **Ok / Apply / Cancel** order.
Its cancellation handler calls `cancelButtonClicked`; the outer Return handler
is explicitly undefined. `TibiaDialog.qml` (18,668 raw bytes, SHA-256
`d8e02ba04bed46d77b93289ab9e8a7eeb23f76cd76381eea264e1a33f6f46c9e`)
routes Escape/Cancel to that cancellation handler. Focused elements can own
Return, but an outer Enter must not silently save all preferences. The existing
Oteryn footer previously had reversed visual order and lacked outer Escape cancellation;
these are local UI gaps, not missing server projections. Inner key-capture and
confirmation dialogs must consume/cancel first without applying the draft.

`TibiaOptionsPage.qml` (527 raw bytes, SHA-256
`56efa0c7f077771967862f9a5de611d4ccee15621719b6ca8a66ee2e623a7511`)
routes the ordinary Reset button to `requestResetCurrentPageToDefault`, not
the Help page's Reset All Options. This establishes scope, not factory values
or C++ profile-table reset behavior. Oteryn's page reset restores only its
implemented page controls to Oteryn defaults, preserves unavailable saved
intent, other pages, bindings and locks, and never applies automatically.
Unverified profile-table and empty Help page resets are disabled. Original
factory-value parity and those table resets remain open.

Resource prefix: `:/qt/qml/qmlcomponents/qml/` in the installed executable.

| Resource | Confirmed contract |
| --- | --- |
| OptionsPageGraphics.qml | FPS integer **10–200**, step one; Unlimited disables and retains it. Renderer and AA models are runtime supplied. VSync depends on renderer limitations; Unlimited has a separate runtime capability predicate. Fullscreen sets the borderless-window property |
| OptionsPageEffects.qml | All seven sliders 0–100, step one. Light gates Ambient/Level Separator/combined Clouds & Indoor. Only Clouds & Indoor has an Off marker at zero; spell opacity is a separate group |
| OptionsPageSound.qml | Root requires initialized sound engine. Child volume group depends on runtime soundEnabled. Anthem additionally requires music > 0; food/move-item require item volume > 0. All volume sliders mark zero Off |
| OptionsPageBattleSounds.qml | Groups require runtime soundEnabled; spell/weapon filters additionally require their group volume > 0. Spell categories additionally require Spells. Children retain selected intent |
| OptionsPageUISounds.qml | UI filters require UI volume > 0 and runtime soundEnabled; message-category children additionally require Console Messages. Volume marks zero Off |
| OptionsPageControls.qml | Custom delay 0–1000 ms; default master disables it. Critical style/help below 50; warning below 250; neither at 250 or above. Shift/Ctrl/Alt rotation selections are separate |
| OptionsPageInterface.qml | Native/Animated/Big are independent controls. Unused expiry is gated by inventory OR container expiry |
| TibiaMenuSlider.qml / TibiaSlider.qml | Off marker supplements the minimum value. Arrow buttons and left/right keys step by one, Shift multiplies by 10, Ctrl by 100; wheel uses a tenth of the range per normal notch |
| TibiaAutorepeatIconButton.qml | Primary click executes on release with its modifiers. A 350 ms timer starts repetition; its first 60 ms interval follows that delay. Further 60 ms repeats carry no mouse modifiers; disable/release stops repetition |

The backend definition of `soundEnabled` was not recovered in this bounded pass;
do not silently equate it with a stored master-volume value. Device enumeration,
renderer capability lists and complete light/effect algorithms remain runtime
work. Source facts eliminate UI guesswork but do not enable absent consumers.

## Correction to historical FPS evidence

The earlier written **1–200** observation is superseded for version 15.33 by
Graphics lines 211–213 and the underlying slider's direct bounds aliases. No
factory preference or original Apply was needed. New Oteryn edits use 10–200;
existing 1–9 values remain valid, rendered without mutation and consumed by the
scheduler until explicitly edited. Unlimited still retains the prior value.

Private source digests (SHA-256, decompressed resource bytes):

- Graphics: `baebabc1add4912336c23dae0fa8286eb9917b94a60b4bea8c0ff7559c69bbf8`
- Controls: `e291d3822c1937c2f7692326c8dc3b594d1b188d1a46c4a08ed0722f50d8c87e`
- Sound: `270ae6c661b93d4b06640cc903bcca86f40b8d50b9d295a6fda6076943b5f37e`
- Battle Sounds: `6acc6c1c1abf68895a5a1f3247cbaf848bcda6b585ece22544f9b7b5904cf238`
- UI Sounds: `12a411dc8e0b88d7bebad2e9124199a8df31aca7b2d87a6931ef52bc3cce5088`
- Menu slider: `8a248cc70f61577e08ed5af862acfc51d14936dbccdc680ad96de15b58132294`
- Slider: `29565cd82ad5a6a1ecdec4cad0e03feef0596d5d5e614dc1681c91a9e9ba1e96`
- Repeating arrow: `217119765d86cddd906bf28cb36d8fdd77d139c1247152611b60c6a74743460b`

Current repairs cover the editable FPS minimum, delay warning boundaries, minimum
Off markers and volume-dependent anthem/item child groups. Audio rows remain
disabled without playback. The next source-backed batch adds arrow/key modifiers,
ordinary line-wheel steps, held-arrow timing and positive-volume Battle/UI filter
gates. It uses upstream egui sliders with a bounded Oteryn adapter, not copied QML
or a dependency fork. Pixel/trackpad wheel equivalence and full native interactive
comparison remain pending; these changes do not establish audio playback.

## Chat selection continuation

`ChatOutput.qml` (10,410 raw resource bytes, SHA-256
`84885036204e09fbf42dcb0f2062a08f15d5aef85a9bf57d94dbe98aaa4a2e50`)
renders immutable rich-text entries and routes primary drag/double-click to
model-owned text-selection operations. Primary rich-text anchor clicks call
`sendKeyword`, not an inferred browser-opening action. Channel/context-menu
handling belongs to the native model. This establishes read-only output
selection, not the external-link clipboard warning policy, which remains unknown.
Do not convert NPC keyword anchors into generic external web links.

Offline `Chat.qml` SHA-256
`247665b41f1251993c94a7d03ed8657d2f8de0068845d4762b1811392771a0d2`
lines 36–38 delegates Select All to the chat input, not the output history.
`ChatInput.qml` SHA-256
`0bff9ce851998ca5c01c523c68164923dc284b55b5db5093c6627fd1482938fd`
lines 22–26 exposes selected input text and delegates selection to its editor.
Oteryn uses upstream egui character-range selection for the actual composer;
Unicode selection/replacement and the exact Chat On default chord are tested.
Unavailable/read-only chat cannot activate it. This does not qualify clipboard
copy, link-warning policy or NPC-dialog routing.

## Graphics continuation: scaling and lighting

Offline `MapWindowPane.qml` inspection (SHA-256
`7163aabd502d60d768e6f0219201a7307216df9e58838cc81118ace62ed927e0`)
establishes these additional contracts:

- Lines 78–89 floor integral maximum height, then retain a half-native minimum.
  Oteryn now retains that half-size scene when it fits the real available
  rectangle; smaller windows still use recovery instead of overlapping the HUD.
  Rendering, picking, HUD and screenshot cropping use the same viewport.
- Lines 119–134 request resampling only when scale differs from one. None uses
  unsmoothed sampling; Antialiasing requests smooth sampling. Smooth Retro requests
  an intermediate ceil(scale)-multiple texture only when enlarging.
- Lines 347–374 expose a live, clamped, non-mipmapped scene texture and the
  intermediate dimensions. This is scene-image resampling, not evidence for
  enabling geometric MSAA or linearly filtering each sprite's atlas independently.
  Lines 286–345 select the whole worldmap or its uncached multiplicative lightmap
  blend as the resampling source; the blend is explicitly layer-enabled so that
  Smooth Retro also works with lighting. HUD overlays are outside that source.
  These observations initially left Oteryn AA unavailable. Successor `28565f4f`
  implements whole-scene resampling and has physical DX12 pixel evidence in
  UI-FIDELITY.md; a saved selection alone is still not rendering acceptance.

Pinned OTClient `mapview.cpp` lines 498–529 and 743–752 use framebuffer smoothing
and a fixed 2x Smooth Retro buffer, unlike the original's ceil(scale) intermediate.
`lightview.cpp` uses a multiplicative per-tile RGB light texture, actual source
position/intensity/colour and shade-reset boundaries; `mapview.cpp` lines 577–582
combine minimum ambient intensity with actual map light. These are reusable
implementation references, not proof that Oteryn's constant ambient overlay is
equivalent. Light sources, floor shading and environment projections are still
absent; no light algorithm or server event is fabricated.

The stock Ubuntu Qt Shader Baker 6.4.2 was unpacked privately without installation
(package SHA-256 `c3822831b75e36b0a0fc23a13f2bda03b64eafcef073ba91371da7dff420fcbc`).
It rejects all four recovered QSB packages as serialization version 9, despite
returning process exit zero. No decoded shader or reflection acceptance follows
from that exit status. A compatible upstream Qt tool is required; do not invent
a QSB parser or modify the version header to force an old decoder.

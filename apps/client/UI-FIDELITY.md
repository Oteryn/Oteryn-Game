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

## Page matrix

| Reference page | Dedicated composition | Actual consumer / remaining acceptance |
| --- | --- | --- |
| Basic | Yes | Display controls/action-row visibility work; own-resource bars/arcs have a vitals renderer behind explicit HUD selections. Audio stays disabled because no backend/content exists |
| Controls | Yes | Four cardinal movement bindings work; diagonal rows remain disabled because the accepted movement wire has no diagonal intent. Keyboard Delay has the observed 0–1000 ms slider and explicit default 250 ms master; applied selections drive a client-owned held-key scheduler independently of OS repeat and frame-rate limits, using the existing paced session step queue. Releases, lost focus/device, modal/text editors, chat-context/profile changes and Stop All cancel repeats. The custom delay is retained under the default master. The exact three mouse presets and the Classic-only `Loot: Right / SHIFT+Right / Left` selector are composed with the observed dependency, but stay disabled because their input consumers are pending; persistence alone is not counted. Fresh native qualification for the repeat scheduler remains pending |
| General Hotkeys | Partial: dedicated profile/action table | All 120 official English action labels are present with unique stable IDs; Polish labels are Oteryn-authored. Add/copy/rename/remove, two chat contexts, delayed-search layout, two captured bindings and cross-family conflict validation work. Cardinal movement accepts one unmodified physical key and an explicit clear cannot reactivate the legacy fallback; diagonal movement stays disabled until the wire supports it. Escape is reserved for Stop All and defaults there in both chat contexts. Character-name profile auto-switch, next-profile cycling, cardinal movement, first/next/previous admitted battle targets, Stop All Actions, Options, exact Custom Hotkeys navigation, the Map/Items/Character/Bestiary/Charms/Magical Archive Cyclopedia destinations, Compendium, all seven observed Social route identities, the existing Exaltation Forge/Reward Wall/Task Board/Weapon Proficiency/Wheel of Destiny compositions, fullscreen, FPS overlay (not yet the labelled lag indicator), minimap zoom/pan, individual/edge action-bar visibility, local/next/previous/help/list/close chat actions and the VIP/Battle/Skills/Spell List/Analytics Selector/Bestiary Tracker/Bosstiary Tracker/Imbuement Tracker/Kill Tracker/Party/Player Guide/Quest Tracker/Quest Log/Prey panel routes have consumers. Dialog actions open idempotently; window actions toggle. Unknown Social interiors are explicit unavailable views. These panel routes expose only existing local compositions and do not imply server-backed content. Chat-text editing actions, unsupported inventory rows and most other observed actions stay disabled until they have exact runtime consumers. |
| Action Bar Hotkeys | Dedicated visible 9 × 50 table; editing disabled | Both stored chord columns remain visible in the distinct Chat On/Off contexts, and legacy 12-slot/default migration remains loadable and validated. The runtime router executes a slot only when that session has an executable assignment and no longer claims an empty slot's key. Production has no authoritative spell/object assignment source, so every binding cell stays disabled and the internal assignment/migration primitives are not counted as reachable activation behavior. A fresh native Windows acceptance for this candidate remains pending. |
| Custom Hotkeys | Partial: observed editor inventory | Text assignments fill the real composer or send through the typed chat intent. Spell/object assignment and legacy bindings are disabled because no authoritative live picker/assignment exists; their persisted presence is not counted as behavior |
| Interface | Yes | Tile-under-pointer highlighting works. Explicit Native off draws an independently authored Oteryn arrow, Native on restores the OS pointer, and Big doubles the custom arrow around the same pointer hotspot. Specialized text/resize cursors retain their OS semantics; contextual Animated cursor states, native/Big combination acceptance, link-copy warning, loot-colour and expiry remain pending. Fresh native qualification of the custom arrow is required. Oteryn language, contrast, motion and scale controls are intentionally absent from this reference page |
| HUD | Partial: groups/arcs and all 35 observed conditions | Exact master labels, the two Harmony radio positions and 35 locally drawn condition symbols are present. Own name and harmony use admitted/session data; own health/mana bars/arcs and other-actor health bars consume real projections. Condition controls and ordering are visible but disabled because the protocol exposes no condition projection; native live-session visual acceptance remains pending |
| Console | Yes | Per-tab read-only mode works; the private-message-tab option uses the exact reference label but stays disabled without routing. Copy uses the clipboard and Save Window writes a bounded, non-overwriting local snapshot. Clear Window hides the current tab's received lines without mutating server/session history; a per-tab accepted sequence cutoff admits later identical messages. Show Timestamps / Show Seconds in Timestamps consume bounded client receipt times from accepted ChatLine events. Missing local-clock metadata has no invented prefix; these are not server timestamps. A retained legacy text-size value still affects chat rendering, but it has no reference-page control and is not counted as an available function. The live console now owns explicit Chat Off / Chat On / temporary Chat On* focus and send behavior; participants, secondary console and event filters remain pending. Fresh native qualification of the clock/clear batch is required |
| Game Window | Yes | Frame & Highlight / Frame Only / Highlight Only / None drive the real target frame and scene overlay. Text/combat/effect projections remain disabled |
| Action Bars | Partial | Three masters and nine independent 50-slot rows drive the live layout; visible stored hotkey labels affect that renderer. Global lock migration, a single trailing lock per nonempty edge group, clear and primary-button move/swap primitives are tested against in-memory session assignments. Production still has no authoritative spell/object assignment source, so an admitted client starts empty; assignment menus, binding/item-amount/tooltip controls, clear controls and both lock surfaces remain disabled. Those primitives are not counted as reachable behavior. |
| Shortcuts | Yes, two lists | Add/remove/order edits for destinations with real local compositions affect draft only; Apply updates the live registry and Cancel discards. Clean profiles and Defaults omit Lenshelp; a Lenshelp entry retained from an existing profile remains visible and removable but disabled, and cannot be added while its destination is unverified. |
| Graphics | Yes | Fullscreen/VSync, the observed integer 1–200 limit, retained disabled value under No Frame Rate Limit, live frame-rate readout and the optional FPS overlay work. Integral-multiple scaling now fits whole physical multiples of the 32-pixel sprite cells and shares its transform with picking, HUD and renderer capture; windows smaller than one native scene expose the existing recovery UI. Engine/AA controls stay disabled pending verified platform capabilities |
| Effects | Yes, lighting + opacity groups | The Light master gates its retained Ambient, Level Separator and combined Clouds & Indoor children; ambient intensity drives the bounded scene overlay. Level separation, the combined attenuation row and spell-effect controls stay disabled pending projections/renderers |
| Sound | Yes, device + five volume groups | Food/move-item controls belong in item group; all controls are disabled because no audio backend/content exists |
| Battle Sounds | Yes, own/others/creatures | Nested spell categories are present but disabled because no mixer/events exist |
| UI Sounds | Yes, UI/social + console categories | Filters are present but disabled because no audio playback exists |
| Miscellaneous | Yes, seven rows | Confirmation and session selections require explicit consumers; secure remembered sign-in stays OFF/unwired |
| Gameplay | Yes | Inspection/chase/loot intent prepared; engine contracts/consumers pending |
| Screenshots | Yes, two trigger columns | The folder action creates and opens the local Oteryn screenshot directory. The screenshot hotkey performs a real non-overwriting PNG renderer capture, and Only Capture Game Window selects the actual scene crop. Seventeen automatic triggers stay disabled because their event pipeline does not exist |
| Help | Partial | Info is local; Compendium opens the existing local composition idempotently during play; complete options export and confirmed reset work. Client Help/Lenshelp, Rule Violations, Manual and FAQ stay visibly disabled because no verified Oteryn destination exists. Export Minimap and the combined Import Options/Minimap action stay disabled until a versioned minimap format and honest format dispatch exist |

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
- This establishes native compiled-page reachability, not exhaustive reference
  comparison or working behavior for disabled rows. Native live-session visual
  acceptance and every row whose consumer remains pending are still open and
  must not be reported as 1:1 complete.

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

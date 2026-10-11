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
| Controls | Yes | Four cardinal movement bindings work; diagonal rows remain disabled because the accepted movement wire has no diagonal intent. The exact three mouse presets and the Classic-only `Loot: Right / SHIFT+Right / Left` selector are composed with the observed dependency, but stay disabled because their input consumers are pending; persistence alone is not counted |
| General Hotkeys | Partial: dedicated profile/action table | All 120 official English action labels are present with unique stable IDs; Polish labels are Oteryn-authored. Add/copy/rename/remove, two chat contexts, delayed-search layout, two captured bindings and cross-family conflict validation work. Cardinal movement accepts one unmodified physical key and an explicit clear cannot reactivate the legacy fallback; diagonal movement stays disabled until the wire supports it. Escape is reserved for Stop All and defaults there in both chat contexts. Character-name profile auto-switch, next-profile cycling, cardinal movement, first/next/previous admitted battle targets, Stop All Actions, Options, exact Custom Hotkeys navigation, the Map/Items/Character/Bestiary/Charms/Magical Archive Cyclopedia destinations, the existing Exaltation Forge/Reward Wall/Task Board/Weapon Proficiency/Wheel of Destiny compositions, fullscreen, FPS overlay (not yet the labelled lag indicator), minimap zoom/pan, individual/edge action-bar visibility, local/next/previous/help/list/close chat actions and the VIP/Battle/Skills/Quest Log/Prey panel routes have consumers. Dialog actions open idempotently; window actions toggle. Unsupported inventory rows are disabled and most observed actions remain pending |
| Action Bar Hotkeys | Dedicated 9 × 50 table | Two captured chords per slot in distinct Chat On/Off contexts, migration from 12 slots, modal/held-key routing and both-chord activation work. New defaults use F1–F12 in both contexts; the exact untouched v1 sequence migrates only when those target chords are conflict-free. This includes F10, which the admitted shell no longer steals from the action router. Compiled Windows cross-check and headless page reachability pass, while a fresh native Windows acceptance for this candidate remains pending |
| Custom Hotkeys | Partial: observed editor inventory | Text assignments fill the real composer or send through the typed chat intent. Spell/object assignment and legacy bindings are disabled because no authoritative live picker/assignment exists; their persisted presence is not counted as behavior |
| Interface | Yes | Tile-under-pointer highlighting works; system/custom cursor, link-copy warning, loot-colour and expiry controls stay disabled pending consumers. Oteryn language, contrast, motion and scale controls are intentionally absent from this reference page |
| HUD | Partial: groups/arcs and all 35 observed conditions | Exact master labels, the two Harmony radio positions and 35 locally drawn condition symbols are present. Own name and harmony use admitted/session data; own health/mana bars/arcs and other-actor health bars consume real projections. Condition controls and ordering are visible but disabled because the protocol exposes no condition projection; native live-session visual acceptance remains pending |
| Console | Yes | Text size and per-tab read-only mode work; the private-message-tab option uses the exact reference label but stays disabled without routing. Copy uses the clipboard and Save writes a bounded, non-overwriting local snapshot. The live console now owns explicit Chat Off / Chat On / temporary Chat On* focus and send behavior; participants, secondary console, clear, timestamps and event filters remain pending |
| Game Window | Yes | Frame & Highlight / Frame Only / Highlight Only / None drive the real target frame and scene overlay. Text/combat/effect projections remain disabled |
| Action Bars | Partial | Three masters/nine independent 50-slot rows, two chords per slot and session action clearing work. Hotkey labels, item amounts and tooltips drive the live action-row renderer. The observed single trailing lock per nonempty edge group and pointer drag/drop assignment are still pending; the settings lock row stays disabled and is not counted as behavior |
| Shortcuts | Yes, two lists | Add/remove/order edits affect draft only; Apply updates live registry, Cancel discards |
| Graphics | Yes | Fullscreen/VSync, the observed integer 1–200 limit, retained disabled value under No Frame Rate Limit, live frame-rate readout and the optional FPS overlay work. Engine/AA controls stay disabled pending verified platform capabilities |
| Effects | Yes, lighting + opacity groups | The Light master gates its retained Ambient, Level Separator and combined Clouds & Indoor children; ambient intensity drives the bounded scene overlay. Level separation, the combined attenuation row and spell-effect controls stay disabled pending projections/renderers |
| Sound | Yes, device + five volume groups | Food/move-item controls belong in item group; all controls are disabled because no audio backend/content exists |
| Battle Sounds | Yes, own/others/creatures | Nested spell categories are present but disabled because no mixer/events exist |
| UI Sounds | Yes, UI/social + console categories | Filters are present but disabled because no audio playback exists |
| Miscellaneous | Yes, seven rows | Confirmation and session selections require explicit consumers; secure remembered sign-in stays OFF/unwired |
| Gameplay | Yes | Inspection/chase/loot intent prepared; engine contracts/consumers pending |
| Screenshots | Yes, two trigger columns | The folder action creates and opens the local Oteryn screenshot directory. The screenshot hotkey performs a real non-overwriting PNG renderer capture, and Only Capture Game Window selects the actual scene crop. Seventeen automatic triggers stay disabled because their event pipeline does not exist |
| Help | Partial | Info is local; complete options export/import and confirmed reset work. Client Help, Compendium, Rule Violations, Manual and FAQ stay visibly disabled because no verified Oteryn destination exists; minimap import/export is likewise disabled because no versioned format exists |

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
  900 × 620 and requires a page-specific marker plus Reset/OK/Apply/Cancel. It
  is a reachability guard; page-family tests and typed setting contracts provide
  additional behavior coverage, but there is not yet an exhaustive per-row
  semantic snapshot.
- A native Windows audit mode opens and scrolls those same production-routed pages,
  reads completed egui geometry and writes captures plus a manifest per page.
  Audit-only frames expand the HUD-condition and both shortcut lists so the
  driven outer scroll reaches every row; ordinary UI keeps the bounded nested
  scrolls. Full marker visibility, horizontal fit, file type, decoded dimensions
  and recomputed SHA-256 are fail-closed. Its implementation is compiled, but a
  fresh run against this candidate remains required before publication evidence
  can be recorded.
- PL/EN composition tests render the dedicated page families without mutating a
  default draft merely by opening them.
- Rust 1.94 locked client tests and strict all-target Clippy pass on Linux. The
  current environment has the Windows GNU Rust target but not
  `x86_64-w64-mingw32-gcc`; the locked cross-target check therefore stops in
  `aws-lc-sys` before compiling the Windows-only shell. Native Windows CI remains
  required for that composition.
- This is compiled reachability and behavior evidence, not a native Windows
  screenshot comparison. Native visual acceptance and every row whose consumer
  remains pending are still open and must not be reported as 1:1 complete.

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

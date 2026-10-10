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
| Basic | Yes | Display controls/action-row visibility work; own-resource bars/arcs now have a vitals renderer behind explicit HUD selections; audio remains pending |
| Controls | Yes | Movement bindings work; three modifier-only controls and grouped delay intent prepared; input consumers pending |
| General Hotkeys | Partial: dedicated profile/action table | Add/copy/rename/remove, two chat contexts, delayed-search layout, two captured bindings and cross-family conflict validation work. Character-name profile auto-switch, movement, Options, fullscreen and individual/edge action-bar visibility have consumers; unsupported inventory rows are disabled and most observed actions remain pending |
| Action Bar Hotkeys | Dedicated 9 × 50 table | Two captured chords per slot in distinct Chat On/Off contexts, migration from 12 slots, modal/held-key routing and both-chord activation work; compiled Windows cross-check and headless page reachability pass, while native Windows visual acceptance remains pending |
| Custom Hotkeys | Partial: observed editor inventory | Text assignments fill the real composer or send through the typed chat intent. Spell/object assignment and legacy bindings are disabled because no authoritative live picker/assignment exists; their persisted presence is not counted as behavior |
| Interface | Yes | Language/contrast work; cursor, link-copy warning and expiry intents prepared; consumers pending |
| HUD | Partial: groups/arcs and all 35 observed conditions | Native PL/EN reachability, per-condition HUD/bar fields, aggregate edits and ordering controls are present; own-health/mana bars/arcs consume real session vitals. Condition-state projection and native live-session visual acceptance remain pending |
| Console | Yes | Chat visibility works; timestamp/message filters await chat/event projection |
| Game Window | Yes | The verified Frame & Highlight / Frame Only / Highlight Only / None dropdown is present; target/effect projection consumers remain pending |
| Action Bars | Yes | Three masters/nine independent 50-slot rows, locks, two chords per slot and session action clearing work |
| Shortcuts | Yes, two lists | Add/remove/order edits affect draft only; Apply updates live registry, Cancel discards |
| Graphics | Yes | Fullscreen/VSync/FPS work; reference arrangement prepared; engine/AA consumers pending |
| Effects | Yes, lighting + opacity groups | Bounded stored percentages; lighting/effect renderer consumers pending |
| Sound | Yes, device + five volume groups | Food/move-item controls belong in item group; audio backend/content pending |
| Battle Sounds | Yes, own/others/creatures | Nested spell categories and volume preferences stored; mixer/events pending |
| UI Sounds | Yes, UI/social + console categories | Filters stored; audio playback pending |
| Miscellaneous | Yes, seven rows | Confirmation and session selections require explicit consumers; secure remembered sign-in stays OFF/unwired |
| Gameplay | Yes | Inspection/chase/loot intent prepared; engine contracts/consumers pending |
| Screenshots | Yes, two trigger columns | Seventeen individual triggers prepared; capture pipeline/storage/folder action incomplete |
| Help | Partial | Info is local; complete options export/import and confirmed reset work. Client Help, Compendium, Rule Violations, Manual and FAQ stay visibly disabled because no verified Oteryn destination exists; minimap import/export is likewise disabled because no versioned format exists |

## Compiled-page verification — 2026-10-10

- A binary-target `egui` regression renders all 20 audited reference pages at
  900 × 620 and requires a page-specific marker plus Reset/OK/Apply/Cancel.
- PL/EN composition tests render the dedicated page families without mutating a
  default draft merely by opening them.
- Rust 1.94 locked client tests and strict all-target Clippy pass on Linux; the
  complete Windows-only client and renderer modules also pass locked cross-target
  check and strict Clippy.
- This is compiled reachability and behavior evidence, not a native Windows
  screenshot comparison. Native visual acceptance and every row whose consumer
  remains pending are still open and must not be reported as 1:1 complete.

## Required evidence per page/panel

- Control type, label, grouping, order and dimensions compared with a named private
  capture; dropdown membership and defaults only when actually observed.
- Unknown settings remain unset; simply drawing a page must not persist defaults.
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

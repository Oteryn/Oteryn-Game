# Native UI fidelity acceptance

Draft #1942 is AUTHORING. Presence of a panel or persisted preference does not
establish visual parity or working gameplay. Original reference captures and
assets remain private; the product ships independently authored code/artwork.

## Reference audit correction

[Live observations](UI-REFERENCE-OBSERVATIONS.md) establish **35 HUD conditions**
and **50 slots in the first bottom action bar**, versus six conditions and 12
slots per row in this draft. Dedicated composition means a page exists; it does
not mean all reference controls were inventoried or implemented. Full replacement
design must follow the completed interaction inventory, not the current forms.

## Page matrix

| Reference page | Dedicated composition | Actual consumer / remaining acceptance |
| --- | --- | --- |
| Basic | Yes | Display controls/action-row visibility work; own-resource bars/arcs now have a vitals renderer behind explicit HUD selections; audio remains pending |
| Controls | Yes | Movement bindings work; three modifier-only controls and grouped delay intent prepared; input consumers pending |
| General Hotkeys | Partial: dedicated profile/action table | Add/copy/rename/remove, two chat contexts, search, two bindings and cross-family conflict validation persist; most general-action runtime consumers and automatic character-profile switching remain pending |
| Action Bar Hotkeys | Dedicated 9 × 50 table | Two chords per slot, migration from 12 slots, modal/held-key routing and both-chord activation work; compiled Windows visual acceptance remains pending |
| Custom Hotkeys | Partial: spell/object/text editors | All three reference editor routes, six object-use modes, two chat contexts and persistence exist; authoritative live spell/item pickers and runtime command consumers remain pending |
| Interface | Yes | Language/contrast work; cursor, link-copy warning and expiry intents prepared; consumers pending |
| HUD | Partial: groups/arcs and only 6 of 35 observed conditions | Native PL/EN reachability, master-dependent editing and six-flag header batch edits verified; complete inventory, ordering and visual acceptance pending; own-health/mana bars/arcs consume real session vitals (live-session visual acceptance pending); names/marks/conditions/order incomplete |
| Console | Yes | Chat visibility works; timestamp/message filters await chat/event projection |
| Game Window | Yes | Effect/message rows prepared; target dropdown membership unverified, legacy frame/highlight intent retained |
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
| Help | Pending | Reference compositions/support actions incomplete |

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

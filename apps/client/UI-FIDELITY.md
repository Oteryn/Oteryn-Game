# Native UI fidelity acceptance

Draft #1942 is AUTHORING. Presence of a panel or persisted preference does not
establish visual parity or working gameplay. Original reference captures and
assets remain private; the product ships independently authored code/artwork.

## Page matrix

| Reference page | Dedicated composition | Actual consumer / remaining acceptance |
| --- | --- | --- |
| Basic | Yes | Display controls/action-row visibility work; pending HUD/audio selections have no consumer |
| Controls | Pending | Movement bindings work; rotation modifiers and keyboard-delay composition need repair |
| General Hotkeys | Pending | Profile/action table and conflict handling incomplete |
| Action Bar Hotkeys | Typed nine-row editor | Slot chords and modal/held-key routing work; reference table arrangement incomplete |
| Custom Hotkeys | Pending | Requires accepted command catalogue, not arbitrary command text |
| Interface | Pending | Language/contrast work; cursor, link-copy warning and expiry controls incomplete |
| HUD | Pending | Actor projection integration and exact two-group/condition-table composition incomplete |
| Console | Yes | Chat visibility works; timestamp/message filters await chat/event projection |
| Game Window | Pending | Target/effect/message controls require projection and composition work |
| Action Bars | Yes | Three masters/nine independent rows, locks, chords and session action clearing work |
| Shortcuts | Yes, two lists | Add/remove/order edits affect draft only; Apply updates live registry, Cancel discards |
| Graphics | Pending | Fullscreen/VSync/FPS work; reference engine/AA arrangement incomplete |
| Effects | Yes, lighting + opacity groups | Bounded stored percentages; lighting/effect renderer consumers pending |
| Sound | Yes, device + five volume groups | Food/move-item controls belong in item group; audio backend/content pending |
| Battle Sounds | Yes, own/others/creatures | Nested spell categories and volume preferences stored; mixer/events pending |
| UI Sounds | Yes, UI/social + console categories | Filters stored; audio playback pending |
| Miscellaneous | Pending | Confirmation and secure session controls require explicit consumers |
| Gameplay | Pending | Inspection/chase/loot contracts and composition incomplete |
| Screenshots | Pending | Separate event triggers, capture pipeline and folder action incomplete |
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

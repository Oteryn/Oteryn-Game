# Winter Update 2026: Quality-of-Life Improvements

Units: WU26-14 to WU26-17.

## 1. Official rules (thread 567, Programmer first post #60921; FAQ #60926 empty)

- **Spellcasting target options** for wave and beam spells, configured per action button
  or hotkey (the same spell can sit on two buttons with different settings). Settings from
  the Magical Archive are not carried over. Three options:
  - *Cast in facing direction* (default): the character's facing.
  - *Cast towards cursor position*: aimed at the mouse cursor **without turning the
    character**.
  - *Cast towards target*: towards the selected target if there is one.
  - Beam and wave patterns: when the direction is ambiguous, east and west win over north
    and south.
- **Stash revisit.** Closing a dialog opened from the Stash (for example the Market)
  reopens the Stash with all filters restored. The sort order is restored every time the
  Stash opens (client-side option).
- **Proficiency catalyst conversion** at Emael (Adventurers' Guild), two dialogue options:
  20 lesser proficiency catalysts -> 1 greater; 4 proficiency catalysts -> 1 greater. No XP
  is granted or lost; the total XP obtainable is unchanged.
- **Mark minimap hotkey**: `Misc.: Create Mark` creates or edits a map mark at the
  character's current position.

Player posts (no CM answer) ask for fall-back behaviour when the cursor is outside the
game window; not part of the spec.

## 2. Client evidence

New strings: `Target Options:`, `Cast in facing direction`, `Cast towards cursor position`,
`Cast towards target`, `Misc.: Create Mark`. Removed: `Aim at target` (the old single
toggle). No appearance changes.

## 3. What Oteryn has today

- Cast intent: `crates/protocol-oteryn/src/actor_spell.rs:65`-`:91`
  (`SpellTarget::{None, AttackTarget, Position}` plus `aim_at_target`; no aim-mode field).
- Server direction: `crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/prepare.rs:305`-`:337`:
  `None` uses the current facing; `AttackTarget` takes the facing toward the target and keeps
  the caster-adjacent origin (`crates/gameplay/spell-rules/src/dispatch_plans/geometry.rs:40`-`:52`);
  `Position` makes the **position itself** the geometry origin, and with `aim_at_target` the
  facing toward it. `facing_to` (`geometry.rs:22`-`:36`) returns eight directions and resolves
  ties to diagonals, not to east/west.
- Client action bar: per-slot key bindings in
  `apps/client/src/features/local-reference-preferences/bindings.rs:30`-`:37`; the directional
  test `apps/client/src/features/local-spell-action-bar/tests/directional.rs` sends
  `SpellTarget::None` (server-owned direction). No per-button target option exists.
- Stash: Market opened from the Stash sets `return_from_market`
  (`crates/client-ui/src/features/0062-stash/state.rs:11`,
  `apps/client/src/features/0063-market/input.rs:90`); returning needs the explicit
  "Stashback" button (`input.rs:103`-`:115`, `apps/client/src/features/0063-market/native.rs`);
  filters and sort survive the round trip
  (`apps/client/src/features/0062-stash/tests/play.rs:113`
  `stash_market_round_trip_keeps_context`). Closing the Market does not reopen the Stash,
  and the sort order is not persisted between openings.
- Catalysts: progress per item `i54266` lesser 5,000, `i51588` 25,000, `i51589` greater
  100,000 (`crates/gameplay/durability/src/durability/item_use/proficiency_catalyst.rs:19`-`:27`),
  so 20 lesser = 4 regular = 1 greater in progress terms. Emael (`oteryn:npc.emael`,
  `content/npcs/definitions/npcs-00000-00499.json`) has an empty `services` list and only
  greet/name/job dialogue.
- Minimap marks: `apps/client/src/features/local-minimap/view.rs:63` (`add_mark`), editor
  opened by Ctrl+Alt+Shift+M
  (`crates/client-ui/src/features/local-minimap/registrations/editor-open.json`). No
  one-key "Create Mark at my position" binding.

## 4. Gap as implementation units

### WU26-14 Spell target options for wave and beam spells (class A, M)

- Owned files: `crates/protocol-oteryn/src/actor_spell.rs` (aim mode), contract under
  `docs/contracts/protocol-oteryn/v1/`, `crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/prepare.rs`,
  `crates/gameplay/spell-rules/src/dispatch_plans/geometry.rs`, client action bar
  (`apps/client/src/features/local-spell-action-bar/`,
  `crates/client-ui/src/features/local-spell-action-bar/` if present) and per-slot
  preference storage.
- Protocol: aim mode on the cast intent: FACING (1), CURSOR (2) with a target position,
  TARGET (3). Old `aim_at_target` maps to TARGET. New capability number (claim it).
- Server: for directional spells only, CURSOR and TARGET pick a **cardinal** direction
  from the caster to the point (`|dx| >= |dy|` -> east/west, else north/south; so ties
  go east/west); the origin stays the tile next to the caster; CURSOR does not change the
  character's facing; TARGET without a target falls back to facing. Non-directional spells
  are unchanged.
- Client: per action button / hotkey option with the three strings; FACING default.
- Acceptance: `wave_cursor_mode_tie_prefers_east_west`,
  `wave_cursor_mode_keeps_caster_facing`, `beam_target_mode_without_target_uses_facing`,
  `directional_position_target_keeps_caster_adjacent_origin`, client test
  `action_slot_target_option_round_trips`.

### WU26-15 Proficiency catalyst conversion at Emael (class B, S)

- Owned files: Emael service and dialogue records in `content/npcs/definitions/` and
  `content/dialogues/definitions/` (service recipes per
  `crates/gameplay/content/src/content/project/v2.rs` `ProjectV2ServiceRecipe`), pin refresh.
- Rules: recipe A 20 x `i54266` -> 1 x `i51589`; recipe B 4 x `i51588` -> 1 x `i51589`; no
  fee stated; no proficiency progress changes.
- Acceptance: `emael_converts_twenty_lesser_catalysts_to_one_greater`,
  `emael_converts_four_catalysts_to_one_greater`,
  `emael_refuses_conversion_with_too_few_catalysts`; `pin-check` green.

### WU26-16 Create Mark hotkey (class B, S; client only)

- Owned files: a new registration in `crates/client-ui/src/features/local-minimap/registrations/`,
  `apps/client/src/features/local-minimap/native.rs` / `view.rs`.
- Rule: one binding `Misc.: Create Mark` that creates a mark at the player's current
  position, or opens the editor on the existing mark at that position.
- Acceptance: `create_mark_hotkey_adds_mark_at_player_position`,
  `create_mark_hotkey_edits_existing_mark`.

### WU26-17 Stash revisit (class B, S; client only)

- Owned files: `apps/client/src/features/0063-market/input.rs`, `native.rs`,
  `crates/client-ui/src/features/0062-stash/state.rs`, client preference storage for the
  sort order.
- Rule: closing any dialog opened from the Stash reopens the Stash with its filters; the
  Stash sort order is persisted client-side and applied on every opening.
- Acceptance: `closing_market_opened_from_stash_reopens_stash_with_filters`,
  `stash_sort_order_persists_between_openings`.

## 5. Open questions

1. Whether "Cast towards cursor position" falls back to facing when the cursor is outside
   the game window (asked by players; no CM answer).
2. Which spells count as wave/beam for the option (Oteryn: profiles with
   `targeting.needs_direction`); whether Divine Dazzle-style chain spells are excluded.

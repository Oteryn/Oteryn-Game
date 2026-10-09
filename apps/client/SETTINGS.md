# Native client preferences

The owner requested the complete intended preferences experience, available before
sign-in and during play. This is the working implementation and remaining product
scope, not a declaration that the whole game client is complete.

Implementation status refers to `feat/client-completion-20261009`. This is an
authoring candidate, not merged main or production readiness.

## Implemented

The Windows client opens the same preferences panel from the login toolbar and
with F10 during an admitted session. Opening the panel suppresses gameplay input;
the session continues ticking. Apply and save validates the draft, persists it,
then updates the current configuration. Cancel discards edits. Defaults resets
the draft and requires Apply to persist. An invalid file falls back to defaults
and reports the read failure inside the panel. A failed save preserves the active
configuration and reports the failure.

| Section | Available behavior |
| --- | --- |
| Display | Borderless fullscreen, five window sizes, VSync, foreground FPS limit including unlimited, background FPS limit |
| Interface | Polish/English, UI scale, panel contrast, reduced loading animation |
| Controls | Arrow/WASD presets, individually assigned physical movement keys, duplicate rejection, click-to-walk switch |
| Connection | Current launch profile, account portal and gateway, refresh public world availability |
| Privacy | Local preferences location, normal browser authentication, account-switch/sign-out entry points, client version |

VSync goes through the renderer's existing surface resize/configuration state
machine. Frame pacing uses event-loop deadlines rather than sleeping the input
thread. Preferences do not alter server movement speed, channel allocation,
admission deadlines or claims. Interface scale does not change the map projection.
The local settings JSON contains no account identifiers, passwords or tokens.

## Remaining intended sections

These categories are visible with their availability explained. No decorative
control pretends to change a subsystem that does not exist.

| Section | Required implementation before exposing active controls |
| --- | --- |
| Audio | Playback/mixer and licensed audio content; master, music, effects, ambience and UI levels; output device; background mute |
| Game overlays | Actor names/health, damage text, map grid, minimap and panel layout, with actual scene/domain data |
| Chat | Chat transport and UI; font size, timestamps, channel filters, notifications, moderation/block-list contract |
| Action shortcuts | Actual inventory, targeting, combat and spell actions, conflict-aware bindings and text/modal routing |
| Accessibility | Color-safe game indicators, scalable HUD/text and keyboard focus across the implemented game UI |

Completion criteria for each setting: a persisted validated preference; an actual
consumer in the owning subsystem; apply/cancel/defaults behavior; no disruption
of authentication or server authority; an observed Windows runtime check. Merely
adding a slider or field does not meet these criteria. Linux native composition
is a separate platform path; this Windows UI does not establish Linux GUI parity.

## Live reference inspection, 2026-10-08

The owner provided the running Synology container `otclient-track-a-kasmvnc` as
an interface/options reference. The inspected window was the official Tibia
client, confirmed by its executable under `CipSoft GmbH/Tibia/packages/Tibia`.
The container name alone does not identify the running implementation as OTClient.

Observed directly: health/mana/status bars, central game viewport, right minimap,
equipment slots, battle list, backpack, quest/player-guide panels, action slots,
bottom tabbed chat and log. The Basic Options page groups Gameplay, Interface,
Graphics and Sound. Advanced navigation groups Controls, Interface, Graphics,
Sound and Miscellaneous. Controls provides General, Action Bar and Custom Hotkeys;
Interface provides HUD, Console, Game Window, Action Bars and Shortcuts.

The follow-up inspection visited every page in the advanced navigation: Controls
and its three hotkey pages; Interface and its five subpages; Graphics and Effects;
Sound, Battle Sounds and UI Sounds; Miscellaneous, Gameplay, Screenshots and Help.
Together with Basic Options, these are 20 visually inspected pages. Screenshots
were checked for the actual page content; capture filenames alone are not proof.
The private coverage catalogue is
`/workspace/artifacts/oteryn-reference/REFERENCE-AUDIT.md`.

Further discovery used **names only** from six preference sections of
`conf/clientoptions.json`: 213 names total. No account, cookie or other preference
values were exported in that catalogue. Earlier incorrectly named captures of the
world remain excluded. The follow-up corrected input to match the installed
KasmVNC frontend's extended pointer-message format and checked the modal before
navigation. No Reset, profile deletion or purchase was executed. During the subsequent
sidebar audit, the owner authorized adding all 17 available shortcuts through
the normal settings UI. This changed shortcut visibility only.

This establishes the visible settings scope, not exhaustive behavior testing of
every combination, dropdown or scrolled binding. The owner restored the session;
the sidebar audit is now in progress. Its verified coverage and outstanding
panels are recorded in [REFERENCE-AUDIT.md](REFERENCE-AUDIT.md).

### Additional directly verified details

- HUD independently controls the owner and other creatures. It includes health,
  mana, names, marks, NPC icons, arcs with size/distance/opacity, special-condition
  visibility/order, customisable status bars and status bars.
- Console filters distinguish own/other status, events and information; private
  messages can open a tab; timestamps optionally include seconds and levels.
- Game Window controls textual effects, private/general messages, potion sounds,
  own/other spells, hotkey notices, loot notices/highlighting, training progress,
  combat/PvP frames, attack animation, banner and target frame/highlight.
- Action Bars expose three rows at each of bottom/left/right, labels, object
  amounts, spell parameters, graphical/numeric cooldowns, tooltips and automatic
  insertion of new spells. Clear-row buttons exist but were not used.
- Shortcuts has separate displayed/available lists and add/remove/order controls.
  Visible entries include skills, battle/spell/VIP lists, help, quests, compendium,
  cyclopedia, highscores, player guide, party, wheel and unjustified points.
- Graphics exposes engine, anti-aliasing, fullscreen, integral scaling, VSync and
  frame limiting. Effects separates ambient light, level separator, indoor/cloud
  attenuation and own/other/creature/boss-area spell opacity.
- Sound separates device, master/music/ambience/items/events; combat mixes own,
  other players and creatures with attack/heal/support/weapon/noise/death filters.
  UI audio includes interactions, party/VIP notices and per-channel chat filters.
- Miscellaneous includes confirmation prompts, session retention, connection
  stability and quick login. Gameplay includes inspect permission, auto-chase
  cancellation and nearby-corpse looting. These require accepted Oteryn mechanics.
- Captures includes game-only capture, five-second backlog and event triggers for
  progression, achievements, bestiary, treasure/loot/bosses, death/PvP, damage,
  healing, low health and Gift of Life. Help exposes documentation plus options
  and minimap import/export and reset; destructive/export actions were not run.

### Expanded intended Oteryn settings

The following is the product scope informed by that reference. It is not a claim
that these controls are currently implemented or that every Tibia feature belongs
in Oteryn. Commercial, inspection, loot, combat and social behaviors require the
corresponding accepted Oteryn domain feature. Preferences cannot create authority.

| Group / subpage | Intended controls | Owning dependency |
| --- | --- | --- |
| Controls / Movement | Mouse presets; arrow/WASD/custom direction keys; click walking; rotation modifier; held-key behavior; drag/drop modifier | Existing input router and actual gameplay actions; current movement settings already available |
| Controls / Hotkey profiles | Named profiles; add/copy/rename/remove; action search; primary/secondary shortcut; conflict feedback; chat-on/chat-off contexts | Action catalogue, persisted bindings, gameplay/text/modal routing |
| Controls / Action shortcuts | Bottom/left/right slot bindings; custom supported actions; visible binding labels | Implemented inventory/spell/combat commands and action bar |
| Interface / General | UI scale/language/contrast; cursor size and animation; target highlight; cooldown visibility; link confirmation | Cursor/egui integration and actual targeting/cooldown state |
| Interface / HUD | Player/creature names, health/mana, marks and status icons; bars/arcs; arc size, distance and opacity; target/PvP frames | Server-backed actor and status projections |
| Interface / Console | Text size; timestamps and seconds; channel tabs; levels; info/event/status/join-leave filters; private message tab behavior | Chat UI and accepted chat/session events |
| Interface / Game window | Resize/fit behavior; integer scaling; configurable top/bottom panes; targeting frame; loot highlight; floating text; own/other spell and system-message visibility | Scene viewport layout, camera/coordinate mapping, actual effects/events |
| Interface / Action bars | Bottom/left/right bars, individual rows, lock/unlock; tooltips; item amounts; hotkey labels; spell parameters; numeric/graphical cooldown | Action model, server-backed inventory/cooldowns and layout persistence |
| Interface / Panels | Minimap, equipment, backpacks, battle list, skills and quest tracking; dock/show/hide/order; retain layout | Actual domain snapshots plus bounded panel layout |
| Graphics / Display | Window mode and dimensions; monitor selection; VSync; foreground/background FPS; FPS/latency indicator | Window/surface/event-loop integration; basic display preferences already available |
| Graphics / Rendering | Supported renderer/filter choice; nearest versus supported smooth scaling; ambient light and light effects; own/other/monster effect opacity | Renderer capabilities, lighting/effect pipeline; no unsupported backend selector |
| Sound / Mixer | Master, music, ambience, events, UI, items; own/other/creature combat levels; output device; mute in background | Playback backend, mixer and licensed sound content |
| Sound / Event filters | UI interactions, item movement, eating, social notices; attack/heal/support spell and creature sound groups | Actual supported audio events, not invented game events |
| Miscellaneous / Captures | Manual screenshot, folder, game-only/full-client capture; optional event-triggered captures and bounded backlog | Renderer capture and actual achievement/level/death/loot events |
| Connection / Diagnostics | Current server profile; world status; safe connection diagnostics; FPS/latency visibility | Public directory and existing session diagnostics; preserve trust and admission limits |
| Account / Privacy | Browser sign-in; explicit account switch/sign-out; local preferences; approved diagnostics controls | Existing PKCE/Platform identity; never local password/token persistence |

### Intended in-world composition

1. A responsive central viewport with correct pointer-to-tile mapping after panels
   resize. Use the loaded world and server-owned actor/session state.
2. Health/mana/conditions at the top, populated only by supported authoritative
   data; no sample statistics presented as character state.
3. Docked minimap, equipment, containers, battle/skills/quest panels with saved
   placement and visible close/open controls.
4. Action bars beside/below the viewport, plus a bottom tabbed console whose text
   focus suppresses movement and action shortcuts.
5. One searchable settings dialog usable before admission and during play, with
   basic/advanced discovery, contextual help, apply/cancel/defaults and explicit
   unsupported-feature handling. UI visibility toggles do not change game rules.

Implement and review in small owning batches: viewport/layout and existing-state
panels; actor HUD/minimap; inventory and action bars; chat; hotkey profiles; effects
and audio; capture/diagnostics. Each batch must connect visible preferences to its
real consumer and pass the completion criteria above. The current login UI and
seven-section preferences panel are the foundation, not this complete composition.

Reference screenshots and the sanitized name catalogue are private task artifacts
under `/workspace/artifacts/oteryn-reference`; they are not bundled game assets.
Oteryn uses its own branding, textures and implementation.

## Prepared reference-completion candidate, 2026-10-09

The complete browser exposes27 groups and276 typed bilingual controls.16 map
to existing client fields; remaining options keep validated user intent with
explicit pending-consumer labels. They do not simulate missing engine behavior.

In-world UI includes28 panel shortcuts with differentiated fields/tabs, six
additional reference-derived dialogs, a minimap of actually loaded terrain and
12 action slots. Session item/spell handles never persist across admission.
Future gameplay values remain unknown; pending mutation actions are disabled.

OS-vault primitives use local Windows credential storage or encrypted Linux
Secret Service, without plaintext fallback. They are not wired into remembered
sign-in yet. Browser PKCE remains current behavior; rotation recovery, per-origin
process serialization and reviewed Platform HTTP integration remain outstanding.


The full preferences tree is the default F10/login entry, with search and its own
Apply/Cancel/Defaults footer. Connection/account details remain accessible from
that footer. 68 additional controls have names-only reference provenance; their
precise original behavior is not inferred from configuration-key names. Future
preferences store user choices without pretending to enable absent consumers.

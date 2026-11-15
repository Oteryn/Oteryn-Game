# Shared Bevy desktop client

`oteryn-client-bevy` is the opt-in desktop presentation over the existing Rust
admission, session, gameplay, preferences, scene and audio owners. It is a partial
replacement for the legacy presentation. Build it from the same checkout as the
server and its pinned content.

## Build

On Windows, Linux or macOS, from the repository root:

```sh
cargo build --locked -p oteryn-client --bin oteryn-client-bevy --features bevy-shell
```

`--features bevy-shell` is a Cargo option, not an executable option. The output is
`target/debug/oteryn-client-bevy` (`.exe` on Windows), unless Cargo's target directory
is configured separately. Native Linux needs the X11 and ALSA development libraries
used by upstream winit and audio. Cross-compilation alone does not qualify native
window, graphics or audio behavior.

## External assets and sign-in

Supply both roots explicitly:

- `--ui-assets`: the private UI-art root containing the required skin images and
  `fonts/verdanab.ttf`. Startup checks this input; original assets are not included
  in the repository.
- `OTERYN_ASSET_DIR`: the matching pinned Oteryn world-asset root. Before starting
  sign-in, the client validates the pinned manifest, catalog and appearance
  metadata. Sprite sheets retain the existing lazy verification.
  This root contains both `content/assets/files/` and
  `imports/official/client-assets/15.30/manifest.json` (normally the checkout root),
  rather than pointing directly at the files directory.

On Linux/macOS, after the approved test connection environment is loaded:

```sh
export OTERYN_ASSET_DIR='/explicit/world-asset-root'
./target/debug/oteryn-client-bevy --ui-assets '/explicit/ui-art-root'
```

On Windows PowerShell:

```powershell
$env:OTERYN_ASSET_DIR = 'C:\explicit\world-asset-root'
.\target\debug\oteryn-client-bevy.exe --ui-assets 'C:\explicit\ui-art-root'
```

The paths above are placeholders, not defaults. Obtain the actual roots and the
approved connection-environment handoff from the test-server coordinator. The
executable does not load a file merely because it is named `client.env`; use the
existing environment-loading procedure for that handoff. Keep its contents out
of logs and the repository.

Open Connect, complete the existing login form and choose a character/channel.
The frontend uses the normal admission path; an offline window does not prove a
successful server connection. Missing or mismatched world metadata stops sign-in
before an admission request is started.

Optional executable arguments:

| Argument | Purpose |
| --- | --- |
| `--preferences <path>` | Override the existing preference-store path. |
| `--login-settings <path>` | Override the existing login-settings path. |
| `--ui-audit <path>` | Write diagnostic UI state; F12 also writes a rendered PNG beside it. Keep captures containing private assets/session UI private. |

## Gameplay UI entry points

With no text field focused, the existing gameplay shortcuts open:

- **Ctrl+Shift+X:** greet the selected, visible NPC. Its real replies, keywords and
  text input share one window with the trade addon when the server opens a shop.
- **Ctrl+J:** Quest Log. Search by quest ID or known mission name/text, cycle
  numeric/status sorting, and toggle completed quests. Selecting a mission shows
  its journal text; Track / Untrack quest updates the existing quest tracker after
  server confirmation. The tracker remains visible beside the journal entry.
  **Ctrl+Alt+Shift+Q** also opens the quest tracker in the journal.
  Quest names and independent mission tracking are not supplied by the current
  protocol, so alphabetical quest sorting and mission-level tracking are unavailable.
- **Ctrl+Alt+Q:** QuickLoot management, subject to the negotiated capability.
- **Ctrl+Shift+Y:** the selected analyser window. The Analysers HUD selector opens
  any of the fourteen retained tools. Reports update live; Reset session uses the
  existing local reset, and Close or Escape dismisses the window.
- **Ctrl+V:** VIP contacts and group management through the existing server requests.
- **Ctrl+I:** Imbuing shrine: choose a carried item, then a slot and imbuement.
  Clear asks for confirmation. Refresh queries the existing shrine command;
  the protocol has no unsolicited open event, balance, or protection charm.
- **Ctrl+D:** Daily Reward wall, free/premium catalogue, streak and account/World
  history. Collect uses the server-confirmed shrine availability.
- **Ctrl+M:** Market catalogue, named wares, offer books and statistics, My Offers
  and Offer History. Create buy/sell offers, cancel selected own offers, and accept
  buy/sell offers through the existing server intents. Accept quantities clamp to
  the remaining offer amount and, for buy offers, the item's known depot stock.
  Create requires amount 1–64000, price ≥ 1, and total ≤ 999999999999 gold.
- **Ctrl+Shift+S:** coin Store, with category browsing, purchase confirmation,
  coin gifts/transfers, purchase history and the coin ledger.
- **Ctrl+Shift+P:** Weapon proficiency. Requests the existing server catalogue,
  shows weapon levels and XP progress, and offers perks only at reached levels.
  This protocol has no server event that opens this window.
- **Ctrl+Shift+H:** house access lists. Query the town catalogue, select a row,
  load guests/subowners/a door, then edit the multiline list and enter CONFIRM
  before saving. The current server requires member UUIDs; name resolution and
  book/sign author/date descriptors are not available on the current wire.
  Server modal messages without an input remain read-only; text inputs retain
  their existing edit and answer path.
- **Ctrl+O:** Cyclopedia character: general and combat stats, recent deaths and PvP
  kills, achievements, item summary, outfits/mounts, store summary, inspection and
  badges. Selecting a tab requests its data; Escape or Close dismisses the window.
- **Ctrl+Shift+G:** Party List and its existing membership actions. The world
  context menu exposes Party and an invitation for a visible player, subject to
  confirmed party state and server authority.

Right-click a visible, named player and choose **Report** to open the rule-violation
window (requires capability 0076). **Ctrl+Alt+R** opens it with an empty character
field. No query is sent on open: protocol 0076 carries report submission and reply
only. Choose a reason and action, enter a comment,
then Send. The window uses the protocol-supported Name, Statement and Bot report
types; legacy OTClient moderation penalties are not report actions in this protocol.

Market and Party also have buttons on the gameplay toolbar.
The gameplay toolbar lists windows alphabetically: Analysers, Battle, Chat,
Inspection, Inventory, Loot Containers, Market, Minimap, Notices, Options, Party,
Quest Log, Skills, Spellbook and VIP. Buttons open the existing retained window
owners or reveal their HUD panel; Skills refreshes its always-visible panel.
The label/action registry is `TOOLBAR` in `src/bevy_shell/ui.rs`.

Achievement notices appear in arrival order, one banner at a time. Each fades in
for 400 ms, holds for 3 seconds, then fades out for 300 ms, matching OTClient's
notification timing constants. Click a banner to dismiss it and start the next.
The queue belongs to the current play session and clears when that session ends.

Ctrl+V in a focused text field remains the native paste action. Click an exposed
world area to leave a text field before using a gameplay shortcut. Map/item drag
currently supports the desktop primary mouse pointer; touch qualification is later.

## Qualification boundary

The shared host has been exercised through native Linux rendering and OS input,
and compiled for Windows GNU. Native Windows execution and a live admitted game
session still require their own test run. It is **not qualified on native macOS**.
Android and browser qualification are outside the current desktop slice.

Item entries and NPC offers without a negotiated, server-verified presentation
binding use neutral unavailable labels/icons. A definition reference is not an
appearance ID. The existing Look/inspection commands remain the source of item
details; the client does not guess a mapping.

## Hotkeys

Open Options (Ctrl+Shift+O), enable Advanced, and select Controls. The hotkey manager
lists key chords, actions and text. Select a row to edit it, or New to add a
binding. Action cycles between text, use on self, use on target, use with
crosshair, and ordinary object use. Object actions take the existing item
definition id and subtype. Send automatically sends a text macro immediately;
when disabled it fills the chat draft. Add / Save updates the draft; Remove
removes the selected binding. Apply or OK persists through the existing scoped
settings store; Cancel discards edits.

## Floor visibility

The world view consumes the map header's authoritative `first_visible_floor`.
Floors above that boundary fade out over 500 ms; returning floors fade back in,
and reversing a transition preserves its current opacity. Minimap and HUD quads
are unaffected. The client retains supplied floor geometry for these transitions.
It does not infer roofs from sprite presence: the wire does not expose the full
stack's terrain kind and projectile blocking facts needed to recompute OTClient's
covered-tile visibility rule locally.

## Outfit editor

Press Ctrl+Alt+A while playing to open Outfit. Select a granted outfit, owned
addons, four palette colours and a mount, then OK to send the existing appearance
change. Cancel closes the editor. The standing preview uses local verified sprite
sheets; mounts display their catalogue keys because the model has no mount sprite
IDs. The catalogue arrives in appearance snapshots and updates; the protocol has
no separate outfit-window event or catalogue request command.

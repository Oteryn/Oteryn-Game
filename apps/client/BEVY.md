# Shared Bevy desktop client

`oteryn-client-bevy` is the opt-in desktop presentation over the existing Rust
admission, session, gameplay, preferences, scene and audio owners. It is a partial
replacement for the legacy presentation. Build it from the same checkout as the
server and its pinned content.

The accepted login bulletin opens after admission. Reopen it with **News** in the
connection form or **Ctrl+Alt+Shift+L** while playing. Its read-only news/calendar
view supports month navigation, scrolling, and Close/Escape dismissal.

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

From the character picker, **Account Services → Premium** opens the dedicated
read-only overview, with expiry, remaining time, benefits and Refresh. Back or
Escape returns to the account menu. It uses the existing separately scoped
account-services sign-in. The accepted Platform `GET /api/v1/native-account/premium`
projection must be deployed for live data; unavailable services show feedback.
Bazaar mutations and pre-login coin purchases remain separate presentation work.

Optional executable arguments:

| Argument | Purpose |
| --- | --- |
| `--preferences <path>` | Override the existing preference-store path. |
| `--login-settings <path>` | Override the existing login-settings path. |
| `--ui-audit <path>` | Write diagnostic UI state; F12 also writes a rendered PNG beside it. Keep captures containing private assets/session UI private. |

F12 also works without `--ui-audit`: it saves a uniquely named BMP from the rendered
window. Options → Show Advanced Options → Screenshots accepts an absolute folder
path (Use folder, then Apply/OK); an empty path uses `screenshots` beside the normal
client preference file. Cancel discards destination edits. Capture game area only
uses the visible map rectangle and reports an error if no map is available. The
status line reports the saved path or write error. Audit mode retains its diagnostic
PNG destination. Automatic event captures/backlogs are not wired in this Bevy batch.

Reference: OTClient `dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`src/framework/graphics/image.cpp:66–78` encodes image pixels and flushes the output;
that revision has no ordinary screenshot action in its modules. This host uses
Bevy framebuffer readback and Oteryn's existing exclusive-create BMP writer.

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
- **Ctrl+W:** Wheel of Destiny allocation and saved point presets. Left click a
  reachable node to add one point; right click removes one when permitted.
- **Ctrl+Shift+T:** Task Board: Bounty Tasks, Weekly progress/results and Shop.
  Rerolls require confirmation and cost one token. Daily tokens are claimed at
  the Daily Reward Wall; the board shows the balance and cap.
- **Task tracker (below the minimap):** stays visible while playing and after closing
  the Task Board. Shows confirmed Bounty/Weekly kills, deliveries, rewards and Prey
  bonuses/hunting time; click anywhere in the tracker to open the Task Board.
  Closed editors refresh through existing read-only queries every five seconds;
  pending requests are never duplicated. Hunting time uses the server's combat
  clock; free-reroll timestamps count down locally. Session changes reload durable tasks.
  Prey remains unavailable until the server offers existing capability 140.
  NEEDS-CONTRACT: `task_board::full::Reply` lacks the authoritative weekly reset Unix
  timestamp; `prey::Slot` lacks kill totals. Neither is inferred from local kills
  or the board's reset-period identity.
  References: Global manual `docs/reference/tibia-manual/interface.md` §3.6.10;
  OTClient `modules/game_questlog/questlog.lua` (server progress and game-end cleanup).
- **Ctrl+Z:** Bug report with category, multiline description, review and Send.
- **Ctrl+Shift+Y:** the selected analyser window. The Analysers HUD selector opens
  any of the fourteen retained tools. Reports update live; Reset session uses the
  existing local reset, and Close or Escape dismisses the window.
- **Ctrl+V:** VIP contacts and group management through the existing server requests.
- **Ctrl+I:** Imbuing shrine: choose a carried item, then a slot and imbuement.
  Clear asks for confirmation. Refresh queries the existing shrine command;
  costs include apply/clear gold, material requirements and success/failure chance.
  Balance and protection availability are explicitly identified; the protocol has no
  unsolicited open event, balance, protection price or protected-apply request.
- **Ctrl+Alt+Shift+I:** independent, nonmodal Imbuement Tracker (also opened from
  Imbuing). Automatically inspects equipped items; All/5/60-minute filters, Refresh
  and Close. Durations remain confirmed server snapshots, never wall-clock estimates.
  Reference: Tibia manual `interface.md` §3.6.24 and `characters.md` §Imbuing.
  OTClient `dab86e610991092fe8b2471f7de55a7c986b3cd2` has no imbuing module;
  its `modules/game_healthinfo/healthinfo.lua:29-66` is the independent HUD reference.
  NEEDS-CONTRACT: negotiated protection quote (item, family/tier, protection gold,
  available gold) and protected apply (item, slot, offer, protection choice). Commands
  85/731 do not carry these fields. Artwork and shrine-driven opening remain absent.
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
- **Ctrl+Shift+H:** Houses. Query the town catalogue and select a numbered row.
  Enter a maximum gold bid and CONFIRM, then Bid to submit an auction bid.
  Owned / incoming loads the separate ownership catalogue; its Previous/Next
  buttons page through owned houses and incoming offers. Use its numbered row
  for disposition actions. For a transfer, enter the recipient character name
  in the member field, the gold price in the amount field, and the effective
  date as YYYY-MM-DD HH:MM:SS UTC in the door/date field. Review the transfer,
  acceptance, rejection, cancellation or move-out terms, then use Confirm
  reviewed action (or Cancel review). Review alone sends no mutation.
  Podium and hireling management remain separate, unimplemented presentations.
  For access lists, select a town catalogue row,
  load guests/subowners/a door, then edit the multiline list and enter CONFIRM
  before saving. The editor identifies the selected house by its server-supplied
  name (falling back to its house key) and requests one member UUID per line.
  Empty lists remove all members. Names cannot be resolved by the current server.
  Server modal messages without an input remain read-only; text inputs retain
  their existing edit and answer path, including any metadata in the message.
  **Batch 52: PARTIAL / NEEDS-CONTRACT.** Capability 49's ACL snapshot contains
  only a revision and member UUIDs. Missing: a World-scoped character-name
  resolution request/reply (name, canonical name, character UUID, and explicit
  not-found/ambiguous results), plus canonical member names in ACL snapshots.
  VIP presence entry numbers are list-local and cannot supply that mapping.
  Capabilities 450/490 have no book/sign descriptor tied to the dialog ID with
  item identity/type, writer and written date; those fields need an owning
  protocol addition before the client can format them independently of body text.
  No protocol, capability or persistence changes are introduced here.
  Client reference: [OTClient text windows](https://github.com/edubart/otclient/blob/dab86e610991092fe8b2471f7de55a7c986b3cd2/modules/game_textwindow/textwindow.lua#L25)
  (structure reference only: explicit writer/time inputs and server-supplied list text).
- **Ctrl+U:** Cyclopedia character: general and combat stats, recent deaths and PvP
  kills, achievements, item summary, outfits/mounts, store summary, inspection and
  badges. Selecting a tab requests its data; Escape or Close dismisses the window.
- **Ctrl+P:** Prey Creatures, with three server-confirmed slots and reroll confirmation.
- **Ctrl+Shift+F:** Exaltation Forge: Fusion, Transfer, Conversion and History.
  Opens locally and requests current forge candidates; this protocol has no server-open event.
- **Ctrl+S:** Supply Stash, with search/category/trader filters, name/stock sort,
  and a bounded withdrawal dialog. Stash rows show definition names because the
  protocol supplies no verified artwork binding.
- **Ctrl+Alt+I:** Inspection. Use **Inspect selected** to request the selected
  inventory item's data. Multi-item replies show a selectable list and one details
  pane, including equipment slots returned by player inspection. Cells use item
  names when appearance IDs are unavailable. Player outfit and profile details
  require additional server data. Right-click a visible player and choose
  **Inspect** to request equipment inspection, subject to server permission.
- **Ctrl+Shift+G:** Party List and its existing membership actions. The world
  context menu exposes Party and an invitation for a visible player, subject to
  confirmed party state and server authority.

Right-click a visible, named player and choose **Report** to open the rule-violation
window (requires capability 0076). **Ctrl+Alt+R** opens it with an empty character
field. No query is sent on open: protocol 0076 carries report submission and reply
only. Choose a reason and action, enter a comment,
then Send. The window uses the protocol-supported Name, Statement and Bot report
types; legacy OTClient moderation penalties are not report actions in this protocol.

**Ctrl+Alt+Z:** explored-world overview (satellite map), exact modifiers; arrows pan,
Escape or Close dismisses it. F10 stays audio mute and Ctrl+Alt+Shift+F10 notice banners.
Like every shortcut it is ignored while a modal window or text field has focus.

The gameplay toolbar opens the retained window owners or reveals their HUD panels;
Skills refreshes its panel. Manage Buttons shows/hides shortcuts, moves displayed
buttons up/down, and restores the alphabetical default. Changes save immediately
in the current local preference scope and do not change keyboard bindings.
The manager remains available when all shortcuts are hidden. Existing keyboard
windows (including Action Bars, Map Overview, Houses, Cyclopedia, Rewards, Forge, Highscores, Imbuing, NPC dialogue,
Outfit, Prey, reports, Stash, Store, Tasks, Proficiency and Wheel) have toolbar entries.
NPC Trade uses the selected NPC, exactly like Ctrl+Shift+X.
Also on the toolbar: Bestiary, Bosstiary, Compendium, Cyclopedia Map, Depot, Social, Store Inbox and
Unjustified Points.
The label/action registry is `TOOLBAR` in `src/bevy_shell/ui/toolbar.rs`.

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

Press Ctrl+Alt+A (rebindable as `0025-appearance-open`) while playing to open Outfit. Select a granted outfit, owned
addons, four palette colours and a mount, then OK to send the existing appearance
change. Cancel closes the editor. Turn left/right rotates the local preview; Movement
on/off switches between the verified walking and standing frame groups. Animation
loops using the asset phase durations, updating the existing preview image without
rebuilding the editor. Both paths retain the selected colours and addons.
The selected mount is drawn beneath the mounted rider, sharing the direction and
animation clock. Its key-to-look binding comes from the existing local cosmetics
metadata and is accepted only when the normalized document digest matches the
server catalogue's cosmetics digest. Missing/stale bindings display an error;
no mount IDs or ownership are inferred. The catalogue arrives in appearance
snapshots and updates; the protocol has
no separate outfit-window event or catalogue request command.

Cyclopedia Items: open **Item encyclopedia** from Character Cyclopedia or press
**Ctrl+Alt+Shift+I**. Browse/search public item templates, cycle the server's
categories, select an item, request details, and view live market offers plus
30-day completed-trade prices. Next page restarts the search if its text changed.
NPC/city price directories need a contract for an item-scoped, paged response
containing NPC name, city, buy/sell side, price and currency; neither the public
item catalogue nor local NPC trade windows provide that directory. Loot-value
editing and Drop Tracker integration belong to a separate batch.

Bestiary is available from the **Bestiary** toolbar button (and the retained
Bestiary shortcut). The dedicated window loads capability-67 progress followed
by its catalogue, with category navigation, first/next pages, creature selection,
three discovery thresholds, confirmed creature details and paged loot. Refresh
progress reloads the server's kill counts; Close/Escape dismisses the window.
Unreported progress stays unknown rather than becoming a fabricated zero.
Charms and tracker controls are separate work.

Full Global discovery disclosure is **NEEDS-CONTRACT**: the capability-67,
command-84 catalogue reply `Card` has a single `detail: String` and no discovery
visibility or stage-tagged loot entries. A stage-aware catalogue card/detail
message must identify which name/appearance, combat facts and loot entries are
revealed at each discovery stage. This window displays the server's confirmed
text; it does not infer loot rarity or discovery visibility by parsing it.
Upstream `edubart/otclient` has no Bestiary module; its
`modules/game_questlog/questlog.lua` provides the toolbar/request/list/detail and
session-cleanup reference pattern. Discovery stages follow the local Tibia manual
(`docs/reference/tibia-manual/interface.md`, Cyclopedia).

### Store Inbox (remote summary)

**Store Inbox** in the toolbar opens confirmed character-inbox quantities, Refresh, and Previous/Next summary-page controls. Capability 199 / command 388 / selector 130 supplies the existing fenced custody summary. Pages cover all locations; only positive inbox quantities appear. An empty filtered page does not mean the whole inbox is empty. The server combines Store and other deliveries: this is a read-only slice, not full parity.

**NEEDS-CONTRACT:** an accepted `StoreInboxOpenRequest` / `StoreInboxView` exchange (proposed names; no IDs allocated) identifying the Store-only root, returning session-scoped live item/container handles, entries and pagination, and authorizing existing use/move commands. The four-location summary has neither Store provenance nor handles. Command 186 claims a Platform fulfilment unit by key; it does not open/enumerate this inbox.

Behaviour: [Tibia manual](../../docs/reference/tibia-manual/interface.md#343-store-inbox), remote access. Structure reference only: edubart/otclient `dab86e610991092fe8b2471f7de55a7c986b3cd2`, `modules/game_inventory/inventory.lua:35–42,102–104`: purse button uses a server-supplied inventory item. No source/assets copied.

### Depot boxes

Open **Depot** in the toolbar or press **Ctrl+Shift+L** near a depot locker.
Select one of the 17 boxes, page through confirmed contents, and refresh.
Item rows withdraw the whole stack to the main backpack; carried rows deposit it.
Server refusals appear in the window. Close or Escape dismisses it.
The existing capability 37 checks locker reach on every request.

NEEDS-CONTRACT: inbox browsing needs an accepted `InboxBrowse { page }` request
and `InboxContents { outcome, page, total, entries: ItemEntry[] }` reply carrying
live item/container handles. These are missing message shapes, not registered
message names. Nested depot-container browsing needs `DepotReply.entries` to
include a live item/container handle (currently only ordinal, definition and
count). No protocol extensions are added here. Search/retrieve and boss
reward chests belong to subsequent batches.
Reference: Tibia manual `world.md:45–48`; OTClient
`dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_containers/containers.lua:59–77` (server-confirmed page navigation).

The offline **Updates** button opens a dedicated maintenance window. **Check / Retry**
checks the signed stable release policy; **Download verified package** shows byte
progress and stages a verified package without activating it. Hiding the window
keeps background work running. Failed operations can be retried and leave the active
release unchanged. **Close client and maintain** hands off to the installed launcher;
on Windows it waits for client shutdown, then offers update, verification and repair.
A missing build-pinned signing root refuses updates; use a trusted installer build.
External UI skins still require `--ui-assets`; this window does not bootstrap them.

Preview reference: Global manual `docs/reference/tibia-manual/controls.md:20-21`
(Movement and rotation arrows). OTClient reference only:
`edubart/otclient@dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`src/client/creature.cpp:105-160` (direction, addon and mask composition),
`modules/game_outfit/outfit.lua:42-50` (mount artwork uses an explicit type binding).
The preview implements Global's movement and rotation controls independently of
OTClient's south-facing `drawOutfit` preview.

The **Unjustified Points** toolbar button opens the dedicated PvP status window
and requests an observation. Refresh requests current daily/weekly/monthly fractional
points, red/black threshold bars and remaining headroom, skull duration, and incoming/
outgoing open situations. The skull countdown uses elapsed time since receipt; open
situations show durations at observation. Close dismisses the nonmodal window.
Unavailable or invalid observations never substitute zero counters.

# Item sweep server defects

Reference revision: CrystalServer `201189f871d23597b82caf88fd611454a2ddefb8`.
The observed requests below come from the seeded World run at `d0884cc73`.
A handle in an item request is the session-local handle of the seeded item.
Slot selection and eligibility are resolved separately. A refusal establishes
the wire outcome. The owner references identify the relevant decision paths.

| Cause | Keys (Tibia item IDs) | Oteryn server or protocol | Crystal reference | Request, expected and observed |
| --- | --- | --- | --- | --- |
| Combat snapshot unavailable | 10323, 10384, 10386, 10388, 10389, 10438, 10439 | `gameplay_transport/attack/cyclopedia.rs:97–123` | `src/server/network/protocol/protocolgame.cpp:2424–2441,5016–5025,5197–5205` | Command 388, self, Offence or Defence tab, sent before equipping and after unequipping. Crystal sends the selected self page with a success byte. Oteryn repeatedly returned Rejected. The Oteryn owner has best-effort lock and live-equipment consistency guards. The observed reply does not identify which guard fired. |
| Hotkey equipment resolution | 10385, 10387, 10412 | `gameplay_transport/hotkey_equip.rs:325–354` | `src/game/game.cpp:3703–3710,3743–3757`, `data/items/items.xml:27260–27275,27290–27299,27504–27509` | Command 348, carried definition reference. Content and Crystal identify Head, Legs and Necklace respectively. Oteryn returned SlotMismatch. The harness replay uses explicit command 9 to check equipment independently of hotkey destination discovery. |
| Wrong-vocation refusal reported as a slot mismatch | 10385 | `gameplay_transport/item_move.rs:126–132` | `src/creatures/players/player.cpp:4631–4636`, `data/items/items.xml:27273–27275` | Command 9, carried helmet handle, Head destination, wrong vocation. Crystal accepts the slot shape and its move event declares Knight and Paladin families. Oteryn returned SlotMismatch where the sweep expected RequirementNotMet. |
| Carried container open returns NothingToUse | 10326, 10346 | `gameplay_transport/connection/routing/use_item.rs:498–563`, `gameplay_transport/loop_move.rs:1637–1648` | `src/lua/creature/actions.cpp:461–467,525–535`, `data/items/items.xml:27016–27024,27122–27130` | Command 2, field 2 item handle for the carried backpack. Crystal opens a container view and both definitions have capacity 20. Oteryn returned Accepted/NothingToUse, leaving capacity unobserved. Its route can also forward an ordinary USE result when the container observation is absent, so the reply alone does not prove line 560 fired. |
| Backpack equipment destination unavailable | 10326, 10346 | `crates/protocol-oteryn/src/item_view.rs:316–328` | `src/creatures/players/player.cpp:4643–4648`, `data/items/items.xml:27023–27024,27129–27130` | Crystal admits the Backpack equipment slot. Command 9 has a MainBackpack custody destination for storing an item inside the current backpack. Equipment destinations exclude the backpack slot. The harness reports the gap before sending an invalid request. |
| Active content admission missing | 10219 | `crates/gameplay/items/src/item_ref_admission.rs:63–74` | `data/items/items.xml:26729–26732` | Qualification provisioning requested `oteryn:item.tibia.i10219` as a carried item. Crystal defines crocodile steak as food. The native fixture lookup returned DefinitionNotFound before an item USE request could be sent. This is an active content coverage gap. |

## Harness findings

The earlier seeds for 10210 and 10211 requested carried custody for content
admitted only to Ground. Crystal gives both lizard weapon racks capacity 25
(`data/items/items.xml:26689–26698`). The sweep now seeds an adjacent Ground
root in the disposable World, refreshes the live projection through a normal
item move, and uses Browse Field and USE to measure the actual result.

Hand defence expectations use the definition's base and extra defence
(`src/creatures/players/player.cpp:646–655`). Elemental bond is checked through
the item's inspection projection, as in Crystal's displayed description
(`src/items/item.cpp:3257–3262`). This check covers the element label. A
runtime spell or combat effect oracle would require a separate supported action.

## Explicit-destination and Ground replay

The World replay at `c5518aa02` sent the self Offence and Defence tab payloads
`08 00 00 14` and `09 00 00 14` respectively (command 388). The three
earlier hotkey failures repeated with command 9 and explicit Head, Legs and
Necklace destinations. The refusal detail was NotPossible. The wrong-vocation
Head move for 10385 also repeated SlotMismatch/NotPossible. The durable owner
can refuse before testing vocation (`character_equipment.rs:1549–1564`), while
the explicit vocation rejection maps separately (`:1492–1504`).

The same replay exposed a request-path difference for weapon items 10391,
10392 and 10406. Their earlier definition hotkeys equipped them. Explicit
command 9 with RightHand, the protocol's WEAPON slot
(`crates/protocol-oteryn/src/item_view.rs:260`), returned SlotMismatch.
The sweep retains this refusal as an equip failure even when the hotkey can
establish custody for later checks.

Browse Field returned both Ground rack handles (16 in each rebuilt session).
Command 2 then sent field 2 containing handle 16 (`12 02 08 10`) and received
Accepted/StaleState on both rows. Crystal opens the reachable container
(`src/game/game.cpp:4178–4227`, `src/lua/creature/actions.cpp:461–467,525–535`).
The local Ground custody and capacity checks are
`gameplay_transport/loop_move.rs:1570–1648`. A refusal on a SQL-seeded row
still needs reproduction with normal map custody to establish a server defect.

For 10346 the carried USE again returned Accepted/NothingToUse. The 10326
session instead closed after its carried USE, before a typed reply arrived
(`gameplay_transport/connection/serve/mod.rs:527–532`).

## Eligibility and hand-defence owner evidence

The native profiles declare the helmet's allowed vocations and leave its
level UNKNOWN. The Zaoan legs and wailing widow's necklace have UNKNOWN
level and vocations. The compiled policy requires both fields to be KNOWN
before it produces a claim (`crates/gameplay/content/src/content/native_gameplay.rs:4791–4798`).
The slot precheck then maps an unavailable claim to SlotMismatch. This is a
content qualification gap ahead of the vocation check.

The `90e3022a8` batch 0:20 completed with 69 PASS and 59 FAIL rows and
`NODE_BOOT_RESULT=NODE_BOOT_FAIL`. Twin hooks (10392) and Zaoan halberd
(10406) both equipped and unequipped. Their accepted self Defence pages
omitted DefenceEquipment while worn. The expected values were 22 and 15
(`data/items/items.xml:27384–27390,27464–27470`). Crystal defaults extra
defence to zero (`src/items/items.hpp:305`) and adds it to base defence
(`src/creatures/players/player.cpp:690–697`). Oteryn requires Known extra
defence and otherwise yields no hand value
(`crates/gameplay/combat-rules/src/resolution/player_defence.rs:48–60`), so the
wire projection omits the field (`gameplay_transport/attack/cyclopedia.rs:475–476`).

The drachaku (10391) returned SlotMismatch to command 9 targeting RightHand.
The fallback command 348, definition reference 430, lost its transport before
a typed reply. The equipped elemental-bond inspection was therefore blocked
for that entry. Crystal's Energy bond is declared in
`data/items/items.xml:27362–27368`.

## Native map custody replay

The opt-in town bundle places both immovable racks on ordinary adjacent map
cells. Each rack contains one captured rope so the live map projection exposes
the fixed container as an item handle. The probe joins afresh for each rack
and sends USE as its first command. It bypasses Browse Field, SQL-seeded
ground rows, and the projection invalidation marker used by the earlier probe.

| Cause | Keys | Oteryn owner | Crystal 201189f8 | Request, expected and observed |
| --- | --- | --- | --- | --- |
| Native map container refuses the server's fresh handle | 10210, 10211 | `gameplay_transport/connection/routing/use_item.rs:498–574`, `gameplay_transport/loop_move/map_sources.rs:4–103` | `src/game/game.cpp:4178–4227`, `src/lua/creature/actions.cpp:461–467,525–535`, `data/items/items.xml:26689–26698` | Actor at (32369,32238,7). The world map published handle 17 for rack 10210 at (32370,32238,7) and handle 16 for rack 10211 at (32368,32238,7). Command type 2 field 2 encoded the fresh handle as `12 02 08 11` and `12 02 08 10` respectively. Crystal opens these reachable, 25-slot containers. Both Oteryn sessions replied `Accepted/StaleState` to command 1 and published no capacity. This reproduces the rejection through native map custody and establishes a server failure for this class. The reply does not reveal which observation guard failed. |

The base-map capacity observer also passes the 20-entry limit to
`captured_source_capacity` (`map_sources.rs:95–103`,
`crates/protocol-oteryn/src/container_tree.rs:109–113`). The rack run
did not reach a published view, so it measures the stale rejection only.

| Batch | Selected | PASS rows | FAIL rows | Terminal result |
| --- | ---: | ---: | ---: | --- |
| `0:3` native rack replay | 3 | 4 | 5 | `NODE_BOOT_FAIL` |

The five failures are the two rack USE refusals and the previously recorded
10219 fixture refusal with its two dependent steps.

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

### Request and custody limitations during the inventory replay

| Observed class | Example keys | Crystal 201189f8 | Evidence and classification |
| --- | --- | --- | --- |
| Direct USE of attack ammunition | 15793, 16141, 16142, 16143 | `data/items/items.xml:33747–33761,34589–34636`, `src/creatures/players/player.cpp:315–358`, `src/items/weapons/weapons.cpp:408–444` | The positive USE rows send command type 2 with the carried handle. Crystal selects ammunition when resolving a ranged weapon and decrements the count on weapon use. The direct USE rows can fail even with correct combat behavior. Restriction rows sent through direct USE also lack the Crystal weapon precondition. Separate explicit Ammo equip refusals are recorded independently. |
| Bed use with generic Ground custody | 26093–26099, 42279–42281 | `data/items/items.xml:51307–51366,74651–74675`, `src/items/bed.cpp:102–106,162–168` | SQL seeding plus Browse Field either produced no handle or sent direct USE with the Ground item handle. Crystal requires a house-linked bed and a player permitted to use it. These rows do not exercise that owner. |
| Ground container USE outside native ownership | 17428, 17439, 26105, 26162, 42072, 42273 | `data/items/items.xml:35559–35650,51397–51566,74423,74621`, `src/lua/creature/actions.cpp:461–467,525–535` | Generic SQL custody produced `Accepted/StaleState` after Browse Field. For corpse keys, no creature death established lifecycle or loot custody. For fixed furniture, no native map or house placement established custody. The native rack result above establishes that specific map-owner failure, while these other classes require their own custody replay. |

### Batch 400:1000 session-release failure

| Cause | Keys | Oteryn owners | Crystal 201189f8 | Expected and observed |
| --- | --- | --- | --- | --- |
| Disconnected combat fixture remained unreleased past grace | 266 triggered the condition, 268 was the first later item blocked | `gameplay_transport/listener.rs:502–513`, `gameplay_transport/composed_admission/grace_expiry.rs:158–173`, `qualification/client_phases/character_release.rs:9–30` | `src/creatures/players/player.cpp:2253–2284`, `src/server/network/protocol/protocolgame.cpp:840–874` | The potion oracle entered combat to create a health deficit. Its projection timed out, and normal logout returned `InFight` with 59,007 ms remaining. After the client disconnected, the server reported grace expiry pending at `combat_hold`. The next fixture's 160-second release wait ended with state 1, no transport and `grace_expired=true`. The qualification gate failed subsequent requests for that fixture before sending item commands. Crystal also blocks logout while in combat and retires a lost player once its ping timeout and logout predicates hold. The observed Oteryn delay establishes a run-blocking lifecycle condition. A combat hold of unknown duration does not establish the reason the owner stayed pending. |

These downstream seed and dependent rows measure fixture availability.
They provide no item-specific gameplay observation.

### Batch 400:1000 item inspection

| Cause | Keys | Oteryn owner | Crystal 201189f8 | Expected and observed |
| --- | --- | --- | --- | --- |
| Elemental bond omitted from item inspection | 3289 | `gameplay_transport/inspection.rs:536–576` | `src/items/item.cpp:3257–3262`, `data/items/items.xml:8619–8629` | The sweep sent inspection command 153 using the current staff handle while equipped and again after unequipping. Both replied with the correct definition 20065 and omitted the Elemental Bond field. Oteryn's `content/items/definitions/items-20000-20499.json` declares an Earth bond. Crystal illustrates the display rule when a weapon has a bond. This pinned Crystal revision defines the staff as a club and has no staff bond, so the expected Earth label comes from Oteryn content. The runtime effect remains untested. |

Wild Growth (3156) and Paralyse (3165) returned `ProtectionZone` to direct USE
and to the low-level repeat. The fixture used the protected town entry and did
not target a legal combat cell or actor. Those refusals leave the rune effect
and level oracle unexercised.

### Batch 400:1000 terminal verdict

| Batch | Selected | Seed PASS | Seed FAIL | PASS rows | FAIL rows | Terminal result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `400:1000` | 1,000 | 64 | 936 | 230 | 3,956 | `NODE_BOOT_FAIL` |

The sweep emitted a verdict for every planned row. The initial fixture
release failure cascaded through later seeds and their dependent steps.
Among the recorded failures, 1,905 rows were blocked by their seed and
923 by ground custody. The run recorded 31 self combat snapshot rejections,
seven explicit equipment `SlotMismatch` rows, and the staff inspection
omission documented above. There were 16 `NothingToUse` and 12 `StaleState`
replies. These include weapon and generic-ground requests with the
precondition gaps described above. The 936 failed seeds leave their items
unverified through gameplay.

### Equipment claim rejects unknown exclusion groups

| Cause | Keys observed through explicit MOVE | Oteryn owners | Crystal 201189f8 | Expected and observed |
| --- | --- | --- | --- | --- |
| Compiled claim refuses an unknown exclusion-group list before eligibility | 34088, 34089, 43870–43875, 43877–43880 | `content/native_gameplay.rs:4776–4798`, `durability/character_equipment.rs:1549–1559`, `gameplay_transport/hotkey_equip.rs:334–347` | `data/items/items.xml:64323–64376,76181–76342,76366–76485`, `src/creatures/players/player.cpp:4631–4636` | Command type 9 targeted RightHand with the fresh carried item handle. Among the positive and restriction probes, 34 rows returned `SlotMismatch/NotPossible` for these twelve keys. The matching native WEAPON patterns reserve SHIELD, give level and vocation, and declare `mutually_exclusive_groups=UNKNOWN`. The content owner rejects that field before evaluating either eligibility gate. Crystal declares the two-handed weapon and its eligible builds without the Oteryn group-list prerequisite. For 43870, the eligible build was a level-600 Knight, and the level and vocation rejection probes also returned `SlotMismatch`. |

### Batch 1400:719 cleanup capacity

The harness drops carried leftovers at the actor's tile, commonly the town
entry (32369,32238,7). Later command 9 cleanup moves at that tile began
returning `NoRoom/NotEnoughRoom` for keys including 44620, 44636, 44637,
44642, 44643, 44648 and 44649. The server maps ground-cell capacity refusal
to this outcome (`durability/item_loop_move.rs:302–303`). Because the
restriction helper returns its cleanup error after an otherwise successful
restriction check, the same disposal refusal also appears under
`equip_level` and `equip_vocation`. Those rows fail end to end and
do not establish a failed eligibility decision.

### Later weapon-projection evidence

| Cause | Keys | Oteryn owner | Crystal 201189f8 | Expected and observed |
| --- | --- | --- | --- | --- |
| Energy bond omitted from the item inspection reply | 50166, alongside the earlier content-backed 3289 case | `gameplay_transport/inspection.rs:536–576` | `data/items/items.xml:84741–84754`, `src/items/item.cpp:2059–2061,3257–3262` | Both Oteryn content and Crystal declare the energy bond. Command 153 inspected the live definition 27841 while worn and after removal. Both item payloads omitted the bond. The movement assertions separately record an explicit slot refusal and a verified unequip. |
| Hand defence omitted from a successful self Defence page | 21176, 22727, 50181, 50182 | `combat-rules/src/resolution/player_defence.rs:48–60`, `gameplay_transport/attack/cyclopedia.rs:475–476` | `data/items/items.xml:42137,45332,84905–84936`, `src/items/items.hpp:305`, `src/creatures/players/player.cpp:690–697` | The live page omitted expected hand values of 18, 28, 14 and 15 respectively. These extend the previously recorded base-plus-extra calculation class. |

### Interrupted batches and resume offsets

The environment interrupted two running processes before either could emit a
terminal marker. These are the persisted step counts at interruption.

| Requested batch | Entries with rows | PASS rows | FAIL rows | Terminal result | Resume slice |
| --- | ---: | ---: | ---: | --- | --- |
| `100:400` | 343 | 514 | 883 | Absent after interruption | `442:58` |
| `1400:719` | 301 | 492 | 852 | Absent after interruption | `1700:419` |

Each resume starts at the last entry observed in its interrupted run. This
replays one entry per batch to cover any partial verdict at the interruption.

### Readable items use a different command in Oteryn

Keys 2821, 2822, 2824, 2825, 2826 and 2829 returned `NothingToUse` to
command 2 with a carried handle. Crystal's default USE sends a text window
for readable items (`data/items/items.xml:5821–5838`,
`src/lua/creature/actions.cpp:538–551`). Oteryn's carried inscription
owner uses command 6246 with an item handle
(`crates/protocol-oteryn/src/item_inscription.rs:6`,
`gameplay_transport/item_inscription.rs:59–101`). The sweep did not send
that command, so these rows leave the text behavior unverified.

### Batch 100:400 combined verdict

| Batch | Entries | Seed PASS | Seed FAIL | PASS rows | FAIL rows | Terminal result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `442:58` resume | 58 | 56 | 2 | 109 | 105 | `NODE_BOOT_FAIL` |
| `100:400` combined | 400 | 237 | 163 | 623 | 985 | Interrupted original, `NODE_BOOT_FAIL` on resume |

The combined count replaces the original rows for overlapping
`tibia.i2821` with its resume rows. The original run also recorded an
expired-grace release timeout. The resume extended the established backpack
destination class to 2853, 2854, 2857, 2859, 2863 and 2867
(`item_view.rs:316–328`, Crystal `items.xml:5951–6032`). The new hand-defence
evidence is recorded above. The glider crank 21902 produced a malformed
overlay during direct USE (`crates/session/src/domain_dispatch.rs:263–272`,
Crystal `items.xml:43460–43464`). That wire symptom remains unresolved.

The `1700:419` resume reproduced the drop limit and established its
downstream effect. After the same-tile cleanup refusals, the node's
qualification owner (`apps/game-server/src/node/serve/qualification_fixture.rs:1007`)
reported `Refused(MainBackpackFull)` while reserving more Knight fixture
items. Near the transition, 664–668 had failing cleanup rows and later
fiery weapon variants had seed refusals with no gameplay probe. The
full-backpack refusals are fixture fallout. Separately, the node logged
`DefinitionNotFound` for typed production keys such as 5678, 5949, 5964,
6125, 6278 and 6392. That explicit active-content refusal extends the
already recorded admission class.

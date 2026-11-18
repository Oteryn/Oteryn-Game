# QCX79 — What a Foolish Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Canonical key
`oteryn:quest.what_a_foolish_quest`.

The Global quest and spoiler revisions are TIBIAWIKI_STRUCTURED evidence.
Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies donor identifiers
and implementation evidence (OTS_HYPOTHESIS_ONLY). The Global revisions take
precedence. [Quest, curid 19799, oldid 1094085](https://tibia.fandom.com/?curid=19799&oldid=1094085)
· [Spoiler, curid 19801, oldid 1056289](https://tibia.fandom.com/?curid=19801&oldid=1056289)

## Comparison

The quest definition retains eleven mission journal tracks plus an outfit track.
The selected recipe covers the first five prank motifs, then reports to Bozo
and records chosen completion. It explicitly omits the later prank sequence
and achievements. Its reward intents list the outfit and staff, even though
Global awards the staff at mission 8. The named quest has no active or held
world callback shard. Eight action/movement records are embedded as source
evidence in the definition. The Bozo and peer NPCs are imported, but the
quest-specific dialogue source links do not activate those mission routes.

| Part | Global expectation and differences from the import |
| --- | --- |
| Entry | Premium, no level-40 gate for the base outfit or staff. Bozo asks for two confirmations after `join`. The definition has Premium and an unparsed level footnote, and the base had no active joining dialogue. The generated Bozo route uses the two captured prompts, supports cancellation, and commits questline and mission 1 only on the Premium confirmation. Crystal Bozo 624–687 has the topic sequence and 779–800 shows cancellation. Crystal has no Premium check at this point. |
| 1 · White flower | Use a kitchen knife on the correct northeast White Flower Temple flower. Success consumes the knife and gives the flower. Other flowers give the refusal with no consumption. The recipe names the objective only. The shared donor action is outside the eight quest-folder captures. Crystal `data-global/scripts/lib/register_actions.lua:1148–1156` binds aid 4200 and position (32349, 32361, 7), knife 3469 and flower 102. The retained correct tile has flower appearance 3656 without aid 4200. |
| 2 · Slime | Bozo supplies a special flask. Use it quickly on the poison field from a freshly killed slime and return the sample. The recipe calls the target a Slime corpse and omits the poison field reaction and Bozo exchange. Crystal Bozo 76–104 binds flask 135 and sample 107. |
| 3 · Pies | Mirabell takes 24 flour and 240 gp for one box of 12 pies. Parcel bugs, guards, carpet, GCTS and teleport-scroll restrictions route the trip via Cormaya and Kazordoon. The selected recipe names twelve pies as the collection target. The imported aid-4201 confiscation branch remains source evidence and the inspected Edron placement region contains no aid-4201 selector. Crystal Mirabell 67–102 creates box 119, whereas the donor movement branch looks for 7484. |
| 4 · Wine | Bozo takes 18 vials of wine, then the player uses his wine crate on Xodet's crate and returns the swapped crate. The recipe lacks the separate use and return stages. Crystal Bozo 150–176 and `action_crate.lua:2–19` show subtype-aware intake and crate exchange, with supplied crate 117 and return 118. |
| 5 · Watch and beard | Bozo authorizes the sealed Triangle Tower door. The watch teleports the player beside the sleeping emperor from the bed's head end, and returns them afterwards. Kitchen knife use yields the beard and summons a Dwarf Guard. Key 3800 may be needed to reach the bedroom. The recipe collapses the watch/beard operation and omits the guard and access details. The door tile (32563, 32116, 4) carries aid 41514 on appearance 5104. Its imported progress track has max 0, and no active transition writes access. Crystal Bozo 179–226, `action_watch.lua:1–27`, and `data-global/scripts/lib/register_actions.lua:1157–1167`. |
| Outfit | Four individual colored cloth hand-ins in transcript order red, blue, green, yellow unlock the Jester base outfit after the beard. No active outfit exchange. Crystal Bozo 562–602 and 758–770 maps cloth 5911/5912/5910/5914 and male/female looks 273/270. |
| 6 · Cushion | Four minotaur leather and one spider silk, then use the cushion on Queen Eloise's throne and report. Omitted from chosen recipe, with the use branch in source evidence. Crystal Bozo 234–294, inputs 5878/5879 and cushion 121, `action_cushion.lua:2–21`. |
| 7 · Mouse | Use the cat basket, pick up the spawned toy mouse (123), evade Kitty, then tell Carina `mouse` and confirm. Omitted, with the basket action retained as source evidence. The placement import lists aid 41518 at (32316, 31757, 9) for the queen’s cat door, but no mission transition grants its track. Crystal `action_cat_basket.lua:18–35` and Carina 66–85. |
| 8 · Cigar | Reusable spoon at an inactive lava hole, kitchen knife at the jungle dweller bush, and deliver the cigar to Theodore Loveless before reporting. Staff item 895 and addon 1. Recipe contains the staff item intent only, with no active hand-in. Crystal Bozo 331–387 and `data-global/scripts/lib/register_actions.lua:988–1009,1168–1176` map sulphur 124, leaves 3129 and cigar 141. |
| 9 · Bargain | Hat line requires level 40. Sample fresh Stalker blood and fresh Quara Constrictor or Scout ink. A player on the same mission can collect the Quara ink. Bozo sells replacement flasks. Ask Sam for 2000 steel shields, sign his contract with the magical inkwell, and return it. Omitted, except for retained contract action evidence. Crystal Bozo 394–456, Sam 97–137 and `action_contract.lua:2–18` map flask 135, blood 125, ink 9149 and inkwell 127. |
| 10 · Cookies | Ten exploding cookies (130), one apiece to Avar Tar, Simon, Ariella, Lorbas, Markwin, Hjaern, Wyda, Hairycles, Orc King and one of Yaman or Nah'Bob. Award Allow Cookies? at the tenth delivery. Wyda's 60×10 HP Soulfire and the Orc King's two Warlords and five Slimes are absent. Greeting guard summons and Archpostman exemption lack this mission's integration. Crystal Bozo 459–489 and `data/libs/functions/player.lua:93–115`, Wyda 64–100, Orc King 88–124. |
| 11 · Hat | Five white cloth (5909), white bale (142) used on the Plague Spike altar, return yellowed bale (143), greet Kazzan in a 10-second mummy disguise (144) or under the mummy Creature Illusion, then return to Bozo. Hat item 894, addon 2, Perfect Fool and Fool at Heart are missing from active quest acquisition. Crystal Bozo 493–554 and 742–754, `action_worn_cloth.lua:3–19`, `action_disguise.lua:1–13`, Kazzan 62–67. |

The watch claim is one item 112, with UID 6259 and appearance 2469 at
(32563, 32115, 4). The time-travel room claim is one bag 2853 containing one
kitchen knife 3469 and one book 2821, with UID 6269 and appearance 2472 at
(32661, 31855, 13). Global corroborates those named contents. Crystal
`data-global/startup/tables/chest.lua:2600–2613` supplies the identifiers
and the existing source reward dispatch fixture covers both. The decoded
placement shards carry the matching appearances. The legacy UID selectors
are supplied by the claim binding projection. The authored entry recognizes the existing native bag projection through
the exact quest and claim provenance. The canonical variant and SOURCE
record preserve the historical hold and payload. No XP reward is stated by the Global revisions.

## Engine dependencies

Main has position-gated teleports, progress doors, registered fresh-corpse
collection, carried/map exchanges, NPC fees, elapsed storage and outfit
reward grants. The matching unbound quest stages are listed under PARTIAL.

BLOCKER mission 2 fresh Slime poison-field sampling with provenance and player poison-field effect data-global/npc/bozo.lua:76-104

BLOCKER mission 5 synchronous public Dwarf Guard summon on kitchen-knife interaction data-global/scripts/lib/register_actions.lua:1158-1167

BLOCKER mission 7 basket-position Kitty summon with ten-second removal alongside floor toy creation data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:32-35

BLOCKER mission 10 NPC-triggered public guard summons on greeting and cookie hand-in data-global/npc/the_orc_king.lua:118-124

BLOCKER mission 10 durable NPC exchange with a live Soulfire condition participant data-global/npc/wyda.lua:87-100

BLOCKER mission 11 live mummy creature-look check on NPC greeting after timed carried disguise or Creature Illusion data-global/npc/kazzan.lua:62-67

## OPEN-FACTS

The facts file gives no sex-specific greeting alternative or walkaway line.
The revisions also omit numeric item/appearance IDs, selector coordinates in
server units, storage and timer keys, exact freshness windows, travel cash
totals and the replacement flask price. Confirmed mappings above come from
Crystal and the existing import. The Global text describes a short-lived pie
box and transport restrictions without an exact expiry or confiscation item
ID. Crystal gives 1200 seconds and mismatched purchase/confiscation IDs.
The level footnote scopes level 40 to the hat line but does not name an
exact Bozo dialogue node for the check. The disguise replacement is free
according to Global, while Crystal's disguise use does not remove the item.
The Global requirements list two kitchen knives even though the method
also calls for one on the mission 8 bush. Reuse or total consumption beyond
the mission 1 break is open.

## PARTIAL

Mission 1 flower and hand-in, missions 2–11, the scoped hat level gate,
Triangle Tower, queen’s cat room and keyed access integration, all outfit/addon and achievement
grants, and greeting differences remain open. The existing
watch and bag payloads required no item/count changes.

## Verification

- The baseline dispatch case reached Bozo and failed in 78.81 s because
  ordinary TALK returned `Ok` for the final confirmation and bypassed
  the Premium gate. The timed baseline result excludes the earlier fixture
  failure from the NPC floor convention.
- PG via shard (pgsplit build files applied on 2330ef5ed26b7b9977ac5c7ef04e9e05141b8b72).
  One passed, zero failed. `bozo_join_requires_premium_and_two_confirmations`
  ran in 102.63 s. It uses the real NPC command owner, a pinned test room
  with Bozo placed alongside the player, and PostgreSQL quest writes.
- The bundled-world generator reproduced the existing pin. Pin check passed
  for one World and a 33,688,785-byte bundle.
- The native binding audit verifies this exact source entry document
  separately from the chosen mission plan. The candidate registries changed
  only their input provenance and digest. The bag proof registration checks
  the exact Foolish Quest and claim keys with the existing native-container
  verifier.
- The same dispatch filter with the PostgreSQL admin URL absent returned
  `Ok(())` for one case in 0.00 s. The route-less result verifies the guard.
- Workspace formatting and both quest-specific generator checks passed.
- Dispatch library Clippy passed with `--all-features --lib -- -D warnings`.
- Quest authoring passed all 44 checks, including 269 schema cases and 27 nested unit tests.

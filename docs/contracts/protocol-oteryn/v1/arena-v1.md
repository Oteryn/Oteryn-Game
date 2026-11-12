# Arena v1

Capability 170 exposes commands 240 and 436 to an admitted session with current channel runtime authority. It remains bundle-world only.

## Request

The payload is empty. Challenge facts are server-owned; any payload byte is refused.

## Reply

A sequence of field 1 messages, one per Svargrond challenge, at most 1024 bytes. Each message carries varint field 1 rank (1 Greenhorn, 2 Scrapper, 3 Warlord), field 2 entry fee in gold, field 3 room count and field 4 seconds per room.

## Match request

Command 436 uses singular uint32 varint fields, at most 32 bytes:

| Field | Meaning |
|---|---|
| 1 | Action: 1 status, 2 enter the next incomplete rank, 3 advance through the exit after the server recorded the opponent's death, 4 leave, 5 claim a reward |
| 2 | Reward rank: 1 Greenhorn, 2 Scrapper, 3 Warlord; claim only |
| 3 | Reward category: 1 present, 2 supplies, 3 weapon, 4 trophy; claim only |
| 4 | Reward choice: 1, or 1–3 for weapons; claim only |

Non-claim actions must omit reward fields or send their zero defaults. Unknown actions/fields, duplicate fields, non-varint fields, invalid reward selectors and oversized messages are refused. The client cannot report kills, set a rank, supply a timer, a price, an item, or a spawn position. Character identity and session generation come from the admitted session.

## Match reply

At most 128 bytes, singular uint32 varint fields (zero is the omitted default):

| Field | Meaning |
|---|---|
| 1 | Disposition: 1 applied, 2 wrong state, 3 insufficient gold, 4 already claimed, 5 inventory full, 6 timed out, 7 busy, 8 invalid reward, 9 all challenges completed, 10 content unavailable |
| 2 | Highest completed rank, 0–3 |
| 3 | Active rank, 0–3; 0 means none |
| 4 | Current pit, 0–10; 0 means none |
| 5 | Phase: 0 idle, 1 fighting, 2 advance ready, 3 reward room |
| 6 | Remaining server-owned pit time, 0–600 seconds |
| 7–9 | Claimed reward masks for Greenhorn, Scrapper and Warlord; bits 0–3 represent present, supplies, weapon and trophy |

The successful or refused transition and its exact reply are recorded with the fenced command receipt. Replaying the same command returns the recorded outcome and never charges the fee, spawns another match, or grants another reward. A client displays the replayed state as the recorded result and can request fresh status to refresh its countdown.

## Native client

The arena panel opens on Ctrl+Shift+Z (Ctrl+Shift+A remains assigned to account achievements) and provides catalogue/status refresh, entry, pit advancement, leaving and the four independent reward claims. Combat remains on the normal attack/spell path; the panel has no kill-report control. Capability 170 is selected only when offered. The panel shows gameplay refusals and does not apply predicted rewards or progression.

## Drome request (additive)

Action 6 of command 436 selects Tibiadrome, keeping Svargrond actions 1–5 and
all their bytes unchanged. Field 2 is the operation: 1 status, 2 enter/join
preparation, 3 leave, 4 lower starting level by five, 5 claim the next earned
potion, 6 purchase a reward. Only operation 6 uses field 3, a server catalogue
selector from 1 through 18; field 4 remains zero. The server owns kills,
participants, waves, deadlines, rankings, prices and rewards.

The Drome response has its own contract version: field 1 = 2; field 2 disposition
(1 applied, 2 wrong state, 3 ineligible, 4 cooldown, 5 busy, 6 content unavailable,
7 no entitlement/insufficient points, 8 inventory full); fields 3–12 are highest
rotation level, starting level, available points, active phase, wave, remaining
seconds, cooldown seconds, current rotation rank, unclaimed potions, and next
rotation boundary in Unix seconds. Limits remain 32 request / 128 response bytes.
A Drome consumer uses `DromeResult`, not the Svargrond response decoder.

Purchases 1–16 are Cube, badge replica, brown/green/blue pit demon,
ogre doll, Domestikion/Hoodinion/Mearidion/Murmillion/Scissorion plushies,
retching horror doll, black pit demon, sublime accolade, accolade, red pit demon.
The MINT owner checks the native Item, inventory capacity, and point balance in
one transaction; replay returns the original outcome. Potion selection belongs
to the earned rotation/ordinal, so inventory refusals cannot reroll it.

### W17 evidence and choices

The pinned Crystal `201189f871d23597b82caf88fd611454a2ddefb8` arena death script,
`data-crystal/scripts/quests/arena/creaturescripts_player_death.lua:2–5`, clears
PitDoor only. Svargrond retains ordinary death losses. Crystal and pinned Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` have no Drome lifecycle.
The recorded official manual `docs/reference/tibia-manual/combat.md:249–284`
owns Drome access, group counts, timers, exclusions, scoring and rotation rewards;
the captured reward catalogue is
`docs/reference/content-prep/arena-rules-content-1/admission-evidence/tibiadrome-reference-evidence.json:291–817`.
Crystal `data/scripts/actions/items/drome_cube.lua:3–24` supplies the twenty
potion identities, already admitted by the native Item producer.

The code marks these source gaps with `oteryn-choice`: epoch-aligned fortnights,
UUID ordering for otherwise identical standings, a stable weighted raffle draw,
stable per-award potion selection, restart cancellation with cooldown, the
qualified Svargrond entrance as the interaction point, qualified native arena
opponents (including their native attacks/conditions) as the physical profiles
and modifiers, and ten-percent-per-level physical scaling.
Preparation entrants within three tiles share the one Channel-local battle.
Selectors 17 and 18 buy Phant and the Lion of War base outfit through the existing
account appearance owner, qualified against Crystal mounts.xml:211 and
outfits.xml:112,244. Missing native definitions refuse with content unavailable.
Oteryn uses its existing account ownership for these permanent cosmetics.

The reserved Channel room isolates targeting, familiars and monster perception.
As an `oteryn-choice`, native spells that would write persistent Items are
unavailable in this practice room; damage/healing runes remain usable without
consumption. Outside casters cannot place fields into it. This follows the
existing atomic spell refusal path and prevents exporting practice Item value.

# Gem Atelier v2

Winter Update 2026 (test server 15.33) removes the gem domain and Vessel Resonance. Each
vocation's Wheel has twelve Gem Vessel nodes, the former Vessel Resonance nodes. Every fully
allocated node opens the next Gem Vessel slot and grants flat damage and healing. A gem fits
any open slot of its own quality.

Commands 6210 (`ATELIER_V2_QUERY`), 6211 (`ATELIER_V2_SET_LOCK`), 6212 (`ATELIER_V2_MUTATE`),
6213 (`ATELIER_V2_REPLACE_VESSEL`) and 6214 (`ATELIER_V2_RESHAPE`) require selected capability
483 (`GEM_ATELIER_V2`),
which requires capability 148. Capability 483 is not offered until the client implements the
slots. Capability 148 and its commands stay for old clients (see [Gem Atelier v1](atelier-v1.md)).

The canonical codec is `crates/protocol-oteryn/src/atelier/v2.rs`. Requests are at most 64
bytes; replies at most 16,384 bytes.

## Requests

- Query: an empty payload.
- Set lock: the v1 lock request bytes.
- Mutate: the v1 mutate bytes for actions 0 (reveal), 1 (dismantle) and 3 (grade up).
  Action 2 (switch domain) is malformed.
- Replace Vessel:

| Field | Wire tag | Value |
| --- | --- | --- |
| occurrence | 1, bytes | Exactly 16 bytes; UUIDv7 Wheel receipt identity |
| expected_wheel_revision | 2, varint | Expected Wheel revision |
| slot | 3, varint | Gem Vessel slot, 0 through 11 |
| gem | 4, bytes | Exactly 16 bytes; absent to empty the slot |

Replace Vessel puts the gem into the slot in one Wheel write, moving it out of any other slot.
The slot's previous occupant returns to the gem list. The Wheel allocation is unchanged. The
write uses the durable, session-generation fenced Wheel writer: a slot that changes to a gem
must be open and of the gem's quality, the gem must be a current gem of the Character, and
no gem fills two slots. A refusal returns `refused=true` with nothing written. The same
occurrence and intent returns its recorded result: the receipt binds the expected Wheel
revision, slot and gem as sent, never the allocation or slots read at the time, so a retry
replays even after later Wheel changes. A capability 148 allocation save binds its four cells
as sent, as before migration 2770, and its gems are placed after the receipt lookup; a gem
socketed beyond the four cells such a client is shown keeps its slot.

Lock and paid requests run through the v1 Atelier writer unchanged.

- Reshape (Mod Reshaping):

| Field | Wire tag | Value |
| --- | --- | --- |
| occurrence | 1, bytes | Exactly 16 bytes; UUIDv7 Atelier receipt identity |
| expected_revision | 2, varint | Expected Atelier revision |
| gem | 3, bytes | Exactly 16 bytes |
| position | 4, varint | Modifier position, 1 through 3 |

Reshaping requires the temple and Premium, an unlocked gem and the position's price: 125,000
gold and one lesser fragment, 1,000,000 gold and two lesser fragments, or 5,000,000 gold and
three greater fragments. The new modifier is drawn from the position's reveal pool minus the
modifier the position holds now, so the result always differs; the two basic positions never
hold the same modifier. The gem keeps its identity, quality and slot.
Outside the temple or without Premium the reply is `refused=true` with nothing written,
unless the occurrence already committed: that receipt is returned, whatever the temple and
Premium state at the retry.

Reshape runs through the v1 Atelier writer as receipt kind `RESHAPE`: the modifier draw, the
gold fee (audited fee cause 483 `gem_reshape`, service key `oteryn:atelier.reshape`), the
fragment BURNs and the receipt commit in one session-generation fenced transaction.
Fragments come from carried containers first. While the character stands at a depot locker,
or a Steward hireling serves its open stash (the hireling depot service), the Supply Stash pays
the remainder through its ledger; otherwise only carried fragments count, and a shortfall is
`refused=true` with nothing written. The server checks this access at commit time. Migration 2771
binds receipt, fee and fragments: the fee names the position, and exactly one modifier of one
unlocked gem changes, only at that position; gems compare without the retired `domain` of
pre-2770 history and without the projected `vessel` flag. The same occurrence and intent
returns its recorded result.

## Reply

| Field | Wire tag | Value |
| --- | --- | --- |
| refused | 1, varint | Rule refusal of the request |
| revision | 2, varint | Atelier revision |
| gem | 3, message, repeated | 1 id (16 bytes), 2 quality (0 lesser, 1 regular, 2 greater), 3 locked, 4 slot (absent when not socketed), 5 modifier (repeated, one per quality step) |
| grade | 4, message, repeated | As in v1 |
| quality quote | 5, message, repeated | 1 quality, 2 reveal gold, 3 fragment kind, 4 fragment min, 5 fragment max |
| grade quote | 6, message, repeated | As in v1 |
| vessel slot | 7, message, none or twelve in slot order | 1 quality, 2 open, 3 gem id (absent when empty) |
| wheel_revision | 8, varint | Wheel revision for Replace Vessel |
| in_temple | 9, varint | The Character stands where Wheel removal is permitted (Oteryn owner decision 2026-11-05) |
| premium | 10, varint | The Premium allocation gate is open |
| vessel node | 11, varint, repeated | Wheel slots (1..=36) of the vocation's Gem Vessel nodes, ascending |
| vessel_node_bonus | 12, varint | Flat damage and healing per fully allocated Gem Vessel node |
| resistance_capped | 13, varint | The socketed gems' summed resistance against at least one element exceeds 30% |

The gem rows carry no domain field. Slot qualities follow one fixed table in
`crates/gameplay/progression/src/atelier/vessels.rs` (lesser, regular, greater, four times);
the published order is still open. Quotes are present on a query only, as in v1.

Summed gem resistance against one element is capped at 30% (test server reply "Resistances
cap at 30%"); each element caps separately and the excess has no effect. `resistance_capped`
is the client's warning for a setup above the cap.

## Migration

Migration 2770 converts each stored four-cell domain vector into the twelve slots: every
socketed gem keeps its socket and takes the first free slot of its quality, in legacy cell
order. Wheel receipts keep their four-cell history and record new changes in
`gem_vessels_before` and `gem_vessels_after`.

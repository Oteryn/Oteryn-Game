# House Catalogue Owner Contract V1

- Status: Contract candidate. Owner direction given 2026-09-30 (prepare the contract, then populate the
  catalogue); needs exact-head independent review before it is accepted.
- Date: 2026-09-30
- Issue: #162; lane `HOUSES`
- Implements: the static-content side of `EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` (ACCEPTED); it adds no
  housing semantics and changes none.
- Aligned with (candidates, #1315): ADR-0021 world map runtime loading (D193, D194, D196) and
  HOUSE-CUSTODY-0 house item custody (§3.1, §3.5, §3.6). If either changes before acceptance, this
  contract follows it.
- Evidence: `imports/cipsoft-staticdata/houses/` (995 client 15.30 records, HOUSES-1),
  `tools/content-schema/house-authoring/` (schema, validator, converter and cross-source reports, HOUSES-2 and
  HOUSES-3)
- Tooling: `tools/content-schema/house-authoring/`
- Does not authorize: a migration, runtime, protocol or client code, auction/rent/ACL behaviour, or Residence
  templates. Populating `content/houses/` is a separate content change under §3.

## 1. Owner and scope

The Game housing domain (`EXP-HOUSES-01` §22.1) owns the **House catalogue**: the static `House` content family
in `content/houses/`, one record per scarce physical house address. The catalogue is the only source of which
physical houses exist, their names, town, size, beds, rent amount, area and doors.

The catalogue holds no runtime or durable state. Ownership, auction, rent payment, delinquency, eviction, ACL,
housing content and runtime interiors stay World-scoped housing state under `EXP-HOUSES-01` §4 and are not
content. Residence / Apartment templates are not in this catalogue (`EXP-HOUSES-01` §6, §26).

## 2. Record

One record per House, validated by `house.schema.json` and `validate_houses.py`:

| Field | Rule |
|---|---|
| `identity.key` | `oteryn:content.house.<slug>`, allocated once from the official name (§2.1). |
| `identity.revision` | Content revision of the record. |
| `name` | Official client name, whitespace-normalized. Display text, never identity. |
| `kind` | `private_house` and `shop` are ordinary physical houses (owner `CharacterId`, `EXP-HOUSES-01` §8; owner decision 2026-09-30): a shop takes the personal physical-house slot and follows the same auction, rent and ACL rules. `guildhall` is the Guildhouse class (owner `GuildId`, §9), whose lifecycle stays deferred; pointer pending on acceptance of GUILD-0 (`OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md` §6-§7): guildhall lifecycle. |
| `town` | `Area` reference `oteryn:content.area.city.<slug>`. |
| `entrance` | The tile in front of the front door that a Character is placed on when leaving or being moved out (`EXP-HOUSES-01` §5.3 exit). |
| `map_marker` | Official map marker position. Presentation only. |
| `size_sqm`, `beds` | Official values. `beds` is the bed count; bed items are world placements. |
| `rent_gold` | Official rent amount. Cadence, grace and formula are tunable economy numbers (`EXP-HOUSES-01` §26), not content. |
| `entry_restriction` | Structured vocation restriction (3 houses in 15.30). |
| `footprint`, `tiles` | Official layout bounding box and area. Neighbouring houses may share wall tiles; a passable tile belongs to one House. |
| `doors` | Door positions. A door belongs to exactly one House. |
| `provenance` | Client `source_id`, record digests, verbatim name and restriction text, engine House id. Never identity. |

### 2.1 Identity

- The key is the identity of the physical house in every World that uses the catalogue. In a World, the
  `HouseId` of `EXP-HOUSES-01` §3-§4 is that World's instance of the key: one per World, never per Channel.
  Its physical representation is persistence detail (`EXP-HOUSES-01` §26).
- A key is allocated once from the official name and then never derived again. A later rename by the client
  keeps the key and changes `name` and `provenance.source_name`; `provenance.source_id` stays the join to the
  client.
- A key is never reused or reinterpreted. A different physical house (a different address, a merged or split
  house) is a new key, even under a reused name.
- A key is never removed from a catalogue. A house that no longer exists in the client is retired in a new
  revision (`retired`, added with the first retirement) and cannot be allocated again (§4).

### 2.2 Door identity

A door is identified by its House key and its position `[x, y, z]`; there is no separate door number. This is
the subject of per-door access (`EXP-HOUSES-01` §16.4 "specific doors"). A revision that moves or removes a door
leaves any access entry for the old position without a door, so it grants nothing; it never attaches to another
door. Engine door numbers are evidence only (they are not unique per door).

## 3. Source precedence for population

The catalogue is populated in a separate content change, not by this contract:

- All fields except `entrance`: the official client 15.30 files (`staticdata`, `staticmapdata`). The owner's
  in-game check (2026-09-29, five houses in three towns) matched the client, not TibiaWiki or CrystalServer.
- `entrance`: CrystalServer `world-house.xml` (owner decision 3a, 2026-09-29), final (owner decision 1a,
  2026-09-30). The client ships no tiles outside a House layout, so walkability cannot be derived from it, and the
  base map the runtime loads (ADR-0021) is the same engine map. It is the tile in front of an outer door for 949
  of 995 houses; the other 46 are listed in the conversion report
  (`entrance_not_in_front_of_an_outer_door`) for the walkability check when the base map is compiled
  (MAP-BUNDLE).
- `beds`: the official count, final (owner decision 2a, 2026-09-30). The client layout carries no furniture;
  bed items are base-map placements. The engine map has two bed items per official bed for 911 of 995 houses;
  the other 84 are listed in `samples/otbm-tile-check.json` for the base-map owner.
- Door item classification: CrystalServer `items.xml` `type="door"` ids, under the recorded assumption that
  engine item ids equal client appearance ids (HOUSES-3).
- The one door in two layouts (East Lane 1a/1b) belongs to East Lane 1a (`SHARED_DOOR_OWNERS`); any new one stops
  the conversion until it is decided.
- TibiaWiki BR and CrystalServer values that differ from the client are reported, never used.
- Earlier population attempts from CrystalServer `world-house.xml` (#1160, #1170) use engine keys and a
  `door_id` model that conflict with §2.1, §2.2 and this section; their House catalogue part is superseded by
  this contract.

## 4. Revisions against live Worlds

The runtime reads House tiles and doors only from the House family compiled into the World Bundle, never from
`content/houses/` directly (ADR-0021 D193, §4.6); the B3 tile house id must match the House family or compilation
fails. A World therefore pins a catalogue revision through its active bundle, and a new catalogue revision reaches a
World only in a new bundle, which activates only at a planned world reset (D194, §4.7):

1. Text, rent amount, bed count, marker or restriction changes of a House are ordinary revisions.
2. A revision that changes the `tiles` or `doors` of a House, or retires a House, activates only at a planned reset
   and must pass the HOUSE-CUSTODY-0 §3.6 target check: every live `HouseInterior` row's `(house_key, position)`
   must be a tile of the same House in the target bundle. A removed House, a re-keyed House, a removed tile or a
   tile moved to another House fails it; the preflight aborts the reset cleanly, and a failure at the step-4
   recheck is cleared only by an `EXP-HOUSES-01` §14.7 evacuation. Settlement of an allocated or auctioned House
   follows `EXP-HOUSES-01` §14.7 and §19, never a silent content swap.
3. A retired House is never offered at auction again.
4. A new House enters a World vacant and is allocated through the public auction (`EXP-HOUSES-01` §11).
5. Shared wall tiles are never House tiles for custody (HOUSE-CUSTODY-0 §3.1): no item custody attaches to them.

Until a House has an owner, its tiles are ordinary map tiles and Ground items on them are retired at the reset like
any other Ground item (D196; HOUSE-CUSTODY-0 §3.5).

## 5. Delivery order

1. This contract (this change).
2. Catalogue population in `content/houses/` under §3, with its own review of the reported divergences.
3. City `Area` records for the `town` references (#1353).
4. Housing persistence and runtime under `EXP-HOUSES-01` §29, each with its own authorization and review.
   Prerequisites: HOUSE-CUSTODY-0 accepted and HOUSE-CUSTODY-1 (the `HouseInterior` storage slice) landed. Until
   then no House may become ownable or auctionable, and Ground items on House tiles are retired at every planned
   reset (D196).

Steps 2 and 3 are done, so the static House catalogue is complete (owner direction 3a, 2026-09-30); step 1, the
acceptance of this contract, is pending its independent review (header). A later catalogue change is a revision
under §4.

## 6. Not decided

- Physical schema, migration and the `HouseId` representation.
- Bed and door item placements: base-map content (ADR-0021 MAP-* children), checked against §3 there.
- Residence templates, Guildhouse lifecycle and all numbers deferred by `EXP-HOUSES-01` §26.

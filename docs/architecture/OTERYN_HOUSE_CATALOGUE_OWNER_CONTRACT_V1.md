# House Catalogue Owner Contract V1

- Status: Contract candidate. Owner direction given 2026-09-30 (prepare the contract, then populate the
  catalogue); needs exact-head independent review before it is accepted.
- Date: 2026-09-30
- Issue: #162; lane `HOUSES`
- Implements: the static-content side of `EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` (ACCEPTED); it adds no
  housing semantics and changes none.
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
| `kind` | `private_house` and `shop` are ordinary physical houses (owner `CharacterId`, `EXP-HOUSES-01` §8); `guildhall` is the Guildhouse class (owner `GuildId`, §9), whose lifecycle stays deferred. |
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
- `entrance`: CrystalServer `world-house.xml` (owner decision 3a, 2026-09-29), until it is derived from the
  official layout. It is next to a door for 968 of 995 houses; the rest are listed in the conversion report.
- Door item classification: CrystalServer `items.xml` `type="door"` ids, under the recorded assumption that
  engine item ids equal client appearance ids (HOUSES-3).
- The one door in two layouts (East Lane 1a/1b) belongs to East Lane 1a (`SHARED_DOOR_OWNERS`); any new one stops
  the conversion until it is decided.
- TibiaWiki BR and CrystalServer values that differ from the client are reported, never used.

## 4. Revisions against live Worlds

A World pins a catalogue revision. A new catalogue revision reaches a World only through an explicit
content update of that World:

1. Text, rent amount, bed count, marker or restriction changes of a House are ordinary revisions.
2. A revision that changes the `tiles` or `doors` of a House that is allocated, in auction or holds housing
   content in that World is a housing lifecycle change: it is applied only under the full housing-content fence
   and value-safe settlement of `EXP-HOUSES-01` §14.7 and §19, never as a silent content swap.
3. Retiring a House that is allocated, in auction or holds content in a World first settles it under the same
   rules; a retired House is never offered at auction again.
4. A new House enters a World vacant and is allocated through the public auction (`EXP-HOUSES-01` §11).

## 5. Delivery order

1. This contract (this change).
2. Catalogue population in `content/houses/` under §3, with its own review of the reported divergences.
3. City `Area` records for the `town` references.
4. Housing persistence and runtime under `EXP-HOUSES-01` §29, each with its own authorization and review.

## 6. Not decided

- Physical schema, migration and the `HouseId` representation.
- Deriving `entrance` from the official layout.
- Bed and door item placements (world placements) and a bed-count check against them.
- Residence templates, Guildhouse lifecycle and all numbers deferred by `EXP-HOUSES-01` §26.

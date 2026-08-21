# Oteryn Game -> Atlas Semantic Search Source Profile v1

- Profile ID: `oteryn-game-atlas-semantic-search-v1`
- Capability: `semantic-search-source-v1`
- Parent contract: `oteryn-game-atlas-export-v1`
- Coordinate profile: `oteryn-world-spatial-v1`
- Legacy import profile: `oteryn-crystalserver-legacy-spatial-import-v1`
- Owner: `Oteryn/Oteryn-Game`
- Consumer: `Oteryn/Oteryn-Atlas`
- Status: ACCEPTED once this exact content is merged to protected `main`.

## Purpose

This profile freezes the smallest executable public search-source shape needed for Atlas global semantic search. Game publishes normalized records; Atlas owns ranking, derived indexing, browser filtering and presentation.

It does not make Atlas a World/Content authority and does not authorize browser access to OTBM, Canary, Crystal or other legacy sources.

## Public record shape

Each record contains exactly the semantic concepts below; physical JSON spelling may be superseded only by a new profile revision:

```json
{
  "kind": "npc",
  "id": "producer-owned stable exported record identity",
  "label": "Sam",
  "aliases": [],
  "position": {"x": 32361, "y": 32198, "floor": -7},
  "bounds": null,
  "provenance": {},
  "capabilities": ["shop", "static-placement"]
}
```

Required fields are `kind`, `id`, `label`, `aliases`, `position`, `bounds`, `provenance` and `capabilities`.

## Kinds

Revision 1 permits only Game-public-allowlisted records of these families:

- `npc`
- `monster`
- `town`
- `waypoint`
- `poi`
- `teleport`
- `house`
- `quest_area`
- `mechanic`

A permitted kind is not evidence that records of that kind are currently emitted. Missing families remain absent; Atlas must not manufacture them.

## Identity

`id` is a stable Game-owned exported-record identity suitable for `id:` lookup. It is not automatically canonical Game entity identity.

For migration-sourced Town/Waypoint placements whose canonical native entity identity is not established, provenance carries `identity_state=UNRESOLVED`. The producer may assign a deterministic exported record identity from the normalized label/position solely to address the exported record.

Legacy AID/UID/town numeric IDs, filenames and display labels are not promoted to canonical identities by this profile. AID/UID search remains unsupported until a later explicit Game export revision public-allowlists them.

## Coordinates and aliases

All record positions use `oteryn-world-spatial-v1` native coordinates.

For the pinned Crystal/OTBM migration profile, Game additionally publishes explicit input aliases:

```text
legacy/display z 0..15 -> native floor = -z
```

This alias table is producer data. Atlas may use it to parse a coordinate query such as `32369 32220 7`; Atlas must not reconstruct the mapping from Tibia conventions.

## NPC capabilities

Public service capabilities are conservative summaries only:

- `shop` only when a static literal `npcConfig.shop` table is present;
- `bank` only when accepted static bank helpers are present;
- `guildBank` only when accepted static guild-bank helpers are present;
- `travel` only when accepted static `StdModule.travel` evidence is present.

Conflicting same-name definitions make service capability resolution `AMBIGUOUS` and publish no service capability claim for that name. Dynamic behavior, private quest state and script execution details are not exported.

## Towns and bounds

The current legacy Town record supplies a public navigation/temple position but no authoritative town polygon/bounds. Therefore `bounds` is `null` and capability is `overlay-point` until Game publishes an explicit public town geometry contract. Atlas must not infer administrative bounds from raster pixels, POIs or neighboring tiles.

## Ranking boundary

Game does not publish search rank. Atlas owns normalization/ranking and may use only public fields from this profile. Ranking must be deterministic and type filters must never alter source facts.

## Validation

Producer requirements:

- deterministic output for identical declared inputs;
- unique bounded `id` values;
- valid native positions;
- supported kinds only;
- explicit source provenance;
- no AID/UID promotion;
- fail closed on unsupported source contract, malformed records or pinned-map digest mismatch.

Atlas consumer requirements:

- validate profile/capability before indexing;
- bound file size/counts/query length/results;
- reject duplicate IDs and malformed coordinates;
- preserve provenance/capabilities;
- use explicit `input_floor_aliases` rather than inferred floor conversion;
- no legacy runtime fallback.

## Pinned migration evidence for v1 implementation

- `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`
- `world.otbm` SHA-256 `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`
- existing Game `static-creatures-v1` producer merged on `main`

This profile grants no asset redistribution rights and no production/deployment authority.

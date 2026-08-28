# Game -> Atlas static creature export v1

Issue authority: `Oteryn/Oteryn-Game#29`; NPC-role extension: `Oteryn/Oteryn-Game#41`; consumer: `Oteryn/Oteryn-Atlas#64`.

This importer is the Game-owned migration boundary for static NPC and monster/spawn facts. Browser/Atlas runtime must consume only its normalized projection, never Crystal XML/Lua directly.

Pinned migration evidence: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`:

- `vendor/map-analysis/crystalserver/data-global/world`
- `vendor/map-analysis/crystalserver/data-global/npc`
- `vendor/map-analysis/crystalserver/data-global/monster`

The legacy Tibia appearance corpus is intentionally not redistributed by this exporter. It exports normalized outfit parameters; Atlas may bind only separately authorized publication assets and must retain an Atlas-owned marker fallback otherwise.

`npc_role_schema_version = 1` adds optional factual NPC roles derived only from explicit authored service/quest constructs. See `docs/contracts/OTERYN_GAME_ATLAS_NPC_ROLES_V1.md`. The parent export remains `semantic_revision = 1`.

Exact pinned-data qualification produced:

- NPC placements: `1068`; monster/spawn placements: `87565`;
- unresolved definition matches: `461`; ambiguous definition matches: `5`;
- NPCs with resolved roles: `705`; role ambiguity: `10`;
- role occurrences: `bank=25`, `travel=51`, `shop=313`, `quest=432`, `blessing=26`, `trainer=54`;
- deterministic semantic digest: `sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4`.

The fixture self-test verifies deterministic output, coordinate conversion, case-insensitive definition matching, outfit normalization, role extraction/order, role-only ambiguity isolation, public output omission of legacy source paths, and fail-closed conflicting appearance definitions.

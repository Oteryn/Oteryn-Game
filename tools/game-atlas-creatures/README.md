# Game -> Atlas static creature export v1

Issue authority: `Oteryn/Oteryn-Game#29`; consumer: `Oteryn/Oteryn-Atlas#30`.

This importer is the Game-owned migration boundary for static NPC and monster/spawn facts. Browser/Atlas runtime must consume only its normalized projection, never Crystal XML/Lua directly.

Pinned migration evidence: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`:

- `vendor/map-analysis/crystalserver/data-global/world`
- `vendor/map-analysis/crystalserver/data-global/npc`
- `vendor/map-analysis/crystalserver/data-global/monster`

The legacy Tibia appearance corpus is intentionally not redistributed by this exporter. It exports normalized outfit parameters; Atlas may bind only separately authorized publication assets and must retain a marker fallback otherwise.

Real pinned-data qualification on Synology/Python 3.8 produced:

- NPC placements: `1068`
- monster/spawn placements: `87565`
- unresolved definition matches: `461`
- ambiguous definition matches: `5`
- deterministic semantic digest: `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`

The fixture self-test also verifies deterministic repeat output, relative XML coordinate conversion, case-insensitive definition matching, outfit normalization, public output omission of legacy source paths, and fail-closed conflicting definitions.

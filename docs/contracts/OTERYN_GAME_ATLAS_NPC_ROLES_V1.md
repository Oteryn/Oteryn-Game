# Oteryn Game → Atlas NPC Roles v1

Issue authority: `Oteryn/Oteryn-Game#41`; consumer lifecycle: `Oteryn/Oteryn-Atlas#64`.

This contract is an additive profile of `static-creatures-v1` under `oteryn-game-atlas-export-v1`. The parent export remains `semantic_revision = 1`; role semantics are identified independently by `npc_role_schema_version = 1`.

## Authority

`Oteryn-Game` owns NPC role facts. Atlas may filter and choose presentation icons from published roles but may not infer a role from an NPC name, coordinates, minimap pixels, outfit pixels, visual similarity or legacy runtime access.

The migration importer may derive roles only from explicit authored operational constructs in pinned evidence `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` at `vendor/map-analysis/crystalserver/data-global/npc`.

No Lua/source path or proprietary asset bytes are exported.

## Role vocabulary

The closed v1 role vocabulary and deterministic order are:

`bank`, `travel`, `shop`, `quest`, `blessing`, `trainer`.

A record can carry multiple roles. Absence of a role is not evidence for any other role.
## Authoritative extraction rules

The importer recognizes only these explicit constructs:

- `npc:parseBank(...)` or `NpcBankGreetCallback` → `bank`;
- `StdModule.travel` → `travel`;
- assignment to `npcConfig.shop` → `shop`;
- `Storage.Quest.` reference → `quest`;
- `StdModule.bless` or `player:addBlessing(...)` → `blessing`;
- `StdModule.learnSpell` → `trainer`.

These rules intentionally do not classify shop inventory into weapon/armor/food/etc. A later Game-owned schema may add narrower roles when authoritative item/service semantics are available.

## Record fields

For an NPC definition whose role evidence is internally consistent, Game exports `role_resolution_state = "RESOLVED"` and an optional non-empty `roles` array in the vocabulary/order above. A resolved definition with no recognized role omits `roles`.

If duplicate definitions resolve to the same appearance but expose different role sets, Game exports `role_resolution_state = "AMBIGUOUS"` and omits `roles`. This role ambiguity must not change the independently computed creature `resolution_state` or appearance identity.

Monster/spawn records never carry NPC role fields.
## Pinned-data qualification

Qualified against the exact pinned migration revision above using the existing static creature world/NPC/monster roots:

- NPC placements: `1068`;
- monster/spawn placements: `87565`;
- creature unresolved: `461`;
- creature ambiguous: `5`;
- NPCs with at least one resolved role: `705`;
- NPC role ambiguity: `10`;
- resolved role occurrences: `bank=25`, `travel=51`, `shop=313`, `quest=432`, `blessing=26`, `trainer=54`;
- semantic digest: `sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4`.

The original creature resolution counts remain unchanged from static-creatures-v1 qualification. The semantic digest changes because the public-safe projection now includes the additive role schema/record fields.

## Consumer rule

Atlas consumers claiming NPC role support must require `npc_role_schema_version = 1`, validate the closed vocabulary and role resolution state, preserve the Game semantic digest, and fail closed on malformed role metadata. Presentation-only fallback such as an `Other` icon is allowed only to mean “no resolved published category”; it is not a Game role.

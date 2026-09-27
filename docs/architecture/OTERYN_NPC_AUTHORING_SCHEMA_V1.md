# Oteryn NPC Authoring Schema v1

- Date: 2026-09-27
- Status: CANDIDATE / authoring schema with executable validation; no `content/` population, no native identity, no runtime
- Task: `OTV2-20260927-npc-authoring-schema-v1`
- Programme: KAN-16 / #504 (content-world B5: NPC/services/dialogues/shops/travel)
- Admission main: `a822326c9cf4607100e58bbc3673748f3fa299bb`
- Machine artifacts: `tools/content-schema/npc-authoring/`
- Companion of: `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (`content/npcs/**`, `content/dialogues/`,
  `content/services/**`) and `OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md` (`NpcServiceDefinition`)

## 1. Outcome

Oteryn has one candidate authoring format for NPCs sourced from Canary and Crystal, a converter that
turns every NPC of both pinned datapacks into that format, a Canary↔Crystal fact diff, a TibiaWiki
(Fandom) comparison and a readiness census. It supersedes the static-text scan of CW2-B5
(`OTV2-20260920-content-world-cw2-b5-npc-service-bindings-504`) as the NPC import route: CW2-B5
recovered 5 complete travel profiles and no dialogue; this route recovers 363 Canary travel rows with
destinations and prices, 1,809 spell-teaching rows and a structured keyword dialogue tree per NPC.

The server does not read this format and nothing in `content/` changes. Promotion into
`content/npcs/definitions/`, `content/dialogues/` and `content/services/**` is later work gated by the
open decisions in §7.

## 2. Shape

```text
candidate bundle (npc.schema.json), one per NPC source file
├── key, source          canary:npc/<file stem> | crystal:npc/<file stem>; repository, revision, path, sha256
├── definition           name, display name, description (text ref), profession,
│                        presentation (outfit or item look, speech bubble, light),
│                        movement (walk interval/radius, floor change), vitals, respawn
├── voices               cadence + lines (text ref, yell)
├── dialogue             messages (greet/farewell/walkaway/send_trade), keyword tree
│                        (keywords, kind, text ref, flags, gate, effect, children), scripted handlers
├── services             trade (currency, offers: item name + client id, buy/sell price, count,
│                        sub type, stock gate), travel, spells, blessings, promotion, kick
├── placements           absolute position, direction, spawn interval, spawn radius, spawn file
└── unresolved[]         path + closed reason + detail, one row for every fact that is not static
```

Keyword `kind` is closed: `say`, `travel`, `learn_spell`, `bless`, `kick`, `promote`,
`rookgaard_hints`, `script` (Lua callback), `none`, `unknown_module`. `gate` is `NONE` or
`LUA_PREDICATE`; `effect` is `NONE` or `LUA_ACTION`. Service rows point back to their keyword node by
`dialogue_path`, so a service is always reached through dialogue, as in
`NPC -> Dialogue -> Service -> Item/currency/Interaction` of the tree contract.

Text is never stored in a committed bundle. Every description, message, keyword answer and voice line is
a text reference `{sha256, length, placeholders, links}` (§6).

## 3. Owner decisions

| ID | Decision |
| --- | --- |
| D1 | First step is schema + converter + census only. No `content/` writes and no native NPC identity; keys stay source-scoped (`canary:` / `crystal:`). |
| D2 | Canary and Crystal are equal sources. No automatic winner: every `CONFLICT` and one-sided fact in the source diff stays an open decision row. |
| D3 | TibiaWiki (Fandom, CC BY-SA) is compared now; TibiaWiki BR is added later. BR is behind a Cloudflare challenge from the build container (HTTP 403), so it needs another access route or an owner-supplied export. Only compared facts and page/revision ids are stored. |

Decisions were taken in the owning session on 2026-09-27.

## 4. Conversion rules

Both datapacks are evaluated the same way; the sources are pinned exactly:

| Source | Repository | Revision | NPC files | Spawn files |
| --- | --- | --- | --- | --- |
| Canary | `opentibiabr/canary` | `47dfd51f45280a59a1d3e50ba7edd573d7234446` (same pin as monster authoring) | `data-otservbr-global/npc/**` (1,036) | `data-otservbr-global/world/**/*npc*.xml` |
| Crystal | `zimbadev/crystalserver` | `ff7ede593c69d4c658b382c97443e8155926924a` (same pin as `imports/crystalserver/sources.json`) | `data-global/npc/**` (1,119) | `data-global/world/**/*npc*.xml` |

Crystal's `data-crystal/` datapack (32 NPCs of Crystal's own map) is out of scope.

- **Evaluation.** Each NPC file runs in a stubbed LuaJIT sandbox, as monster authoring does. The
  sandbox loads the pinned `keyword_handler.lua`, `custom_modules.lua` and `string.lua` from the same
  checkout, so keyword aliasing and the greet/farewell/spell helpers behave as in the engine. Engine
  globals are recording stubs; `os`, `io`, `require`, `dofile`, `loadfile`, `package` and `debug`
  are removed, the JIT is off and an instruction budget stops runaway files. Only top-level code runs;
  `onSay`, `creatureSayCallback`, keyword conditions and keyword actions are never called and become
  `LUA_CALLBACK`, `LUA_PREDICATE` or `LUA_ACTION` rows.
- **Placement.** `spawn_npc.cpp` (identical in both sources): position =
  (`centerx + x`, `centery + y`, `centerz`); the child `z` is ignored; direction defaults to north
  (`DIRECTION_NORTH=0 … WEST=3`, `position.hpp`); a spawn interval outside 1 s–1 day never spawns
  (`MINSPAWN_INTERVAL`/`MAXSPAWN_INTERVAL`). Placements join to NPCs by registered NpcType name.
- **Conditional services.** The engine answers with the first matching sibling whose condition passes.
  A service row is `gate=LUA_PREDICATE` when its own node, an ancestor, or an earlier sibling with the
  same keyword carries a condition (e.g. Captain Bluebear's Yalahar passage).
- **Shops.** `itemName`/`itemname`/`name` and `clientId`/`clientid` spellings are accepted; `itemid`
  is kept as `server_item_id`. Offers are stored in a canonical order because shops filled from a
  `pairs` loop have no stable declaration order. `storageKey`/`storageValue` become a
  `PLAYER_STORAGE_GATE` row. No offer is joined to a native Item yet.
- **Determinism.** Two conversions at the same pins are byte-identical; each census records
  `bundle_digest`.

## 5. Mapping to the target tree

| Bundle part | Target | Promotion work |
| --- | --- | --- |
| `definition`, `voices`, `placements` | `content/npcs/definitions/` (+ `presentation` bindings) | native NPC key; placement identity inside the world/channel model |
| `dialogue` | `content/dialogues/` | text source (§6); scripted handlers and predicates → Interaction/Quest owners |
| `services.trade` | `content/services/trade/` | client id → native Item via the G4 crosswalk; currency; Item files never get reverse `sold_by` arrays |
| `services.travel` | `content/services/travel/` | destination → world position/area; premium, level and discount rules |
| `services.spells`, `blessings`, `promotion` | `content/services/{tasks,blessings}/` or ability owners | spell/vocation/blessing keys |

## 6. Text

Dialogue, voice and description text is Tibia narrative content, reserved by `LICENSE-ASSETS.md`.
Committed bundles carry text references only; `convert.py --include-text` adds text for local review and
`validate_npc.py` rejects such bundles unless `--allow-text` is given. How Oteryn obtains or authors NPC
text for promotion is open decision O2.

## 7. Import readiness and open decisions

Readiness (census, per source):

| Class | Canary | Crystal | Meaning |
| --- | ---: | ---: | --- |
| `STATIC_COMPLETE` | 383 | 215 | every fact static, no unresolved row |
| `STATIC_SERVICES` | 546 | 745 | all service rows ungated and placed; open rows are dialogue scripting only |
| `SCRIPTED` | 35 | 39 | at least one gated service row or non-dialogue Lua behaviour |
| `UNPLACED` | 65 | 116 | registered but not placed by the spawn files |
| `LOAD_ERROR` / `NOT_AN_NPC` | 5 / 2 | 2 / 2 | top-level code needs a live player/engine, or a helper file |

Services recovered (Canary / Crystal): trade 301 / 318 NPCs with 10,055 / 12,187 offers; travel 71 / 69
NPCs with 363 / 359 rows; spells 50 / 53 NPCs with 1,809 / 1,855 rows; blessings 27 / 27; promotion
5 / 6; placements 1,029 / 1,072. Crystal's 413 `addDialogOptions` calls are recorded as
`UNMAPPED_ENGINE_CALL`.

Canary↔Crystal (D2): 1,026 NPCs pair; 1 identical, 953 complementary (one-sided facts only, mostly
Canary's `profession`/`speech_bubble`), 67 conflicting; 10 Canary-only and 93 Crystal-only NPCs.
Fact conflicts: keywords 120, trade 9, travel 21, spells 2, messages 10, definition 8.

TibiaWiki (Fandom) comparison: pending in this task (`wiki_fandom.py`).

Open decisions before promotion:

- **O1 identity:** native NPC key format and placement identity under
  `OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`.
- **O2 text:** source or authoring route for NPC text (§6).
- **O3 conflicts:** resolution of the 67 conflicting NPCs and the one-sided NPCs (D2 keeps them open).
- **O4 scripted behaviour:** owner for Lua predicates/actions/handlers (quest state, storage gates),
  i.e. Interaction/Quest vs. NPC service.
- **O5 item join:** offers → native Items through the G4 crosswalk.
- **O6 TibiaWiki BR:** access route (D3).

## 8. Validation

```sh
cd tools/content-schema/npc-authoring
python -m unittest test_npc_authoring.py
python validate_npc.py samples/bundles
python wiki_fandom.py self-test
```

Full-population validation needs the pinned checkouts (README). At this revision all 2,155 converted
bundles validate and two conversions are byte-identical.

## 9. Boundaries

No `content/` or `rulesets/` change, no native identity, no WorldProject/runtime change, no Lua
transliteration into Oteryn behaviour, no committed Tibia text, no Reference-parity claim. Evidence is
`OTS_HYPOTHESIS_ONLY`.

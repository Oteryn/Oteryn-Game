# Oteryn NPC Authoring Schema v1

- Date: 2026-09-27
- Status: CANDIDATE / authoring schema with executable validation; native keys assigned in promotion candidates only; no `content/` population, no runtime
- Tasks: `OTV2-20260927-npc-authoring-schema-v1` (PR #975), `OTV2-20260927-npc-promotion-candidates` (PR #983)
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

Committed bundles store no text: every description, message, keyword answer and voice line is a text
reference `{sha256, length, placeholders, links}` (§6). Admitted dialogue text (greet, farewell,
walk-away, send-trade, `say` keyword replies and voices) is stored in full in WorldProject/v2 as reference data (D9).

## 3. Owner decisions

| ID | Decision |
| --- | --- |
| D1 | First step is schema + converter + census only. No `content/` writes and no native NPC identity; keys stay source-scoped (`canary:` / `crystal:`). |
| D2 | Canary and Crystal are equal sources. No automatic winner: every `CONFLICT` and one-sided fact in the source diff stays an open decision row. |
| D3 | TibiaWiki (Fandom, CC BY-SA) and TibiaWiki BR are compared. BR is behind a Cloudflare challenge from the build container (HTTP 403); it is captured by a repository workflow instead (`npc-tibiawiki-br-capture.yml`, owner request 2026-09-28), whose raw wikitext stays a CI artifact. Only compared facts and page/revision ids are stored: for BR, `imports/tibiawiki/npc-br/<date>/` keeps each page's ids and raw SHA-256, `implemented`/`removed`, map positions, trade lists and the lines the NPC itself speaks in its transcript (Tibia NPC text, D9), never wiki prose; a trade table or transcript section headed as from before an update is left out. |

| D4 | Native NPC key `oteryn:npc.<slug>` (resolves O1). The slug is derived once from the registered name (ASCII fold, lower case, non-alphanumerics to `_`) and is frozen at promotion; a later rename keeps the key. Source names, file stems and numeric ids stay provenance only. Two NPCs with one slug are both held. A travel service is `oteryn:service.travel.<slug>`. Placements carry no identity yet. |
| D5 | NPC text is authored by Oteryn (resolves O2). Canary/Crystal supply structure only (keywords, services, placeholders, links); no Tibia text is promoted. Description, voices and dialogue stay out of promotion until authored. |
| D6 | TibiaWiki (Fandom) is the tie-breaker between Canary and Crystal (resolves O3 and O7): the source the wiki agrees with wins; without wiki agreement the fact stays open. The wiki never supplies a value itself. |
| D7 | WorldProject/v2 is extended with typed travel routes and offer quantities before NPCs are admitted, as for monsters ("druga droga"); see `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`. |
| D8 | The wiki completes held NPCs (owner request 2026-09-27). An NPC that neither source places is promoted when its TibiaWiki page has a position; that position becomes its candidate placement (`origin: wiki`, direction and spawn interval unknown). A single-source NPC whose name is a Day/Night or stage variant (` (Day)`, ` (Night)`, ` Init`, ` Vampires Lair`, ` Back`) is confirmed by its base name's page. Names match the page title, its `name` or its `actualname` (the in-game name). A name that still has no page is matched only by a single edit (insertion, deletion, substitution or adjacent transposition), only when it is at least 10 characters and exactly one wiki NPC is that close (`WIKI_SPELLING`). An unplaced NPC whose wiki page has no position (a seasonal NPC such as Santa Claus) is promoted without a placement (`WIKI_CONFIRMED`). When the two sources place an NPC differently and the wiki position matches neither, the wiki position wins (D6); without a wiki position the NPC is promoted without a placement. The server-only NPCs Canary and Loot Buyer are rejected (`OWNER_REJECTED`). |
| D9 | Supersedes D5 (`LICENSE-ASSETS.md` after #1050; owner request 2026-09-28). Tibia Global NPC text is admitted 1:1 as reference data, with Canary/Crystal provenance. A dialogue is admitted when both sources agree, or when only one source has the NPC. Only static `say` keyword nodes become Dialogue keywords; action keywords (`travel`, `learn_spell`, `bless`, `promote`, `kick`, `rookgaard_hints`) belong to their Service or ability owners. Conditional or scripted nodes and conflicting dialogues stay held. |
| D10 | Tibia Global transcripts break dialogue ties (owner request 2026-09-28: reproduce Global as closely as possible from the wiki and the internet). When Canary and Crystal stage different dialogues for an NPC, the in-game transcripts of `s2ward/tibia` (pinned commit) are the tie-breaker: the source whose differing texts match more transcript lines, and at least one, is admitted whole. A tie, no match or no transcript keeps the NPC held. A transcript never supplies text itself; the digest of every transcript used is recorded. |
| D11 | NPCs removed from Tibia Global are not admitted (owner request 2026-09-28: reproduce current Global and cross-check every NPC against TibiaWiki BR). An NPC is held `REMOVED_FROM_GAME` when TibiaWiki BR records a `removed` version and TibiaWiki Fandom marks it `status = deprecated`, through a fixed table in `promotion_candidates.py`. The first entries are the five Duelling Arena supervisors removed in 13.12 (Brom, Brutus, Roughington, Shadowpunch, Victor). An NPC only Fandom marks deprecated, such as a Rookgaard tutorial NPC that still exists but is no longer reached, stays admitted (owner decision 2026-09-28). |
| D12 | A price both wikis agree on replaces the source price (owner request 2026-09-28: reproduce Global prices; "Kontynuuj" on the cross-check fix order). With `--br-facts`, an admitted offer takes the price TibiaWiki Fandom and TibiaWiki BR both state for that NPC, item name and direction, when it differs from the source price; every BR row of the offer must give that same explicit price. The candidate records `{"fact": "trade.<item>.<direction>", "rule": "WIKI_PRICE", "chosen": "wiki"}` and the BR facts digest. One wiki alone, or two wikis that disagree, never change a price. |

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

Dialogue, voice and description text is Tibia narrative content. `LICENSE-ASSETS.md` allows it to be
recorded as reference data for faithful reconstruction.
Committed bundles carry text references only; `convert.py --include-text` adds text for local review and
`validate_npc.py` rejects such bundles unless `--allow-text` is given. Promoted NPC text is taken 1:1
from the sources (D9). The later dialogue admission slice stages it from bundles converted with
`--include-text` and keeps each `say` node's conversation flags (`only_focus`, `only_unfocus`, `reset`,
`ungreet`, `move_up`).

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

TibiaWiki (Fandom) comparison (`wiki_fandom.py`, snapshot 2026-09-27: 1,246 NPC infoboxes,
9,799 item infoboxes, 504 NPCs with trade prices; snapshot SHA-256 in the compare evidence):

| Fact | Canary | Crystal |
| --- | --- | --- |
| NPCs joined by name (wiki-only / source-only) | 955 (292 / 81) | 1,017 (230 / 102) |
| position exact / within 3 sqm / mismatch | 234 / 573 / 92 | 252 / 584 / 98 |
| travel price match / mismatch / wiki-only / source-only | 86 / 22 / 48 / 121 | 85 / 23 / 49 / 119 |
| trade buy price match / mismatch | 5,089 / 229 | 5,192 / 229 |
| trade sell price match / mismatch | 3,905 / 93 | 3,952 / 90 |

TibiaWiki `posx`/`posy` usually sit one tile from the spawn tile, so `NEAR` is the expected good
outcome. Checked mismatches are real source differences, not parser artefacts (e.g. Captain
Breezelda's Carlin and Thais fares are swapped in the Canary script). Wiki facts are comparison
evidence only; they do not override either source.

Open decisions before promotion:

- **O1 identity:** resolved by D4; placement identity is still open.
- **O2 text:** resolved by D9 (Tibia Global text as reference data, superseding D5).
- **O3 conflicts:** resolved by D6; facts the wiki cannot decide stay held (§8).
- **O4 scripted behaviour:** owner for Lua predicates/actions/handlers (quest state, storage gates),
  i.e. Interaction/Quest vs. NPC service.
- **O5 item join:** resolved: offers resolve through the protected Item identity map (§8).
- **O6 TibiaWiki BR:** resolved by D3 (repository workflow capture; committed facts only).
- **O7 wiki disagreements:** resolved by D6.
- **O8 admission route:** resolved: `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md` admits NPCs through
  WorldProject/v2, with the content tree regenerated from it.

## 8. Promotion candidates

`promotion_candidates.py` merges each Canary/Crystal pair (or a single-source NPC that the wiki knows)
into the record a later promotion would write:

```text
promotion candidate (samples/promotion-candidates-v1.json)
├── identity          {family: NPC, key: oteryn:npc.<slug>, revision: definition-r1}         (D4)
├── name, profession, presentation (outfit, speech bubble), movement
├── placements[]      position, direction, spawn interval, spawn radius
├── travel_service    {identity: oteryn:service.travel.<slug>, routes[]: destination keyword,
│                     position, price, premium, min level, discount} or null
├── trade_service     {identity: oteryn:service.trade.<slug>, currency (null = gold, else Item),
│                     offers[]: Item, source item id, direction, unit price, count, sub type} or null
├── provenance        source key + file SHA-256 per source; wiki page id + revision id
├── arbitration[]     facts decided by the wiki (D6) and the source chosen
└── left_out[]        routes and offers not promoted: GATED_ROUTE, ROUTE_CONFLICT_WIKI_UNDECIDED,
                      ROUTE_UNCONFIRMED, GATED_OFFER, OFFER_UNCONFIRMED, OFFER_CONFLICT_WIKI_UNDECIDED,
                      ITEM_NOT_REGISTERED, CURRENCY_CONFLICT
```

Definition fields both sources state identically, or only one states, are adopted. Placements and
routes that differ, or exist in one source only, need the wiki (D6): placements go to the source with
the better position match (MATCH over NEAR, never MISMATCH); routes go to the source whose price
equals the wiki price, with one destination. A definition conflict holds the NPC, since the wiki has no
outfit or movement facts. Offers follow the route rule, with the wiki's buy and sell price as
tie-breaker; their Items resolve through the protected Item identity map (`--item-map`, the output of
`export_reference_item_identity_map`, SHA-256 `83ba3c26…`), which resolves O5. Source `buy` becomes
`SellToPlayer`, source `sell` becomes `BuyFromPlayer`. Description, voices, dialogue (staged separately, D9), spells,
blessings and promotion are not part of a candidate.

Result at this revision (deterministic; snapshot SHA-256 recorded):

| Outcome | NPCs |
| --- | ---: |
| candidates | 984: 289 with a trade service (10,723 offer rows), 53 with a travel service (189 routes); 154 facts decided by the wiki |
| held: unplaced in both sources | 91 |
| held: single source, not on the wiki | 32 |
| held: definition conflict (outfit/movement) | 7 |
| held: placement conflict the wiki cannot decide | 6 |
| held: key collision (`Harlow` and `Harlow` trade variant) | 2 |
| held: no slug (the NPC named `...`) | 1 |
| not loadable in either source | 6 |

Left out of the candidates: 65 routes (46 gated by Lua predicates, 15 conflicts the wiki cannot
decide, 4 one-sided routes the wiki does not confirm) and 218 offers (122 gated by player storage,
52 one-sided and unconfirmed, 38 Items newer than the Oteryn Item registry, 6 conflicts).

## 9. Validation

```sh
cd tools/content-schema/npc-authoring
python -m unittest test_npc_authoring.py
python validate_npc.py samples/bundles
python wiki_fandom.py self-test
python validate_promotion.py samples/promotion-candidates-v1.json
```

Full-population validation needs the pinned checkouts (README). At this revision all 2,155 converted
bundles validate and two conversions are byte-identical.

## 10. Boundaries

No `content/` or `rulesets/` change, native keys only inside candidate evidence until the admission slices of `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`, no WorldProject/runtime change, no Lua
transliteration into Oteryn behaviour, no committed Tibia text, no Reference-parity claim. Evidence is
`OTS_HYPOTHESIS_ONLY`.

# Native entry source profile — retained field-level fit

Status: **RETAINED EVIDENCE for owner-accepted source-shape direction**, [#822 comment 5846264141](https://github.com/Oteryn/Oteryn-Game/issues/822#issuecomment-5846264141). Protected delivery follows the exact-head architecture PR. No implemented consumer, admitted product source, qualified frame/pair or executable release is established here.

Bieżący protected main wg potwierdzonego readbacku root: `de7c5499a2e6ad4ec533ea895754bdd82bda292e`; #935/#936 są zintegrowane. Immutable code/contract evidence poniżej pochodzi z dokładnego `91fb3135a8a67d012bfe048b230f3501780d244b`; jego odczyt nie jest twierdzeniem o ponownym live odczycie main.

## Wynik

**Existing v2 does not fit unchanged.** Pokrywa geometrię i część definition graph, ale nie wszystkie obowiązkowe FirstProduction fields. Nie istnieje lawful reinterpretation candidate fields, editor tags, WorldObject, Area.parent ani Reference loot algorithm, która usuwa te braki.

Najmniejszy konkretny wariant zachowujący istniejący v2 project/roles i ordinary artifact profile: **jeden optional, typed `native_first_entry` overlay w istniejącym `definitions/declarations.json`**, przyjęty w bounded source schema/semantics amendment. Exact nowy root source-profile selector to `OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-entry-qualification-1`; declarations schema to `OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1`. Native admission wybiera ten profil przed parse/lowering i wymaga exact nowego schema/overlay; old v1/v2 profile nie wybierają tej ścieżki. Overlay ma niżej dokładnie wymienione pola. Native qualifier korzysta tylko z typed records i tego overlay; nie interpreteruje ogólnych `fields`. Default Reference lowering nie uzyskuje native semantics.

To zmiana znanego fizycznego shape dokumentu i source executable meaning. Dzisiejszy parser ma `deny_unknown_fields`; odrzuci overlay. Nie przedstawiamy tego jako istniejącego, niezmienionego v2. Root/manifest/lock, osiem roles i ich locators mogą pozostać; old admitted documents zachowują dokładne bytes, brak nowego public wire ID i brak zmian FIRST_PRODUCTION runtime encoding. Known-schema change wymaga protected owning acceptance przed produkcyjnym parser/lowering worker.

## Exact evidence

- `project/v2.rs:1087-1121`: World/key/id/frame/bounds/floors i Placement/world/map_revision/definition/area/frame/x/y/floor/order/CandidateOnly.
- `project/v2.rs:2353-2451`: valid nonempty increasing floors; finite bounds; exact frame equality; typed reference; in-bounds placement.
- `project/v2.rs:1244-1257`: declarations/world document schemas i unknown-field refusal.
- `project/v2.rs:128-131,489-504`: declarations i named fields jawnie candidate/nonexecutable.
- `project.rs:551-596,795-835`: Reference record fields; LootEntry ma item/count/probability, nie RNG-purpose/entry key.
- `production.rs:386-395,428-568`: dokładne wymagane types i fields.
- `production.rs:874-1030,1084-1275`: kardynalność i referencje; wymagany pełny graph nawet dla trzech pól.
- WorldProject v2 decision L13/L38/L40/L44: późniejszy evidence-qualified schema revision przed nowym runtime family path; tags/observations nie zmieniają collision.
- FirstProduction decision §3.1: ordinary compile wyłącznie typed graph, product formula/policy refs; §3.4/§5.2 source parser nie jest wybrany.
- Amendment02 §3: source physical bytes/file count/nesting DEFERRED do decyzji przy production source pipeline; §7: actual immutable manifest -> actual digest -> package provenance -> lock -> pair; raw payload wyłączony z runtime.

Wszystkie relative code paths są pod `apps/game-server/src/content/`. Dokładne immutable source URLs są poniżej.

## Metadane i globalny context

| FirstProduction field | Istniejące source pole / producer | Wymagany nowy meaning lub brak |
| --- | --- | --- |
| package_manifest.package_key/package_revision/semantic_schema_version/licensing_metadata | ManifestDocumentRoot; obecne `package_binding` w project.rs | Reuse exact bytes. `PENDING` obecnego item package nie stanowi genuine native licensing. |
| package_manifest.source_manifest_digest | Actual manifest bytes -> existing sha256/package_binding | Nie istnieje ready native manifest. Qualifier hash actual bytes po source admission; bez placeholder digest. |
| content_lock.revision_digest_token/entries | Existing LockDocument i content_lock_binding | Reuse exact one-package matching resolved lock; nie invented `lock:r1`. |
| world_id | ReferenceDocument.world_id i wybrany ProjectV2World.world_id | Same actual owner-assigned World; reject mismatch. Local examples mają null: brak przypisania, nie nowy WorldId. |
| revisions.content/map/ruleset/world_policy/compiler/canonicalization/sim_profile/profile_revision | World placements tylko map_revision; root project_revision nie jest automatycznie pełnym RevisionSet | Overlay.revisions ma wszystkie osiem named values; map musi równać się placement map revision. Source/content/ruleset/policy/compiler/SIM bindings wymagają realnych accepted refs, nie test_source strings. |
| capability_profile | Brak swobodnego source selector | Native consumer używa istniejącego FIRST_PRODUCTION_CAPABILITY_PROFILE, bez nowego profile ID. |
| migration_class | Brak source field | Profil dopuszcza wyłącznie istniejące CompatibleNoMigration; inne przypadki refused. |
| qualified frame contract/version/axes/origin | V2 World ma tylko coordinate_frame, bounds, floors | **Named gap F1:** brak authored origin/explicit canonical-contract binding. Overlay.frame zapisuje exact profile+revision, frame name i origin oraz canonical directions; World supplies bounds/floors. Existing string equality nie zastępuje źródłowego deklarowania frame. |

## Wszystkie obowiązkowe record fields

| Typed record / wszystkie fields | Existing v2 source fit | Named gap / exact proposed overlay mapping |
| --- | --- | --- |
| Region: key | Brak Region family/type | **G1** overlay.region.key; nie Area.parent/World key relabel. |
| Area: key | ProjectV2Declaration::Area.identity.key | Exact sole Area; parent/candidate fields nie mają physics meaning. |
| Terrain: key | ProjectReferenceRecord::Generic identity.family=Terrain, identity.key | Exact sole Terrain. Generic record nie niesie collision. |
| Cell: key | Placement.key | Wybrane trzy Terrain placements -> three cell identities; ta interpretacja wymaga native semantic acceptance. |
| Cell: region_key | Brak | **G2** overlay.cells[].region_key -> sole Region. |
| Cell: area_key | Placement.area.key; family Area, exact revision | Native consumer wymaga Some exact sole Area; nie defaultuje przy None. |
| Cell: terrain_key | Placement.definition.key/family Terrain/revision | Wszystkie wskazują sole Terrain. Nie konwertować WorldObject/LocalObject placement w cell. |
| Cell: x/y/z | Placement.x/y/floor | Exact integer copy; checked spatial membership/unique cell coordinate; source map/frame exact match. |
| Cell: collision | Brak w World/Placement/Terrain | **G3** overlay.cells[].collision, typed existing Walkable/Blocked; nie key suffix, asset id/tag/order/candidate-field selector. |
| Relocation: key/from_cell/to_cell | V2 Transition istnieje tylko candidate fields; brak endpoints | **G4** overlay.relocation exact cell keys; endpoints istnieją w tych trzech cells/same scope. Nie wybiera native runtime transition behavior ani teleport protocol. |
| Behavior: key/policy_revision | Generic Behavior.identity.key; brak policy_revision | **G5** overlay.behavior.definition exact ref + policy_revision. No fixture/test marker; current accepted product policy requirement pozostaje. |
| Presentation: key/metadata_token (3) | Generic Presentation.identity.key; projection/appearance asset binding to inne typy | **G6** overlay.presentations[3].definition + metadata_token. Asset digest/string nie jest automatycznie metadata token. |
| Creature: key/behavior_key/presentation_key/policy_revision | Reference Creature identity/behavior/presentation | **G7** overlay.creature.definition + policy_revision. Existing refs copy exact; local example loot=None. No implied AI activation. |
| Spawn: key/creature_key/behavior_key/cell_key/population_limit/recovery/multiplicity/eligibility_scope | Brak typed spawn occurrence. Creature fields/Encounter source-only overlay nie są tym rekordem | **G8** overlay.spawn dokładnie wszystkie osiem fields; references exact. population_limit=1 to accepted profile shape, enum applicability ma owning Game/Channel/Recovery proof. Null w przykładzie nie ma defaultu. |
| FormulaProfile: key | Reference Formula.identity.key | Source identity bez formuły matematycznej. Binding do accepted product formula pozostaje wymagany; proposed key nie jest takim dowodem. |
| Effect: key/family/formula_profile_key | Reference Effect.identity/effect_family/formula | Native subset accepts existing Damage tylko; Heal refused. Exact Formula ref. Nie tworzy matematyki. |
| Ability: key/effect_key/presentation_key | Reference Ability.identity/effects; brak presentation | **G9** exactly one existing Effect ref (nie wybór first-of-many) + overlay.ability.definition/presentation. |
| Item: key/presentation_key/materializable | Reference Item.identity/materializable; brak Presentation ref | **G10** overlay.item.definition/presentation; materializable=true existing firstprod shape. Nie promuje niekwalifikowanego imported item albo invent materialization rights. |
| LootTable: key/entries[1]; LootEntry: key/item_key/rng_purpose_key | Reference Loot ma inne algorithm/count/probability shape; brak entry key i RNG-purpose | **G11** overlay.loot_table(key,entry(key,item exact ref,rng_purpose_key)). Nie reinterpretować Reference Loot algorithm/probability ani wybierać nowej loot formula. |
| XpDefinition: key/formula_profile_key | Brak XP source family/type | **G12** overlay.xp.key/formula exact ref; nie Creature.experience ani arbitrary candidate field. |
| RngContext: profile_revision/purpose_keys[1] | Brak RNG profile/purpose source; SourceId/provenance nie są RNG | **G13** overlay.rng.profile_revision/purpose_key; purpose matches sole LootEntry; no seed/entropy/secret/mutable RNG. |

Powyższe wylicza każdy wymagany field i exact mapping nowego wybranego wariantu; **unchanged v2 fit remains incomplete**. Nie dowodzi przyjętych authored values ani product qualification. 21 definitions przy trzech cells oraz reszta exact profile cardinality wynikają z istniejącego profilu, nie nowych caps.

## Dokładny nowy typed overlay

`native_first_entry` jest optional wyłącznie w nowym versioned declarations shape; native consumer wymaga jego obecności. Old declarations/v2 pozostaje unchanged. Wszystkie poniższe fields są mandatory podczas native qualification:

- world_key; frame(coordinate_profile, contract_revision, coordinate_frame, origin(x,y,floor), x_direction, y_direction, higher_floor).
- revisions(content,map,ruleset,world_policy,compiler,canonicalization,sim_profile,profile_revision).
- region(key).
- cells[3](placement_key,region_key,collision).
- relocation(key,from_cell,to_cell).
- behavior(definition exact typed ref,policy_revision).
- presentations[3](definition exact typed ref,metadata_token).
- creature(definition exact typed ref,policy_revision).
- spawn(key,creature exact ref,behavior exact ref,cell_key,population_limit,recovery,multiplicity,eligibility_scope).
- ability(definition exact ref,presentation exact ref).
- item(definition exact ref,presentation exact ref).
- loot_table(key,entry(key,item exact ref,rng_purpose_key)).
- xp(key,formula exact ref).
- rng(profile_revision,purpose_key).

Typed enums używają istniejących domen; nie String->arbitrary callback. Nieznane fields/enum variants/extra native graph families refused. Wszędzie exact reference key+revision; key-only lookup jest forbidden. Profile cardinality jest validator gate, nie ogólny import API.

V2 source records są nadal source-only w default consumer. Native qualifier ma osobny explicit API i wymaga native overlay plus exact project/package/world. Nie łączy Reference artifact z first-production artifact; kompiluje własny typed FirstProductionContentSource.

## Lokalne konkretne bytes

- `worlds.world.proposed.json`: istniejący Worlds/v2 shape, trzy explicit Terrain placements i authored envelope 0<=x<2, -1<=y<1, floor set [0].
- `reference.proposed.json`: istniejące typed Reference source fields dla Terrain/Behavior/Presentation/Creature/Formula/Effect/Ability/Item; nie wynik Reference artifact.
- `project.proposed.json`: exact nowy root source-profile selector, existing root physical shape z wszystkimi named fields; nieobliczone manifest/lock SHA fields są null. To propozycja nowego wariantu admission, nie existing parser capability.
- `declarations.proposed.json`: existing Area declaration + exact nowy declarations schema i powyższy **proponowany** native overlay, wraz z per-cell collision.

Coordinates: start(0,0,0) walkable; east(1,0,0) walkable; north(0,-1,0) blocked. cell key -> matching placement jest jawny w overlay; nie inference z nazwy.

**Null fields są świadomym refusal boundary**: rzeczywisty WorldId, mandatory root manifest_sha256/content_lock_sha256, product policy/revision refs, three presentation metadata values i spawn/RNG applicability nie zostały wymyślone. Proposed source-local names nie są public runtime IDs ani accepted product formula/policy identities. Przykłady nie są admitted project, gotowym manifestem, qualified frame ani compile evidence. Dzisiejszy parser musi je odrzucić za nowy profile/schema oraz null mandatory values; przyszła native ścieżka także musi odmówić przy null/missing actual digest/binding. Zastąpienie null realnymi accepted wartościami, rzeczywiste licensing, remaining managed documents, canonical writer+manifest+lock i release qualification należą do późniejszego jawnie przydzielonego workera.

Nie obliczono ani nie podano produkcyjnego WorldId/package digest/current activation. Nie ma nowych formula expressions, timings ani resource maxima.

## Source-to-frame proof po przyjęciu

Native parser czyta explicit frame declaration z tych samych admitted source bytes. Source manifest wiąże declarations.json i world.json digest/count/schema; existing source-manifest/package provenance/lock chain wiąże te bytes do pair/generation. Qualifier checks explicit directions/profile/revision, origin, World/bounds/floors i exact same map/cells before compile/stage. To dowód do wytworzenia i independently verify, nie obecny dowód.

Runtime nie otrzymuje raw JSON, file parsera ani manifest payloadu; tylko bounded typed genuinely qualified binding i istniejący validated pair. Same source bytes po qualification nie przyznają current activation/Channel pin/position write. Activation issuer i first entry mają osobne gates #935.

## Immutable exact-revision links

- [WorldProject v2 executable boundary](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md#L13).
- [World/Placement actual fields](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/apps/game-server/src/content/project/v2.rs#L1087).
- [Declarations document closed shape](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/apps/game-server/src/content/project/v2.rs#L1244).
- [Reference typed fields](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/apps/game-server/src/content/project.rs#L551).
- [FirstProduction complete typed graph](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/apps/game-server/src/content/production.rs#L428).
- [Profile scope/graph/producer boundary](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md#L46).
- [Amendment02 source-size trigger/runtime exclusion](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_02.md#L133).
- [Canonical native spatial semantics](https://github.com/Oteryn/Oteryn-Game/blob/91fb3135a8a67d012bfe048b230f3501780d244b/docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md).

## Admission discriminators, flags and exact joins

The owning amendment selects the exact native root profile before parse/lowering, with its exact new declarations schema. Default root-schema=v2 dispatch is insufficient. Required and optional manifest feature arrays are both empty; no flag supplies native semantics. Existing root manifest_sha256/content_lock_sha256 and exact root/package/lock revisions remain mandatory.

The source manifest covers one document at each exact owning role/locator; declaration/world schema equality, byte counts and actual SHA-256 are independently checked. Native graph joins use exact family/key/revision in the captured typed identity set; key-only/latest/alias lookups are refused. All native production keys must also satisfy existing global FirstProduction uniqueness. Unsupported candidate fields/editor/source-ID data cannot fill a native gap.

The new profile is source-only. Its producer is not the existing default Reference lowering, and none of the examples is a Reference artifact or proof of cross-profile activation. Full source byte/file/depth/work/report/retention applicability remains DEFERRED until accepted bounds before executable worker release. Fixed roles and semantic graph cardinality are not a production physical-source sizing result.

## Retained concrete examples

The following are the exact prepared local JSON shapes. They are not an admitted project or qualification output. Null WorldId/digests and unresolved product policy/classifier values deliberately refuse; no hash/licensing/product formula/policy proof is fabricated. Four documents are shown, not the full eleven-locator source capture. Proposed names are source-local examples, not public wire IDs or accepted product bindings.

### project.json

```json
{
  "schema": "OTERYN_WORLD_PROJECT_ROOT/v2",
  "source_profile": "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-entry-qualification-1",
  "project_revision": "entry-source-r1",
  "manifest_locator": "manifest.json",
  "manifest_sha256": null,
  "content_lock_locator": "content.lock.json",
  "content_lock_sha256": null
}
```

### worlds/world.json

```json
{
  "schema": "OTERYN_WORLD_PROJECT_WORLDS/v2",
  "worlds": [
    {
      "key": "oteryn:entry.world",
      "world_id": null,
      "coordinate_frame": "oteryn:entry.frame",
      "bounds": {
        "min_x": 0,
        "min_y": -1,
        "max_x_exclusive": 2,
        "max_y_exclusive": 1
      },
      "floors": [
        0
      ]
    }
  ],
  "placements": [
    {
      "key": "oteryn:entry.cell.east",
      "world": "oteryn:entry.world",
      "map_revision": "entry-map-r1",
      "definition": {
        "family": "Terrain",
        "key": "oteryn:entry.terrain",
        "revision": "entry-source-r1"
      },
      "area": {
        "family": "Area",
        "key": "oteryn:entry.area",
        "revision": "entry-source-r1"
      },
      "coordinate_frame": "oteryn:entry.frame",
      "x": 1,
      "y": 0,
      "floor": 0,
      "presentation_order": {
        "plane": 0,
        "order": 0
      },
      "disposition": "CandidateOnly"
    },
    {
      "key": "oteryn:entry.cell.north",
      "world": "oteryn:entry.world",
      "map_revision": "entry-map-r1",
      "definition": {
        "family": "Terrain",
        "key": "oteryn:entry.terrain",
        "revision": "entry-source-r1"
      },
      "area": {
        "family": "Area",
        "key": "oteryn:entry.area",
        "revision": "entry-source-r1"
      },
      "coordinate_frame": "oteryn:entry.frame",
      "x": 0,
      "y": -1,
      "floor": 0,
      "presentation_order": {
        "plane": 0,
        "order": 0
      },
      "disposition": "CandidateOnly"
    },
    {
      "key": "oteryn:entry.cell.start",
      "world": "oteryn:entry.world",
      "map_revision": "entry-map-r1",
      "definition": {
        "family": "Terrain",
        "key": "oteryn:entry.terrain",
        "revision": "entry-source-r1"
      },
      "area": {
        "family": "Area",
        "key": "oteryn:entry.area",
        "revision": "entry-source-r1"
      },
      "coordinate_frame": "oteryn:entry.frame",
      "x": 0,
      "y": 0,
      "floor": 0,
      "presentation_order": {
        "plane": 0,
        "order": 0
      },
      "disposition": "CandidateOnly"
    }
  ]
}
```

### definitions/reference.json

```json
{
  "schema": "OTERYN_WORLD_PROJECT_REFERENCE_RECORDS/v1",
  "world_id": null,
  "coordinate_frame": "oteryn:entry.frame",
  "records": [
    {
      "kind": "Ability",
      "identity": {
        "family": "Ability",
        "key": "oteryn:entry.ability",
        "revision": "entry-source-r1"
      },
      "effects": [
        {
          "family": "Effect",
          "key": "oteryn:entry.effect",
          "revision": "entry-source-r1"
        }
      ]
    },
    {
      "kind": "Generic",
      "identity": {
        "family": "Behavior",
        "key": "oteryn:entry.behavior",
        "revision": "entry-source-r1"
      },
      "client_projection": "ServerOnly"
    },
    {
      "kind": "Creature",
      "identity": {
        "family": "Creature",
        "key": "oteryn:entry.creature",
        "revision": "entry-source-r1"
      },
      "client_projection": "ClientSafe",
      "presentation": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.creature",
        "revision": "entry-source-r1"
      },
      "behavior": {
        "family": "Behavior",
        "key": "oteryn:entry.behavior",
        "revision": "entry-source-r1"
      },
      "loot": null
    },
    {
      "kind": "Effect",
      "identity": {
        "family": "Effect",
        "key": "oteryn:entry.effect",
        "revision": "entry-source-r1"
      },
      "client_projection": "ServerOnly",
      "effect_family": "Damage",
      "formula": {
        "family": "Formula",
        "key": "oteryn:entry.formula",
        "revision": "entry-source-r1"
      }
    },
    {
      "kind": "Formula",
      "identity": {
        "family": "Formula",
        "key": "oteryn:entry.formula",
        "revision": "entry-source-r1"
      }
    },
    {
      "kind": "Item",
      "identity": {
        "family": "Item",
        "key": "oteryn:entry.item",
        "revision": "entry-source-r1"
      },
      "client_projection": "ClientSafe",
      "materializable": true,
      "stack_class": "NonStackable"
    },
    {
      "kind": "Generic",
      "identity": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.ability",
        "revision": "entry-source-r1"
      },
      "client_projection": "ClientSafe"
    },
    {
      "kind": "Generic",
      "identity": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.creature",
        "revision": "entry-source-r1"
      },
      "client_projection": "ClientSafe"
    },
    {
      "kind": "Generic",
      "identity": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.item",
        "revision": "entry-source-r1"
      },
      "client_projection": "ClientSafe"
    },
    {
      "kind": "Generic",
      "identity": {
        "family": "Terrain",
        "key": "oteryn:entry.terrain",
        "revision": "entry-source-r1"
      },
      "client_projection": "ServerOnly"
    }
  ]
}
```

### definitions/declarations.json

```json
{
  "schema": "OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1",
  "records": [
    {
      "kind": "Area",
      "identity": {
        "key": "oteryn:entry.area",
        "revision": "entry-source-r1"
      },
      "fields": []
    }
  ],
  "native_first_entry": {
    "world_key": "oteryn:entry.world",
    "frame": {
      "coordinate_profile": "oteryn-world-spatial-v1",
      "contract_revision": 1,
      "coordinate_frame": "oteryn:entry.frame",
      "origin": {
        "x": 0,
        "y": 0,
        "floor": 0
      },
      "x_direction": "East",
      "y_direction": "South",
      "higher_floor": "Up"
    },
    "revisions": {
      "content": "entry-source-r1",
      "map": "entry-map-r1",
      "ruleset": null,
      "world_policy": null,
      "compiler": null,
      "canonicalization": null,
      "sim_profile": null,
      "profile_revision": "FIRST_PRODUCTION_CONTENT_PROFILE/v1"
    },
    "region": {
      "key": "oteryn:entry.region"
    },
    "cells": [
      {
        "placement_key": "oteryn:entry.cell.east",
        "region_key": "oteryn:entry.region",
        "collision": "Walkable"
      },
      {
        "placement_key": "oteryn:entry.cell.north",
        "region_key": "oteryn:entry.region",
        "collision": "Blocked"
      },
      {
        "placement_key": "oteryn:entry.cell.start",
        "region_key": "oteryn:entry.region",
        "collision": "Walkable"
      }
    ],
    "relocation": {
      "key": "oteryn:entry.relocation",
      "from_cell": "oteryn:entry.cell.start",
      "to_cell": "oteryn:entry.cell.east"
    },
    "behavior": {
      "definition": {
        "family": "Behavior",
        "key": "oteryn:entry.behavior",
        "revision": "entry-source-r1"
      },
      "policy_revision": null
    },
    "presentations": [
      {
        "definition": {
          "family": "Presentation",
          "key": "oteryn:entry.presentation.ability",
          "revision": "entry-source-r1"
        },
        "metadata_token": null
      },
      {
        "definition": {
          "family": "Presentation",
          "key": "oteryn:entry.presentation.creature",
          "revision": "entry-source-r1"
        },
        "metadata_token": null
      },
      {
        "definition": {
          "family": "Presentation",
          "key": "oteryn:entry.presentation.item",
          "revision": "entry-source-r1"
        },
        "metadata_token": null
      }
    ],
    "creature": {
      "definition": {
        "family": "Creature",
        "key": "oteryn:entry.creature",
        "revision": "entry-source-r1"
      },
      "policy_revision": null
    },
    "spawn": {
      "key": "oteryn:entry.spawn",
      "creature": {
        "family": "Creature",
        "key": "oteryn:entry.creature",
        "revision": "entry-source-r1"
      },
      "behavior": {
        "family": "Behavior",
        "key": "oteryn:entry.behavior",
        "revision": "entry-source-r1"
      },
      "cell_key": "oteryn:entry.cell.east",
      "population_limit": 1,
      "recovery": null,
      "multiplicity": null,
      "eligibility_scope": null
    },
    "ability": {
      "definition": {
        "family": "Ability",
        "key": "oteryn:entry.ability",
        "revision": "entry-source-r1"
      },
      "presentation": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.ability",
        "revision": "entry-source-r1"
      }
    },
    "item": {
      "definition": {
        "family": "Item",
        "key": "oteryn:entry.item",
        "revision": "entry-source-r1"
      },
      "presentation": {
        "family": "Presentation",
        "key": "oteryn:entry.presentation.item",
        "revision": "entry-source-r1"
      }
    },
    "loot_table": {
      "key": "oteryn:entry.loot.table",
      "entry": {
        "key": "oteryn:entry.loot.entry",
        "item": {
          "family": "Item",
          "key": "oteryn:entry.item",
          "revision": "entry-source-r1"
        },
        "rng_purpose_key": "oteryn:entry.rng.loot"
      }
    },
    "xp": {
      "key": "oteryn:entry.xp",
      "formula": {
        "family": "Formula",
        "key": "oteryn:entry.formula",
        "revision": "entry-source-r1"
      }
    },
    "rng": {
      "profile_revision": null,
      "purpose_key": "oteryn:entry.rng.loot"
    }
  }
}
```

## Evidence classification

- PROVEN: named v2 fields/gaps at the immutable cited revision; exact prepared proposal bytes and successful JSON syntax parsing only.
- DERIVED: one closed source variant can represent each required typed graph field with the explicit joins in the owning amendment.
- UNKNOWN: actual canonical World assignment, licensing/authentic product source values, applicable policies/formulas/revisions/classifiers, accepted physical-source/retention limits, qualified release/frame/pair, activation and playable-control proof.
- No product implementation, runtime/data/registry/workflow write, source parser execution, ordinary compile or performance/capacity measurement was performed by this docs task. Actor registry delta=[]; 131072 remains preproduction config only.

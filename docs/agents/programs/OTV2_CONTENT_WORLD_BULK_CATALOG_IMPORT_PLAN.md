# Content / World — bulk catalogue import plan

```yaml
status: PROPOSED_EXECUTION_PLAN
version: 1.0
date: 2026-09-18
repository: Oteryn/Oteryn-Game
parent_coordination: 162
parent_programme: OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME
canonical_import_alias: "Oteryn: content world import"
canonical_build_alias: "Oteryn: content world build"
parallel_with_cw4: CONDITIONALLY_YES_PATH_DISJOINT_ONLY
implementation_authority: NONE_BY_THIS_DOCUMENT
worker_release: NONE
production_authority: NONE
legal_access_metadata_readiness_gate: NO
```

## 1. Cel

Uczynić szeroki import katalogu Content jawnie zaplanowaną częścią programu CW, zamiast odkładać go do nieokreślonego etapu po CW5/CW6.

Ten plan nie tworzy nowego subsystemu ani nowego konkurencyjnego workera. Reuse istniejących ról jest wiążący:

- CW2 / `Oteryn: content world import` — źródła, mappery, provenance, konflikty, reimport i candidate catalogue;
- CW3 / `Oteryn: content world build` — wspólny typed model, linker, walidacja, compiler/bundle i executable promotion;
- CW4/CW5/CW6 — konsumenci dopuszczonego Content, nie właściciele bulk importu.

Szeroki katalog może rosnąć równolegle z CW4/CW5, jeśli live #162 przydzieli rozłączne ścieżki i wspólny model nie ma jednoczesnego writera.

## 2. Binding boundaries

Zachować istniejące authority i semantykę:

- ADR-0005 / DUR-04: native Content/World, stabilne identity, immutable generations, staged activation;
- GAME-ITEM-01 / DUR-03: ItemType/ItemInstance, legalność, custody/value/anti-duplication;
- GAME-AI / GAME-ABILITY / GAME-INTERACTION i właściwi ownerzy runtime: Content dostarcza definitions/bindings, nie przejmuje wykonania;
- Reference target/evidence: candidate data może być szersze niż executable Reference promotion;
- legacy numeric IDs, OTS IDs, file paths i nazwy display są provenance/mapping inputs, nie kanoniczną identity Oteryn.

Nie budować drugiego parsera/mapera/modelu, jeśli istniejący Game-owned lineage można rozszerzyć wąsko.

## 3. Strumienie katalogowe

### B0 — live inventory i source admission

Przed każdym mutującym batchem:

- fresh-read protected main, #162, #504 i aktualne open-PR custody;
- zinwentaryzować istniejące extractory/products zamiast powtarzać research;
- przypiąć source revisions/digests i zweryfikować techniczną dostępność; status licensing/access zachować informacyjnie, ale nie używać go jako readiness gate;
- określić exact write custody;
- wskazać, czy batch jest candidate-only czy może zawierać executable closure.

B0 nie otwiera ponownie Newhaven/Targuna research i nie wymaga ukończenia CW4.

### B1 — items / native identity

Pierwszy szeroki batch itemów ma zbudować stabilne mapowanie źródeł do natywnych `ContentKey` / ItemType definitions, używając istniejącego GAME-ITEM modelu.

Zakres pól według dostępnego, kwalifikowanego źródła:

- identity + provenance;
- presentation/appearance binding;
- physical/materializable classification;
- stack/charges/temporal/decay;
- container;
- equipment slot claims i requirements;
- weapon/use;
- protection/modifiers/resists;
- binding/transfer restrictions;
- imbuement slots + allowed family/tier semantics;
- light/readable/fluid/field/attachment/wrap/rotate;
- materialization/pickup eligibility.

Nie zgadywać brakujących unitów, limitów, statów, slotów lub target-sensitive wartości. Unknown pozostaje jawne.

### B2 — creatures + spawns

Reuse istniejącego `tools/game-atlas-creatures/**` i `tools/game-atlas-creature-gameplay/**`.

Batch ma wiązać candidate creature identity do natywnego Content modelu i rozliczać osobno:

- definition/profile identity;
- presentation;
- stats/HP/speed;
- behavior/targeting;
- abilities/attacks;
- armor/defense;
- resistances/immunities;
- corpse;
- XP/reward references;
- world spawn occurrences i placement identity;
- provenance/evidence classification.

Existing source corpus counts są evidence wejściowym, nie automatycznym production promotion.

### B3 — loot -> native item bindings

Zamknąć obecny istotny gap pomiędzy creature loot rows a natywnymi itemami:

```text
creature loot candidate
  -> stable source item identity/mapping
  -> native ItemType/ContentKey
  -> typed LootEntryDefinition
  -> explicit unresolved/conflict report
```

Legacy/client/display IDs nie mogą być hashowane lub przemianowane w fake stable ItemKey.

Batch raportuje co najmniej: total rows, resolved native bindings, unresolved, ambiguous, conflicting, excluded by evidence/policy oraz selected executable closure.

### B4 — abilities/effects/formulas

Mapować definicje i bindings do istniejących ownerów GAME-ABILITY/SIM bez kopiowania runtime logiki do Content.

### B5 — NPC/services/dialogues/shops/travel

Reuse istniejącej granicy NPC. Oddzielać definition, placement, dialogue, service/catalogue, travel i price-rule candidates. Rozmowa i katalog nie są authority dla trwałej transakcji.

### B6 — quests/interactions/remaining families

Importować semantic candidates i jawne grafy/bindings. Nie promować legacy `storage[key]=integer` ani Lua runtime jako kanonicznego modelu Oteryn.

## 3A. Operational owner decision — licensing/access is non-blocking

Dla CW2/CW3 status prawny/licencyjny/redystrybucyjny źródła nie jest technicznym kryterium dopuszczenia batcha. Importer ma zachować exact provenance i zweryfikować, że źródło jest faktycznie dostępne i deterministycznie przypięte, ale `PENDING` / `NOT_CLEARED` w metadanych legal/access nie blokuje katalogowania, native binding, typed promotion ani późniejszej ścieżki build/runtime.

Nie wolno używać tego wyjątku do obchodzenia jakości danych: evidence strength, identity ambiguity/conflict, unsupported/loss, target-sensitive unknowns oraz wymagania bezpieczeństwa/runtime pozostają obowiązujące.

## 4. Batch acceptance contract

Każdy batch CW2 musi dostarczyć:

1. exact source snapshot i digests/revisions;
2. mapper revision;
3. field/source -> native binding map;
4. deterministic counts;
5. loss/unsupported/unknown/ambiguous/conflict report;
6. provenance/evidence classification;
7. deterministic repeat/reimport result;
8. selected closure: executable / candidate-only / blocked + dokładny powód;
9. żadnego runtime/PG/parity claim tylko na podstawie ekstrakcji.

CW3 konsumuje tylko batch z wystarczającym typed closure dla konkretnego release. Niekompletny szeroki katalog nie blokuje małego zamkniętego release.

## 5. Równoległość z CW4

Bulk catalogue nie powinien czekać na CW5/CW6.

Dozwolona organizacja po live allocation:

```text
CW4 world runtime writer
        ||
        || path-disjoint
        \/
CW2 bulk catalogue writer
        ->
CW3 typed promotion/build when shared model custody is free
```

Zakazane:

- CW2 i CW3 jednocześnie mutują ten sam model/schema;
- CW2 zmienia CW4 runtime;
- CW4 tworzy własne item/creature definitions poza Content;
- dwa osobne item/creature catalogues;
- szeroki import osłabia provenance/digest/evidence fences.

## 6. Pierwsza rekomendowana fala

Po świeżym #162 preflight pierwsze mutujące dzieci powinny być małe i serializowane tam, gdzie dotykają wspólnego modelu:

1. `CW2-B1 item identity/catalog source batch`;
2. `CW3-B1 minimal typed item semantic delta` tylko dla zaakceptowanych pól wymaganych przez pierwszy durable item + katalogową kompatybilność;
3. `CW2-B2 creature/spawn native binding batch`;
4. `CW2-B3 loot -> native ItemKey binding batch`;
5. `CW3-B2/B3 executable closure` dla pierwszego realnego journey.

Dalsze B4-B6 mogą postępować partiami według grywalnej potrzeby.

## 7. Alias i uruchamianie

Nie tworzyć nowego aliasu.

Canonical alias szerokiego importu:

```text
Oteryn: content world import
```

Ten alias najpierw wykonuje live preflight. Bez dokładnej #162 allocation pozostaje read-only i zwraca najmniejszą propozycję batcha/custody.

Wspólny model/build po zaakceptowanym batchu:

```text
Oteryn: content world build
```

## 8. Handoff

Ten dokument jest planem wykonawczym, nie przydziałem zapisu.

```text
CONTROL_PLANE_ACTION: Oteryn: work coordinator — po fresh-read #162 przydzielić pierwszy path-disjoint CW2 bulk-catalog batch
NEXT_WORKER: Oteryn: content world import
RUN_WORKER_WHEN: istnieje live allocation z exact source/batch/write custody i brak overlap z aktywnymi writerami
WHY: CW2 jest istniejącym właścicielem źródeł, mapperów i rozszerzania katalogu; nowy alias/system byłby duplikacją
```

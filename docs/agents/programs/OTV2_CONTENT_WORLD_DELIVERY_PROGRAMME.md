# Content / World — architektura wdrożenia i podział wykonawczy

```yaml
status: PROPOSED_EXECUTION_PLAN
version: 1.0
date: 2026-09-17
repository: Oteryn/Oteryn-Game
parent_coordination: 162
retained_design_pr: 641
inspected_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
inspected_design_head: 40fcb356d7a24b3e4c2b6e33c693b078072d9a5c
implementation_authority: NONE_BY_THIS_DOCUMENT
worker_release: NONE
format_acceptance: NONE_BY_THIS_DOCUMENT
production_authority: NONE
```

## 1. Cel i zakres

Dostarczyć natywny Content/World na rzeczywistej ścieżce Oteryn: źródła → projekt → kompilator → pakiety serwera/klienta → mapa i interakcje → właściwe domeny gameplay/trwałości → reconnect/restart. Rozwijać ten sam model do pełnego katalogu i Studio. Nie budować mini-serwera, zastępczego loadera, fake persistence ani świata wymagającego później przepisania.

Ten plan rozpisuje wykonanie [syntezy właściciela](../evidence/OTV2-20260917-content-world-owner-synthesis-and-implementation-plan.md), nie zastępuje jej ani przyjętych kontraktów. Pełna architektura pozostaje. Ograniczamy niepotrzebne customowe biblioteki, forki i infrastrukturę, nie potrzebne funkcje lub inwarianty. Nie kopiować błędnej, wycofanej interpretacji „uprościć = okroić produkt”.

Etapy i CW0–CW6 to podział odpowiedzialności, nie siedem nowych zespołów/usług ani nowe globalne bramy. Istniejący control plane #162 zachowuje alokacje i integrację. Lead techniczny nie może sam udzielać praw zapisu. Reusable alias nie uruchamia workera i nie ustanawia lease.

## 2. Podstawa i aktualność

Podstawa źródłowa na inspected main: [ADR-0005](../../architecture/ADR-0005-native-world-format-and-oteryn-studio.md), [DUR-04](../../architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md), [FND-02](../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md), [FND-03](../../architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md), [GAME-ITEM-01](../../architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md), [playable-first/upstream-first](../../repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md), [BUILD_TEST_MATRIX](../BUILD_TEST_MATRIX.md).

Przy użyciu odczytać ich aktualną authority oraz tylko istotne live locators: #162, #641, #504 Content successor, #64/#511 źródła świata, #483/#486 Reference, #139 Movement, #508 actor/Ability resolution, #513 DUR-03, #500 NPC i #247 Server Seam. Numery są lokalizatorami; nie oznaczają bieżącego statusu ani konieczności czekania na wszystkie jednocześnie. Najnowsze komentarze/alokacje mają znaczenie; nie używać samego historycznego opisu issue.

Zachowany [audyt C01–C36](../evidence/OTV2-20260917-content-world-coverage-audit.md) określa coverage i testy przy właściwych triggerach, nie uniwersalne blockery. [Local-transition revision 2](../evidence/OTV2-20260917-content-world-local-transition-contract-candidate.md) pozostaje kandydatem, nie gotowym adapterem. Green dokumentacyjnego PR nie jest akceptacją formatu lub dowodem gameplay.

## 3. Architektura komponentów i interfejsów

```text
przypięte zewnętrzne źródła / własne dane
  → istniejące ekstraktory i dekodery Game
  → kandydaci + pochodzenie + jawne straty/konflikty
  → typowany model + edytowalny World Project
  → linker + walidacja + zamknięcie zależności wybranego wydania
  → obecny kompilator Content z jawnym profilem następczym
      → server-authoritative bundle
      → client-safe bundle
  → istniejące staging / kontrolowana aktywacja
      → current ChannelRuntime / InstanceRuntime
      → normalna projekcja klienta
      → domenowe operacje GAME-ITEM / Character / DUR
Studio → ten sam model, walidacja, zapis i kompilator
Atlas ← Game-owned publiczna projekcja, nigdy odwrotna authority
```

Nazwy interfejsów poniżej opisują odpowiedzialność, nie nakaz nowych crates, usług ani API o dokładnie tych nazwach.

| Granica | Dane / odpowiedzialność | Dostawca → odbiorca | Warunek poprawnej kompozycji |
|---|---|---|---|
| Source snapshot | Rewizje/digests wszystkich użytych plików, techniczna dostępność, provenance i profil importu. | CW2 → CW1/CW3 | Spójny input, bez zmiennego latest i ukrytych sidecarów. |
| Typed source | ContentKey, PlacementKey, rodzina, jednostki, stany, footprinty, odniesienia, dispositions. | CW1 kontrakt; CW3 wspólny model; CW2 mappery | Jeden model; mapper nie dubluje schema/runtime i nie zgaduje braków. |
| Release closure | Wybrane korzenie + rzeczywiście wymagane definicje, pola, capabilities, evidence i zasoby. | CW2/CW3 → CW3 | Niekompletny niezależny katalog nie blokuje zamkniętej partii. |
| Artifact pair | Indeksowane sekcje/chunki, integralność, kompatybilność, allowlist klienta. | CW3 → CW4/CW5 | Czytelnicy i kompilator używają przyjętego schematu; brak drugiego loadera. |
| World view / mutation | Niezmienna baza + overlay aktualnego ownera, typed intent i footprint delta. | CW3 → CW4 / Movement | Plik nie wybiera ownera; jedna mutacja stanu i jego wkładów przestrzennych. |
| Observation | Rejestrowany command/result/delta/snapshot i jawny kontekst widoku. | CW4 → CW5 | Wynik komendy nie cofa nowszego świata; FND order/replay/egress obowiązują. |
| Value/progression | Rozwiązana definicja i przyczyna operacji, nie bezpośrednie zapisy. | CW4 → obecni ownerzy domenowi | GAME-ITEM/Character/DUR zachowują legalność, conservation, fencing i recovery. |
| Authoring | Typowane operacje edycji, partial/atomic save i preview. | CW5 Studio → CW3/CW2 | Brak drugiego zestawu reguł i importerów w edytorze. |

### Projekt i mapa

Zachować układ źródeł opisany w syntezie: manifest projektu i lock, definicje rodzin, worlds/regions/areas/zones/shards, pola i placementy, spawny, przejścia, provenance oraz metadata autora. JSON/JSONL są kandydatami, nie nowym językiem lub decyzją przyjętą przez ten plan.

Pole określa istnienie i teren; placement określa pozycję raz. Kompilator wyprowadza indeks pola, nie dwie ręcznie edytowane prawdy. Tożsamość nie zależy od ścieżki ani adresu chunka. Jeden obiekt wielopolowy ma jeden placement; visual/collision/interaction/support footprints są oddzielne. Missing data nie jest void ani walkable empty. Kolejność prezentacji nie rozstrzyga automatycznie look/use/move/Browse Field.

Source shard, compiled chunk i runtime sector pozostają niezależnymi podziałami. Area/Subarea/Zone/encounter geography nie zależą od technicznego chunkowania. Rozmiary, pakowanie pięter i codec zamknąć przez właściwą decyzję na realnym korpusie, bez narzucania historycznego bootstrap 32x32 jako limitu produktu.

### Istniejące komponenty i technologie

Rozwijać `apps/game-server/src/content/` oraz istniejący lineage `tools/game-atlas-fullworld-source/`, `tools/game-atlas-appearances/`, `tools/game-atlas-creatures/`, `tools/game-atlas-creature-gameplay/`, `tools/reference-world-corridor-census/`. Najpierw sprawdzić actual API i input restrictions. Eksport wizualny nie jest lossless gameplay IR. Nowy zaakceptowany input wymaga jawnego rozszerzenia kwalifikacji, nie wyłączenia kontroli digestów.

Przeczytać wyniki `tools/content-format-spike/` i #95; nie wskrzeszać zakończonego workera/spike. FlatBuffers/Zstandard pozostają kandydatami upstream. Sprawdzić rozwiązanie istniejące i najprostszy realny wariant, nie benchmarkować wszystkich serializerów. Własna semantyka gry jest potrzebna; własny JSON parser, kompresor, VM, allocator, database layer, broker lub fork biblioteki wymaga wykazanego braku w konkretnej wersji/API. Wymagania poprawności i bezpieczeństwa nie czekają na benchmark; hipotezy optymalizacyjne nie uzasadniają forków.

## 4. Role i granice zapisów

| Rola | Dostawa | Kandydacka powierzchnia, zawsze zawężana przez live allocation |
|---|---|---|
| CW0 lead | Aktualny plan zależności, reuse istniejących workers, dokładny następny przyrost i handoff. | Tylko przydzielone dokumenty planowania; bez grantów lease, runtime i integracji. |
| CW1 architecture | Pozostałe decyzje source/bindings, formatu i kompatybilności; nie ponowny audyt całej architektury. | Przydzielone docs/architecture, dowody istniejącego spike; brak produkcyjnej implementacji. |
| CW2 import | Przypięte partie danych, mappery, provenance, konflikty i reimport. | Przydzielone ekstraktory/mappery i katalog źródłowy; nie compiler/runtime core. |
| CW3 build | Wspólny typowany model, schema validation, linker, compiler, indexed artifacts, loader i aktywacja. | Przydzielone obecne Content i wspólne schema/codecs; root Cargo/registry tylko osobna jawna custody. |
| CW4 runtime | Operacje obiektów, world view i kompozycja owner/FND/Movement; adaptery do istniejących domen. | Przydzielone runtime/object paths; nie przejęcie całych Foundation, Movement, Ability, DUR lub Server Seam. |
| CW5 client | Normalna projekcja mapy, interakcje i reconciliation; następnie moduły Studio na wspólnym modelu. | Przydzielone client/authoring paths, bez drugiego renderer/runtime i bez konfliktu Native UI. |
| CW6 qa | Testy integracyjne i wynik prawdziwej gry; kwalifikacja ograniczeń i recovery. | Przydzielone tests/fixtures/evidence; naprawy produkcyjne wracają do ownera. Nie formalny independent reviewer. |

Wskazane ścieżki nie są lease. Wspólne Cargo/lock, publiczne schematy, protocol registry, resource registry i shared composition muszą mieć jednego jawnego writera. CW2 nie tworzy alternatywnych typów, gdy CW3 zmienia model. CW4/CW5 uzgadniają jeden payload kontrakt przez właściwego ownera; nie dwa osobne formaty.

Jeżeli canonical worker już realizuje ten przyrost, otrzymuje właściwy prompt jako delta i zachowuje branch/PR/custody. Nie uruchamiać obok niego nowego writera pod inną nazwą. Obecne aliasy Movement, Combat, Ability, Character, DUR, Server Seam i Native UI pozostają właścicielami swoich prac. Nowy pakiet nie superseduje ich lifecycle.

## 5. Przyrosty i zależności

Wszystkie zapisy poniżej wymagają live przydziału właściwego zakresu. Sam start badania nie wymaga kompletnego runtime; uruchomienie produktu wymaga jego rzeczywistych zależności.

| Przyrost | Wynik końcowy | Właściciel | Rzeczywisty start / następca |
|---|---|---|---|
| D0 — preflight | Reuse map + jedna propozycja następnej alokacji, rozdzielenie CURRENT/COMPOSED/LATER/UNKNOWN. | CW0 | Może zacząć od odczytu; mutacje wyłącznie dokumentacyjne pod istniejącą authority. |
| D1 — źródła i decyzja modelu | Przypięta partia, wymagane rodziny i dokładna delta #504; uzgodnione interface boundaries. | CW1 + CW2 evidence | Równolegle, po rozdzieleniu ścieżek; nie wymagają skończonego Server Seam. |
| D2 — wspólny model i import | Przyjęty model w obecnej ścieżce + rzeczywista partia danych + jawny raport strat. | CW3 + CW2 | Model/contract potrzebny mapperowi musi być zaakceptowany/dostępny w dozwolonej kompozycji; schema ma jednego writera. |
| D3 — format/bundle | Wybrany fizyczny format po wymaganym sprawdzianie; jawny successor, loader i coherent activation. | CW1 decyzja + CW3 wykonanie | Korpus D2 i właściwa akceptacja formatu/zasobów; nie zmieniać interpretacji bootstrap v1. |
| D4 — obiekt przez runtime i klienta | Rzeczywisty current-owner commit + normalny klient, komendy i obserwacja. | CW4 + CW5 | D2/D3 w potrzebnym zakresie, accepted owner seam i gameplay wire registration, rzeczywiste zależności #139/#508/#247. |
| D5 — grywalny journey | Wybrane movement/combat/attack/heal/creature/loot/XP/item/NPC/death/recovery działają razem. | Obecni ownerzy + CW6 | Rzeczywiste domenowe zależności i dane wymagane przez wybrany journey; nie cała lista przyszłych mechanik. |
| D6 — rozszerzanie | Większy katalog/mapa oraz Studio, na tych samych typach i ścieżce runtime. | CW2/CW5 + właściwi ownerzy | Kolejne partie mogą postępować równolegle z D3–D5; executable promotion tylko po ich własnym domknięciu. |

```text
CW0 preflight
  ├─ CW1 decyzje ─────────────┐
  └─ CW2 źródła / kandydaci ──┤
                             ▼
                   CW3 model + bundle
                     │           │
                     ▼           ▼
                 CW4 runtime   CW5 klient
                     └─────┬─────┘
          obecni ownerzy gameplay / DUR / Server Seam
                           ▼
                    CW6 realne E2E

CW2 rozszerza katalog równolegle;
CW5 rozwija Studio, gdy wspólny model i przydział są gotowe.
CW6 projektuje i dodaje testy wraz z komponentami, nie dopiero po całości.
```

DAG nie narzuca oczekiwania na kompletne D3 przed każdym component testem CW4. Nie wolno jednak przedstawiać bezsieciowego testu albo fixture codec jako końcowej implementacji gry/formatu. Podobnie nie wymaga pełnych questów/economy dla zwykłego obiektu; wybrany quest door lub trade musi użyć rzeczywistych zależności.

## 5A. Równoległy bulk catalogue CW2/CW3

Szeroki import itemów, creatures, spawns, loot, abilities, NPC/services i kolejnych rodzin jest jawną częścią programu CW, a nie pracą odkładaną do momentu po CW5/CW6.

Szczegółowy batching i granice są w [OTV2_CONTENT_WORLD_BULK_CATALOG_IMPORT_PLAN.md](OTV2_CONTENT_WORLD_BULK_CATALOG_IMPORT_PLAN.md).

Zasada wykonawcza:

- CW2 (`Oteryn: content world import`) rozszerza candidate catalogue partiami i dostarcza provenance/mapping/loss report;
- CW3 (`Oteryn: content world build`) jest jedynym writerem wspólnego modelu/compiler path dla executable promotion;
- zakończony bootstrap single-item służy jako evidence/regression; **normalna dalsza native/executable promotion jest batchowa**, nie rekord-po-rekordzie;
- pojedynczy rekord wolno przydzielić tylko jako fixture/regression/diagnostykę albo gdy świeży, konkretny blocker uniemożliwia bezpieczny batch; wyjątek musi nazwać blocker i warunek przejścia do partii oraz nie liczy się jako postęp katalogowy;
- control plane nie powinien serializować kolejnych pojedynczych itemów/creatures/NPC, gdy dana rodzina jest technicznie i semantycznie gotowa do ograniczonego reprezentatywnego batcha lub całej partycji;
- CW2 może działać równolegle z CW4/CW5 tylko przy live #162 allocation i faktycznie rozłącznych ścieżkach;
- D6 oznacza kontynuację skali katalogu/mapy i Studio, **nie pierwszy moment rozpoczęcia bulk importu**;
- nie uruchamiać osobnego `content catalog` subsystemu lub konkurencyjnego aliasu.

Pierwsza preferowana fala katalogowa to item identity/catalog -> creature/spawn binding -> loot-to-native-ItemKey binding, następnie abilities/NPC/quests według potrzeb grywalnego journey.

## 6. Przenoszenie zawartości do Oteryn

Korzystać z istniejącego [source registry #486](OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md), nie nowej hierarchii wymyślonej na potrzeby tego planu. `tibiawiki.com.br` jest first-class structured bulk source; Tibiopedia/inne rzeczywiście niezależne źródło służy kontroli. CipSoft/oficjalne dane i kontrolowane obserwacje rozstrzygają odpowiednie target-sensitive zachowania. Crystal/Canary/legacy dostarczają aktywnie danych kandydackich, coverage i testów; wspólne pochodzenie nie jest niezależnym potwierdzeniem.

Target pozostaje `global-tibia-observable-2026-07-28-post-server-save`. Wiki-only nie oznacza automatycznie PROVEN, ale nie jest też automatycznie bezużyteczne: używać dopuszczonego DERIVED/continuity procesu. Nie wymagać osobnego black-box eksperymentu dla każdego zwykłego pola statycznego. Konflikt jednej cechy nie unieważnia innych dobrze popartych cech. Brak dostępu do źródła nie pozwala zgadywać wartości.

| Partia | Zawartość | Wymagane rozliczenie |
|---|---|---|
| Świat/presentation | Geometria, appearances, floors, order, strefy i przejścia. | Cały input, transform, footprint, nieznane rekordy, klient/server split. |
| Obiekty/interakcje | Typy, placements, stany, binding/use/read/rotate/Browse Field/transition candidates. | Kolejność źródeł, ambiguity, supported capability i różnica widoku od custody. |
| Itemy | Sloty/requirements/stats, stack/container/equipment/temporal/imbuement/proficiency. | Typed units i kompletne ograniczenia użytej rodziny; instancje/wartość pozostają DUR-owned. |
| Creatures/abilities | Staty, spawny, Damage/Heal, combat/reward/loot/XP references. | Algorytm doboru oddzielony od wartości/probability i od trwałego przyznania. |
| NPC/quests/services | Tożsamość/placement, dialog, katalogi i podróże, warunki/progres kandydacki. | Dane != Lua runtime; rozmowa != trade commit; storage integer != automatycznie stage. |
| Rozszerzenia | Houses/encounters/world changes i reszta właściwych rodzin. | Jawny trigger wdrożenia i kwalifikacji, bez kasowania ich z modelu docelowego. |

Każda partia daje: exact source snapshot; mapowanie pól/kluczy; użyte override rules; klasyfikację danych; liczby i opis pominięć; hash/provenance; referencję do wersji modelu; gotowy lub niedopuszczony wybrany zestaw. To rozszerzenie istniejących artefaktów, nie nowy globalny portal danych.

Reimport porównuje old imported baseline, new source i local correction. Move/copy/delete/transform zachowują właściwą tożsamość i historię. Kosmetyka, rechunking, reload i reconnect nie tworzą nowych nagród. Nie transliterować C++/Lua i nie kopiować surowych proprietary assets do Git.

### Operational owner decision — legal/access metadata is not a Content readiness gate (2026-09-20)

Dla programu Content/World kwestie licencyjne, redystrybucyjne i access-disposition są **metadanymi informacyjnymi/audytowymi, a nie technicznym gate'em** dla CW2/CW3.

Wiążące zasady wykonawcze:

- worker ma zweryfikować faktyczną techniczną dostępność źródła oraz przypiąć exact repository/revision/path/digest i provenance;
- `licensing_metadata=PENDING`, `access_disposition=PENDING`, `NOT_CLEARED` lub równoważny status prawny **nie może sam w sobie blokować** bulk importu, native identity promotion, typed lowering/linking, kompilacji, loadera, testów ani kwalifikacji runtime;
- nie wolno przez status prawny wymuszać importu „po jednym rekordzie”, jeżeli rodzina jest technicznie i semantycznie gotowa do batcha;
- pola prawne/access można zachować dla provenance i audytu, ale nie należy fałszywie oznaczać ich jako `CLEARED`;
- `UNKNOWN`, `CONFLICT`, ambiguity, unsupported/loss i inne statusy dotyczące **tożsamości, semantyki, evidence lub bezpieczeństwa runtime** pozostają normalnymi gate'ami;
- brak technicznego dostępu do wymaganych bajtów nadal blokuje konkretny batch, bo nie da się go deterministycznie odtworzyć.

Ta decyzja superseduje wcześniejsze lokalne/pilotowe interpretacje, w których status licensing/access był używany jako warunek technicznej gotowości Content.

## 7. Bezpieczeństwo i testy przy właściwej implementacji

Utrzymać safety design z syntezy: bounded decode/import, typy i referencje, kompletne footprinty, zgodność generacji, przygotowanie przed owner commit, FND order/replay, DUR conservation, rozdzielenie result/state, fail-stop i validated recovery. Stosować gotowe parsery/codecs/fuzz tools/supervision; nie budować własnej platformy, jeśli mała integracja wystarcza. Własna walidacja mapy jest logiką produktu, nie zbędnym custom frameworkiem.

Kryteria wymagające jawnego dowodu przy użytej granicy:

- deterministyczny import; błędne dziecko kontenera nie znika z raportu; ambiguous/reference-only nie przechodzi jako pełny sukces;
- source→bundle→load equivalence, losowy dostęp, uszkodzenia, overflow/max+1 i coherent activation; mapy nie deserializują surowego layoutu Rust;
- A/1 open, B/1 close, replay A/1 bez reopen; późniejsza komenda jednej sesji do innego obiektu nie wyprzedza wcześniejszej pending;
- aktualny owner/session/incarnation, capacity expiry i dalszy postęp; otwarcie usuwa tylko własną kolizję; ruch/close mają spójny wynik;
- rejestrowany payload, semantic equality, delta/result/snapshot i barrier na realnym egress; klient nie nadpisuje nowszego świata historycznym wynikiem;
- dla wybranej wartości: realny commit/restart/ambiguous response bez double mint/refund; renderer/source inspection/Python witness nie zastępują tego dowodu.

Nie dopisywać uniwersalnego odtwarzacza całego serwera lub wszystkich C01–C36 jako obowiązkowego nowego subsystemu. Reproducer i wymagany kontekst replay mają używać existing QA/ANL/FND. Rozpoznanie defektu nie upoważnia do samowolnego usunięcia obiektu, zmiany przejścia lub stanu gracza. Kwarantanna tylko w zakresie bezpiecznej przyjętej polityki; inaczej istniejący fail-stop.

## 8. Jak unikamy powtórzenia WP3 bez okrajania produktu

Najpierw istniejący Oteryn/upstream, potem supported API/config, mały adapter, dopiero wykazana minimalna poprawka. Fork to wyjątek. Każda materialna zależność/patch ma konkretne wymaganie, dokładną wersję oraz dowód braku; zachować drogę powrotu do upstream. Nie przejmować odpowiedzialności za wnętrze bibliotek na podstawie samej hipotezy.

Bieżący invariant blokuje właściwą implementację; dowód zależny od późniejszej kompozycji ma exact trigger; pomiar skali wymaga reprezentatywnej gry; historyczny mechanizm jest evidence; nierozstrzygnięte pozostaje UNKNOWN. Nie przenosić dawnych liczb ani macierzy jako automatycznych blockerów. Nie wyłączać przyjętego limitu pod pretekstem prostoty.

Nie robić osobnego frameworka dla każdej rodziny danych. Nie wymagać nowego dokumentu akceptacji dla rutynowej decyzji implementacyjnej mieszczącej się w kontrakcie. Materialna zmiana schematu/public API/authority idzie istniejącą ścieżką owning decision. Stop rozszerzania przyrostu: jego produktowy wynik i właściwe testy są spełnione.

## 9. Dostarczanie i zakończenie

[Runbook i aliasy](OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) określają start, maksymalnie trzy sensownie równoległe role i wymagany successor footer. To zalecana obsada, nie potwierdzenie dostępnego limitu narzędzia. Nie uruchomiono agentów przez publikację tego planu.

Dla przyrostu wskazać dokładny kod/dane/PR/head, wykonane testy, current blocker albo wynik oraz następnego rzeczywistego workera. Nie tworzyć plików checkpointu po każdym odczycie. Reuse bieżącego task/issue zamiast drugiego ledger subsystemu.

Przed substantial worker release aktywny control plane ponownie sprawdza faktyczną ścieżkę wykonania i integracji według związanej polityki; historyczne błędy quota/capability nie są wieczne ani automatycznie rozwiązane. Nie obiecywać pracy w tle bez istniejącego mechanizmu wznowienia.

Koniec wybranego etapu to rzeczywisty, zweryfikowany rezultat. Game gate nie zastępuje native-client ani PG evidence, a ukończenie dokumentów nie jest ukończeniem Content. Pełna mapa/katalog/Studio rosną na tej samej architekturze; nie udajemy ich kompletności po jednym obiekcie.

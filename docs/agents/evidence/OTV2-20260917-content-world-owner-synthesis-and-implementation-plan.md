# Content / World — ustalenia właściciela i plan wdrożenia oraz importu

```yaml
status: PROPOSED_IMPLEMENTATION_PLAN
classification: PROPOSED_NONCANONICAL
date: 2026-09-17
repository: Oteryn/Oteryn-Game
pr: 641
branch: agent/content-world-design-dossier-20260917
inspected_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
publication_predecessor: 0682d66d5f526d8119a8e8d5ab215ff120be0f59
document_authority: explicit_owner_request_to_retain_discussion_and_prepare_plan
implementation_authority: NONE
worker_release: NONE
format_acceptance: PROPOSED_NOT_ACCEPTED
resource_registry_mutation_authority: NONE
production_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
```

## 1. Ostateczna dyspozycja właściciela — nie zmniejszać architektury

Właściciel nakazał zachować wcześniej zaprojektowaną pełną architekturę Content/World, a ograniczyć niepotrzebne tworzenie własnej infrastruktury, bibliotek i forków. Ostrzeżenie przed powtórzeniem problemu WP3 nie jest zgodą na protezę, osobny mini-serwer, tymczasową trwałość ani okrojenie potrzebnych możliwości mapy. Odniesienie do WP3 zapisuje intencję właściciela; ten dokument nie jest nowym audytem WP3.

**Zachowujemy docelową architekturę; upraszczamy sposób realizacji.** Własne są semantyka gry, schemat, powiązania i reguły Oteryn. Parser, serializer, kompresja, runtime bibliotek, kryptografia i narzędzia testowe mają korzystać z istniejących rozwiązań, jeżeli spełniają rzeczywiste wymaganie.

Wcześniejsza odpowiedź asystenta redukująca projekt do uproszczonego zestawu plików, usuwająca chunkowanie/indeksy z kierunku i sugerująca zastępowanie docelowych elementów prowizoryczną ścieżką została odrzucona. Nie wolno jej traktować jako specyfikacji wdrożenia. Nie wycofuje się wymaganych zabezpieczeń, Studio jako kierunku ani możliwości rozwoju pełnego Content. Etapy dostarczają części tej samej architektury.

Wcześniejsze rekomendacje JSON/JSONL, FlatBuffers, Zstandard, podziału na chunki i indeksów pozostają kandydatami technicznymi, nie zaakceptowanymi wersjami zależności lub permanentnym formatem. Wymagany sprawdzian formatu nadal obowiązuje. To samo dotyczy propozycji kwarantanny czy automatycznej diagnostyki: zachowujemy problem i oczekiwaną własność, ale nie zakładamy konieczności nowej usługi lub frameworka.

Dokument utrwala całość merytorycznych ustaleń rozmowy wraz z korektą, nie stenogram. Nie zmienia przyjętych kontraktów i nie stanowi samodzielnej alokacji. Istniejący #162 rozstrzyga zakres wykonawczy, aktualne zależności i ownership. Nie otwieramy drugiego programu koordynacyjnego.

## 2. Jak czytać zachowany pakiet

- [Dossier](OTV2-20260917-content-world-design-dossier.md): model świata, źródła, interakcje, itemy, NPC, questy, import i wcześniejsze korekty.
- [Execution design](OTV2-20260917-content-world-execution-design.md): powiązania z kodem i granice wykonania.
- [Local-transition revision 2](OTV2-20260917-content-world-local-transition-contract-candidate.md): konkretny kandydat operacji lokalnego obiektu i źródło modelu Python.
- [Zachowany audyt pokrycia](OTV2-20260917-content-world-coverage-audit.md): 36 obszarów, źródła, testy, ograniczenia dowodów; nie 36 obligatoryjnych blockerów każdego etapu.
- [Dotychczasowa historia](OTV2-20260917-content-world-design-chat-handoff.md) i [wcześniejszy checkpoint](OTV2-20260917-content-world-final-checkpoint.md) pozostają niezmienionymi zapisami poprzednich etapów. Ten plan dopowiada późniejszą korektę właściciela.

Statusy źródeł: `PROVEN` oznacza bezpośredni odczyt lub wskazany wynik testu, `DERIVED` — wniosek, `UNKNOWN` — brak dowodu, `CONFLICT` — nierozstrzygniętą sprzeczność. Rozwiązania poniżej to `RECOMMENDATION`, chyba że wskazują istniejący kontrakt. Odczyt źródła nie jest dowodem wykonania produktu.

## 3. Fundamenty, których nie projektujemy ponownie

Podstawa: chronione `main@b44fefe08f6aaf1b2c1c23dedd92bab0de87146e`.

| Istniejąca podstawa | Zastosowanie w planie |
|---|---|
| [ADR-0005](../../architecture/ADR-0005-native-world-format-and-oteryn-studio.md) | Natywny projekt, model, World Bundle, Studio, warstwy przestrzenne; OTBM tylko na granicy importu. |
| [DUR-04](../../architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md) | Content Lock, deterministyczna kompilacja, staging, projekcje, aktywacja, migracje i granica WIT. |
| [Playable-first policy](../../repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md) | Rzeczywista ścieżka produktu, upstream-first, minimalna uzasadniona zmiana; bez protezy. |
| [Content runtime](../../../apps/game-server/src/content/mod.rs) i [activation.rs](../../../apps/game-server/src/content/activation.rs) | Rozbudowa istniejącego kompilatora/artefaktów/aktywacji, nie równoległy loader. Profil bootstrapowy nie zmienia znaczenia po cichu. |
| [FND-02](../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md), [FND-03](../../architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md), [Foundation](../../../apps/game-server/src/foundation/mod.rs) | Tożsamość i kolejność komend, aktualny owner, replay, obserwacja, zasoby i zatrzymanie po naruszeniu spójności. |
| [GAME-ITEM-01](../../architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md) | Legalność itemów, wzorce wyposażenia, containment, modyfikatory i temporal modes; DUR pozostaje właścicielem trwałych operacji. |
| [Multichannel scope matrix](../../architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md) i [VSL-MOVE](../../architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md) | Publiczny mutable overlay i ruch w obecnym ChannelRuntime/InstanceRuntime, nie w nowej usłudze obiektów. |
| [Istniejący format spike](../../../tools/content-format-spike/README.md) | `chunked-json-tree`, `sqlite-project`, `indexed-zlib-bundle`; prototypy dowodowe, nie permanentny format. Najpierw ocenić istniejące wyniki. |
| [Game-owned fullworld producer](../../../tools/game-atlas-fullworld-source/README.md) | Wykorzystać lineage dekodera i transformację przestrzenną w jej dopuszczonym zakresie. Wizualna projekcja nie odtwarza pełnego gameplay. |

Odczyt katalogu `tools/` potwierdził także `game-atlas-appearances`, `game-atlas-creatures`, `game-atlas-creature-gameplay` i `reference-world-corridor-census`. W `game-atlas-creature-gameplay` istnieją `export.py`, `self_test.py` i fixtures. To inwentarz do ponownego użycia, nie kwalifikacja ich API jako kompletnego importera. Dokładne wejścia/wyjścia i aktualne prawa do ścieżek należy sprawdzić przy alokacji.

## 4. Docelowa architektura Content i plików mapy

```text
przypięte źródła / własny Content
 -> ograniczone importery istniejącego lineage
 -> Legacy/Source IR z pochodzeniem i disposition
 -> typowany model Oteryn
 -> edytowalny World Project
 -> walidacja + linkowanie + deterministyczna kompilacja
 -> server-authoritative bundle + client-safe bundle
 -> staging + autoryzowana aktywacja
 -> rzeczywisty runtime / klient
```

### 4.1 Projekt źródłowy

Kandydat: ścisły UTF-8 JSON dla manifestów i definicji, JSONL dla licznych rekordów mapy. Preferować upstream parser/serializer i walidację domenową, nie własny parser JSON. Ścisłość jest wymaganiem naszego profilu: duplikaty, nieznane critical fields, niejawne jednostki i błędne referencje muszą zostać wykryte. Samo użycie biblioteki nie dowodzi tych własności.

Proponowany układ jest logiczny; nie tworzymy pustych katalogów ani nowej biblioteki dla każdej pozycji:

```text
content/projects/reference/
  project.json
  content.lock.json
  profiles/reference-playable.json
  definitions/
    terrains/ world-objects/ items/ creatures/ abilities/ effects/
    loot/ npcs/ dialogues/ services/ interactions/ quests/
  worlds/reference/
    world.json
    regions/ areas/ subareas/ zones/
    shards/<floor>/<address>.cells.jsonl
    shards/<floor>/<address>.placements.jsonl
    spawns/ npc-placements/ transitions/ houses/ encounters/
  presentations/
  assets/manifest.json
  provenance/
    sources.lock.json
    mappings/ evidence/ corrections/ reports/
  editor/
```

`project.json` określa projekt i wersje schematu; Content Lock wiąże dokładne zależności. Profil wybiera korzenie wydania i dopuszczone możliwości. `world.json` określa autorski świat, układ współrzędnych, granice/piętra i katalog fragmentów. Autorski world key nie jest runtime `WorldId`. Nazwy plików nie są tożsamościami obiektów.

Nie zapisujemy pliku na kafelek, jednego monolitycznego JSON-a całego świata ani live state w projekcie. `.omap` pozostaje roboczą nazwą projektu/formatu wymiany; `.owb` — pakietu runtime. Finalne rozszerzenia nie są decyzją tego dokumentu.

### 4.2 Definicja, placement, instancja, stan

- Definicja: stabilny ContentKey, typowana rodzina, jawne możliwości, stan początkowy/przejścia i prezentacje tam, gdzie rodzina ich wymaga.
- Placement: stabilny PlacementKey, referencja do definicji, pozycja/anchor, porządek i footprint. Dokładną definicję rozwiązuje Content Lock/linker.
- Instancja/stany runtime: aktualny scope/owner, inkarnacja i rewizja; item-value instance pozostaje GAME-ITEM/DUR, nie kopią placementu.
- Move zachowuje tożsamość tej samej autorskiej instancji; copy ją rozdziela. Rechunking, rename, kosmetyka i reload nie tworzą nagrody ani nowego placementu.

Pole deklaruje istnienie/terrain. Placement deklaruje pozycję raz. Kompilator buduje uporządkowany indeks placementów na polu; nie utrzymujemy dwóch ręcznie edytowanych prawd. Porządek prezentacji nie jest automatycznie wyborem celu `look/use/move/Browse Field`.

Oddzielamy visual, collision, interaction i attachment/support footprints. Jeden wielopolowy obiekt przecinający shardy ma jedną tożsamość; indeksy wskazują ten sam placement. Brak wymaganego pola/fragmentu to `UNKNOWN`, nie walkable empty. Jawny `void` lub deklaracja rzadkiego, kompletnego fragmentu nie są brakiem danych.

### 4.3 Warstwy przestrzenne

Zachowujemy rozdzielenie source shard, compiled chunk i runtime spatial sector. Area/Subarea, Region i nakładające się Zone mają odrębne znaczenie; encounter placement pozostaje osobną warstwą. Granica pliku nie przydziela authority ani nie powoduje automatycznego scope handoff.

32x32 oraz 64x64 i strategia pakowania pięter pozostają kandydatami do wymaganego sprawdzianu, nie produkcyjnymi limitami. Zmienny podział nie może renumerować świata. Nie odrzucamy chunkowania/streamingu z projektu; zakres cache i harmonogram ładowania realizujemy proporcjonalnie do faktycznego odbiorcy i danych.

### 4.4 World Bundle i dystrybucja

Docelowy pakiet: wersjonowany nagłówek/manifest, katalog definicji, indeksy chunków, sekcje przestrzenne/stref/przejść, metadane integralności i oddzielne projekcje. Loader nie interpretuje surowego layoutu Rust ani OTBM podczas gry.

FlatBuffers oraz kompresja niezależnych sekcji/chunków przez Zstandard pozostają sensownymi kandydatami upstream, nie obowiązkowymi zależnościami. Uwzględnić istniejący format spike; wybrać najmniej złożone rozwiązanie spełniające random access, wersjonowanie, deterministyczność, walidację i budżety. Nie rozwijać starego eksperymentalnego kontenera tylko dlatego, że jest własny.

Lokalne palety i treściowe identyfikatory chunków mają ograniczać niepotrzebną zmianę wszystkich bajtów po dodaniu jednej definicji. Manifest wiąże spójne wydanie; nie trzeba bez potrzeby wpisywać nowego globalnego identyfikatora wydania w każdy niezmieniony blob. Jest to cel projektu do zweryfikowania kosztem aktualizacji, nie polecenie budowy własnego CDN/patch service.

Client-safe projection jest allowlistą, nie pełnym eksportem ukrytym w UI. Bezpieczność pól i prawo do otrzymania fragmentu mapy to oddzielne kwestie. Błędna para generacji fail-closed. Aktualny stan obiektów przychodzi przez właściwą obserwację serwera.

### 4.5 Runtime, interakcje i trwałość

Content deklaruje obsługiwane typowane operacje; nie nadaje callbackom dowolnego zapisu świata/SQL. FND ingress już istnieje: nie dodawać drugiego per-door receipt store. Potrzebna jest rzeczywista kompozycja kolejności całej GameSession z obecnym ownerem, nie koniecznie nowy podsystem kolejkujący.

Stan obiektu, wszystkie jego wkłady przestrzenne oraz outcome muszą stać się spójne w jednej nieprzeplatanej operacji. Otwarcie drzwi usuwa tylko ich wkład w kolizję. Range/policy/revision/current-owner checks wykonuje serwer. Replay zwraca pierwotny wynik, nie wykonuje operacji na nowo. Wynik komendy nie jest aktualną deltą świata.

Movement pozostaje position writerem. Stateful relocation wymagające workflow nie jest collision callbackiem. Wartościowy klucz, nagroda, handel, custody i progres wymagają odpowiednich GAME-ITEM/Character/DUR operacji. Kilka dzieci Interaction nie tworzy samo przez się atomowej transakcji między ownerami.

Pierwsza polityka aktywacji może odrzucać zmianę Content przy live scopes; obecny `ContentActivationController::activate` już wymaga quiescence i braku live scopes w swoim profilu. Nie budujemy konkurencyjnej blokady. Docelowo staging, pełna generacja i kontrolowane aktywowanie pozostają obowiązkowe; kompatybilny live rebind wymaga osobnego uzasadnienia. Rollback mapy nie cofa zatwierdzonych danych graczy.

### 4.6 Studio

Kierunek Studio pozostaje: wspólny model, walidacja, kompilator, historia operacji i preview, bez drugiej implementacji zasad świata. Moduły dostarczać razem z używanymi możliwościami, nie czekać z prawdziwym Content na kompletne IDE. Zachować partial/atomic save, odtwarzanie przerwanego zapisu i spójny snapshot kompilacji. Gotowe biblioteki i obecne narzędzia mają zastępować zbędną własną infrastrukturę, nie samą semantykę Oteryn.

## 5. Bezpieczeństwo i defekty — zachowane własności, proporcjonalne mechanizmy

| Granica | Wymagana własność | Zalecany sposób bez rozbudowy dla samej rozbudowy |
|---|---|---|
| Import | Złe pliki nie wpływają na aktywną grę ani sekrety. | Offline tool/process z limitami i bez produkcyjnych credentials; wykorzystać istniejące środowisko, nie nową platformę sandboxów. |
| Source/linker | Błędne typy, duplikaty, unknown critical fields, referencje, footprinty i niedozwolone capabilities nie przechodzą. | Upstream parser + Oteryn domain validation; wspólne reguły CLI/CI/Studio. |
| Topologia | Wymagane spawny/wyjścia/przejścia są poprawne; trigger work jest ograniczone. | Sprawdzać deklarowane wymagania. Zamknięty pokój lub cykl stanów nie jest automatycznie błędem. Bez samowolnego naprawiania mapy. |
| Loader | Sprawdza te same bajty, które użyje, przed authority. | Rozmiary/offsety/overflow, integralność, kompatybilność, bounded decode/index build i staging. Hash sam nie przydziela prawa aktywacji. |
| Zasoby | Dane nie wymuszają nieograniczonej pamięci/CPU/kolejek. | Limit używanej granicy i test max/max+1. Nie tworzyć własnego alokatora lub forka na podstawie samej hipotezy. |
| Operacja | Oczekiwany błąd nie publikuje częściowego stanu; lineage komendy pozostaje. | Typed Result, prevalidation/reservation i istniejący owner commit. Nie kopiować całego świata przed każdym kliknięciem. |
| Nieoczekiwany invariant failure | Nie kontynuować mutation na niepewnym stanie. | Istniejący FND-03 fail-stop scope; cały GameNode, gdy izolacja nie jest dowiedziona. Task nie jest granicą procesu. |
| Recovery | Brak crash-loop, fałszywego dobrego checkpointu i duplikacji wartości. | Istniejące supervision/recovery, bounded restart policy, poprzedni wiarygodny stan i DUR reconciliation. |
| Diagnostyka | Da się wskazać build/generację, placement/definicję, pozycję, operację i kolejność. | Rozszerzyć obecne logi/source locators; ograniczone dane, bez publicznych sekretów i nowej platformy debugowania. |
| Klient | Złe assets i nadmierne zasoby nie są pomijane jako poprawna scena. | Upstream decode/renderer checks + test realnego klienta. Opcjonalny efekt może mieć fallback; istotna ściana/przeciwnik/zagrożenie nie może po prostu zniknąć. |
| Skrypty, gdy używane | Brak bezpośredniej władzy nad SQL/światem, bounded deterministic proposal. | Istniejący kontrakt WIT, upstream Wasmtime jako kandydat; limity hosta również obowiązują. Nie budować własnego VM. |

Kwarantanna nie znaczy usunięcie drzwi, otwarcie przejścia, zniszczenie skrzyni ani reset nagrody. Jeżeli nie ma przyjętej, bezpiecznej izolacji pojedynczej funkcji, stosować istniejące szersze zatrzymanie. Raport klienta jest evidence, nie rozkazem zmiany świata. `catch_unwind` nie jest rollbackiem ani ochroną przed każdym crashem. Timeout nie zastępuje ograniczonej pracy w kodzie nieoddającym sterowania.

Dla realnych parserów i loaderów wykorzystać istniejące property/fuzz tools oraz mały korpus regresji. Fuzz bajtów i poprawnych strukturalnie kombinacji mają różne zadania. Nie tworzyć własnego fuzz frameworka ani wymagać nieskończonej kwalifikacji przed każdym przyrostem. Każdy odtworzony defekt ma test wykrywający ten sam błąd. Zachować niezależny oracle tam, gdzie wspólny kodek mógłby ukryć błąd klienta i serwera.

## 6. Co oznacza kopiowanie zawartości do Oteryn

**Importujemy dopuszczone dane i rekonstruujemy znaczenie. Nie kopiujemy ani nie transliterujemy runtime C++/Lua do Rust.** Nie uruchamiamy niezweryfikowanych skryptów legacy po to, żeby poznać ich dane. Proceduralne fragmenty wymagają ograniczonej analizy istniejącymi narzędziami albo jawnego `REWRITE_AS_NATIVE_RULE`/`UNKNOWN`.

Trzy niezależne osie: prawo do użycia/dystrybucji, pochodzenie i zgodność z targetem, obsługa w runtime. Dostęp do repo lub pliku nie dowodzi żadnej z pozostałych. Niewyjaśnione proprietary assets nie trafiają do publicznego Git ani paczki klienta. Plan nie stanowi opinii prawnej ani zgody na redystrybucję.

Źródła odkrywania przypadków:

| Źródło | Rola i ograniczenie |
|---|---|
| `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a` | Historycznie odczytany kod/pola/bindingi; OTS hypothesis, nie target truth. |
| `blakinio/canary@12df285bd181ad72eaa23901b0e5b19f37352634` | Historycznie odczytane zachowania/edge cases, w tym nested items i Browse Field. Nie osobny runtime Oteryn. |
| Pinned Game-owned legacy map/assets z fullworld producer README | Dozwolony zakres istniejącego dekodera; inna wersja danych wymaga jawnego dopuszczenia. |
| TibiaWiki i inne community sources | Wskazówki i corroboration z rewizją strony; brak oldid/target continuity nie staje się pewną wartością. |
| Official Tibia/CipSoft i lawful observations | Główne źródła Reference. Target według #483: `global-tibia-observable-2026-07-28-post-server-save`, nie automatycznie bieżąca Tibia. |

Nie mieszamy mapy z jednej wersji, appearance IDs z drugiej i gameplay z trzeciej bez jawnego mapowania/compatibility evidence. Jeden kompletny Source Lock obejmuje wszystkie uczestniczące warstwy, ich kolejność, digests i mapper/compiler versions. Hash mapy sam nie obejmuje XML, spawn sidecars i startup bindings.

### 6.1 Proces pojedynczej partii importu

1. Wybrać wejściowy snapshot i prawa do konkretnej klasy danych; raw input pozostaje w dopuszczonym offline storage.
2. Uruchomić istniejący Game-owned decoder/extractor na obsługiwanym wejściu; minimalnie rozszerzyć tylko brakującą granicę.
3. Zachować source locator, rodzaj rekordu, źródłowe ID i warstwy nadpisań w Legacy/Source IR.
4. Mapować do ContentKey/PlacementKey i typowanych pól. Definicję/pozycję/stany/przyszłą wartość rozdzielić.
5. Dla każdego istotnego rekordu/pola nadać disposition: extract, normalize, derive, rewrite, reference-only, reject, ambiguous/unknown/conflict. Istniejące nazwy w kontraktach mają pierwszeństwo przed nowymi enumami.
6. Rozliczyć rekordy i semantyczne pominięcia; nie nazywać pełnym sukcesem pominiętego dziecka kontenera. Diagnostyka może być ograniczona, ale raportuje truncation i liczbę nierozwiązanych przypadków.
7. Dopuścić kompletny wybrany podzbiór do kompilacji zgodnie z prawami, capability/resource i Reference evidence. Niekompletny niezależny katalog nie blokuje tej partii.
8. Przy reimporcie porównać previous imported baseline, new source i local correction. Move/copy/delete/transform rozstrzygać semantycznie. Konflikt nie ma cichego zwycięzcy.

### 6.2 Kolejność i równoległość przenoszenia danych

| Partia | Co przenosimy | Czego import sam nie dowodzi |
|---|---|---|
| Prezentacje + teren + geometria | Appearance metadata, map records, transform współrzędnych, porządek warstw, lokatory. | Praw do assetów, legalności przejścia lub parity kolizji. |
| Obiekty i interakcje | Placements, footprinty, finite states, use/read/rotate/door/transition candidates i konflikty bindings. | Że dowolny legacy type swap jest jednym obiektem albo że binding ma działający owner adapter. |
| Item catalogue | Typowane możliwości, staty, sloty, wymagania, temporal/imbuement/proficiency references. | ItemInstance custody, czasu wykonywania ani prawa do mint/transfer. Sloty imbuement są częścią mocy przedmiotu. |
| Creatures/spawns/abilities | Combat/reward references, algorytm doboru, spawn policy, Damage/Heal candidates i prezentacje. | Global stats/drop probabilities, runtime AI, XP commit lub loot settlement. |
| NPC/services/quests | Definicje, placements, dane dialogów/katalogów/podróży i czytelne warunki kandydackie. | Że rozmowa jest transakcją, storage integer jest etapem questa albo Lua nadaje authority. |
| Pozostały świat | Zone/house/encounter/world-change/field/fluid/trap families z jawnym statusem. | Że wszystkie funkcje muszą być gotowe przed jakąkolwiek grą lub są zbędne w produkcie docelowym. |

Szeroki katalog można ekstrahować równolegle z implementacją. Nie trzeba ręcznie przepisywać tysięcy definicji do struktur Rust. Podzbiór służy pierwszemu sprawdzeniu pełnej ścieżki; nie ogranicza modelu do jednych drzwi czy jednego potwora.

## 7. Plan wykonania — przyrosty tej samej architektury

Etapy poniżej to kolejność dostaw, nie nowe programme gates ani automatyczne alokacje. Nie wymagają osobnego dokumentu zatwierdzającego każdy wiersz. Przy rozpoczęciu istniejący #162 sprawdza live state, canonical workers, path overlap i tylko faktycznie potrzebne dependencies. Nie duplikować prac #504/#64/#483/#139 ani ich obecnych następców.

| Etap | Konkretna dostawa | Weryfikacja zakończenia | Zależności i możliwa równoległość |
|---|---|---|---|
| A — inwentarz i istniejące komponenty | Jeden kompletny zestaw wejść do pierwszej partii, prawa/disposition, target, lista narzędzi i luk. Wskazanie istniejących dokumentów i kodu zamiast nowego frameworka. | Każde wejście ma dokładną rewizję/digest/rolę; brak udawania, że visual export jest lossless gameplay. | Read-only/evidence może postępować teraz; zapis materiałów tylko w dopuszczonym zakresie. Nie wymaga działającego Server Seam. |
| B — source contract i pierwszy import | Typowane definition/placement/state/binding records oraz ekstrakcja reprezentatywnego korpusu przez istniejący lineage. Candidate catalogue + raport, nie ręcznie pisany miniświat jako docelowe rozwiązanie. | Duplicate/unknown/alias/bounds/overrides/zero-silent-loss tests; deterministic output i sprawdzony move/copy/reimport dla użytych przypadków. | Po właściwej alokacji parsera/mapperów; A jest źródłem. Katalogi mogą być rozwijane niezależnie od wire. |
| C — wybór fizycznego formatu i produkcyjne lowering | Przegląd istniejącego spike, wąskie porównanie realnych kandydatów, decyzja formatu przed jego trwałym przyjęciem; jawny successor #504 i projekcje przez obecny Content. | Source→bundle→load equivalence, random access, corruption/limits, deterministic artifact, partial edit/update koszt na tym samym korpusie. Brak zmiany interpretacji bootstrap v1. | B dostarcza dane. Zmiany Cargo/shared/schema/registry tylko przez właściwe ownership. Nie otwierać szerokiego benchmark programu wszystkich serializerów. |
| D — rzeczywisty obiekt i mapa w runtime/kliencie | Wpięcie typed object operation w obecnego ownera, Movement i FND; registered payloads oraz client result/delta/snapshot. | Dwie poprawne sesje, dwa kanały, stale owner/incarnation, wcześniejsza pending komenda, replay/expiry, kolizje, close/ruch, failure przed commit, snapshot barrier. | Prawdziwy wire wymaga rejestracji i kwalifikowanego resolvera. Component Rust może powstać wcześniej, ale nie jako zastępczy runtime lub terminalny proof produktu. |
| E — pełna wybrana pętla gry | World entry, movement/interactions, wybrane attack+heal, creature/corpse/loot/XP, item/equipment, właściwe NPC usługi, death/reconnect/restart zgodnie z milestone. | Native client i server path; trwałe value/progress na realnym DUR/PG; brak duplikacji po ambiguous/lost response, zachowane recovery i aktualny target evidence. | Dołączać właścicieli Ability/AI/Character/GAME-ITEM/DUR i #500 NPC tylko do operacji, które ich potrzebują. Handel nie może udawać zwykłej zmiany liczb. |
| F — katalog, skala i autorowanie | Kolejne partie rzeczywistego świata i danych, Studio na wspólnym modelu, potrzebne streaming/cache/indeksy i zwykłe narzędzia diagnostyczne. | Ten sam format i ścieżka runtime; pomiary rzeczywistego korpusu/klienta; reimport bez utraty poprawek; atomowy zapis projektu; brak wycieków server-only. | Rozwój danych i modułów Studio może iść równolegle, gdy model/interfejs jest gotowy. Nie czekać z rozgrywką na całe Studio; nie usuwać Studio z architektury. |

### 7.1 Reprezentatywny materiał

Pierwszy zamknięty zestaw powinien łączyć realnie wybrany fragment mapy z wieloma warstwami, przejściem pięter, footprintem przez granicę pliku i zwykłym obiektem stanowym, dalej z itemem, creature i NPC potrzebnymi przez aktualny journey. Dokładne współrzędne, nazwy i liczby wynikają z #64/#483/#504, nie z tego planu.

Dodatkowe syntetyczne fixtures są prawidłowe dla błędnych wejść i przypadków granicznych, ale nie wolno przedstawiać ich jako obserwacji Globala ani budować dla nich osobnego serwera. Plain door nie dziedziczy pełnych quests/economy/house ACL; jeżeli wybrany obiekt ich naprawdę wymaga, trzeba zastosować właściwy kontrakt, a nie udawać plain door.

### 7.2 Decyzje i stop condition dla bibliotek

Dla nowej technologii lub własnego mechanizmu kolejność jest jedna: existing Oteryn/upstream → supported configuration/API → mały adapter → udowodniona minimalna poprawka → fork wyłącznie w ostateczności.

Przed wyjątkiem wskazać konkretne wymaganie, dokładną wersję, reproducer/test albo source-level incompatibility; wydajność uzasadniać reprezentatywnym pomiaremrem. Nie narzucać z góry własnych allocatorów, registry frameworków, schedulerów, kompresji, parserów, baz ani deep instrumentation. Brak idealnego ogólnego API nie uzasadnia przebudowy biblioteki, jeżeli ograniczona integracja spełnia potrzebę.

Gdy wymaganie jest spełnione, zakończyć etap i używać wyniku w grze. Nie dodawać przypadków hipotetycznej przyszłej skali jako automatycznych blockerów. Nie mylić tej zasady z zakazem Oteryn-owned game logic.

## 8. Testy i klasyfikacja luk

Macierz C01–C36 w zachowanym audycie pozostaje listą coverage i testów dla konkretnych triggerów. Nie jest checklistą 36 rzeczy przed otwarciem drzwi i nie zostaje usunięta pod hasłem prostoty.

- B: C01–C08, C10 i C36 w zakresie użytego importera; exact evidence C02 dopiero dla deklarowanego Reference pola.
- C: C03/C05/C06/C08/C20/C21/C36 oraz wymagane corruption/random-access/round-trip i format fixtures.
- D: C09–C21, stosownie do actual source/local/wire boundaries. Szczególnie A/1 open, B/1 close, replay A/1 bez reopen; C13 obejmuje różne cele tej samej sesji.
- E/F: C22–C35 uruchamiane przy danej funkcji. Nie uruchamiać pełnych split/merge/quest/market suites jako warunku pojedynczego transferu, jeśli nie są w jego kontrakcie; nie twierdzić, że przez to te funkcje są pokryte.

Wymagane existing repository checks według [BUILD_TEST_MATRIX](../BUILD_TEST_MATRIX.md) pozostają. Nie tworzyć nowego globalnego required status. Zmiana semantyki safety/public IDs wymaga właściwego owning review; niskiego ryzyka evidence nie dziedziczy automatycznie ciężkiego zewnętrznego AI review.

Granice dowodów: 21 testów Python/4096 ścieżek to model; source inspection to odczyt; loaded environment to nie E2E; build klienta to nie render qualification; napisany plan to nie rozpoczęty import.

## 9. Niepewności, których nie zamieniamy w nowe blockery

| Brakująca informacja | Wpływ |
|---|---|
| Dostęp do exact raw map/asset/source bytes i prawa do ich użycia | Bez tego nie twierdzimy, że rzeczywisty import lub redystrybucja już się odbyły. Nie blokuje zachowania projektu i pracy na legalnych fixtures. |
| Finalny wybór source/bundle codec, chunk/floor packing, dokładne upstream versions | Wymaga existing format decision przed permanentnim formatem. Nie uzasadnia custom implementation na zapas. |
| Aktualna alokacja wykonawcza i gotowość resolvera/runtime/wire | Worker musi odczytać live #162 i owning contracts. Ten plan nikogo nie zwalnia do runtime mutations. |
| Granice zasobów successor dla konkretnego korpusu | Potrzebne przed użyciem tej granicy; nie wolno skopiować starych liczb lub wymyślić headroom. |
| Target evidence dla konkretnej wartości/mechaniki | `UNKNOWN/CONFLICT` w Reference; candidate ingestion może postępować. Nie zastępujemy braku danymi OTS lub zerem. |
| Rzeczywista odporność runtime/PG/klienta | Musi zostać zmierzona/przetestowana przy owning implementation. Dotychczasowy model nie zamyka tej luki. |

## 10. Walidacja i publikacja tego dokumentu

Plan oraz zachowany audyt są publikacją dokumentacji na istniejącej gałęzi #641, nie wdrożeniem formatu, importera czy runtime. Nie kopiujemy raw map/assets ani kodu zewnętrznych OTS. Nie zmieniamy Cargo, protokołu, produkcyjnych rejestrów, workflow, przyjętej architektury lub uprawnień.

Oryginalny załączony pakiet audytowy zachowano jako źródło. Przy przygotowaniu tej publikacji ponownie uruchomiono dokładny model o SHA-256 `e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa`: Python 3.13.5, 21 testów PASS, 4096 bounded serialized traces; cztery celowe mutanty zostały wykryte, a cztery boundary probes potwierdziły opisane ograniczenia. Model 26-testowy i historyczne 63 testy nie były częścią tego uruchomienia.

Bezpośredni `git ls-remote` w tej sesji nie powiódł się: `Could not resolve host: github.com`. Nie wybrano ani nie odtwarzano lokalnego kandydata Git. Zapis jest jawnie API-native edycją dokumentacji, dopuszczoną przez candidate-specific [Publication Integrity Policy](https://github.com/Oteryn/Oteryn/blob/33b212e652c680bd4047be3b414c9a358b8bf26f/docs/agents/contracts/PUBLICATION_INTEGRITY_POLICY.md). Lokalne kontrole tekstu/modelu nie zastępują pełnego repository governance/build. Exact-head hosted checks i readback po publikacji są zapisywane w komentarzu PR, bez mieszania wyników poprzedniego head.

## 11. Źródła zewnętrzne i granice ich użycia

Źródła primary dokumentacji sprawdzone dla decyzji o ponownym użyciu bibliotek:

- [Serde JSON](https://docs.rs/serde_json/latest/serde_json/): typowana serializacja/deserializacja jako kandydat; exact version i zachowanie naszego ścisłego profilu wymagają kwalifikacji w workspace.
- [FlatBuffers Rust](https://flatbuffers.dev/languages/rust/): schemat/codegen i weryfikujące funkcje odczytu jako kandydat. Nie oznacza zgody na `_unchecked` ani na własny allocator bez potrzeby.
- [Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html): upstream kontrola wykonania, gdy skrypty zostaną dopuszczone; nie nowy wymóg dla zwykłych obiektów.

Dokładne lokatory Crystal/Canary/wiki/CipSoft i zastrzeżenia datowania pozostają w audycie. Historyczna sprzeczność indeksowanych kosztów imbuement nie jest teraz rozstrzygnięta. Ponowny odczyt wskazanej strony CipSoft podczas przygotowania planu zakończył się błędem narzędzia; nie przedstawiamy tego jako świeżego potwierdzenia komunikatu ani nowej daty targetu.

## 12. Następna konkretna czynność

Po zachowaniu tego pakietu #162 powinno skoordynować istniejące prace i najbliższy rzeczywisty przyrost: Source Lock + wykorzystanie obecnego dekodera + typed source-to-owner bindings na reprezentatywnych danych, równolegle do gotowych path-disjoint prac runtime. Wymagane decyzje formatu/protokołu/zasobów należy zamknąć tam, gdzie dany przyrost ich używa, bez nowej ekspedycji architektonicznej.

**Kryterium sukcesu programu jest grywalny Oteryn korzystający z własnego Content przez docelową architekturę. Nie jest nim liczba dokumentów, testów modelu, forków ani zaimportowanych rekordów bez działającej semantyki.**

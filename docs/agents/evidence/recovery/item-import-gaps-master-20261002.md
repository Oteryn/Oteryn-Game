# Itemy — wspólna lista braków importu z Canary, Crystal i wiki

Stan na 2 października 2026, opublikowane dane Oteryn: `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`. Ten raport zbiera wszystkie obecnie zidentyfikowane grupy braków, holdów i luk implementacyjnych. Nie ustanawia nowego źródła prawdy ani dopuszczenia danych do runtime.

Pula klasyfikacji obejmuje **12 487 ID**, z czego **12 252 mają rodzinę**, a **235 pozostaje otwartych**. Istnieją wszystkie **22 profile rodzin i 23 szablony**. Dane native zawierają szerszy zbiór **34 031 Itemów / 57 320 rekordów wszystkich rodzajów**, a Itemy mają **69 wspólnych shardów**. Te zakresy nie są zamienne. Chroniony historyczny fixture importera38 157 nie jest liczbą aktualnych braków.

**7 171 ID ma co najmniej jeden kwalifikowany fakt. Pełna liczba kompletnych i niekompletnych Itemów pozostaje NOT_ESTABLISHED/NULL.** Końcowy licznik wymaga przyjętego zestawu właściwych wymagań dla każdej rodziny i itemu. UNKNOWN jest stanem wiedzy; sama obecność UNKNOWN nie dowodzi, że dane pole ma zastosowanie.

## 1. Przygotowane paczki wymagające domknięcia

| Paczka | Zakres | Co pozostało |
| --- | ---: | --- |
| Weapon103 |103 itemy:78 modyfikatorów ataku,25 bezwzględnych procentów trafienia|Końcowe guardy typów/tożsamości, nowe seale, niezależny review, Rust/testy, generowanie i publikacja. Gałąź już ma cały poprzednik Forge; ostatni kod jest niezamrożonym AUTHORING.|
| StackableFalse8 |8 wartości|Przenieść pełne źródła do właściwego inwentarza, zakwalifikować aktualny parent, przetestować atomowy import i opublikować. Wartości są DERIVED_DOCUMENTED_TEMPLATE_DEFAULT, nie literalnymi flagami oficjalnymi.|
| Physical8 |8 całych wektorów /16 atomów|Kontrakt wersji i zakresu artefaktu, kodek/oracle i zgodność, aktualne seale źródeł oraz atomowy import8 i końcowa kwalifikacja. Nie remapować Physical ani pomijać atomów.|

Weapon103 i StackableFalse8 obejmują **109 różnych ID**; z rozłącznym Physical8 **117 ID**. To zakres tych paczek. Ich zakończenie nie zamyka pozostałych kategorii tego raportu.

## 2. Liczniki braków danych i przypadków do kwalifikacji

Liczby w tabeli opisują wskazany zakres dowodów. Zbiory przecinają się; **nie sumujemy ich jako liczby niedokończonych Itemów**. Wiersz z kandydatami lub zachowanym holdem nie jest obietnicą automatycznego importu.

| Zakres | Potwierdzony licznik / jednostka | Potrzebna praca |
| --- | --- | --- |
| Rodzina i tożsamość |235 ID bez rodziny|Rozstrzygnąć własne źródło, typ, konflikty i domenę Item/World. Powody:170 bez zachowanej własnej obserwacji wiki,59 nieprzyjętych typów,4 konflikty,2 bez jawnego typu.|
| Maksymalny stos |2493 kandydaty z dodatnim stackable|Uzyskać niezależny, właściwy limit. Nie stosować globalnego default100. To licznik zachowanego researchu, nie kompletny bieżący residual.|
| Movable=false |4091 przypadków diagnostycznych;0 zakwalifikowanych do automatycznego false|Rozstrzygnąć domenę przenośnego Itemu i brak afirmatywnej flagi Take. Nie wyprowadzać false z nieobecnej flagi.|
| Forge |870 oficjalnie dodatnich kandydatów bez niezależnego maksimum|Własna para klasyfikacja/max_tier. Osobny historyczny zakres Wiki upgradeclass nie jest drugim zbiorem do dodania.|
| Imbuement — rodziny i tiery |656 Itemów z dodatnią liczbą slotów, ale UNKNOWN allowed_family_tiers|Kwalifikować rodziny i maksymalne tiery per Item. Surowe nazwy i sufity OTS mogą się różnić. Liczba wszystkich KNOWN slot_count wynosi661; nie utożsamiać jej z660 dawnymi promocjami wiki.|
| Nowe kandydaty slot_count |10 zachowanych obserwacji wiki, obecnie UNKNOWN|Sprawdzić własne ID, warianty, binding i składnię przed importem.|
| Augments |62 Itemy /77 klauzul /47 etykiet czarów;0 kwalifikowanych typed bindings w tym zakresie|Jawne tożsamości czarów, właściwości, efekty i wartości rang. Dopasowanie samej nazwy nie wystarcza. OTS mają dodatkowe obserwacje, których nie dodajemy do62.|
| Equipment |19 zachowanych holdów z bieżącym UNKNOWN patterns|Sloty/ręce/wymagania level i vocation, warianty i atomowe całe wzorce.|
| Odporności |39 zachowanych holdów z bieżącym UNKNOWN resistances|Kwalifikacja całych wektorów, jednostek, domeny i wariantów.|
| Modyfikatory |103 zachowane holdy z bieżącym UNKNOWN vector;75 dawnych holdów już rozwiązano nowymi kwalifikowanymi ownerami|Zakwalifikować pozostałe całe wektory, składnię, warianty i tożsamość. W tym zakresie są nakładające się rodzaje statystyk;13 niepoprawnych wektorów Mantra/Bond i Physical nie są osobnymi licznikami do sumowania.|
| Critical/leech i kontekst aktywacji |494 zapisane wektory /796 atomów; wszystkie494 mają UNKNOWN konteksty;212 atomów critical/leech|Jawne zasady target_domain/evaluation_phase/priority, właściwe jednostki i zależności. Zapis statystyki nie dowodzi działania w walce.|
| Niepełne pary leech OTS |31 unikalnych ID;8 ma obecnie UNKNOWN modifier vector|Amount bez chance nie dowodzi chance=0. Zachować osobno life/mana, właściwy zasób i źródło;31 nie jest liczbą gotowych nowych importów.|
| Duration |52 historyczne holdy nadal UNKNOWN;osobno54 nowsze zachowane obserwacje nadal UNKNOWN|Sprawdzić jednostkę, wariant, zużywanie czasu/ładunków i pełne źródła. Te dwa zakresy mogą się pokrywać.|
| Dokumenty |48 kandydatów writeable,47 max_text_length i2 oficjalne readable nadal UNKNOWN|Kwalifikacja per Item. Liczniki są oddzielnymi obserwacjami pól, nie97 różnymi Itemami.|
| Pojemność kontenerów |Z33 zachowanych wykluczeń:4 UNKNOWN,26 już KNOWN wymagających rozliczenia pochodzenia,3 poza pulą|Rozstrzygnąć konflikt lub wariant; nie oznaczać wszystkich33 jako brakujących pojemności.|
| Market eligibility |Z227 zachowanych wykluczeń:121 UNKNOWN,1 KNOWN,3 poza pulą,102 bez dokładnego targetu w tym starym pakiecie|Ponownie sprawdzić aktualne bindingi i powody wykluczeń.102 nie oznacza102 brakujących definicji Itemów. Market flag nie jest ceną NPC ani pełną ofertą.|
| Pozostałe defaulty stackable |157 zachowanych przypadków UNKNOWN,7 KNOWN do rozliczenia pochodzenia|Kwalifikacja własnych źródeł i domyślnych wartości; nowa paczka8 jest osobnym zamkniętym zakresem.13 wykluczeń innej paczki leży poza pulą portable.|
| Waga i range_cells |4 konflikty wagi,3 niepoprawne zakresy|Rozstrzygnąć wariant i jednostkę; nie nadpisywać wartości większością stron.|
| Broń poza Weapon103 |2 konflikty nazwy,4 przypadki współdzielonego own-ID/wariantu|Jawna kwalifikacja wariantów i źródeł. Wartości pozostają wstrzymane.|
| Charges run |39 run do rozliczenia pochodzenia:29 KNOWN>1,7 KNOWN=1,3 UNKNOWN|Oddzielić liczbę użyć od ilości wytwarzanej przez czar. Nie twierdzimy, że wszystkie29 są błędne; zapis liczby nie zamyka kwalifikacji jej jednostki.|
| Proficiency |664 opublikowane bindingi istnieją;2 wykluczenia staging i71 obserwacji SpellId/augment OTS wymagają rozliczenia|Nie przedstawiać całego proficiency jako niezaimportowanego. Domknąć dokładne tożsamości, klasy progów i projekcję właściwych profili.|
| Pozostałe obserwacje Use |W zachowanym zakresie2 damage_range,2 damage_type,5 mana_cost|Rozstrzygnąć tożsamość i kontekst Item/Ability. Opublikowane345 obserwacji nie ustanawia automatycznie wykonywalnej Ability.|

Pełne per-ID partycje, source coordinates, 43 ścieżki stanów native i źródła liczb są w `item-consolidated-native-gaps-after-forge4f-20261002.json`. Stara unia3407 kandydatów parametrów jest osobnym historycznym zakresem; nie jest globalną liczbą niedokończonych Itemów.

## 3. Luki w projekcji danych i semantyce importu

| Grupa | Obecny stan | Brakująca część |
| --- | --- | --- |
| Efekty i dźwięki |Formalny schemat i konwerter XML mają efekty/pociski/melee; pole sounds istnieje. Aktualne ItemAuthoring:0 Presentation refs. Native prezentacja tylko name/description.|Import powiązań per Item, skryptowych efektów trafienia, reguł broni/amunicji i osobnych zdarzeń audio, kwalifikacja własnych assetów. Nie spłaszczać C++ i skryptu do jednej nadpisującej wartości.|
| Światło |Formalne light i ogólne Presentation istnieją; typed light emission/toggle Item jest candidate-only.|Źródło/kolor/intensywność/radius, warunki equipped/use/toggle i właściwa projekcja.|
| Używanie i use-with |ItemAuthoring ma pola Interaction/Ability i obserwacje Use; aktualnie0 on_use_interactions/use_ability.|Jawne powiązanie zachowania i celu, atrybutów użycia i transformacji ze zdarzeniem. Forceuse/action/eventtype i opis wiki wymagają kontekstu.|
| Runy i czary |runespellname występuje w historycznych censusach;36 obserwacji nie ma konwertera w tym sample.|Tożsamość właściwej Ability, wymagania magic level/mana/vocation, cel, charges i jednostki.36 jest licznikiem historycznego konwertera, nie bieżących brakujących run.|
| Consumable/food/fluid |Schematy i część native facts istnieją; aktualnie0 profili ItemAuthoring consumable.|Własne wyrażone efekty, nutrition, regeneracja, zasób, warunki konsumpcji i powiązanie z use; nie ustawiać neutralnych efektów z braku danych.|
| Łóżka |Formalne bed.sleepable jest candidate-only.|Kontekst World/Item, własne warunki użycia, części i transformacje. Element geometrii nie jest automatycznie przenośnym Itemem.|
| Warianty i transformacje |Native use_transform oraz lifecycle istnieją; presentation variants i ogólne opisy nie ustanawiają pełnej projekcji.|Dokładne target IDs, wariant/charged/depleted/equipped, zmiany wyglądu, rotate/destroy/enchant i właściwy event.|
| Wiązania account/character i compatibility |Native jawnie odrzuca KNOWN dla tych nieprzyjętych kontraktów.|Właściwy zaakceptowany owner/grammar i reguły kompatybilności; nie wpisywać bindingu konta do zwykłego stringa ani wyprowadzać braku ograniczeń z braku danych.|
| Fluid i role pojemników |Native ma scalar fluid_type, obecnie0 KNOWN; rola źródła/pojemnika/defaultu to odrębne pojęcia.|Dokładny typ i rola, a zawartość bieżąca należy do instancji. Placowany cask/tile wymaga World/Interaction.|
| Zawartość kontenerów/quiver |Capacity ma własny carrier, lecz nie pełne zasady accepted contents.|Typed ograniczenia i rola kontenera/quiver, nested bags; capacity nie dowodzi, co można do niego włożyć.|
| Document/write-once |Read/write/max mają zapisane fakty; current distance_read/write_once0 KNOWN oraz ItemAuthoring.document0.|Target write-once, dokument/treść i własny zakres odczytu;208 wcześniejszych pól nie zamyka wszystkich relacji dokumentów.|
| Aktywne zegary i transformacje |Native duration ma138 KNOWN, a consumption_mode/stop_duration/decay_target/use_transform mają0 KNOWN w tym artefakcie.|Właściwy zegar i zdarzenie, target i owner, bez default continuous/stopFalse lub targetu z samej nazwy.|
| Opisowe attrib/notes |Źródła mogą przechowywać fakty tylko w tekście; część obserwacji już zachowano.|Bezstratnie zachować oryginał i kwalifikować rozpoznane fakty. Opis, flavor i instrukcja skryptu nie są automatycznie wykonywalnym zachowaniem.|
| Tożsamość/provenance |Source catalogs i explicit bindings istnieją, ale współdzielone strony, puste/niezgodne actualname i różne epoki pozostają holdem.|Own-ID, pełne artifacts/parts, cutoff, wersja, tożsamość i all-present konflikt guards dla każdej kolejnej paczki.|

Aktualne316 ItemAuthoring mają taxonomy164, Forge147, use_observation139, lifecycle136, source_lifecycle121 i required_magic_level39. Nie ma tu profili proficiency/augments/consumable/presentation/document ani linków use. To census tego artefaktu, nie twierdzenie o nieistnieniu innych modułów lub wcześniej zapisanych crosswalków. Licznik12252 rodzin pochodzi z osobnej kwalifikowanej nakładki; native classification group ma0 KNOWN i nie jest równoważnym licznikiem integracji tej nakładki.

Pełna kontrola29 grup obejmuje wszystkie32 główne pola formalnego schematu, bez pominiętych korzeni: `item-import-semantic-group-gap-audit-forge4f-20261002.json`.

Formalne historyczne raporty Canary/Crystal wykazują także nonzero brak konwersji `action`, `augments`, `eventtype`, `flags.forceuse`, `flags.unmove`, `runespellname`, `type`, `weapontype`. Ich old population outcomes i liczniki są diagnostyką danego konwertera, a nie końcową liczbą ukończonych Itemów Oteryn. Szczegóły: `item-import-formal-mapping-audit-20261002.json`.

## 4. Dane wymagające powiązań poza rekordem Item

- `droppedby`, raid/event drops i loot chance → Creature/Loot/Encounter → Item.
- `buyfrom`, `sellto`, `npcprice`, `npcvalue`, currency/count/subtype → oferty NPC Service → Item.
- Quest Items, Quest Log, taskitem, nagrody i cele → Quest/Task/Objective/Interaction → Item; sam napis quest_item nie daje całego powiązania.
- Step-in/out, teleports, blocking/pathfinding, bed geometry i terrain → World/Interaction. Stan obecnej nakładki World nie jest automatycznie dowodem ukończenia wszystkich takich relacji.
- Store/drome/task/tournament values → właściwy właściciel ekonomii/systemu. Wiki community value pozostaje metadanymi źródła.
- Szablonowe parametry MediaWiki, render/query controls i pola bez skutecznego handlera silnika mają jawne dyspozycje; nie są brakującymi runtime atrybutami Itemu.

Te powiązania należy rozliczyć w odpowiednich domenach. Nie kopiujemy ich do intrinsic Item arrays i nie liczymy wszystkich jako brakujących pól Item.

## 5. Warunki zamknięcia importu

1. Przyjąć wymagania właściwe dla rodziny/konkretnego Itemu i policzyć je na jednym spójnym opublikowanym zestawie danych.
2. Każde właściwe wymaganie ma kwalifikowaną wartość albo uzasadnione NOT_APPLICABLE w audycie; pozostałe braki, konflikty i źródłowe holdy są widoczne.
3. Istnieje jednoznaczna ścieżka źródło → kwalifikacja → typed owner/projekcja → asset/relacja, z zachowaniem jednostek i wszystkich właściwych poprzednich wartości.
4. Końcowy walidator zgłasza0 nierozstrzygniętych właściwych wymagań; testy migracji, zgodności, generowania i readback przechodzą na zamrożonym SHA.
5. Integracja danych/draftów jest rozliczona przez właściwy control plane. Implementacja zachowania i jego test E2E są osobnym warunkiem gotowości gameplay, gdy dany zakres tego wymaga.

Praktyczna kolejność: domknąć109 prepared IDs, odrębnie Physical8, następnie kwalifikować większe grupy statystyk/source holdów i typowane zależności, a równolegle uzupełnić Presentation/Interaction/Ability i kompletność per-family. Nie kończy się to wyłącznie dopisaniem liczb do JSON.

## Źródła i sposób odczytu

Oteryn: pełne lokalne Gitbloby opublikowanego Forge4f i zachowane paczki kwalifikacji. Canary47df oraz Crystalff7/00ce: przypięte XML/C++/Lua, lokalny odczyt i zwykły HTTP publicznego GitHuba. OTS zachowują rolę OtsHypothesisOnly; zgodność implementacji upstream nie jest dowodem oficjalnego zachowania Tibii. Wiki: zachowane own-ID/rawparts/full article captures, wcześniej HTTP/search i anonimowy Chrome/CDP po blokadzie HTTP. W tej konsolidacji nie wykonano nowego researchu publicznych wiki ani Remote Desktop; aktualna ciągłość rewizji jest UNKNOWN. Dane niepewne są jawnie oznaczone.

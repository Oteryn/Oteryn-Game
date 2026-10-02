Z 12 487 kandydatów **12 252 ma przypisaną rodzinę; pozostaje 235**. To 98,12% przypisanych. Wśród pozostałych: 170 bez zachowanej obserwacji Wiki Item, 59 z niedopuszczonym primarytype, 4 z konfliktem rodzin i 2 bez jawnego primarytype. Poza tą pulą wyłączono 21 544 wskaźniki właścicieli mapy. Samo wyłączenie mapy nie dowodzi przenośności wszystkich pozostałych kandydatów.

**Wszystkie 22 profile są w schemie i mają szablony: 23 pliki, 0 brakujących profili/szablonów.** Definicje native są przechowywane wspólnie w content/items/definitions/ (69 shardów). Rodziny mają profile i przypisania w taksonomii, a nie osobny katalog definicji native dla każdej rodziny. Istnienie profilu/szablonu nie oznacza kompletnych danych każdego Itemu.

| Profil rodziny | Przypisane Itemy | Schema | Szablon | Katalog native |
| --- | ---: | --- | --- | --- |
| container | 237 | tak | tak (1) | wspólny |
| container_equipment | 8 | tak | tak (1) | wspólny |
| decoration | 3316 | tak | tak (1) | wspólny |
| document | 446 | tak | tak (1) | wspólny |
| equipment_armor | 459 | tak | tak (1) | wspólny |
| equipment_offhand | 337 | tak | tak (1) | wspólny |
| event_collectible | 192 | tak | tak (1) | wspólny |
| fluid | 87 | tak | tak (1) | wspólny |
| food | 195 | tak | tak (1) | wspólny |
| key | 13 | tak | tak (1) | wspólny |
| light_source | 166 | tak | tak (1) | wspólny |
| material_valuable | 3140 | tak | tak (1) | wspólny |
| plant | 906 | tak | tak (1) | wspólny |
| progression_material | 11 | tak | tak (1) | wspólny |
| quest_item | 1108 | tak | tak (1) | wspólny |
| rune | 41 | tak | tak (1) | wspólny |
| tool | 354 | tak | tak (1) | wspólny |
| transformation_item | 30 | tak | tak (1) | wspólny |
| trash | 206 | tak | tak (1) | wspólny |
| weapon_distance | 177 | tak | tak (2) | wspólny |
| weapon_magic | 128 | tak | tak (1) | wspólny |
| weapon_melee | 695 | tak | tak (1) | wspólny |

Przygotowane lub zablokowane paczki pozostałej pracy:

| Paczka | Itemy | Zakres | Pozostała praca |
| --- | ---: | --- | --- |
| HitML67 | 67 | 67 pól: 28 względny hit + 39 ML | cały Name139 przeniesiony; trwa implementacja przed Cargo |
| Mantra49 | 49 | 49 wektorów / 114 atomów | źródła gotowe; czeka cały HitML67 |
| Use345 | 139 | 345 obserwacji źródłowych | źródła gotowe; czeka cały Mantra49 |
| Forge1 | 1 | 1 para class/max (2 pola) | źródła gotowe; czeka cały Use345 |
| Weapon79 | 79 | 79 pól: 54 atk modifier + 25 absoluteHit | źródła przygotowane; czeka cały Forge1 i walidacja typów |
| Physical8 | 8 | 8 wektorów / 16 atomów | blokada zgodności typu native/ABI |
| DefaultFalse8 | 8 | 8 pól stackable=false | kandydaci potwierdzeni; potrzeba nowego pakietu |

Siedem nieopublikowanych paczek dotyczy **312 różnych ID** po uwzględnieniu nakładania się. To liczba ID dotkniętych tymi paczkami, nie pełna liczba niedokończonych Itemów ani liczba Itemów, które będą w pełni gotowe po wdrożeniu. Przykładowo HitML67 i Use345 współdzielą 27 ID, a HitML67 i Weapon79 współdzielą 4. Atom modyfikatora, pole i Item to różne jednostki.

Modifier26 jest już opublikowany: lokalnie sprawdzono dokładny pakiet 26 pełnych wektorów / 63 atomów i wszystkie odpowiadające zapisane wektory native. Name139 jest już opublikowany w VALIDATE pod SHA 1945520b41fc1c71856ba52b282c3f4171e083b6 (PR #1525); Root potwierdził pełny odczyt 26 plików. Nie liczymy go jako oddzielnej nieopublikowanej paczki, lecz część tych samych ID nadal wymaga innych pól. Publikacja/VALIDATE nie oznacza merge ani pełnej gotowości Itemów. PR #1531 opublikowany w VALIDATE odtwarza oryginalne paczki 1487 + 7 i nie dodaje nowych faktów.

Use345 zapisuje SOURCE-ONLY obserwacje w istniejącym authoring.use_observation: 111 damage, 138 damage_type, 96 mana_cost. To nie 345 kompletnych Itemów ani uruchomienie intrinsic/runtime attack lub kosztu zużycia runy. Physical8 wymaga zgodnego typu i wersji: nie można pominąć atomu Physical z całego wektora. DefaultFalse8 wymaga osobnego nowego pakietu po rzeczywistym Name139; oryginalnych 1487 + 7 nie rozszerzamy.

**870 oficjalnych kandydatów Forge nadal wymaga niezależnie źródłowanego maksimum per Item.** Gotowa para Hammer3332 (class 2 / max 2) jest osobną paczką. Istnieje już 146 zapisanych authoring.forge: 145 zgadza się z oficjalną klasą, a 1 nie ma tego oficjalnego flagu. Sama klasa ani globalna tabela class→tier nie dowodzi indywidualnego max_tier. Stare 865 kandydatów upgradeclass z Wiki jest innym zakresem i nie należy go dodawać do 870 brakujących maksimów.

Największe konkretne blokady źródeł i przypisania danych (odrębne, nakładające się zakresy):

| Zakres | Liczba | Czego brakuje |
| --- | ---: | --- |
| Dodatnie stackable / nieznane maksimum | 2493 kandydatów w zachowanym zakresie badania | Brak właściwego maksimum per Item lub jawnie obowiązującej reguły bez wyjątków. Nie ustawiamy domyślnie 100. Badanie jest ograniczone, nie stanowi dowodu braku źródeł wszędzie. |
| movable=False | 4091 kandydatów diagnostycznych; 0 kwalifikowanych | Wszędzie Take jest ABSENT, a nie False. immobile=yes/unmoveTrue nie dowodzi przenośnego Itemu; brakuje potwierdzenia właściciela/domeny. |
| Forge max_tier | 870 oficjalnych kandydatów | Niezależne maksimum per Item; gotowy Hammer3332 jest poza tym zakresem. |
| Augmenty | 62 adnotacje Itemów / 77 klauzul / 47 etykiet zaklęć | 44 etykiety nie mają nawet zgodnego literalnego klucza. Wszystkie mają 0 kwalifikowanych typedbindings; 3 dopasowania leksykalne nie są wiązaniami źródłowymi. |
| Konflikty wag | 4 konflikty z obecnego raportu stats-v2 | Rozstrzygnięcie wariantu i zgodności własnych stron, m.in. Rainbow Torch 5,00 vs 4,50. |
| Malformed range_cells | 3 wartości z obecnego raportu stats-v2 | Poprawna interpretacja literalnej składni i jednostek. To zakres dystansu w komórkach, nie damage_range z Use345. |

Tych liczb nie sumujemy z 235 brakami rodzin ani 312 planowanymi ID. Mają różne źródła i znaczenia, a część ID się powtarza. Nie podajemy niepotwierdzonej liczby 28 brakujących wag; powyżej są konkretne 4 konflikty i 3 malformed range_cells z przypiętego raportu.

Pełny końcowy licznik „kompletne / niekompletne” pozostaje **nieustalony**. Stare 2521 Itemów ze źródłami i UNKNOWN oraz 986 Itemów z polami niezmapowanymi/zależnymi od kontekstu to nakładające się unie z native 8257ba68, sprzed Modifier26 i Name139. Collector nie uwzględnia pakietów tych dwóch paczek. Odjęcie 26 lub 139 od 2521 byłoby błędne: uzupełniony Item może nadal mieć inne braki. Po całym łańcuchu potrzebna jest ponowna inwentaryzacja per ID na jednym złożonym kandydacie z nowymi ownerami pól, zastosowaniem, zależnościami, jednostkami i blokadami. UNKNOWN bez dowodu zastosowania nie jest automatycznie brakującym wymaganym polem.

Zakres opublikowanych źródeł: taxonomy 88edf552b323cef8cb68cef5b10949fbf0c85424; poprzedni native Doc208 8257ba68d449a4975345897823d8282393e5bae2; Modifier26 2ec1f34f05fd89abbb041f313b0852c331ae2b53; kompatybilność #1531 488e80b68160a02486fa764cb911788a71dc0516; Name139 #1525 1945520b41fc1c71856ba52b282c3f4171e083b6. Zestawienie jest odczytem nakładających się checkpointów, nie dowodem złożenia lub merge do main.

W tej pracy odczytano lokalne snapshoty, pakiety i kod, bez nowych połączeń HTTP lub Remote Desktop. Dziedziczone Wiki zawiera zachowane publiczne odczyty Chrome/CDP po blokadzie HTTP; aktualna ciągłość artykułów pozostaje UNKNOWN. Pełne ID, przecięcia paczek i SHA źródeł zapisano w item-remaining-work-ledger-20261002.json.

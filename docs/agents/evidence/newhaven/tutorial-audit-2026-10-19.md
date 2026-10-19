# Newhaven: audyt mapy, danych i tutoriala — 2026-10-19

## Status i zakres

Audyt na polecenie właściciela, dla koordynatora. Sprawdzony Oteryn-Game `main@466c9142ad41223181f998878fa9b8a633be4b4f`. PR [#2186](https://github.com/Oteryn/Oteryn-Game/pull/2186): otwarty draft, odczytany head `a8a631517cef574f15c67e2708c136f229b864b4`. Ten raport jest dokumentacją dowodów; nie przydziela implementacji, nie zmienia startu, mapy, serwera, klienta ani istniejącego PR.

**Wniosek: nie mamy jeszcze kompletnego, odtwarzającego film tutoriala Newhaven.** Obie wyspy i przejście są w danych mapy. Brakuje rozmieszczeń sześciu NPC, kilku usług, wykonania tutoriala i jego skutków. Zapis teleportu nie dowodzi możliwości przejścia w uruchomionym bundle.

Klasyfikacja: **PROVEN** = bezpośrednio odczytany plik/rekord; **DERIVED** = wniosek z danych i kodu; **CONFLICT** = źródła różnią się; **UNKNOWN** = brak wystarczającego dowodu. Są to klasy dowodów, nie zgoda na zmianę produktu.

Zakres obejmuje wszystkie pliki z `newhaven` w ścieżce, sześć lokalnych NPC, wszystkie referencje tekstowe Newhaven w zachowanych plikach Lua/XML przypiętego korpusu, powiązane katalogi Oteryn, wszystkie shard-y spawnów oraz regiony B3 przecinające wskazany prostokąt. Nie przeprowadzono sesji E2E, pełnego porównania każdej kratki z oryginalnym OTBM ani weryfikacji każdego sprite’a/animacji. Podziemny prostokąt może zawierać inne obszary; jego statystyk nie należy utożsamiać z granicami Newhaven.

Koordynacja: [#1622](https://github.com/Oteryn/Oteryn-Game/issues/1622). [Wstrzymanie #2186](https://github.com/Oteryn/Oteryn-Game/pull/2186#issuecomment-6254969126) pozostaje w mocy: brak nowych ramek/hintów/pushy do tego PR. Raport jest oddzielnym materiałem do decyzji koordynatora.

## Źródła

- CrystalServer `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, zachowany w `tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz`. Mapa Oteryn ma odrębne bindingi mapy z rewizji `00ce02a57ca5a12e48f32a3476e37471167e4c3f`; katalog NPC deklaruje także `ff7ede593c69d4c658b382c97443e8155926924a`. Nie zakładać identyczności tych rewizji.
- Film: https://www.youtube.com/watch?v=Jp_L0jbfsao, 33:16. Poniżej zachowano wcześniejszą rekonstrukcję: 28 zdarzeń, 101 wystąpień tekstu, 1941 próbek OCR co sekundę od 00:55 do 33:15 i odczyty kluczowych okien. W tym audycie nie wykonano ponownego oglądania. To indeks obserwacji, bez dowodu braku komunikatów krótszych niż sekundę.
- OTClient `53d3a8ac2a091c12a445ec16b7ea8ada613acee0`, `modules/game_tutorial/{controller/tutorial_controller.lua,const/vocations.lua,template/html/tutorial.html}`. Wcześniej sprawdzony kontroler obsługuje ID 2, pozostałe numery nie są kompletną implementacją.
- Snapshot pozycji NPC: `imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json`. Pozycja obserwowana/wiki nie jest nieruchomym punktem spawnu.
- Wcześniej zbadany katalog tekstów klienta Tibia `15.33.9d148f`, SHA-256 `ea8d9fc576f3cad4b2976f1fa8d52a599ad74c951252c814a6e50e99d0ab3808`. Inna wersja niż film: obecność klucza tłumaczenia nie ustala jego wyzwalacza.

## Start, przejścia i kluczowe kratki

Wszystkie poniższe pozycje mają globalny zapis Tibia `(x,y,z)`. Natywny `floor` Oteryn wynosi `-z`, więc `z=7` oznacza `floor=-7`. Bieżący `content/world/pins/oteryn.json` startuje w **Thais `(32369,32238,floor=-7)`**, nie na wysepce.

| Punkt | Pozycja | Dowód / zachowanie |
| --- | --- | --- |
| Pierwsze logowanie Newhaven | (32534,32513,7) | on_login_newhaven.lua: zwykła postać bez profesji, citizen <= 0; ustawienie citizen=1 |
| Dziura powrotna | (32533,32513,7) | movement_init_newhaven.lua: cofnięcie gracza na fromPosition i komunikat |
| Wejście portalu | (32538,32514,7) | movement_select_vocation.lua; rekord Transition.Teleport i atrybut B3 |
| Wyjście portalu | (32544,32515,7) | Ten sam cel w skrypcie, rekordzie przejścia i B3 |
| Hint polowania | (32561,32485,7), (32562,32485,7) | Jeśli hunting > 0: sendTutorial(3), potem hunting=-1 |
| Drzwi promu | (32556,32497,7), (32556,32498,7) | B3: i51778 / i51777; źródłowe onUse rejestruje oba ID |
| Cel akcji drzwi | (32555,32497,7) | on_use_newhaven.lua: wymagany poziom >= 8 |
| The Corruptor | (32614,32422,7) | Jeden zapisany spawn, respawn 60000 ms |
| Hector the Mentor | (32474,32288,7) | Pozycja z osobnego snapshotu; nie dowodzi punktu lądowania promu |

Portal źródłowy teleportuje każdego gracza bez warunku premium/profesji. Dla braku profesji: `sendTutorial(2)` i maksymalnie 60 prób przyznania wyposażenia w odstępach sekundy. Dla wybranej profesji: wyposażenie od razu. Następnie sześć znaczników mapy i flagi `newhavenCitizen=1`, `newhavenTutorialHunting=1`, `newhavenNewLootTheCorruptor=1`. W Oteryn zadanie `NEWHAVEN-TELEPORT-1` wdrożyło samo przejście i wyłączyło z zakresu te efekty.

**DERIVED — blokada przechodniości:** i19394 występuje na starcie i na wyjściu, i37001 na wejściu portalu. Żaden z tych dwóch kluczy nie ma profilu Item w `content/items/definitions/spell-native-profiles.json`. `node/serve.rs::boot_bundle_items` pobiera solidity z tego profilu; `map/facts.rs::palette_facts` blokuje nieznany zwykły Item, z wyjątkami tylko dla rozpoznanego terenu/floor-change. Samo `WorldObject.collision.blocks_movement=false` nie zasila tej ścieżki. Archiwalne zadanie wprost odnotowuje nierozwiązaną blokadę wyjścia przez i19394. Brak aktualnego testu przejścia na realnym bundle; nie oznaczać portalu jako działającego E2E.

Dowody wykonania: `apps/game-server/src/node/serve.rs::boot_bundle_items`, `apps/game-server/src/map/facts.rs::{palette_facts,terrain_floor,walkable_floor_change}`, `apps/game-server/src/map/floor.rs`, `docs/agents/tasks/archive/NEWHAVEN-TELEPORT-1.md`.

### Stosy na sprawdzonych kratkach (PROVEN)

| Punkt | Pozycja | Przedmioty / atrybuty | Plik |
| --- | --- | --- | --- |
| corruptor_spawn | [32614, 32422, 7] | 16484, 2518 | `content/world/placements/region-z07-x127-y126.b3` |
| hunting_west | [32561, 32485, 7] | 4515, 13765 | `content/world/placements/region-z07-x127-y126.b3` |
| hunting_east | [32562, 32485, 7] | 103, 4532, 4534, 4533, 8812, 13764 | `content/world/placements/region-z07-x127-y126.b3` |
| gustavo_anchor | [32560, 32488, 7] | 4515 | `content/world/placements/region-z07-x127-y126.b3` |
| flavius_anchor | [32566, 32489, 7] | 408 | `content/world/placements/region-z07-x127-y126.b3` |
| anna_anchor | [32552, 32497, 7] | 408, 19394 | `content/world/placements/region-z07-x127-y126.b3` |
| ferry_door_destination | [32555, 32497, 7] | 408, 19394 | `content/world/placements/region-z07-x127-y126.b3` |
| avriel_anchor | [32554, 32505, 7] | 486 | `content/world/placements/region-z07-x127-y126.b3` |
| viola_anchor | [32571, 32507, 7] | 4525 | `content/world/placements/region-z07-x127-y126.b3` |
| seagull_anchor | [32535, 32512, 7] | 4405, 618, 19394 | `content/world/placements/region-z07-x127-y127.b3` |
| return_hole | [32533, 32513, 7] | 4403, 51921 | `content/world/placements/region-z07-x127-y127.b3` |
| start | [32534, 32513, 7] | 4403, 6378, 19394 | `content/world/placements/region-z07-x127-y127.b3` |
| portal_source | [32538, 32514, 7] | 4515, 37001 {"teleport": [32544, 32515, 7]} | `content/world/placements/region-z07-x127-y127.b3` |
| portal_destination | [32544, 32515, 7] | 870, 4539, 19394 | `content/world/placements/region-z07-x127-y127.b3` |

### Znaczniki oryginalnego portalu

| Opis | Pozycja | Typ |
| --- | --- | --- |
| Avriel’s Shop | (32554,32507,7) | 11 |
| Spells | (32570,32503,7) | 3 |
| Ferry House | (32553,32497,7) | 9 |
| Bank | (32567,32490,7) | 13 |
| Corrupted Mines | (32595,32458,7) | 8 |
| Muglex Clan Camp | (32526,32430,7) | 8 |

## Mapa i zakres importu

**PROVEN:** `content/world/areas/islands/islands-00000-00059.json`, `oteryn:area.island.newhaven`: główny komponent powierzchni x=32506..32634, y=32405..32522, z=7, 9189 kratek; mała wyspa x=32532..32539, y=32511..32518, z=7, 50 kratek. To komponenty obszaru, nie liczba wszystkich kratek w prostokącie. Stary `component_note` o niezatwierdzonym teleporcie jest nieaktualny względem obecnego rekordu przejścia.

Odczytano B3 dla prostokąta x=32506..32634, y=32405..32522 na wszystkich dostępnych piętrach:

| z | Liczba zapisanych kratek w prostokącie |
| --- | --- |
| 4 | 1121 |
| 5 | 1816 |
| 6 | 3169 |
| 7 | 15222 |
| 8 | 9070 |
| 9 | 13292 |
| 10 | 13312 |
| 11 | 10155 |
| 12 | 10709 |
| 13 | 13891 |

W tym prostokącie jest **329 różnych prowizorycznych kluczy palety**, łącznie 8624 wpisów stosu. To wskaźnik niepełnego kwalifikowania importu, a nie lista brakujących kratek ani dowód, że wszystkie te obiekty należą do tutoriala. Nie zmieniać zbiorczo unknown na walkable.

Nie znaleziono na zeskanowanych stosach regionalnych i44774 (Book of Wisdom używanego przez źródłowy skrypt Violi) ani i3052 (nagroda Gustavo). Brak przedmiotu na ziemi nie dowodzi braku sposobu jego uzyskania: wymaga osobnego powiązania kontenera, lootu lub nagrody.

## NPC, rozmieszczenia i usługi

**PROVEN:** wszystkie sześć NPC ma definicję, dialog i zachowanie, ale żaden nie występuje w `content/world/npc-placements/placements-00000-00024.json`. Katalog zawiera 25 innych NPC; `held.json` nie zawiera oczekujących wpisów. Brak rozmieszczenia uniemożliwia uznanie rozmów za dostępne w świecie tylko na podstawie katalogu.

| NPC | Pozycja źródłowa | Usługa w katalogu Oteryn | Brak do realizacji |
| --- | --- | --- | --- |
| a_seagull | spawn (32535,32512,7), wiki (32536,32513,7); r=3 | brak przypisanej | Opcjonalny jednokierunkowy transport Rookgaard, potwierdzenia |
| anna | spawn (32552,32497,7), wiki (32553,32497,7); r=2 | brak przypisanej | Transport + próg 8 + wybór zgodnego z filmem celu |
| avriel | (32554,32505,7); r=2 | trade.avriel | Spawn, aktywacja handlu i kwalifikacja ofert/przedmiotów |
| flavius | (32566,32489,7); r=2 | brak przypisanej | Spawn i usługa bankowa (obecna usługa bankowa katalogu dotyczy Pauliego) |
| gustavo_the_guard | (32560,32488,7); r=2 | brak przypisanej | Spawn, quest 20 Muglexów, licznik i nagroda |
| viola | (32571,32507,7); r=2 | brak przypisanej | Spawn, nauka czarów, quest książki i nagroda |

Rekordy NPC i dialogów do odczytu:

| NPC | Definicja | Dialog | Zachowanie |
| --- | --- | --- | --- |
| a_seagull | `content/npcs/definitions/npcs-00000-00499.json/records/35` | `content/dialogues/definitions/dialogues-00000-00499.json/records/18` | `content/behaviors/behaviors-01500-01999.json/records/398` |
| anna | `content/npcs/definitions/npcs-00000-00499.json/records/105` | `content/dialogues/definitions/dialogues-00000-00499.json/records/62` | `content/behaviors/behaviors-01500-01999.json/records/468` |
| avriel | `content/npcs/definitions/npcs-00000-00499.json/records/133` | `content/dialogues/definitions/dialogues-00000-00499.json/records/83` | `content/behaviors/behaviors-01500-01999.json/records/496` |
| flavius | `content/npcs/definitions/npcs-00000-00499.json/records/438` | `content/dialogues/definitions/dialogues-00000-00499.json/records/289` | `content/behaviors/behaviors-02000-02499.json/records/301` |
| gustavo_the_guard | `content/npcs/definitions/npcs-00500-00999.json/records/76` | `content/dialogues/definitions/dialogues-00000-00499.json/records/389` | `content/behaviors/behaviors-02000-02499.json/records/439` |
| viola | `content/npcs/definitions/npcs-01000-01281.json/records/211` | `content/dialogues/definitions/dialogues-00500-00835.json/records/290` | `content/behaviors/behaviors-03000-03144.json/records/74` |

### Handel Avriela

Przypięty skrypt Lua zawiera 45 ofert, natywny rekord `content/services/trade/trade-00000-00323.json#/records/27` zawiera 38 ofert. Poniższe pary kierunek/ID z tego Lua nie mają identycznego odpowiednika. Katalog ma też metadane oficjalnego rozstrzygnięcia cen; różnica nie oznacza automatycznie błędu — koordynator powinien wybrać obowiązujące źródło i sprawdzić mapowanie ID.

| Nazwa źródłowa | ID źródłowe | Kierunek | Cena Lua | Cena tego ID/kierunku w Oteryn |
| --- | --- | --- | --- | --- |
| lightest missile rune | 3174 | SellToPlayer | 20 | brak |
| bone sword | 3338 | BuyFromPlayer | 20 | brak |
| fish | 3578 | BuyFromPlayer | 2 | brak |
| mace | 3286 | BuyFromPlayer | 30 | brak |
| rope | 3003 | BuyFromPlayer | 8 | brak |
| short sword | 3294 | BuyFromPlayer | 10 | brak |
| spear | 3277 | BuyFromPlayer | 3 | brak |
| white mushroom | 3723 | BuyFromPlayer | 2 | brak |

### Zadania i nauka u NPC

- Gustavo: start zadania → licznik 0; warunek ukończenia 20 Muglexów; nagroda i3052. Przeszukanie zachowanych Lua/XML znajduje `newhavenMuglexKillCount` tylko w deklaracji storage i w skrypcie Gustavo (odczyt/reset), bez znalezionego producenta przyrostu. **UNKNOWN/źródłowa luka:** nie dopisywać domyślnego hooka jako faktu oryginału.
- Viola: zgoda na odzyskanie Book of Wisdom → flaga startu; oddanie i44774 → usunięcie książki, flaga ukończenia i nagroda zależna od profesji. Sorcerer i3074, Druid i3066, Paladin i3350, Knight i3264, Monk i3290, fallback 50 złota. Książka i44774 nie ma dokładnego klucza definicji w obecnym katalogu Oteryn.
- Viola, deklarowana tabela nauki: Wound Cleansing / exura infir ico (0, lvl 1), Cure Poison / exana pox (150, lvl 10), Magic Rope / exani tera (200, lvl 9), Light / utevo lux (0, lvl 8). Są to deklaracje źródła, nie dowód aktywnej usługi Oteryn ani kompletnej walidacji wszystkich ścieżek potwierdzenia.
- Seagull: opcjonalna podróż do Rookgaard (32097,32219,7), dialog z kolejnymi potwierdzeniami; poza obowiązkową ścieżką filmu.

## Potwory, spawny, loot

Wszystkie sześć sprawdzanych typów ma definicję i tabelę zwykłego lootu. Liczby regionalne dotyczą tylko podanego prostokąta, na wszystkich piętrach; liczby globalne obejmują cały katalog. Obecność danych nie dowodzi dostępności areny, dojścia, działania ataków ani przyznania nagrody E2E.

| Typ | HP | XP | Spawny regionalne | Spawny globalne | Definicja | Loot |
| --- | --- | --- | --- | --- | --- | --- |
| corrupted_skeleton | 45 | 34 | 15 | 15 | `content/creatures/definitions/creatures-00000-00499.json/records/254` | `content/loot/loot-00000-00499.json/records/175` |
| corrupted_ghost | 30 | 29 | 20 | 20 | `content/creatures/definitions/creatures-00000-00499.json/records/253` | `content/loot/loot-00000-00499.json/records/174` |
| muglex_clan_footman | 30 | 24 | 34 | 50 | `content/creatures/definitions/creatures-01000-01499.json/records/89` | `content/loot/loot-00500-00999.json/records/277` |
| muglex_clan_assassin | 45 | 34 | 24 | 54 | `content/creatures/definitions/creatures-01000-01499.json/records/87` | `content/loot/loot-00500-00999.json/records/275` |
| muglex_clan_chief | 100 | 175 | 0 | 0 | `content/creatures/definitions/creatures-01000-01499.json/records/88` | `content/loot/loot-00500-00999.json/records/276` |
| the_corruptor | 100 | 170 | 1 | 1 | `content/creatures/definitions/creatures-01500-01862.json/records/85` | `content/loot/loot-01000-01289.json/records/104` |

**PROVEN:** Muglex Clan Chief nie ma spawnu pod swoim kanonicznym kluczem w żadnym shardzie `content/world/spawns/spawns-*.json`. The Corruptor ma jeden spawn (32614,32422,7), respawn 60 s.

Źródłowy `#ondroploot_the_corruptor.lua` dodaje łup wspólny, a przy uzbrojonej fladze pierwszy łup zależny od profesji, po czym czyści flagę. Zwykła tabela Loot nie zastępuje tej jednorazowej akcji. **Korekta po kontroli kompletności:** nazwa pliku zaczyna się od `#`; przypięty `src/lua/scripts/scripts.cpp:128–136` pomija takie pliki w standardowym automatycznym ładowaniu. Jest to wyłączony fragment referencyjny, a nie dowód aktywnego dodatkowego lootu. Różne ID nagród w skrypcie i tabelach oraz brakujące definicje poniżej wymagają jawnego rozstrzygnięcia, nie kopiowania po numerze.

## Przedmioty: pełny sprawdzony zbiór ID z lokalnych skryptów

Zbiór obejmuje startery, oferty Avriela, loot czterech plików `newhaven_update_2025`, nagrody NPC oraz przedmioty przejść. Dla każdego wykonano dokładne wyszukanie klucza `oteryn:item.tibia.i<ID>` i profilu `production_definition.production_key`. To porównanie **dokładnych ID**, nie semantyczna crosswalk między rewizjami. Brak klucza może oznaczać konieczność mapowania do innego ID, nie brak grafiki.

Wynik: **75 sprawdzonych ID, 68 definicji, 11 profili natywnych**. Identity-only nie oznacza materializowalnego wyposażenia.

| ID | Definicja | Profil natywny |
| --- | --- | --- |
| 266 | materializable=True | brak |
| 268 | materializable=True | brak |
| 285 | materializable=True | brak |
| 2643 | materializable=False | brak |
| 2853 | materializable=True | brak |
| 2854 | materializable=True | obecny |
| 3003 | materializable=True | brak |
| 3031 | materializable=True | obecny |
| 3049 | materializable=True | brak |
| 3052 | materializable=True | brak |
| 3059 | materializable=True | brak |
| 3066 | materializable=True | brak |
| 3074 | materializable=False | brak |
| 3081 | materializable=True | brak |
| 3115 | materializable=True | brak |
| 3174 | materializable=False | obecny |
| 3178 | materializable=False | obecny |
| 3264 | materializable=False | obecny |
| 3272 | materializable=False | brak |
| 3274 | materializable=False | brak |
| 3277 | materializable=False | obecny |
| 3285 | materializable=True | brak |
| 3286 | materializable=True | brak |
| 3290 | materializable=False | brak |
| 3294 | materializable=True | brak |
| 3337 | materializable=False | brak |
| 3338 | materializable=False | brak |
| 3350 | materializable=True | obecny |
| 3355 | materializable=False | brak |
| 3359 | materializable=False | brak |
| 3372 | materializable=True | brak |
| 3375 | materializable=False | brak |
| 3409 | materializable=True | brak |
| 3412 | materializable=True | obecny |
| 3447 | materializable=True | obecny |
| 3462 | materializable=True | brak |
| 3552 | materializable=True | brak |
| 3559 | materializable=False | brak |
| 3561 | materializable=False | brak |
| 3571 | materializable=False | brak |
| 3576 | materializable=False | brak |
| 3578 | materializable=True | brak |
| 3723 | materializable=True | brak |
| 7618 | materializable=False | brak |
| 7620 | materializable=False | brak |
| 7884 | materializable=False | brak |
| 7991 | materializable=False | brak |
| 7992 | materializable=False | brak |
| 8095 | materializable=False | brak |
| 8820 | materializable=False | brak |
| 8871 | materializable=False | brak |
| 10319 | materializable=False | brak |
| 10321 | materializable=False | brak |
| 19394 | materializable=False | brak |
| 21348 | materializable=False | brak |
| 21350 | materializable=False | brak |
| 21351 | materializable=False | obecny |
| 21400 | materializable=False | brak |
| 21470 | materializable=False | obecny |
| 34017 | materializable=False | brak |
| 35562 | materializable=False | brak |
| 37001 | materializable=False | brak |
| 44772 | brak | brak |
| 44773 | brak | brak |
| 44774 | brak | brak |
| 44775 | brak | brak |
| 48843 | brak | brak |
| 48845 | brak | brak |
| 48847 | brak | brak |
| 50166 | materializable=False | brak |
| 50257 | materializable=False | brak |
| 51119 | materializable=False | brak |
| 51760 | materializable=False | brak |
| 51777 | materializable=False | brak |
| 51778 | materializable=False | brak |

Brak dokładnych definicji: i44772, i44773, i44774, i44775, i48843, i48845, i48847. Pełne shard-y definicji: `content/items/definitions/items-*.json`; profile: `content/items/definitions/spell-native-profiles.json`.

### Wyposażenie startowe według źródła

Wspólne sloty: i3355 head, i3561 armor, i3559 legs, i3552 feet, i34017 ammo; plecak i2854, jeśli gracz nie ma plecaka. Przyznanie raz, pod ochroną `newhavenStarterItems`; komunikat: `You received your starter equipment.`

| Profesja | Dodatkowe sloty | Zawartość plecaka |
| --- | --- | --- |
| Sorcerer | i21400 right; i21348 left | i268×10, i266×2, i3031×50, i3578×1, i21351×1 |
| Druid | i21400 right; i21350 left | i268×10, i266×2, i3031×50, i3578×1, i21351×1 |
| Paladin | i3350 left; i35562 right, kołczan: 2×100 i3447 | i268×7, i266×5, i3031×50, i3578×1 |
| Knight | i3272 left; i3412 right | i268×2, i266×10, i3031×50, i3578×1 |
| Monk | i50166 left | i268×5, i266×7, i3031×50, i3578×1 |

## Postęp, śmierć i ograniczenia podróży

Źródłowe storage: citizen=48031; hunting=48032; first Corruptor loot=48033; goblin start=48034; goblin done=48035; kill count=48036; Viola book start=48037; Viola book done=48038; starter items=48039. To historyczne identyfikatory źródła, nie przydział natywnych ID/migracji Oteryn.

Źródłowa śmierć: postać bez profesji i bez access wraca na (32534,32513,7); dla town DAWNPORT lub DAWNPORT_TUTORIAL skrypt wskazuje (32544,32515,7). Nie zastępować tym globalnej obsługi śmierci bez przydziału i uzgodnienia z istniejącym właścicielem.

**PROVEN:** `content/quests/definitions/quests-00200-00299.json#/records/78` (`oteryn:quest.newhaven_quest`) ma `classification=OTS_HYPOTHESIS_ONLY`, `completeness=NOT_ASSESSED`, `native_lowering.state=WAITING_IMPLEMENTATION`, puste canonical_interaction_refs/canonical_progress_refs, oraz zgłoszone 3 luki interakcji i 9 nierozstrzygniętych przedmiotów. Sześć etapów wybranego przepisu nie stanowi wiernej, wykonywalnej realizacji. Oddzielny `Newhaven Legion Helmet Quest` również jest wybraną aproksymacją, a nie dowodem kompletności tutoriala.

## Konflikty i niewiadome do rozstrzygnięcia

| Klasa | Temat | Dowód / skutek |
| --- | --- | --- |
| CONFLICT | Anna: cel podróży | Film: Thais Peninsula. Przypięty donor Anna: Targuna (31938,31928,7), min lvl 8. Nie zastępować filmowego celu tym punktem. |
| UNKNOWN | Dokładna kratka lądowania Thais Peninsula | Film nie pokazuje współrzędnych. Pozycja Hectora nie ustala lądowania. Historyczny wariant Anny (32310,32210,6) także nie został potwierdzony. |
| CONFLICT | Hint polowania | Crystal wywołuje stałe ID 3; enumeracja OTClient: Knight=3, Paladin=4. Film jest ścieżką Paladina. |
| CONFLICT | Komentarz arrival Targuna | movements_main_continent_hint.lua opisuje ID 7, ale wykonuje sendTutorial(13). To inna trasa niż nagrany Thais. |
| UNKNOWN | Czas i próg bannerów | Film pokazuje Managing your riches i Off to New Shores, ale nie dowodzi dokładnego warunku/timera. |
| UNKNOWN | Zliczanie Muglexów i źródło książki | Nie znaleziono producenta przyrostu newhavenMuglexKillCount w zachowanych Lua/XML; samo zerowanie i odczyt u Gustavo nie wystarczają. Brak powiązania otrzymania książki i44774. |
| UNKNOWN | Pełna zgodność geometrii/grafiki | Brak porównania całego oryginalnego OTBM i assetów z bundle oraz testu wszystkich dojść. |

## Zamknięta lista braków dla koordynatora

Poniższe zakresy są propozycją kolejności i kryteriów, **nie nowymi przydziałami ani zgodą na implementację**. Koordynator ma dobrać właścicieli oraz sprawdzić aktualne kolizje owned_paths. Raport nie zmienia briefu Thais ani nie odwiesza #2186.

| Zakres | Klasa dowodu | Co pozostaje | Kryterium sprawdzenia |
| --- | --- | --- | --- |
| 1. Start i dostępność terenu | PROVEN / DERIVED | Wybrać aktywną trasę startową; kwalifikować fakty i19394/i37001 i istotnych dekoracji bez zbiorczego unknown=false | Nowa postać, wejście na portal i poprawne lądowanie na realnym bundle; restart/reconnect |
| 2. NPC i usługi | PROVEN | Sześć rozmieszczeń; transport Anny i Seagull; bank Flaviusa; handel i nauka Violi | Każdy NPC widoczny, dostępny dialog i poprawna usługa; bez duplikatów |
| 3. Tutorial i wyposażenie | PROVEN / CONFLICT | Zgodny tekst/wyzwalacz/kolejność, wybór profesji, starter raz, znaczniki, hunting | Obserwowany Paladin i uzgodnione inne profesje; ponowne wejście nie powiela przedmiotów/postępu |
| 4. Walki i zadania poboczne | PROVEN / UNKNOWN | Brak spawnu Chief; zliczanie Muglexów; książka Violi; pierwsza nagroda Corruptora | Dojście i walka; poprawny licznik/claim; brak ponownej nagrody po reconnect/restart |
| 5. Wyjście i zakończenie | CONFLICT / UNKNOWN | Rozstrzygnąć Thais kontra Targuna; ustalić dokładny cel i reset flag; próg 8 na drzwiach/podróży | Lvl 7 odmowa, lvl 8 poprawna podróż; zgodny ekran przybycia i postęp |
| 6. Kwalifikacja końcowa | UNKNOWN | Rejestrowane przejście całej ścieżki i weryfikacja assetów/dojść | Słowo w słowo teksty, faktyczne wyzwalacze, pozycje i trwałość; ograniczenia nagrania jawne |

## Obserwowany przebieg i teksty z filmu

Poniższe dane przeniesiono z wcześniejszej rekonstrukcji, bez ponownego oglądania. Czasy są obserwacjami z dokładnością około sekundy; nie timestampami pakietów. `PROVEN_VIDEO` w oryginalnym materiale oznacza wcześniejszą obserwację filmu; pozycje są ustalane osobno ze źródeł, a nie z minimapy. Błędy pisowni/formatowania zachowano zgodnie z rekonstrukcją.

### 00:59 — Wysepka startowa (`start`)

Początek pokazanego fragmentu gry; postacie stoją na małej wyspie.

Pozycja źródłowa: `[32534, 32513, 7]`.

Uwaga: Pozycja startu pochodzi ze skryptu logowania. Film zaczyna się już po logowaniu; nie pokazuje całego tworzenia postaci.

### 02:25 — Ustawienie sterowania (`controls`)

Autor otwiera Options → Controls i stosuje ustawienia.

> Mouse Preset: Classic Controls

> Loot: Right

> Use Default Keyboard Delay (250 ms)

> Keyboard Delay: 150 ms

Uwaga: To ręczna konfiguracja autora filmu. Nie jest dowodem obowiązkowego etapu samouczka.

### 02:37 — Portal i wybór profesji (`vocation-window`)

Wejście w portal na wysepce; pojawia się okno wyboru oraz informacja o rozpoczęciu questa.

Pozycja źródłowa: `[32544, 32515, 7]`.

> Quest started

> You have begun "Welcome to Newhaven".

> Your goal: Reach level 8 and head for the main continent.

> Knight

> Resilient Melee Fighter

> Sword, Axe and Club

> Beginner Friendly

> Select Knight

> Sorcerer

> Destructive Magic User

> Fire, Energy and
> Death Magic

> Select Sorcerer

> Druid

> Supportive Magic User

> Ice, Earth and
> Healing Magic

> Select Druid

> Paladin

> Ranged All-Rounder

> Bows, Crossbows and
> Holy Magic

> Select Paladin

> Monk

> Melee All-Rounder

> Fist Fighting and
> Healing Magic

> Select Monk

> Gear

> Spells

> Newhaven

> Welcome to Newhaven

> Reach level 8. Then, talk to Anna in the ferry house.

Uwaga: Portal: (32538,32514,7); punkt przybycia: (32544,32515,7), według skryptu CrystalServer. Kolejność kart: Knight, Sorcerer, Druid, Paladin, Monk.

Uwaga: Gear i Spells pokazują się w rozwijanym panelu karty po najechaniu.

### 04:13 — Wybranie Paladyna (`paladin`)

Wybór Paladin; okno znika, zmienia się wyposażenie i zestaw hotkeyów.

Pozycja źródłowa: `[32544, 32515, 7]`.

> Your vocation has been set to Paladin.

> Switched to hotkey preset 'Paladin'.

Uwaga: Film nie dowodzi osobnego kliknięcia Confirm. Obecny OTClient zawiera taki przycisk, więc nie dopisujemy go do zaobserwowanej ścieżki.

### 04:25 — Rozmowa z Avrielem (`avriel-hi`)

Rozpoczęcie rozmowy ze sklepikarzem.

Pozycja źródłowa: `[32554, 32505, 7]`.

> Welcome to my little shop, Fiz Praensinar! Are you looking to trade anything precious?

Uwaga: Fiz Praensinar to nazwa postaci z filmu; w implementacji jest parametrem player_name.

### 04:25 — Handel (`avriel-trade`)

Wybranie lub wpisanie słowa trade.

Pozycja źródłowa: `[32554, 32505, 7]`.

> trade

> Of course, take a look and see what might interest you.

Uwaga: Otwiera się okno handlu.

### 05:00 — Ponowny handel (`avriel-repeat`)

Ponowne trade w tej samej rozmowie.

Pozycja źródłowa: `[32554, 32505, 7]`.

> trade

> Of course, take a look and see what might interest you.

### 07:22 — Combat Basics — Paladin (`combat`)

Wyjście na północ z osady, obok Gustavo; wejście na pole samouczka walki.

Pozycja źródłowa: `[32561, 32485, 7]`.

> Combat Basics

> Paladin

> Strengths:

> Ranged Combat

> Physical & Holy Damage

> Allrounder

> How to play

> Mountain Troll

> Click on monster
> or use battlelist to
> attack

> Spells

> Magic Patch

> Self

> Healing

> F3

> Lesser Ethereal Spear

> Single Target

> Physical Damage

> F4

> Ok

Uwaga: Dopuszczalne pole wyzwalające ze skryptu: (32561,32485,7) lub (32562,32485,7), gdy flaga hunting > 0; po pokazaniu jest czyszczona.

Uwaga: Grafika hint_04_vocationpaladin.png z OTClient odpowiada oknu filmu. W tabeli OTClient Paladin ma ID 4; badany CrystalServer wysyła stałe 3, dlatego nie wolno utożsamiać ich bez korekty.

### 07:51 — Awans umiejętności (`distance16`)

W trakcie walki rozwija się Distance Fighting.

> Distance Fighting

> Your skill has advanced to level 16

Uwaga: To komunikat systemowy wynikający z treningu, nie obowiązkowy punkt postępu questa.

### 08:07 — Ostrzeżenie o dystansie (`too-close`)

Atak dystansowy na cel stojący obok postaci.

> Target Too Close

> You are using a ranged auto-attack at melee distance.

Uwaga: Ostrzeżenie powtarza się przy podobnym położeniu celu.

### 08:18 — Poziom 3 (`level3`)

Zdobycie doświadczenia w walce.

> Level 3

> You gained hit points, mana, and capacity.

### 11:47 — Poziom 5 (`level5`)

Zdobycie doświadczenia w walce.

> Level 5

> You gained hit points, mana, and capacity.

### 15:21 — Distance Fighting 19 (`distance19`)

Kolejny awans podczas polowania.

> You advanced to distance fighting level 19.

Uwaga: Tekst awansu widoczny w polu gry; toast z poziomem używa tego samego szablonu co poprzednio.

### 16:08 — Poziom 7 (`level7`)

Zdobycie doświadczenia w walce.

> Level 7

> You gained hit points, mana, and capacity.

### 18:32 — Distance Fighting 20 (`distance20`)

Kolejny awans podczas polowania.

> You advanced to distance fighting level 20.

### 20:25 — Cel: wrócić do Anny (`reach8`)

Po osiągnięciu poziomu 8 zmienia się tekst Quest Trackera.

Pozycja źródłowa: `[32553, 32497, 7]`.

> Newhaven

> Welcome to Newhaven

> You reached level 8. Talk to Anna in the ferry house.

Uwaga: Osiągnięcie poziomu 8 jest warunkiem dalszej podróży; gracz z filmu poluje dalej do poziomu 9.

### 22:45 — Poziom 9 (`level9`)

Dalsze polowanie po spełnieniu celu poziomu 8.

> Level 9

> You gained hit points, mana, and capacity.

### 22:59 — Player Guide — kamień milowy (`milestone`)

W trakcie dalszej gry zmienia się panel Player Guide.

> Congratulations!

> Milestone unlocked:

> Managing your riches

Uwaga: Film pokazuje ten stan panelu; nie ustala dokładnego progu złota ani reguły wyzwolenia. Nie należy wymyślać progu na podstawie samego ujęcia.

### 23:24 — Distance Fighting 23 (`distance23`)

Dalszy awans w trakcie polowania.

> You advanced to distance fighting level 23.

### 23:27 — Przypomnienie o wypłynięciu (`new-shores`)

Poziom 9, podczas wychodzenia z rejonu polowania; pokazuje się przypomnienie.

> Off to New Shores

> Leave the village and set sail to start your real adventure.

Uwaga: Treść jest zgodna z katalogiem oficjalnego klienta. Film nie ujawnia dokładnego timera przypomnienia.

### 26:17 — Rozmowa z Anną (`anna-hi`)

Wejście do domu promowego i rozpoczęcie rozmowy z Anną.

Pozycja źródłowa: `[32553, 32497, 7]`.

> Hello, my friend. Do you seek passage to the Mainland, you might also know it as Thais Peninsula.

Uwaga: Pozycja z wiki jest punktem obserwowanym; kotwica spawnu w donorze wynosi (32552,32497,7), a NPC ma promień chodzenia 2.

### 26:22 — Oferta przeprawy (`anna-passage`)

Wpisanie passage.

Pozycja źródłowa: `[32553, 32497, 7]`.

> passage

> I'm offering a ferry service to Thais Peninsula, are you interested?

### 26:29 — Odmowa (`anna-no`)

Wybranie no.

Pozycja źródłowa: `[32553, 32497, 7]`.

> no

> You want to stay here for a little longer? All right, return if you are ready for a passage.

Uwaga: Pozostajesz na Newhaven; odpowiedź nie kończy questa.

### 26:33 — Ponowienie prośby (`anna-repeat`)

Ponowne wpisanie passage.

Pozycja źródłowa: `[32553, 32497, 7]`.

> passage

> I'm offering a ferry service to Thais Peninsula, are you interested?

### 26:38 — Zgoda i odpłynięcie (`anna-yes`)

Wybranie yes po ponowionej ofercie, przy poziomie co najmniej 8.

> yes

> Very well, I'll take you to the Thais Peninsula then. Don't forget to talk to Hector at the port for directions!

Uwaga: Dłuższa wersja zdania z at the port jest potwierdzona historycznym skryptem Anny i odczytami OCR filmu. Ujęcie należy zachować do kontroli tekstu.

Uwaga: Miejsce docelowe w filmie to Thais Peninsula. Nowszy donor kieruje do Targuny; starszy donor podaje (32310,32210,6), które nie pasuje do mapy filmu. Żadnej z tych kratek nie przyjmujemy jako poprawnego lądowania z nagrania.

### 26:38 — Zakończenie questa (`quest-complete`)

Po zaakceptowaniu przeprawy pojawia się baner kończący quest.

> Quest completed

> You have finished "Welcome to Newhaven (completed)".

### 26:38 — Arrival in Thais (`arrival-thais`)

Po przeniesieniu na Thais Peninsula pojawia się mapa orientacyjna.

> Arrival in Thais

> Travel
> around

> Manage
> your loot

> Learn new
> Spells

> Wolf
> Den

> You are
> here

> Ok

Uwaga: Grafika hint_08_arrivalmainpaladin.jpg z OTClient odpowiada oknu w filmie.

Uwaga: Mapa wskazuje położenie gracza, port, handel, naukę zaklęć i Wolf Den. Nie zawiera liczbowych współrzędnych.

### 27:06 — Hector the Mentor (`hector`)

Postać przechodzi przy Hectorze, którego wskazała Anna.

Pozycja źródłowa: `[32474, 32288, 7]`.

> Hector The Mentor

Uwaga: Pozycja pochodzi ze snapshotu wiki. W pokazanym fragmencie nie ma rozmowy z Hectorem, więc nie dopisujemy jego dialogów do ścieżki filmu.

## Manifest źródłowy (zachowane referencje)

| Ścieżka CrystalServer | Git blob SHA |
| --- | --- |
| data-global/lib/core/storages.lua | af3e5b1317ef73e2eb6e870835a785f2cf10212a |
| data-global/lib/others/load.lua | 22e0ef0346ee0aba252a8979533cba3a42f6adbf |
| data-global/lib/others/newhaven.lua | 57ad7e825a4ec059d2fc565f4c7436b632b301a9 |
| data-global/lib/tables/town.lua | 3c6ce626adc8abb1161f8552972cb2d515bb1892 |
| data-global/monster/newhaven_update_2025/corrupted_skeleton.lua | 25aaaadf4bd46ba461e0de6bef0f1945fbaaa717 |
| data-global/npc/a_seagull.lua | e5519f2e061668a205d18a0cea24a7a9e0b61beb |
| data-global/npc/anna.lua | 84a187502960e4517d7d28ee328b9618018a4241 |
| data-global/npc/avriel.lua | 07618e3f310a28c4e46e10642ffca92bd8bd99a8 |
| data-global/npc/flavius.lua | 5743887b3b10a0d057ebd6af4080589293057491 |
| data-global/npc/gustavo_the_guard.lua | 1d363da9b905c9b0ccc5723bda6eea177af6664d |
| data-global/npc/viola.lua | 7892070bc6002c369cc12da82d196d1d3dcce47e |
| data-global/scripts/quests/newhaven/#ondroploot_the_corruptor.lua | a6984946eeeda31766f9fc31d1008017ac557b65 |
| data-global/scripts/quests/newhaven/movement_init_newhaven.lua | 9d4adb9b1f28d31016c1dbaef68b53e935416108 |
| data-global/scripts/quests/newhaven/movement_select_vocation.lua | 8366ff4e96404ca207aa731525c2755e4d974f5e |
| data-global/scripts/quests/newhaven/movement_update_tutorial_hunting.lua | 4c3cb41f6a4f98f1dd316c9e3669ae4b7f612d1f |
| data-global/scripts/quests/newhaven/on_death_newhaven.lua | 680ec85a9301d560a9c0b64ce67126abda169842 |
| data-global/scripts/quests/newhaven/on_login_newhaven.lua | b101507cb10357e076cd8759dc3bde35a3ff5615 |
| data-global/scripts/quests/newhaven/on_use_newhaven.lua | b11e9c6a528e3ab5522740e401bfedf3d84d23c8 |
| data-global/scripts/quests/targuna/movements_main_continent_hint.lua | eb5e69830f683706f3989ba3a7dff3feac36ed84 |

Manifest obejmuje 19 plików z tekstowymi referencjami Newhaven. Dodatkowo bezpośrednio odczytano cztery definicje potworów w `data-global/monster/newhaven_update_2025/` (Chief, Corruptor, corrupted ghost, corrupted skeleton); część nie zawiera słowa Newhaven w treści.

## Odtwarzalność i granice weryfikacji

1. Z przypiętego drzewa w korpusie rozwiązano ścieżki do SHA blobów; pojedynczym przebiegiem tar odczytano ich zawartość i referencje Newhaven w zachowanych Lua/XML.
2. JSON odczytano parserem; porównano dokładne klucze NPC, dialogów, zachowań, usług, przedmiotów, profili, questów i spawnów. Puste usługi na NPC sprawdzono też w jego deklaracji.
3. B3 odkodowano repozytoryjnym `tools/content-schema/world-authoring/world_region_codec.py::decode_region`; indeksy rozwiązano przez `content/world/placements/index.json`. Sprawdzono portal, stosy POI, drzwi i regionalne wystąpienia wskazanych ID.
4. Różnicę między danymi przejścia a wykonywaniem oceniono przez odczyt kodu boot/map i zadania NEWHAVEN-TELEPORT-1. Nie wykonywano buildów Rust ani gry; testów E2E nie zaliczono na podstawie danych.
5. Kontrole dokumentacji i dokładny opublikowany head są raportowane w PR/handoffie. Brak zmian w runtime/content/pinach oraz brak płatnego review.

## Ponowna kontrola kompletności — 2026-10-19

Właściciel poprosił o sprawdzenie, czy zbadano wszystko. **Nie ma podstaw do oznaczenia całego tutoriala jako sprawdzonego w 100%.** Poniższa kontrola rozszerza dowody źródłowe, ale nie zastępuje testu gry ani ponownego odczytu każdego kadru filmu.

### Dodatkowo zamknięte sprawdzenia

- **PROVEN — pokrycie korpusu:** przypięte drzewo CrystalServer deklaruje 8243 ścieżki Lua/XML; blob każdej z nich jest obecny w zachowanym archiwum (8243/8243, brak brakujących ścieżek). Wszystkie zostały ponownie przeskanowane pod kątem nazwy licznika Muglexów, numeru storage 48036, książki 44774 i specjalnych nagród 48843/48845/48847. Jest to pokrycie wyszukiwania tekstowego, nie deklaracja semantycznego przeanalizowania wszystkich 8243 plików.
- **PROVEN — wyłączony dodatkowy loot:** `src/lua/scripts/scripts.cpp`, Git blob `d17b50e14dbe15b05c9c1a7fe37663fcb0d8301a`, funkcja `Scripts::loadScripts`, linie 128–136: nazwa zaczynająca się od `#` powoduje `continue`. `#ondroploot_the_corruptor.lua` jest więc pomijany przez tę ścieżkę. Nie znaleziono jawnego odwołania aktywującego go w przeskanowanych Lua/XML. Samo istnienie callbacku nie jest wymaganiem przeniesienia jego skutków do Oteryn.
- **PROVEN — siedem nierozwiązanych ID ma problem również u dawcy:** sprawdzono 75 ID w pełnym `data/items/items.xml`, uwzględniając zarówno `id`, jak i zakresy `fromid..toid`. Brakuje 44772, 44773, 44774, 44775, 48843, 48845 i 48847. To dokładnie te ID, dla których brakowało kluczy definicji Oteryn. Nie wolno kwalifikować tej różnicy jako samego błędu importu; źródłowy skrypt wymaga naprawy/mapowania lub odrzucenia jako wzorca. XML: Git blob `0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f`, SHA-256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`.
- **PROVEN — nazwy istotnych obiektów:** źródłowe XML opisuje 19394..19396 jako `RESERVED SPRITE`, 37001 jako `magic forcefield`, 51777..51778 jako `sealed door`, a 3052 jako `life ring`. Komentarz Lua o „Ring of Healing” nie zmienia tożsamości i3052.
- **DERIVED — licznik Muglexów nadal bez znalezionego producenta:** wyszukanie nazwy storage znajduje tylko deklarację i alias u Gustavo (którego użycia odczytują/zerują licznik). Jedyny dodatkowy literal `48036` w XML jest identyfikatorem przedmiotu `naga archer soul core`, a nie zapisem storage. Nie dowodzi to niemożliwości dynamicznego użycia obliczonego numeru; takiego połączenia nie ustalono.

### Co nadal nie jest dowiedzione

| Pytanie właściciela | Stan dowodu |
| --- | --- |
| Każdy komunikat słowo w słowo, bez żadnego pominięcia | 101 wystąpień w zachowanej rekonstrukcji; skan co sekundę nie wyklucza krótszych komunikatów ani pominięć OCR. Nie wykonano ponownej kontroli wszystkich klatek. |
| Dokładnie po jakiej akcji i z jakim opóźnieniem | Część reguł ze skryptów ustalona; dokładne timery i progi dwóch bannerów nadal nieustalone. |
| Cała kolejność, również inne profesje i odgałęzienia | Nagranie dokumentuje ścieżkę Paladina. Inne profesje, Rookgaard i zadania poboczne nie mają pełnego nagranego przejścia. |
| Wszystkie pozycje i dojścia | Start, portal i wskazane POI ustalone źródłowo; dokładna kratka lądowania Thais Peninsula nadal nieznana. Dojścia nie zostały przegrane. |
| Każda kratka mapy odpowiada oryginałowi | Zbadano regionalne B3 i katalog obszarów; brak porównania pełnego oryginalnego OTBM z wynikiem importu. Oryginalny `world.otbm` ma 52267895 bajtów, Git blob `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`; nie jest zawarty w użytym archiwum korpusu skryptów. |
| Wszystkie grafiki, animacje i interfejs klienta | Nie przeprowadzono kompletnego porównania ani kontroli wizualnej w działającym kliencie Oteryn. |
| Wszystko działa po restarcie, reconnect i śmierci | Brak przejścia E2E; same definicje i testy ogólnego teleportu tego nie dowodzą. |

**Dyspozycja dla koordynatora:** raport jest inwentaryzacją z jawnymi lukami, nie certyfikatem kompletności ani gotowości do gry. Ustalenia dotyczące wyłączonego lootu i brakujących ID źródła supersedują wcześniejsze traktowanie ich jako jedynie niepotwierdzonych efektów portu. Implementacja pozostaje nieprzydzielona.

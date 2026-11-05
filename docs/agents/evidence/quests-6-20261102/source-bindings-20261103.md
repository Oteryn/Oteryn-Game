# Partia 6 — uzupełnienie źródeł mapy i NPC (2026-11-03)

## Zakres i wynik

To uzupełnienie raportu `OTV2-20261102-quests-6-20261102.md`, nie aktywacja questów. Odniesienie Oteryn: `b595422d912bbfdc8447f6e31053a1fc322fbe44`; donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Klasa dowodów: **OTS_HYPOTHESIS_ONLY**. Fakty o przypiętych plikach są potwierdzone; nie stanowią pomiaru Tibia Global ani produkcyjnego powiązania w Oteryn.

Ponowna praca obejmowała trzech czytających agentów z rozłącznymi zbiorami po siedem questów. Lead zweryfikował ich wskazania bezpośrednio w źródłach. Nie przeniesiono automatycznie wniosków agentów do danych.

**Nowy materiał:** źródłowe wpisy startowe dla wszystkich 67 callbacków z niezakwalifikowanym produkcyjnym UID/AID, siedmiu deskryptorów skrzyń i 13 wystąpień 12 potrzebnych NPC. Każdy odczytany plik donora pobrano z obiektu Git przypiętego commita (`git --no-replace-objects show <pin>:<ścieżka>`), sprawdzono SHA-1 blobu oraz zapisano SHA-256 zawartości. Pakiet maszynowy: `source-bindings-20261103.json.gz`.

| Zakres | Callbacki z wpisem startowym donora |
|---|---:|
| Pits of Inferno | 30 |
| Inquisition | 19 |
| Rookie Guard | 13 |
| Shadows of Yalahar | 2 |
| Waterfall | 3 |
| Razem | 67 |

Osobno sprawdzono deskryptory `ChestUnique` o UID 6052, 6053, 6054, 6075, 6157, 6158 i 6258. To siedem deskryptorów, nie siedem nowo dopuszczonych nagród. Hold Desert2/6158 pozostaje bez zmian.

## Skąd biorą się UID/AID

W tym pinie Crystal mapę uzupełnia rzeczywisty startup: `data-global/scripts/globalevents/others/map_attributes_loader.lua:1-23` wywołuje `loadLuaMapAction` i `loadLuaMapUnique` dla 20 tabel. `onStartup` wywołuje ten mechanizm w `:63` przez `loadMapAttributes` (`:39`); rejestracja zdarzenia jest w `:60-68` tego samego pliku. Listę rzeczywistych wywołań i numery linii zachowuje `startup_calls` w pakiecie.

`data-global/startup/others/functions.lua:49-126` wyszukuje obiekt na polu, a `:260-266` nadaje `ITEM_ATTRIBUTE_UNIQUEID` albo `ITEM_ATTRIBUTE_ACTIONID`. `findAttributeItem` (`:271-305`) może również przeszukiwać zawartość pojemników. Źródłowy wpis startowy nie dowodzi więc automatycznie zgodności z wymaganym przez Oteryn obiektem najwyższego poziomu. Przypadek `itemId=false` ma odmienną semantykę: obejmuje top-down, top-top i ground; nie wybieramy arbitralnie jednego obiektu.

Dla każdego callbacku pakiet podaje dokładny symbol tabeli, UID/AID, ścieżkę i linię wpisu, itemId, wszystkie literalne pozycje oraz powiązany caller startup. Przykład: `data-global/startup/tables/item.lua:2102` opisuje AID 28810 z itemId 353; `data-global/startup/tables/chest.lua:970` opisuje UID 6052 z itemId 2472 na `(33295,31658,13)`. Wszystkie odnośniki w tej sekcji dotyczą niezmiennego pinu Crystal powyżej.

Nie pominięto błędów parsera: wszystkie 20 wywoływanych tabel zostało odczytanych, bez brakujących tabel. Nieselekcjonowany UID 6335 występuje dwukrotnie w `ChestUnique`; nie rozstrzygamy go i nie importujemy. Sprawdzanie odtwarzalności odrzuca duplikat klucza wybranego do tego pakietu.

## Sprawdzenie rzeczywistej mapy donora

Mapa `data-global/world/world.otbm` ma blob Git `e61658ec0ab7b70ec35025ae969426d6fc57ae75` i SHA-256 `c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`. Użyto istniejącego `tools/content-schema/world-authoring/otbm_reader.py`, bez uruchamiania Lua i bez zapisu mapy.

Pełny przebieg struktury mapy objął 171381 obszarów i 19492788 pól. Wszystkie **92 różne badane pola** istnieją. Atrybuty przedmiotów zdekodowano tylko dla tych pól; nie wystąpił tam nieznany typ atrybutu. Nie jest to deklaracja kompletnego odczytu semantyki atrybutów całej mapy.

**83 sprawdzenia literalnego itemId** wykazały dokładnie jeden zgodny obiekt najwyższego poziomu. Liczba ta zlicza sprawdzenia wpis–pozycja, nie unikalne obiekty: współdzielony AID Waterfall występuje przy kilku callbackach. **Cztery sprawdzenia z `itemId=false`** zachowują dynamiczny wybór obiektu i nie są zakwalifikowane jako pojedyncze cele native. Surowe wiersze wybranych pól, głębokości i atrybuty są w `donor_map.selected_tiles`.

To mapa **donora**, a nie ponowny skan skompilowanego WorldBundle Oteryn. Wcześniejsze 19373519 pól surowego snapshotu Oteryn i obecne 19492788 pól donora nie są tą samą próbą; nie wyrównujemy tych liczb i nie zastępujemy nimi wcześniejszego dowodu.

## Źródłowe rozmieszczenie NPC

W `data-global/world/world-npc.xml` znaleziono wszystkie 12 nazw wcześniej wskazanych jako nieobecne w katalogu 33 placementów Oteryn. Telas występuje dwa razy; zachowano oba wpisy, bez arbitralnego wyboru. Poniższe pozycje są wyliczone zgodnie z `src/creatures/npcs/spawns/spawn_npc.cpp:49-54,87-96`: środek plus offset x/y, z ze środka. Atrybut z dziecka nie zastępuje z środka.

| NPC | Pozycja źródłowa x,y,z | Linie wystąpienia nazwy w XML |
|---|---|---|
| Telas | 33276,31800,6 oraz 33273,31788,11 | 619, 2913 |
| Golem Servant | 32548,31254,9 | 2621 |
| Milos | 33258,31823,3 | 118 |
| Boveas | 32431,32090,15 | 3300 |
| Cerdras | 32301,31815,6 | 634 |
| Dreadeye | 32800,31572,14 | 3174 |
| Scutty | 32574,31915,12 | 3012 |
| Ceiron | 32665,31717,7 | 1520 |
| Ustan | 32580,32757,6 | 1013 |
| A Majestic Warwolf | 33355,31991,8 | 2519 |
| Elane | 32342,32235,7 | 1976 |
| Legola | 32302,31782,7 | 1592 |

Legola ma środek x=32303 i offset x=-1. Pakiet zachowuje oryginalne atrybuty XML i ścieżkę każdego wystąpienia, a nie tylko wyliczoną pozycję. Są to źródłowe kandydatury placementów, bez dopuszczenia kanonicznej tożsamości NPC, dialogów, grafiku czy spawnu w Oteryn.

## Odrzucone wskazanie i niedostępne źródło

Sugestia jednego agenta, że ring349 jest cytowany z niewłaściwego sharda, nie potwierdziła się. W `Oteryn/Oteryn-Game@b595422d912bbfdc8447f6e31053a1fc322fbe44:content/items/definitions/items-21000-21499.json:1` wskaźnik JSON `/records/154/definition/identity/key` zawiera dokładnie `oteryn:item.tibia.i349`. Definicja ma `materializable=false`, `stack_class=Unknown`; pełny odczyt `spell-native-profiles.json` nie znalazł profilu i349. Zachowano istniejącą tożsamość i hold, bez duplikatu ani zmiany admission. Numery w nazwie sharda nie są przedziałem source itemId.

Jedno zwykłe zapytanie MediaWiki API z komputera właściciela, obejmujące 21 revid z S7 raportu, zwróciło **HTTP 403** o `2026-11-03T08:17:38.598270+00:00`. Bez ponowienia, HTML, cookies, logowania lub zmiany User-Agent. Łącznik Browser nie miał aktywnego kontekstu. Historyczne revid i opisane wcześniejsze odpowiedzi 200 zachowano; nie przedstawia się ich jako ponownie pobranych ani jako aktualnie odtworzonych raw captures. Pełny nowy capture Wiki pozostaje niedostępny.

## Odtwarzanie i granice

Z głównego katalogu repozytorium:

```sh
python3 docs/agents/evidence/quests-6-20261102/source_bindings_check.py
```

Domyślnie sprawdzanie używa istniejącego lokalnego repozytorium `~/oteryn-cache/ref/crystal-git`. W innym środowisku wskaż repozytorium argumentem `--donor /sciezka/do/crystalserver`; jego checkout nie musi odpowiadać pinowi, bo bajty pochodzą wyłącznie z obiektów przypiętego commita.

Sprawdzenie weryfikuje SHA-1 blobów, SHA-256 i rozmiary źródeł, odtwarza wpisy startowe i NPC, odczytuje rzeczywiste pola OTBM i porównuje projekcję z pakietem. Wymaga lokalnego donora z obiektem wskazanego commita. Nie pobiera sieci, nie wykonuje Lua i nie zapisuje treści gry. Python i narzędzia repo zachowują własne wymagania środowiskowe; wynik walidacji dokładnego nowego head jest osobno w PR.

**Zamknięto lukę w materiałach źródłowych**, nie zależności runtime. Utrzymano ANSWER6589532280 pkt5/7/8, ANSWER6589908362 oraz ANSWER6590303753. Nie zmieniono `content/`, kodu gry, UID/AID mapy, holdów itemów, limitu8 ani flag aktywacji. Wszystkie 87 zakresów Left out pozostają; pozycje i obiekty są teraz jawnie dostępne do odrębnej kwalifikacji przez właścicieli mapy/NPC. DRAFT, bez merge i deploy.

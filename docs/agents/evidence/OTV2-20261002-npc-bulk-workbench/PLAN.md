# Plan masowego uzupełniania NPC

Decyzja właściciela z 2026-10-02: maksymalnie uzupełnić NPC z Canary, Crystal/summer-update i wiki, oznaczyć przybliżenia oraz braki, uruchamiać użyteczną wersję i poprawiać ją stopniowo. Pełna zgodność z real Tibią nie jest warunkiem podstawowej gotowości NPC. To plan pracy i jawna decyzja o jakości danych; nie zmienia protokołu, tożsamości, trwałości transakcji ani uprawnień aktywacji.

Punkt startowy: draft PR #1433 zawiera 1149 deklaracji NPC i 703 Dialogue; na pierwotnej liście160 są27 zakwalifikowane definicje i133 pozostałe postacie. Przygotowanie stanowiska nie dopisuje133 NPC do katalogu i nie uruchamia ich na serwerze.

## 1. Jedno stanowisko, istniejące narzędzia

Używać istniejących `npc_sandbox.py`, `convert.py`, parserów wiki, generatora WorldProject/v2 i walidatorów. Jedna zewnętrzna przestrzeń robocza przechowuje cache źródeł, cele, poprawki, wynik generacji i logi. Pobierać pinned Canary i Crystal tylko raz, przeliczać plik tylko po zmianie jego SHA, wiki czytać dla konkretnych braków. Nie budować drugiego parsera ani alternatywnego silnika gry.

Pierwszy gotowy element: `tools/content-schema/npc-authoring/bulk_workbench.py` przygotowuje kolejkę z istniejącego ledgeru, sprawdza klucze/flagi i uruchamia istniejące testy authoringu. Wyniki zapisuje poza repozytorium. To stanowisko przygotowania i walidacji danych; nie działający serwer gameplay.

```sh
python tools/content-schema/npc-authoring/bulk_workbench.py prepare --workdir /path/outside-repository/npc-work
python tools/content-schema/npc-authoring/bulk_workbench.py check --workdir /path/outside-repository/npc-work
python tools/content-schema/npc-authoring/bulk_workbench.py smoke --workdir /path/outside-repository/npc-work
```

Uruchamiać przez interpreter środowiska z zależnościami istniejącego authoringu, w szczególności jsonschema. Aktualne lokalne stanowisko: `/workspace/npc-bulk-workbench`; interpreter: `/workspace/npc-r7-work/presentation/venv/bin/python`.

## 2. Uzupełnianie, zamiast blokowania całej postaci

Każde pole ma wartość, pochodzenie i jakość. Wykorzystywać kompletny, spójny wariant jednego donora; przy różnicach zachować alternatywę i flagę konfliktu, zamiast mieszać niezgodne części skryptów. Wiki uzupełnia nazwę, rolę, rozmowy, ofertę i położenie. Bieżącego dopasowania pikseli wymagać dla deklaracji `verified`, nie jako warunku użycia jawnie przybliżonego wyglądu.

| Flaga pola | Znaczenie |
|---|---|
| `verified` | Zachować wcześniej zakwalifikowane dane i ich dowody. |
| `donor` | Dane z konkretnego pinned Canary/Crystal; dopuszczalna różnica względem real Tibii. |
| `defaulted` | Jawny default silnika lub wybrana reguła projektu, z opisem; nie udawać faktu o aktorze. |
| `placeholder` | Tymczasowy wygląd, pozycja testowa lub podstawowa odpowiedź, z jawnym oznaczeniem. |
| `todo` | Brakująca funkcja; blokuje tylko tę funkcję. |

Przykłady: brak kwalifikacji koloru nie zatrzymuje NPC z wyglądem donora; brak radius przy znanym silniku może użyć opisanego defaultu; brak Dialogue może użyć neutralnych odpowiedzi Oteryn; brak pozycji może użyć jawnej pozycji na mapie developerskiej. To przybliżenia projektu, nie `verified` ani automatyczne uznanie źródła OTS za Game truth. Nie zmieniać historycznych dowodów ani poluzowywać testów dla istniejącego trybu verified; nowy tryb musi jawnie przenosić jakość danych.

Brak questa, waluty tokenowej lub specjalnego efektu nie zatrzymuje wyglądu i podstawowej rozmowy. Oznaczyć brak i wyłączyć tylko niewspieraną akcję. Aktywne oferty nadal wymagają prawidłowych Item keys, waluty i jednostek, rozsądnych cen oraz spójności kupna/sprzedaży; podróż wymaga istniejącego, poprawnego celu. Nie zamieniać nieznanych tokenów na złoto ani nie wykonywać opaque callbacków Lua jako natywnego kodu.

## 3. Trzy paczki rzeczywistego uzupełniania

Pozostałe133 cele dzielić na45/45/43. Trzech wykonawców dostaje rozłączne klucze NPC i uzupełnia komplet podstawowych pól oraz możliwe funkcje; czwarty scala i generuje, piąty sprawdza brak duplikatów, flagi, handel i podstawowy scenariusz. Jeden writer zapisuje wspólny wynik do gałęzi. Nie tworzyć osobnego materializera, protokołu ani kompletu review dla każdego NPC.

Najpierw przejść wszystkie133 postacie, budując podstawowe, oznaczone wersje. Kosztowny research specjalnych questów lub dokładnego wyglądu trafia do backlogu po tej rundzie. Pierwszy etap to jednorazowe dodanie jawnego profilu przybliżeń do danych; następnie przepuszczać duże paczki przez ten sam generator.

Cel pierwszej paczki:45 danych NPC doprowadzonych do podstawowej gotowości albo jawnego konkretnego błędu strukturalnego. To cel implementacyjny, nie liczba wyszukanych stron ani obietnica już działających45 NPC. Rozróżniać liczniki `imported`, `data_ready_partial`, `runtime_loaded`, `interaction_smoke_passed`, `disabled_features` i `todo_fields`.

## 4. Testy danych oraz rzeczywisty serwer developerski

Po każdej paczce: walidacja kluczy/referencji/flag, zbieżności generacji i nowych ofert/tras; testy tylko zmienionych funkcji. Generator budować raz, używać już istniejącego trybu qualified predecessor, a cały historyczny pipeline wykonywać dla zmiany generatora lub zamknięcia wspólnego releasu. Nie powtarzać całego source researchu i full-server builda dla każdej postaci. Wymagane kontrole repozytorium pozostają obowiązkowe.

Oddzielny licznik testów gameplay wymaga realnej ścieżki Oteryn: jedna mała mapa developerska, wszystkie NPC paczki ustawione w czytelnej siatce, stały zestaw kont/postaci testowych, reset danych lokalnych i jeden wspólny scenariusz: spawn, wygląd, hi/name/job/bye, dostępne trade, wspierana travel, zapis/reconnect dla włączonych transakcji. Nieobecne funkcje mają flagi i osobny wynik, nie udawane PASS.

Istotna zależność potwierdzona w tej gałęzi: deklaracje NPC są candidate-only; nie znaleziono jeszcze implementacji natywnej obsługi NPC talk/trade/travel. Dokument `NPC0-NPC-RUNTIME-SERVICE-V1` ma status CANDIDATE i opisuje te dzieci. Sam katalog nie uruchomi rozmów na serwerze. Przed etapem mapy sprawdzić aktualną chronioną integrację i alokację NPC-CONTENT/NPC-PLACE/NPC-TALK; jeśli brakują, włączyć minimalną zaakceptowaną implementację w istniejący serwer, bez równoległego mini-serwera. Potem wykonywać lokalne testy na tej samej ścieżce runtime, która obsłuży graczy.

Najpierw uruchomienie na serwerze developerskim i test podstawowych interakcji; później zwykły chroniony release. Nie traktować przygotowania cache, przejścia parsera ani flagi `donor` jako dowodu działającego gameplay.

## 5. Kolejność dalszej pracy

1. Gotowe w tym zadaniu: zapis decyzji i planu, wspólna kolejka133, cienki runner istniejących testów, draft PR zabezpieczający pliki.
2. Następne: profil oznaczonych przybliżeń + generator podstawowej wersji pierwszych45 NPC, z flagami pól i niewspieranych funkcji.
3. Kolejne: pozostałe45 i43 przez ten sam generator; jeden wspólny raport danych.
4. Równolegle do importu: rozwiązać realną zależność natywnego NPC runtime i mapy developerskiej; scenariusz wszystkich postaci jednej paczki w jednym miejscu.
5. Po działającym smoke: serwerowy rollout zwykłą ścieżką i późniejsze poprawki z backlogu.

Raportować liczbę faktycznie wygenerowanych NPC, wczytanych NPC i przetestowanych interakcji osobno. Nie przedstawiać rozmiaru grup badawczych jako wdrożonych postaci ani proponowanego środowiska jako już działającego serwera.

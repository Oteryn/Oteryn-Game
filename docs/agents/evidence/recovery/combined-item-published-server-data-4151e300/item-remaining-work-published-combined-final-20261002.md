# Import przygotowanych danych Itemów — 2 października 2026 UTC

**Przygotowane dane zostały włączone do contentu repozytorium serwera i opublikowane w commicie `4151e3005daa41ed848533e45b63fadc5ed78fa3`, [draft PR #1562](https://github.com/Oteryn/Oteryn-Game/pull/1562).** Root ma fazę `VALIDATE`. Pełne bajty 158 plików commitu odpowiadają końcowemu manifestowi i lokalnym plikom. Dane mierzone podczas pierwszej generacji są identyczne z opublikowanymi. Gotowość systemów runtime oraz odczyt danych przez działający proces serwera pozostają nieustalone (`null`).

| Miara | Opublikowany stan |
| --- | ---: |
| Itemy / wszystkie rekordy Native | 34031 /57320 |
| Znane opisy / nazwy | 1629 /12100; 15 nazw poprawionych |
| Całe wektory / atomy modyfikatorów | 511 /846 |
| Właściciele Source ItemAuthoring / Forge | 700 /436 |
| Powiązania BR / źródła relacji / relacje | 454 /539 /752 |
| Profile rodzin / szablony / ograniczenia klas | 22 /23 /22 |

Paczka zmieniła Native na **1655 różnych Itemach**; łącznie z właścicielami Source dotknęła **1859 targetów**. To liczby zmienionych rekordów, nie Itemów całkowicie ukończonych. Zachowano pozostałe pola, tożsamości, admission i World; szczegóły kontroli i autorytatywny Source64v6 z zachowanym lineage v5 są w JSON. Końcowe kontrole pracownika mają status PASS. CI dla dokładnego commitu i integracja w control plane są nadal osobnymi etapami oczekującymi w stanie Root.

**Następna paczka danych FX/audio295 jeszcze nie została zaimportowana.** Propozycja otrzymała niezależny przegląd Source:295 wierszy i1 kontekst jako296 tekstowych stanów istniejącego reimportu. Pozostaje integracja oraz rzeczywiste walidacje parsera/generatora (13→14 paczek, dokładny limit357 stanów). To surowe dowody źródłowe bez nowych assetów, Native efektów lub gotowości runtime.

Pozostałe uzupełnienia danych i kontraktów:

| Pozostająca praca | Zakres po publikacji | Blokada / następny krok |
| --- | --- | --- |
| Maksymalny stack | **2493**, w tym2491 typedUNKNOWN i2 identity-only;0 nowych max | Własny limit albo zaakceptowana reguła bez wyjątków dla tych wariantów; nie default100 |
| Forge | **581** z zachowanej kohorty870 | Dodane289 profili zostało potwierdzone. Dalsze jawne maksima i dokładne powiązania źródeł; tytuł/klasa nie wyznacza max |
| Rodziny | **235** otwartych przypisań w overlay12252/12487 | Własna dokładna kategoria/tożsamość i integracja. Dziesięć szablonów nie klasyfikuje dziesięciu Itemów |
| Regeneracja | **14 wektorów,0 zakwalifikowanych** | 8 braków własnej fazyXML,4 współdzieloneID/fazy,2 sprzeczne/brakujące tuple. Nie przenosić equipTarget ani uśredniać ticków |
| Physical8 | **8 wektorów /16 atomów** | Zaakceptowana decyzja wersji/profilu codec, rzeczywisty input artefaktu i dowód zachowania starego lazyreader. Prototyp nie jest zamknięciem |
| Imbuement | **656** dodatnich slotów;0 zamkniętych list rodzin/tierów | Pełna kwalifikacja dopuszczeń/wykluczeń. Dwa częściowe fakty28715/29427 wymagają osobnego kontraktu Source; nie stanowią whitelisty |
| BR3 / family15 | **3 mosty kategorii BR +15 followupNAV,0 zastosowanych** | Name15 ma opublikowane poprawne nazwy. BR3 ma poprawiony przegląd źródłowy, lecz decyzja bridge nadal nie przyjęta; family15 potrzebuje pełnego DISAMBIG/articlejoin |
| FX/audio | **295 upstreamID**,0 nowych Native pól/assetów | 149 statycznych powiązańItem,141 Worldowner,5 identityholds. Same source-literals pozostają OTS_HYPOTHESIS_ONLY; potrzebne asset/event-purpose/runtime bindings |


Kohorty nakładają się: nie sumujemy ich do liczby nieskończonych Itemów. Wszystkie 22 rodziny mają profile; 23 szablony pokrywają je bez braków. Itemy korzystają z **69 wspólnych shardów liczbowych**, a nie osobnych katalogów każdej rodziny. Zamrożony overlay kategorii12252/12487 i235 otwartych przypisań jest osobną miarą od Native.

| Źródło | Sposób odczytu i ograniczenie |
| --- | --- |
| Pinned CrystalServer /Canary | Zwykły publiczny GitHubHTTP, pełne zachowane pliki i SHA. Zgodność silników sama nie ustanawia GameOwned prawdy |
| TibiaWikiBR Forge | ZwykłyHTTP najpierw; po rzeczywistym403 publiczny Chrome/CDP MediaWikiAPI, zachowane dokładne raw rewizje |
| Fandom własne infoboksy /Look | Dokładne zachowane cutoff/API źródła; udane browserfallback tylko tam, gdzie zapisano odczyt. Nie twierdzimy, że wszystkie strony live są dostępne lub świeżo przeczytane; niedostępne pozostają niepewne |
| FX/audio295 | Wyłącznie pinned publiczny GitHubHTTP, bez wiki/browser i bez przyznania Native/asset authority |
| Ten bilans /Tavily | Lokalny odczyt, bez nowych zapytań internetowych. Tavily nie wywoływano; limit jest wyczerpany |


**Liczba wszystkich kompletnie ukończonych i wszystkich nieukończonych Itemów pozostaje nieustalona (`null`).** Brakuje globalnego dowodu wymaganych i zastosowalnych pól. Historyczny chroniony V4artifact38157 i aktualne Itemrecords34031 to różne przestrzenie;4126 nie jest backlogiem. Pierwszy raport materializacji zachowano bez zmian.

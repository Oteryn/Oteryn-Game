# Zaakceptowane szacunki mitygacji Oteryn

Właściciel 02.10.2026 zaakceptował uzupełnienie brakującej mitygacji szacunkami własnego balansu, bez deklaracji zgodności z Global. `OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE` oznacza akceptację do przygotowanego zestawu danych. Klasyfikacja dowodów pozostaje `DERIVED`, a `global_parity` jest `false`. Akceptacja nie zmienia estymaty w fakt ze źródeł ani nie aktywuje produkcji.

Estymator używa 1053 istniejących wartości i wypełnia wyłącznie 644 brakujące. Nie zastępuje istniejących wartości, również zera. Odległość uwzględnia logarytm HP, pancerz, obronę i opcjonalnie logarytm XP, skalowane odchyleniem standardowym treningu. Predykcja jest średnią ważoną odwrotną odległością najbliższych sąsiadów. Przy identycznych używanych statystykach używa średniej wszystkich dokładnych sąsiadów, bo te same statystyki mogą mieć różną mitygację.

Porównano 16 konfiguracji modelu ogólnego i 16 bossowego przez pięcioczęściową walidację grupową. Identyczne statystyki, znormalizowane nazwy i jawne relacje form pozostają w tej samej części; brakujące jednostki łączą powiązane grupy. Kryterium wyboru: MAE + 0,2 × błąd p90, z preferencją prostszego wariantu w granicy 2% od najlepszego wyniku. Wybrane modele: ogólny k=9, wagi HP/pancerz/obrona/XP 1/2/1/0,5; bossowy k=3, wagi 1/2/1/0. Bossowy model ma 89 potwierdzonych bossów z istniejącą wartością; stosujemy go także do bossów oznaczonych wnioskowaniem. To jawne przybliżenie.

Wyniki wyboru modelu: ogólny MAE 0,3683 i p90 0,769819 punktu procentowego; bossowy MAE 0,479114 i p90 1,527346 punktu procentowego. Te same części wykorzystano do wyboru parametrów, więc nie są to wyniki niezależnego końcowego sprawdzianu. Mogą być optymistyczne; nie dowodzą dokładności dla brakujących jednostek. Znane wartości również nie są wszystkie zweryfikowane z Global. Zakres niepewności w ledgerze to heurystyka z reszt i rozrzutu sąsiadów, nie skalibrowany przedział ufności.

Własna reguła balansu: estymaty mają minimum 0,01%, zaokrąglenie do 0,01% i maksimum zaobserwowanego treningu 8,16%. Jednostki mechanik, treningowe i familiars mają dodatkowy konserwatywny limit 1%; ich HP nie jest wiarygodną miarą siły. To wybór balansu Oteryn, nie wynik źródłowy ani stan „nie dotyczy”. Nie ekstrapolujemy w nieskończoność. W ledgerze zapisano predykcję przed ograniczeniem, sąsiadów, zakres niepewności i wyjście poza zakres cech treningu. Wyjście poza zakres dostaje dodatkową flagę w indeksie i stage.

Wynik: 304 oszacowania modelem ogólnym, 326 bossowym, 14 regułą specjalną; 15 poza zakresem cech treningu. Wszystkie 1697 definicji mają po uzupełnieniu wartość; 644 z nich zachowują oznaczenie szacunku. Nowy typ źródła manifestu `oteryn_balance_estimate` wymaga SHA ledgera, jawnej akceptacji, klasy `DERIVED` i `global_parity=false`; nie podszywa się pod wiki ani donorowy Git.

```bash
python tools/content-migration/estimate_monster_mitigation.py --population BASELINE --classification CLASSIFICATION --out NEW_ESTIMATOR_DIR
python tools/content-migration/apply_mitigation_estimates.py --baseline BASELINE --ledger NEW_ESTIMATOR_DIR/mitigation-estimates.json --classification CLASSIFICATION --annotations SOURCE_EVIDENCE --output NEW_POPULATION
python tools/content-migration/creature_admission_stage.py --bundles NEW_POPULATION/bundles --index NEW_POPULATION/population-index.json --encounters NEW_POPULATION/encounters --item-map ITEM_MAP --out NEW_POPULATION/creature-admission-stage.json
```

Wejścia pozostają niezmienne; każdy output jest nowy. Przyjęcie sprawdza schemy, tożsamości, SHA, pełne pokrycie wyłącznie brakujących definicji i przenosi flagi do stage. Po uzupełnieniu usuwa odziedziczoną flagę MITIGATION_UNKNOWN, zachowując inne ograniczenia. Katalog klasyfikacji zostaje ponownie związany z nowymi digestami. Archiwum tego następcy zawiera delta 644 paczek, indeks, stage, katalog i ledger; niezmienione paczki zachowują wcześniejsze drafty. Canary/Crystal i istniejące wiki służą jako zachowane dane wejściowe, bez nowego researchu lub działań projektowych przez Remote Desktop. Nie stosujemy mnożnika runtime Crystal ×1,5 do wartości bazowych.

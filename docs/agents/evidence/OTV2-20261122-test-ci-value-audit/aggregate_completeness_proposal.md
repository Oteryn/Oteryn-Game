# BB-02: minimalna poprawka kompletności plan → wynik

**Baza:** `Oteryn/Oteryn-Game@8e9f0e16631aab1beb8416f8d5ca707aa3031083`. Źródła: `<baseline-checkout>`. `matrix.py`, `test_matrix.py` i dispatcher nie miały lokalnych zmian przy rozpoczęciu tej lane.

**Rekomendacja:** wdrażać etapami. Można już poprawnie sprawdzać dokładne terminalne ID wszystkich wybranych indexed rows bez zmian produkcyjnego runtime. Pełna kompletność grup, basic, spells oraz capture→replay wymaga ich własnych deklaracji wykonania. Nie można jej wyprowadzić z jednej liczby scenariuszy ani z samego `NODE_BOOT_PASS`.

Przygotowany patch jest **propozycją do kolejnej fali**, nie zmianą zastosowaną do wspólnego checkoutu. Nie należy przedstawiać go jako pełnej naprawy pokrycia wszystkich trybów. Root zdecydował, że pierwszy draft otrzyma raport/propozycję; szersza zmiana zachowania capture/no-op wymaga osobnej kwalifikacji.

## 1. Dowód i kontrakt najmniejszego etapu

**PROVEN:** bazowy `tools/qualification/node_boot/matrix.py` przyjmuje `I01.full PASS` i `NODE_BOOT_PASS` w artifactcie `rows-206-207`, mimo że plan `rows="206:207"` wybiera index 206, czyli `S09.client`. To poprawny artifact sharda z niewłaściwą zawartością. Oryginalny wynik: `shard_passed=True`, `summary_problems=[]`.

Najmniejszy poprawny kontrakt:

1. Plan wymienia każdą wybraną parę `{index, scenario}` ze źródłowego dispatchera.
2. Każde zaplanowane scenario musi mieć **dokładnie jeden terminalny verdict własnego ID**. `S09.full`, `S09.logout`, inny `.client` albo dowolna liczba innych PASS nie zastępują `S09.client`.
3. `CAPTURED` nie jest terminalnym verdict. Capture pozostaje osobnym dowodem pomocniczym.
4. Brak verdictu i duplikat są błędami kompletności także przy `--missing-informational`. Flaga dotyczy istniejących verdictów `MISSING-*`, nie znikających wywołań.
5. Kompletność wykonania i wynik testu pozostają odrębne. Jeden terminalny `FAIL` jest dowodem, że wiersz wydał wynik; istniejący runner i baseline nadal decydują o tym, czy taki wynik może być historycznym długiem. Patch nie dodaje żadnego wyjątku i nie rozluźnia bramki FAIL.

**PROVEN po patchu:** ten sam syntetyczny reproduktor daje `rows-206-207: missing terminal verdict for S09.client`. `shard_passed` pozostaje True, bo nadal opisuje exit i terminal runnera; **summary zgłasza problem, a jego CLI zwraca 1**. Egzekwowanie tego etapu odbywa się w istniejącym obowiązkowym aggregate step, nie poprzez udawanie, że sam shard już umie zweryfikować cały plan.

## 2. Jak pozyskać właściwe ID bez fikcyjnego licznika

### Istniejący dispatcher wystarcza dla row scope

Źródło: `apps/game-server/src/qualification/client_phases/world.rs`. Istnieją **302 wywołania selektora**; indeksy zajmują zakres 1–305 z lukami. Nie należy wymagać 305 wyników.

- 298 bloków ma bezpośredni literal `verdict::indexed("<ID>", async { ... })`.
- Cztery starsze wywołania nie używają tego wrappera:

| Indeks | Wywołanie | Oczekiwany terminalny ID | Weryfikowany producent |
|---|---|---|---|
| 85 | `full_e2ef19::run_packet` | `S04b.full` | `client_phases/full_e2ef19.rs:53` |
| 86 | `full_e2ef18::run_packet` | `C14.full` | `client_phases/full_e2ef18.rs:52` |
| 87 | `full_e2ef20::run_packet` | `E02.full` | `client_phases/full_e2ef20.rs:56` |
| 227 | `client_row_c07::run_client_packet` | `C07.client` | `client_phases/client_row_c07.rs:40` |

Prototyp rozpoznaje **zamknięty, aktualny kształt bloków dispatchera**, nie próbuje odgadywać semantyki dowolnego Rust. Dla czterech starszych wywołań ma jawny kontrakt call→ID i sprawdza literal emitera. Następnie porównuje komplet z istniejącym `row_indices()` przeglądającym wszystkie `.rs` w tej lane. Nowy selector w innym pliku, nieznany kształt bloku, brak ID lub powtórzony ID blokują plan. Nigdy nie prowadzą do pominięcia nieobsługiwanego wiersza.

To jest celowo rygorystyczny parser małej konwencji repozytorium: zmiana formatowania/kształtu dispatchera może wymagać aktualizacji narzędzia. Nie jest to dowód, że regex potrafi analizować cały Rust. Testy mutacyjne potwierdzają brak cichego ubytku dla nieznanej składni i dodatkowego selectora. Jeżeli dispatcher zacznie używać makra generującego wiele scenariuszy lub dynamicznego ID, najmniejszym kolejnym krokiem będzie **jawna deklaracja kontraktu przy tym dispatcherze**, zamiast rozluźniania parsera.

Uzyskane 302 kontrakty to 160 `.full` i 142 `.client`. Różnica względem wcześniejszej liczby 157 full / 141 client dotyczy czterech wywołań bez `verdict::indexed`.

### Dane planu i ich zgodność ze źródłem

Każdy shard dostaje `expected_rows`, przykładowo:

```json
{
  "name": "rows-206-207",
  "entry": "node_boot",
  "world": true,
  "env": {"NODE_BOOT_SKIP_BASIC": "1", "NODE_BOOT_E2E_ONLY": "rows", "OTERYN_E2E_ROW_BATCH": "206:1"},
  "expected_rows": [{"index": 206, "scenario": "S09.client"}]
}
```

Fragment pomija niezmieniony `platform_ref` wyłącznie dla czytelności. W rzeczywistym planie pin pozostaje obowiązkowy.

`validate_shard` wymaga pola i porównuje je ze źródłowym dispatcherem oraz wybranym zakresem. Podmiana ID albo usunięcie jednego elementu nie przechodzą walidacji. `reference` bierze default z `E2E_ROW_BATCH_DEFAULT`, więc nie zamraża dodatkowo zakresu w nowej stałej.

| Tryb | Liczba wymaganych row verdictów w audytowanym źródle | Granica dowodu |
|---|---:|---|
| reference | 14 | Te same aktywne indeksy 1–14 plus dotychczasowe wykonanie grup; patch sprawdza tylko te 14 row IDs. |
| pr | 14 w trzech row shardach | Podział 0:5 / 5:5 / 10:5, liczba wyprowadzona z istniejących selectorów. |
| full | 302 w 31 row shardach | Wszystkie aktualnie wybrane row IDs; to nadal nie cała semantyka grup, capture lub produktu. |
| custom rows | Dokładnie aktywne indeksy w żądanym zakresie | Własny przykład 206:207 wymaga wyłącznie `S09.client`. |

## 3. Osobna semantyka każdego rodzaju lane

| Lane / rodzaj dowodu | Co istnieje dziś | Co sprawdza prototyp | Co jest jeszcze potrzebne |
|---|---|---|---|
| Basic | `qualification::node_boot_seam_against_running_node`, stage lifecycle i wiele `SEAM_EVIDENCE` spoza `e2e_all`; World bundle nie jest użyty w tej gałęzi (`qualification.rs:1457–1584`) | `expected_rows=[]`; nie wymaga wymyślonych `.full` ani nie deklaruje kompletności basic | Jawny dowód wykonania właściwego pojedynczego testu oraz wymaganych faz. Liczba PASS z innych komend nie zastępuje właściwego testu. |
| Grupy `core`, f4–f7, mp | Registry `verdict.rs::GROUPS` deklaruje nazwy grup, nie wymagane scenario IDs. f6 używa `Row::new` i makr; f7 ma kilka oddzielnych entry points | Bez row IDs w group-only shardach; stara kontrola niepustego World evidence pozostaje | Mały kontrakt oczekiwanych wyników przy każdej grupie, weryfikowany z callerem. Na przykład f6 obejmuje C19a/C20d/S16a/C20a/C20b/P16/C20c/P10, a nie „minimum osiem dowolnych wyników”. |
| `creature_sweep` | Lista gatunków pochodzi z danych, jest sortowana; `run` wybiera batch i emituje `CS.<species>.<step>` / `.full` (`creature_sweep.rs:132,177,358–404`) | Nie sprowadza dynamicznych gatunków do statycznej liczby rows | Oczekiwane gatunki/steps wyprowadzone z dokładnego pinned content i batch selectorów, z powiązaniem digestu. Sam census emitowany przez badaną pętlę nie stanowi niezależnego dowodu, że niczego nie pominęła. |
| Indexed rows `.full` | Literalne ID dispatchera, verdicty full, część z historycznym baseline | Dokładnie jeden końcowy verdict każdego wybranego pełnego ID | Zachować dotychczasowe wymagania mechaniki; sama obecność verdictu nie dowodzi poprawności jego assertions. |
| Indexed rows `.client` | Mieszanka żywych queries/assertions, capture-only i no-op bez capture env | Dokładnie jeden terminalny verdict własnego `.client`; capture tego nie zastępuje | Jawnie określić dla każdego ownera, czy deklaruje live-client test, capture, czy replay. Nie nadawać PASS tylko za wejście do funkcji lub stworzenie pliku. |
| Spells | `node_boot_spells_against_running_node`, `spell_probe`, `spell_seam`, `spell_scenario`; katalog i selekcja zależą od manifestu. Test wymaga m.in. realnego cast, nie deklaruje pełnego gameplay każdego spella | `expected_rows=[]`; nie używa liczby e2e rows jako proof spells | Odrębny kontrakt zakresu: wymagane representative scenarios / manifest inputs i właściwy test. Nie wymagać arbitralnie PASS dla całego katalogu pod nazwą obecnego bounded lane. |
| Game–Platform | `game_platform/lifecycle.sh:165–218` już wymaga konkretnych flow evidence po dokładnych auth tests; kończy `GAME_PLATFORM_RESULT=PASS` | `expected_rows=[]`; zachowany własny terminalny marker i dodatkowo zgodność `result.entry` z planem | Można przenieść listę już egzekwowanych flow do planu/summary, bez wymyślania `.full` dla auth. Weryfikacja ma pozostać w realnym lifecycle. |

**Ważne:** `world=false` dla `game-platform` w istniejącym planie oznacza brak node_boot World row scope, nie brak rzeczywistego World bundle w jego fizycznej topologii.

## 4. Capture, replay, bounded slices i dodatkowe wyniki

`qualification.rs::e2e_result_line` przenosi `CAPTURED` do `e2e_capture`. Prototyp `extract` zachowuje te ID osobno w `captures`; służą one wyłącznie do diagnostyki „capture only”, nie do wyniku PASS.

`crates/session/src/tests/e2e_c01.rs` pokazuje, dlaczego ogólny sukces cargo nie wystarcza: bez env generuje synthetic payloads i wykonuje prawdziwe assertions na Session przez duplex; z `E2E_ALL_CAPTURE_DIR` odczytuje capture. Test nie emituje własnego `e2e_all ... PASS` per capture owner. Dzisiejszy `e2e_all/run.sh` uruchamia zbiorczo library tests, bez kompletnego manifestu ich zależności od capture IDs.

Pełne rozwiązanie capture→replay wymaga małego jawnego kontraktu: producent, wymagane artifacty, tożsamość przebiegu i consumer test. Przydatne dane to ID capture, ścieżki/digests plików, source SHA i nazwa wymaganego testu replay. Dopiero wynik tego testu na tych artifactach jest dowodem replay. To oznacza rozszerzenie **testowego protokołu evidence**, nie zmianę Game wire protocol. Nie wdrożono go w tym patchu.

W prototypie dodatkowe bounded slices i wyniki grup mogą być obecne w logu. Nie zaspokajają required ID i pozostają widoczne w istniejącym summary. **Patch nie egzekwuje zakazu każdego dodatkowego `.full`/`.client` ID**: taki zakaz wymaga kompletnej mapy ownership i dozwolonych zagnieżdżonych wyników, zwłaszcza dla reference. Sprawdza wykonanie wymaganych identities, nie hermetyczność całego strumienia evidence. Duplikat wymaganego ID jest błędem.

`MISSING-SERVER`, `MISSING-CLIENT`, `SKIPPED` oraz `FAIL` to terminalne wyniki wykonanej próby. Licznik „row verdicts” oznacza odnotowane wyniki, **nie PASS coverage**. Istniejąca polityka FAIL/baseline i `--missing-informational` nadal obowiązuje; brak rezultatu nie jest nowym dozwolonym wyjątkiem.

## 5. Ryzyko ujawnionych awarii i etapowanie

1. **Teraz — raport i prototyp:** zachować reproduktor oraz zidentyfikowane luki w draftcie. Nie deklarować pełnej naprawy BB-02 ani pełnego black-box coverage.
2. **Następna fala — required row verdicts:** po osobnym przeglądzie wykonać reprezentatywne row shards z nowym planem i realnymi logami, w tym batch mieszany full/client, jeden capture-only i cztery starsze dispatchery. Dodać same wymagania identity bez zmiany baseline. Root może zacząć ocenę od obecnego PR scope 1–14; docelowa reguła pozostaje taka sama dla pozostałych rows.
3. **Przed egzekwowaniem pełnego client scope:** uzgodnić i wdrożyć semantykę live-client/capture/replay u rzeczywistych ownerów. Brak testu ma pozostać jawny. Nie dodawać listy ignorowanych missing IDs, nie konwertować CAPTURED na PASS, nie emitować sztucznego PASS z wrappera.
4. **Potem — deklaracje grup i pozostałych lane:** zacząć od statycznych grup f4/f6 i istniejących Game–Platform flow, następnie dynamicznego creature sweep i manifestu spells. Registry powinien deklarować wymagane zachowanie/ID, a nie odzwierciedlać dowolną obserwowaną liczbę logów.

**Przewidywane skutki (INFERENCE ze zweryfikowanego kodu):** pełny plan ujawni no-op/capture-only rows, które dotąd mogły znikać w niepustym shardzie. Co najmniej 31 producerów ma bezpośredni return bez capture env; jeśli cleanup nie wygeneruje własnego FAIL, nie powstanie dla nich terminalny verdict. Włączenie capture env nie rozwiązuje automatycznie replay ani capture-only. PR scope wymaga obecnie wyłącznie 14 `.full` rows, lecz brak realnych logów na badanym SHA nie pozwala obiecać, że wszystkie obecne przebiegi spełnią nową regułę.

Przy zmianie parsera/dispatchera nowy kształt może zablokować **plan**, nawet jeśli kod Rust działa poprawnie. To świadomy koszt kontrolowanej konwencji, nie błąd do obejścia fallbackiem. Jeśli ta konwencja zacznie przeszkadzać, przenieść deklarację do małego kanonicznego registry współdzielonego przez dispatcher i planer. Nie rozbudowywać heurystycznego parsera do pozornego analizatora Rust.

## 6. Patch i weryfikacja

Patch: `./aggregate_completeness_proposal.patch`.

Zakres to dwa istniejące pliki: `tools/qualification/node_boot/matrix.py` i `test_matrix.py`. 261 dodanych linii, w tym regresje i testy nieobsługiwanych kształtów. Wszystkie nowe testy są w **istniejącym wywoływanym przez CI `test_matrix.py`**; nie powstał niewywoływany dodatkowy target testowy.

**PROVEN / wykonane:**

- RED na oryginalnym `matrix.py`: 11 nowych testów, **8 failures oraz 3 errors** (brak nowych metadanych). Błędna akceptacja obcego ID, częściowego batcha, duplikatu, bounded slice i capture jest sprawdzana przez rzeczywiste `plan → extract → summarize`.
- GREEN na propozycji: **33/33 tests**, exit=0. To 17 istniejących testów, 11 regresji identity/completeness i 5 testów kontrolujących kontrakt źródeł. Ostatni przebieg: 3,766 s.
- Dwa stare fixtures zaktualizowano: test tożsamości reference uwzględnia nowe pole; test layoutu pojedynczego artifactu emituje wszystkie wymagane ID zamiast jednego obcego `I01.full`. Asercje nie zostały osłabione.
- Oryginalny reproduktor BB-02 na propozycji zgłasza brak `S09.client`; wynik w `aggregate_completeness_repro_green.json`.
- `git apply --check` patcha względem shared repo: exit=0, bez aplikacji.
- Rzeczywisty niezmieniony `prebuilt.py::select` przyjął wszystkie shardy `reference`, `pr`, `full` z nowym polem. Nie uruchamiano buildów. Rozmiar compact full plan wyniósł 19 325 bajtów.

**Nie wykonano:** pełnego Rust build/test, node_boot/PG/Platform, capture/replay, workflow CI ani publikacji. Wynik syntetyczny dowodzi naprawy określonego oracla agregatora, nie ukończenia produktu.

### Odtworzenie wyników

Historyczne tymczasowe drzewo proposal nie jest wymagane. Użyj osobnego checkoutu przypiętej bazy i zastosuj wyłącznie zachowany patch (zapisany bez context lines):

```sh
# W osobnym checkout baseline, nie na opublikowanej gałęzi audytu:
git apply --unidiff-zero --check /path/to/aggregate_completeness_proposal.patch
git apply --unidiff-zero /path/to/aggregate_completeness_proposal.patch
python tools/qualification/node_boot/test_matrix.py -v
python /path/to/blackbox_repros.py --repo /path/to/proposal-checkout --mode aggregate

# RED: te same regresje na niezmienionym baseline (świadomie niezielone)
NODE_BOOT_MATRIX_SOURCE=/path/to/baseline-checkout/tools/qualification/node_boot/matrix.py \
python /path/to/aggregate_row_regressions.py -v
```

Oryginalne wyniki: `aggregate_matrix_red.txt`, `aggregate_matrix_green.txt`, `aggregate_completeness_evidence.json`, `aggregate_completeness_repro_green.json`. Patch pozostaje nieaplikowany do gałęzi audytu. Standalone driver służy reprodukcji; nie jest dodatkowym CI targetem. Liczby linii w JSON opisują oryginalny patch przed normalizacją context lines; proponowany wynik źródeł nie zmienił się.

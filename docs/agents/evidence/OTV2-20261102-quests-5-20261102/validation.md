# Walidacja partii 5

Wyniki poniżej pochodzą z eksportu zakończonych sprawdzeń w izolowanym checkoutcie na komputerze właściciela. Pełne polecenia, kody wyjścia, czasy, podsumowania i manifest plików znajdują się w `validation-results.json`. Dokument wymaga wszystkich 14 nazwanych sprawdzeń: 13 wyników PASS z kodem 0 oraz jednego oczekiwanego RED Alawar z kodem 101. Przerwany wcześniejszy runner regeneracji treści nie należy do tego zestawu.

Ten eksport dokumentuje stan plików z manifestu po zatwierdzonym rebindzie i nie potwierdza jeszcze kontroli końcowego commita ani PASS po publikacji. Dokładny końcowy SHA oraz osobno wyniki 10 sprawdzeń kandydata przed publikacją i 12 pełnych sprawdzeń po publikacji, obejmujących ponowną regenerację treści oraz registry, zostaną podane w [DRAFT PR #2846](https://github.com/Oteryn/Oteryn-Game/pull/2846) po ich zakończeniu. Po ostatniej publikacji lead zamraża SHA i ponownie uruchamia pełny wymagany gate na tym SHA; wcześniejszego wyniku nie wolno przemianować na post-publication PASS. Przy ocenie publikowanego head należy sprawdzić tam pełny SHA i przypisane do niego wyniki. Lista commitów w `commit-map.json` opisuje podział zmian; sama lista ani utworzenie obiektu commit nie potwierdzają jego przetestowania lub aktualizacji gałęzi PR.

| Sprawdzenie | Polecenie z eksportu PC | Wynik | Kod wyjścia | Czas, s |
| --- | --- | --- | --- | --- |
| `final-alawar-red` | `cargo test --locked -p oteryn-gameplay-quests --test quests5_alawars_vault` | Oczekiwany RED (osobny test Alawar przed dodaniem callbacka) | 101 | 3.82 |
| `final-fmt` | `cargo fmt --all` | PASS | 0 | 7.28 |
| `final-quests-clippy` | `cargo clippy --locked -p oteryn-gameplay-quests --all-targets -- -D warnings` | PASS | 0 | 0.44 |
| `final-quests-test` | `cargo test --locked -p oteryn-gameplay-quests` | PASS | 0 | 3.19 |
| `final-content-regeneration` | `python3 tools/content-migration/regenerate_content.py --resolve --no-stage` | PASS | 0 | 918.46 |
| `final-registry` | `python3 tools/contracts/registry_fragments.py` | PASS | 0 | 0.32 |
| `final-content-pin-check` | `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /home/mole/quests5-evidence/final-pin-check-20261103T003719Z-e12f295d9b6141d4b785f4f6ac6fdff5` | PASS | 0 | 21.16 |
| `final-content-integration` | `cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet` | PASS | 0 | 4.09 |
| `final-content-world` | `cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet` | PASS | 0 | 30.80 |
| `final-quest-authoring` | `python3 tools/content-schema/quest-authoring/run_checks.py --part all --jobs 2` | PASS | 0 | 1291.67 |
| `final-governance` | `python3 tools/agents/validate_governance.py` | PASS | 0 | 1.59 |
| `final-governance-tests` | `python3 -m unittest discover -s tools/agents/tests` | PASS | 0 | 1.11 |
| `final-fmt-check` | `cargo fmt --all -- --check` | PASS | 0 | 18.79 |
| `final-diff-check` | `git diff --check` | PASS | 0 | 1.14 |

## Zakres dowodu

- `final-quests-test`: 134 zaliczonych testów według podsumowań cargo i pola `rust_passed` w eksporcie PC; 0 błędów, 0 pominiętych, 0 filtrowanych. Oczekiwany RED jest osobnym sprawdzeniem i nie powiększa tego wyniku. Testy sprawdzają reguły danych i publiczny model stanu; nie stanowią przejścia całych questów na żywej mapie.
- Katalog JSON zawiera 93 callbacki: 20 wcześniejszych rekordów bez zmian oraz 73 nowe. Jego SHA256 z manifestu zakończonych checks: `b6d6cbca87402689e7643dc979ed568868949777c04c06e8c720697b1c37c661`; generator dokumentacji sprawdza zgodność tego skrótu z publikowanym katalogiem.
- `final-quest-authoring`: unittest zgłosił 1048 przypadków; 1040 wykonał, a 8 pominął warunkowo. Nie wliczono pominięć do wykonanych testów. Dokładne podsumowania `python_skip_summaries` zachowano w eksporcie; wcześniejsze wyniki nie są nadpisywane późniejszym uruchomieniem z dodatkowymi fixtures. Wyniki dokładnego końcowego SHA zostaną podane osobno w PR.

- Dane NPC: 317 wybranych reguł, 514 literalnych akapitów i 23 deklaracje rozmów, wszystkie jawnie nieładowane do runtime. Ich podłączenie należy do wspólnego PR koordynatora.
- Brak nowych plików produkcyjnego Rust, SQL, capability i komend protokołu. Zestaw powyżej nie obejmuje pełnych testów PostgreSQL i nie jest dowodem ich wykonania. Wyniki wcześniejszych, wyłączonych prototypów adapterów nie są dowodem tej gałęzi.
- Regenerację wykonują wyłącznie narzędzia repo. Niezmienione wyniki generatorów nie są ręcznie przepisywane. Wykaz rzeczywistych plików i blobów z końca sprawdzeń znajduje się w wynikach JSON. Manifest opisuje ten moment; późniejsze korekty dokumentacji i nowe ANSWER wymagają osobnego potwierdzenia na końcowym SHA w PR.

## Środowisko Python i wcześniejsza nieudana próba

Wcześniejsza próba odtworzenia donor corpus pod Python 3.14 zakończyła się błędem wyboru plików związanym z zachowaniem `PurePosixPath.suffix`. Tego uruchomienia nie wliczono do wyników PASS. Ponowienie wykonało istniejący generator w izolowanym środowisku Python 3.12 zgodnym z konfiguracją CI; nie zmieniono corpus, pinów źródeł, guardów ani generatorów, aby obejść błąd. Ta historyczna informacja opisuje wcześniejszy przebieg pracy, a nie dodatkowy wynik w tabeli 14 sprawdzeń.

Świadek `python312-runtime.json` zapisuje **Python 3.12.15**, interpreter `/home/mole/quests5-python312-venv/bin/python3`, obraz `python:3.12-slim-bookworm` o identyfikatorze `sha256:34386ef0cb081344d7ec1c103ba398e6e9f64e9ab3a1509accc92a4e24a07258` oraz źródło konfiguracji CI `Oteryn/Oteryn-Game@5fbe99b378d51c9cb69b2a49b28edf3b63ba7de4:.github/workflows/quest-authoring-schema.yml:75-84`. Probe rozszerzenia zwrócił `.lua`. Świadek nie zawiera czasu pobrania ani wyniku `pip check`; dokumentacja nie przypisuje mu tych informacji. Sam wybór interpretera nie dowodzi zaliczenia żadnego gate.

Późniejsza próba uruchomienia pełnych sprawdzeń zatrzymała się na regeneracji z powodu brakującej zależności `rfc3339-validator` w środowisku Python 3.12. Również tej niepełnej próby nie wliczono do wyników PASS. Zależności uzupełniono z istniejących plików `item-authoring/requirements.txt` i `imbuement-authoring/requirements.txt` zgodnie z CI. Osobny `python312-content-dependencies.json`, zapisany 2026-11-03T00:36:30Z, zachowuje pełne polecenie, hashe tych plików, wersje pakietów oraz kody 0 instalacji i `pip check`; źródło: `Oteryn/Oteryn-Game@5fbe99b378d51c9cb69b2a49b28edf3b63ba7de4:.github/workflows/content-tree-migration.yml:74-77`. Ten świadek dotyczy przygotowania środowiska, nie zastępuje zakończonych wyników wszystkich gates ani nie zmienia wcześniejszych świadków.

`generator11-result.json` dokumentuje zakończone ponowienie `quest_donor_source_authoring.py`: kod wyjścia 0, przegląd zmian 0 i `git diff --check` 0, czas 503.07 s. Jest to wynik generatora i jego bounded review, odrębny od 13 zielonych sprawdzeń wejść oraz późniejszych kontroli dokładnego opublikowanego SHA.


## Zatwierdzony rebind source receipt

[ANSWER6591355533](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6591355533) dopuszcza istniejący `amend_monk_source_receipt.py` oraz istniejące generatory pochodne. Nie wprowadza nowej reguły źródłowej ani nowego zatwierdzenia core. `source-receipt-before.json` zachowuje wcześniejszy świadek tylko do odczytu; jego historyczne `refresh_authorized=false` nie jest nadpisywane późniejszą decyzją. Końcowy `source-receipt-after.json` musi mieć `PASS`, pełne odświeżenie, zero zmian semantycznych, zero zmian runtime admission i zero zmian poza zakresem.

W samych approval/receipt zmieniają się dokładnie trzy wskazane piny: `approval:/proof_inputs/1/sha256`, `receipt:/proof_inputs/1/sha256` i `receipt:/approval/sha256`. Pierwsze dwa wskazują niezmienione bajty katalogu callbacków; trzeci wskazuje rzeczywisty nowy hash approval. Pozostała kaskada obejmuje wyłącznie pochodne hashe, świadków i provenance. Core, graph approvals, opisy kompilatorów, niezmienne wejścia, anotacja Monka, 20 wcześniejszych callbacków, owner associations, guardy, testy i admission pozostają chronione.

Wszystkie **30 wygenerowane pliki** znajdują się w pełnym manifeście **82 plików** zakończonych checks. Dla każdego porównano `after_sha256` z review, SHA256 i długość z manifestu oraz rzeczywiste lokalne bajty, w tym oba gzipy bez reinterpretacji. Końcowy hash approval: `f21a7f740154c22946349ba9bddd379388f967e77c4699ed59c82df61abf4234`; receipt: `9974e49b2e0874ed109a22fc9edfaa585d592834299245b4c6d7fff143fb2cf7`.

Osobny świadek `monk-native-raw-bytes.json` potwierdza literalną zgodność bajtów wszystkich **100/100 anotacji** w shardzie `quests-00200-00299.json`. Anotacja Monka ma przed i po odświeżeniu **1956 bajtów** oraz SHA256 `f31fa160030acc45c401c115a3f0d5f92b4f199437b1a932e236ba7fa60fef46`. Cały shard zmienił bajty wskutek zatwierdzonej aktualizacji pochodnych provenance; twierdzenie o niezmienionych bajtach dotyczy anotacji, nie całego pliku. Builder wiąże ten świadek z SHA256 surowego `source-receipt-after.json` oraz oboma hashami shardu zapisanymi w tym przeglądzie.

Odnowienie oraz wszystkie wygenerowane pliki pochodne należą wyłącznie do końcowego wspólnego commita, którego rodzicem jest wspólny commit raportu i wejść `6444b83dbb9f69a34186526218af952b45471307`. Łańcuch 21 prywatnych commitów questów kończy się na `0595b55822c66c7770762e546fa1186d0e6c2fea`. Prywatne commity questów nie zawierają tego rebindu; nie powstaje pośredni commit na same hashe.

Inne PR-y questów dodają callbacki do tego samego pliku. Hashe są danymi pochodnymi; obowiązuje merge note z [ANSWER6591355533](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6591355533):

> Do not merge hashes by hand against another PR. The coordinator re-runs the same tool on the merged tree.

## Ostatnie decyzje

[ANSWER6591355533](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6591355533) rozstrzyga QUESTION6591275599 w zakresie opisanym wyżej; nie rozstrzyga pytania o Feaster rocks i nie zmienia statusu DRAFT ani zakresu runtime.

[ANSWER6590729496](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6590729496) potwierdza Left out fizycznego UID5008 skrzyni klucza4502 oraz action4502 drzwi Alawar, zachowanie istniejącej nagrody i dopuszczenie obu źródłowych teleportów przez obecny StepIn. VFX pozostaje niepodłączony. Źródła i odczyt mapy/guardów są w trzech plikach `alawar-*.json`; nie zmieniono B3 ani gotowości claimu.

[QUESTION6590265482](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6590265482) o dwóch rock Use Feaster pozostaje w Left out według ogólnego ANSWER6589528748 pkt7, dopóki nie zostanie zakwalifikowana zgodność selektora. Ostatni stan komentarzy i ewentualne późniejsze odpowiedzi podaje PR.

# Prepublication peer check — 8 plików

**Aktualizacja końcowa: C1 FIXED, niezależny ograniczony recheck PASS.** Wynik poprawki, dokładny nowy hash i zakres wykonania są w końcowej sekcji „Recheck C1”. Poniższy pierwotny finding OPEN zachowano jako historię pierwszego snapshotu; jego niezmieniona kopia to `peer_review_evidence/original_code_review_findings.md`.

**Wynik: jeden otwarty finding P2 (C1), przekazany root do poprawy.** Pozostałe sprawdzone zmiany zachowują unikalne przypadki i wzmacniają lub zachowują assertions. Jest to lokalny peer check proponowanego diffu, nie wymagany zewnętrzny Codex Review, nie wynik CI i nie końcowa kwalifikacja opublikowanego SHA.

## Dokładny przedmiot przeglądu

Bazowy commit: `8e9f0e16631aab1beb8416f8d5ca707aa3031083`.
Przejrzany diff SHA-256: `eaf5d7a52de930320238e14977cebee0269fbcc2262d5cf9956b64fd5ca3927b`.
Snapshot diffu: `./peer_review_evidence/reviewed.patch.json`.
SHA-256 każdego z ośmiu plików przed/po: `./peer_review_evidence/source_manifest.json`.
Dokładne odczytane bytes leżą w `peer_review_evidence/current/` i `peer_review_evidence/baseline/`.

Odczytano cały diff tych plików, następnie celowany kontekst ich wywołań i assertions:

1. `.github/workflows/agent-governance.yml`
2. `tools/content-schema/craft-authoring/test_tainted_souls.py`
3. `tools/qualification/node_boot/known_fail_rows.txt`
4. `tools/qualification/node_boot/run.sh`
5. `tools/qualification/node_boot/test_diagnostics.py`
6. `tools/qualification/node_boot/test_node_boot_tools.py`
7. `tools/repository/test_agent_governance_pr_targets.py`
8. `tools/repository/test_validate_pr_gate_pg_sim.py`

Root zapowiedział poprawę C1 po otrzymaniu reproduktora. Ten raport ocenia powyższy snapshot; nie deklaruje zweryfikowania jeszcze nieodczytanej poprawki.

## C1 — P2, OPEN: osierocony drugi `cause` lub `root` nadal daje known/PASS

**Miejsce:** `tools/qualification/node_boot/run.sh`, nowy `stage_fail_rows`, linie 1021–1027 w snapshotcie. `re.findall` zlicza kompletne, sąsiadujące pary `cause=… root=…`, wyszukiwane na całym wierszu. Nie zlicza samodzielnego kolejnego klucza i nie wymaga, aby para była końcowym suffixem właściwego verdictu.

**FACT / PROVEN — reproduktor na rzeczywistym helperze i rzeczywistym `stage()`:** dla baseline `E01.full assertion at E01/greet` każdy z poniższych logów zostaje sklasyfikowany jako known:

```text
SEAM_EVIDENCE e2e_all scenario=E01.full status=FAIL seconds=1 cause=assertion root=E01/greet cause=transport
SEAM_EVIDENCE e2e_all scenario=E01.full status=FAIL seconds=1 cause=assertion root=E01/greet root=E01/relog
```

Oba wykonania stage wypisały:

```text
NODE_BOOT_KNOWN_FAIL sample rows=E01.full
STAGE_TEST_STATUS=PASS
```

Trzeci, helper-only wariant bierze starą parę z prefiksu przed markerem `e2e_all`, mimo odmiennego, niekompletnego cause przy właściwym FAIL. Wszystkie trzy wyniki są w `./peer_review_evidence/signature_cases.json`; wykonania pełnego stage w `stage_malformed_repro.json`.

**Wpływ:** format niejednoznaczny lub uszkodzony może nadal dostać wyjątek known-failure zamiast zostać uznany za nowy FAIL. Nie ma dowodu, że obecny Rust reporter emituje taki uszkodzony wiersz w produkcyjnym runie; to reproduktor odporności proponowanej granicy fail-closed. Poprawna, pojedyncza zmiana cause/root już jest wykrywana.

**Rekomendowana poprawka:** ograniczyć interpretację do części po właściwym `e2e_all scenario=… status=FAIL`; wymagać dokładnie jednego osobnego tokenu `cause=` i `root=`, z parą jako ostatnimi dwoma tokenami w kolejności `cause` → `root`. Prefiks, duplikat, niekompletny token albo dopisek po root mają dawać `new`. Zgodny suffix jest już kontraktem `apps/game-server/src/qualification/client_phases/verdict.rs:69–70` i zakotwiczonego `RESULT` w `tools/qualification/e2e_all/report.py:10–15`. Zachować też istniejącą zasadę: wszystkie FAIL-verdicts tego samego scenariusza muszą mieć tę samą znaną sygnaturę.

Do domknięcia wystarczy odtworzyć wskazane trzy przypadki przeciw poprawionym bytes i ponownie sprawdzić mały zestaw podpisów/stage. Nie trzeba uruchamiać prawdziwego node boot.

## Co już działa fail-closed

Wykonano 17 małych przypadków na nowym helperze. Czternaście spełniło oczekiwanie; trzy wyjątki opisuje C1. Potwierdzone poprawne zachowania:

- dokładna sygnatura i komentarz `; evidence` w baseline pozostają known;
- nowy cause, nowy root, nieznany scenariusz, brak sygnatury lub brak root dają new;
- znana i nowa sygnatura tego samego scenariusza w różnych wierszach dają new;
- znana i niesklasyfikowana awaria tego samego scenariusza dają new;
- dwie kompletne pary cause/root na jednym wierszu dają new;
- pusty baseline nie ukrywa żadnej awarii;
- błędny wpis baseline oraz powtórzony scenariusz kończą parser kodem 1, także przy pustym logu.

Dodatkowo dwa malformed-baseline przypadki wykonano przez rzeczywiste `stage()` z bezpiecznymi stubami zewnętrznej obserwowalności: oba procesy zakończyły się kodem 1 przed wydrukiem terminalnego statusu. Dowód: `peer_review_evidence/malformed_baseline_stage.json`. W rzeczywistym runnerze `set -Eeuo pipefail` jest włączone, `stage` jest wywoływane zwykłymi komendami, a domyślne `result=NODE_BOOT_FAIL` poprzedza trap cleanup (`run.sh:333,397–438`). Nie ma tu dowodu całego uruchomienia Docker/PostgreSQL; nie było ono potrzebne do sprawdzenia tego błędu.

## Selekcja i wykonanie testów: bez utraty unikalnych cases

Własny probe ładował suites przez `unittest.TestLoader`, zapisał komplet ID przed wykonaniem, a następnie uruchomił te same suites. Dowód i reproduktor: `historical scratch peer_review_evidence/probe.py (not a portable retained runner)`, `direct.json`, `discovery.json`, `direct.txt`, `discovery.txt`.

| Sposób wyboru | Zebrane | Unikalne ID | Faktycznie wykonane | Wynik |
|---|---:|---:|---:|---|
| Bezpośrednio moduł `test_diagnostics` | 46 | 46 | 46 | PASS, zero failures/errors/skips; 2.008 s lokalnie |
| Discovery `test_*.py` w node_boot | 88 | 88 | 88 | 0 failures, 7 errors środowiskowych, zero skips; 4.076 s lokalnie |

Bezpośrednio ładowane są 18 własnych cases diagnostyki i wszystkie 28 cases sibling modułu. Discovery wybiera je po jednym razie, dodatkowo matrix 17, prebuilt 6, provision 10 i shared-prebuilt 9. Zbiór 46 direct IDs jest podzbiorem 88 discovery IDs. `load_tests(..., pattern)` robi zatem to, co deklaruje dla tych dwóch rzeczywistych wywołań: bezpośredni moduł obejmuje sibling, discovery nie dubluje sibling.

Siedem errors dotyczy wyłącznie `test_provision`: pięć `ProvisionTests` i dwa `SpawnTests`, każde `PermissionError: [Errno 1] Operation not permitted` podczas `socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)`. Nie oznaczono ich jako PASS i nie pominięto ich filtrem. Pełne tracebacki są w `discovery.json`/`discovery.txt`.

Niezależna kolekcja bazowych dwóch modułów ze snapshotu HEAD pokazała: stary direct 38 ID; oba moduły w discovery 65 wykonań/40 unikalnych ID, czyli 25 duplikatów. Żaden z tych 40 unikalnych ID nie zniknął z obecnego direct. Obecne 46 obejmuje wszystkie 40 oraz sześć nowych regressions (pięć w KnownFailRowsTests i jeden w StageDump). Ponadto dwa wcześniej istniejące cases niewciągnięte ręczną listą importowanych klas stają się dostępne przez direct. To kolekcja baseline, nie twierdzenie o wykonaniu całego baseline. Szczegóły: `baseline_collection.json`.

## Assertions i pozostałe pliki

### StageDumpTests — wzmocnione

Nowy fixture ustanawia brakujące `GAME_SOURCE`, node log i znany baseline; wyciąga rzeczywisty `stage_fail_rows`. Stuby dotyczą efektów poza badanym etapem. Dodano sprawdzenie `returncode == 0` całego disposable skryptu oraz końcowego `STATUS`, których poprzednio nie było. Zachowano assertions dump/verbose; nowy test pokrywa udaną komendę zawierającą zmieniony cause/root. Nie znaleziono osłabienia poprzednich assertions. Do dopięcia pozostają przypadki C1.

### WorldFixtureTests — aktualizacja oczekiwania jest uzasadniona

`run.sh:20,156–161` ma LIGHT_ACCOUNT_ID i dodaje go do świata; `run.sh:1387` eksportuje NODE_BOOT_LIGHT_ACCOUNT_ID, którego `qualification.rs:1479` wymaga. Oczekiwanie 11 odpowiada istniejącej produkcyjnej konfiguracji: 2 core + 2 packet + 4 party + 2 sweep + light. Zmiana z 10 na 11 naprawia test, który wcześniej nie był wybierany direct; nie zmienia konfiguracji produktu. Pozostawiono sprawdzenie unikalności, dodano zgodność liczby intents/names, unikalność intents, obecność light i rozłączność z PvP.

### TaintedSouls — usunięto identyczne kopie, zachowano ciała testów

Porównanie AST wykazało: wszystkie trzy usunięte metody klasy Integration są identyczne z zachowanymi metodami klasy TaintedSouls. Wszystkie siedem zachowanych metod ma niezmienione AST. Usunięcie pośredniego `unittest.main()` odblokowuje dwa istniejące końcowe integration cases przy bezpośrednim uruchomieniu. Liczba metod w źródle 10 → 7 wynika z usunięcia trzech kopii; ten peer check nie uruchamiał suite craft. Dowód: `craft_ast.json`.

### Governance/repository — zachowani właściciele wywołań

Workflow `agent-governance.yml` teraz jawnie wywołuje każdy z trzech owner modules. Usunięte importy z `test_agent_governance_pr_targets.py` nie usuwają metod testowych. Odczytano niezmieniony sweep `.github/workflows/merge-gate.yml:397–417`: wywołuje wszystkie `tools/repository/test_*.py` dokładnie raz, z dwoma jawnie wcześniejszymi wyjątkami. Zatem usunięte kaskadowe `main()` trzech siblingów oraz ręczny stale-JUnit test nadal mają właściciela wykonania w sweep. Nie zmieniono ich asercji ani aktywnych game-gate/protection. Ten peer check nie powtarzał całego governance suite; wcześniejsze niezależne dowody before/after pozostają w `ci_dedup_*.json`.

### Known-failure baseline — brak rozszerzenia wyjątków

Diff zmienia jedynie komentarz opisujący dopasowanie. Żaden z zapisanych scenariuszy/cause/root nie został dopisany ani zmieniony. Test generujący dokładne sygnatury ze wszystkich aktualnych wpisów przeszedł w direct suite.

## Granice i stan domknięcia

- C1 pozostaje OPEN dla przejrzanego snapshotu; root zapowiedział poprawkę, której ten raport jeszcze nie obejmuje.
- Nie przeprowadzono pełnego CI, Docker/PostgreSQL node-boot, Windows ani wymaganej zewnętrznej review.
- Discovery nie jest w całości zielone w tej maszynie: siedem niezmienionych socket tests blokuje lokalne AF_UNIX EPERM.
- Nie oszacowano kosztu CI na podstawie pojedynczych lokalnych pomiarów.
- Nie modyfikowano źródeł repozytorium. Wszystkie reproduktory, snapshots i wyniki zapisano pod `./peer_review_evidence`.

## Recheck C1 — FIXED

Root zmienił parser i dodał regresje; reviewer nie edytował repozytorium. Odczytano wyłącznie zmianę C1 oraz jej nowe assertions, a następnie wykonano ograniczony zestaw sprawdzeń. Nowy diff SHA-256: **`cf105002bcde92ce5f54d20727fafd0a4aefd55a4839015abc461920088fb67e`**. SHA-256 wszystkich ośmiu plików są w `peer_review_evidence/c1_fixed/source_manifest.json`; ich źródła są w opublikowanym drzewie, a oryginalny diff w `peer_review_evidence/c1_fixed/reviewed.patch.json`. Tymczasowych kopii `source/` nie archiwizowano osobno.

**FACT / PROVEN:** parser bierze `payload = line[row.end():]`, sprawdza dokładną listę osobnych kluczy `['cause', 'root']`, a dopasowanie pary jest zakotwiczone końcem wiersza. Nie pobiera znanej pary z prefiksu ani nie przyznaje wyjątku po dodatkowym cause/root lub dopisku po końcowej parze.

Niezależne wyniki:

- **17/17 synthetic signature cases PASS.** Te same trzy wcześniejsze kontrprzykłady zwracają teraz `new`; dokładna poprawna sygnatura nadal daje known. Malformed i duplicate baseline nadal kończą się błędem. Wynik: `c1_fixed/signature_cases.json`.
- **12/12 ograniczonych test cases PASS**, zero failures/errors/skips, 1.286 s: pełne `KnownFailRowsTests` oraz `StageDumpTests`. Obejmuje to sześć malformed-signature subcases, trzy malformed-baseline subcases i faktyczny `stage()` z dodatkowym cause/root. Wyniki: `peer_review_evidence/c1_fixed/bounded_tests.json` i `peer_review_evidence/c1_fixed/bounded_tests.txt`.
- **Tylko kolekcja po poprawce:** direct 48/48 unikalnych, discovery 90/90 unikalnych, direct nadal podzbiorem discovery; żaden wcześniejszy direct/discovery ID nie zniknął. Wynik: `c1_fixed/collection.json`. Pełnych 48/90 przypadków reviewer ponownie nie wykonywał; poprzednie pełne wykonania 46/88 i siedem AF_UNIX errors pozostają historycznym, dokładnie opisanym dowodem.

**Stan końcowy tego peer check:** C1 zamknięty, brak dalszego otwartego finding w przejrzanym diffie. Ograniczenia dotyczące pełnego CI, AF_UNIX, zewnętrznej required review i final exact-head qualification pozostają bez zmian. To potwierdzenie naprawy zidentyfikowanego błędu, nie deklaracja wykonania całej kwalifikacji produktu.

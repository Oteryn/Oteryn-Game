# Oteryn Game — audyt repozytorium i kwalifikacja wykonawcza

**Data:** 6 września 2026. **Issue:** #359. **PR publikacji:** #360.

**Badany produkt:** `Oteryn/Oteryn-Game@7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`.
**Drzewo:** `359a52348bfdf8088a7cd456f4015b05279721b6`.

## 1. Wynik i granice zapewnienia

To zapis wykonanej pracy audytowej, nie certyfikat bezbłędności produktu. Ukończono inwentaryzację i pełnotekstowe kontrole **803/803 plików** oraz rozliczono **32 obszary kontroli**. Wykonano nowe buildy i testy na Linux/Windows, PostgreSQL, rzeczywisty test prezentacji DX12 oraz ukierunkowane próby negatywne. **Nie ukończono semantycznego przeglądu każdej linii i wszystkich aspektów eksploatacji.** Rejestr wskazuje 97 ścieżek objętych ukierunkowaną analizą znaczenia kodu; pozostałe mają jawny status kontroli statycznej, nie zatwierdzenia semantycznego. Nie nazywamy tego „100% zweryfikowanej poprawności” ani pełnym wykonaniem pierwotnego nieograniczonego żądania.

Wykryto 17 publicznie opisanych ustaleń o różnym charakterze: błędy, luki walidacji, ograniczenia pomiaru i zalecenia projektowe. Nie wszystkie są defektami produkcyjnymi. Najważniejszy nowy dowód: **zielona kanoniczna walidacja współistnieje z uszkodzoną serią testów governance**, która przy niezależnym uruchomieniu kończy się trzema błędami przygotowania testu. Drugim ważnym dowodem jest natywne odtworzenie maskowania wcześniejszego kodu błędu w zgrupowanych poleceniach PowerShell.

**Ocena architektoniczna:** warto zachować Rust, autorytatywny model serwera, jeden logiczny właściciel mutacji na zakres świata oraz rozdzielenie decyzji semantycznych od adapterów I/O. Największą słabością jest odległość między samodzielnie testowanymi komponentami a rzeczywistą kompozycją aplikacji. Należy poprawić integrację, granice modułów i znaczenie dowodów CI, nie automatycznie przepisywać całą platformę.

Żaden znaleziony problem produktu nie został w ramach tego PR naprawiony. Audyt nie zmienił runtime, protokołu, Cargo, migracji, istniejących workflow, rulesetów ani Merge Queue. Tymczasowy kolektor Actions został użyty wyłącznie na gałęzi audytowej; finalna dostawa usuwa go z drzewa publikacji. Jego wersje i uruchomienia pozostają dowodem historycznym, nie nowym kontrolerem CI. Nie wykonano niezależnego drugiego audytu przez innego agenta; autor nie przypisuje sobie niezależności względem własnych sond.

## 2. Metoda, ewidencja i odtwarzalność

`coverage-register.json` jest zwartą, bezstratną definicją pokrycia: wskazuje dokładne drzewo Git, liczbę plików, listę ścieżek analizy ukierunkowanej, domyślny status wszystkich pozostałych oraz hash pełnego CSV. `verify_evidence.py` odtwarza każdy z 803 wierszy z tego drzewa, sprawdza rozmiar i Git blob SHA i porównuje wynik z zapisanym SHA256. Dzięki temu nie tworzymy drugiej kopii całego kodu ani setek ręcznie utrzymywanych rekordów. Pełny CSV i jego wersja gzip znajdują się także w pakiecie przekazanym właścicielowi.

```sh
python docs/agents/reports/repository-audit-20260906/verify_evidence.py \
  --source /path/to/Oteryn-Game \
  --inventory-out /tmp/oteryn-audit-inventory.csv
```

Wskazane repozytorium musi zawierać obiekt badanego commitu. Skrypt nie pobiera sieci, nie wykonuje kodu projektu, nie instaluje zależności i nie zmienia checkoutu. Weryfikuje pochodzenie dowodów i odtwarza ewidencję; **nie powtarza buildów ani audytu semantycznego**. Nie jest podłączony do wymaganych bramek.

`execution-evidence.json` i `EVIDENCE.md` zachowują źródła pochodzenia, wyniki poleceń, skrócone transkrypcje reprodukcji i zastrzeżenia pomiarowe. Ich hashe zapisano w rejestrze. Pełne logi, CSV i eksporty oryginalnych artefaktów są dodatkowo w pakiecie właściciela; same krótkotrwałe linki Actions nie są jedynym dowodem. Rejestr tekstowy pozostaje czytelny w GitHub bez narzędzia do rozpakowywania binariów.

Poziomy dowodu są odrębne: sprawdzenie bajtów, parsowanie i analiza statyczna, ukierunkowany przegląd semantyki, wykonanie oryginalnych testów, sonda autora audytu. Brak znalezionego błędu nie oznacza zatwierdzenia obszaru. Obserwowane `FAIL` nie zostało zamienione w `PASS`.

### Inwentaryzacja

| Miara | Wynik i interpretacja |
|---|---|
| Pliki śledzone | 803; każdy odczytany w całości do kontroli tożsamości i spisu |
| Rozmiar | 12 050 024 bajty; 251 806 fizycznych linii, razem z testami, komentarzami i danymi |
| Markdown | 557 plików, 114 023 linie |
| Rust | 102 pliki, 58 922 linie; nie jest to miara produkcyjnego LOC |
| Python | 53 pliki, 15 343 linie |
| Workflow | 17 plików, 41 definicji jobów |
| Rzeczywista metadana Cargo | 21 pakietów workspace, 43 wewnętrzne zależności, 369 rozwiązanych pakietów łącznie |
| Graf deklarowany a obserwowany | Zgodny w badanej konfiguracji; brak cyklu wewnętrznych zależności |
| Analiza ukierunkowana | 97 nazwanych ścieżek; bez twierdzenia o pełnym dowodzie każdej linii |

Wszystkie pliki snapshotu były tekstem UTF-8. Ograniczony skan wzorców kluczy prywatnych i tokenów nie znalazł dopasowań; **nie zastępuje pełnego audytu sekretów, historii Git ani konfiguracji usług**. Lokalny parser TOML 1.0 odrzucił wieloliniową tabelę inline dozwoloną przez TOML 1.1; rzeczywisty Cargo 1.94 zaakceptował ją i zbudował projekt. To ograniczenie narzędzia audytora, nie błąd składni repozytorium.

## 3. Nowa kwalifikacja wykonawcza

Każdy job budował przypięty produkt `7ce1d88`, nie przypadkowy ruchomy head. Checkout sond autora był oddzielny. Sondy kopiowano do nieśledzonych celów testowych jednorazowego checkoutu; nie zmieniano śledzonych plików produktu. Sam workflow diagnostyczny nigdy nie stał się wymaganą bramką.

| Wykonanie | Run / job | Wynik |
|---|---|---|
| Inwentaryzacja całego źródła | 34049229486 / 101529713197 | SUCCESS; 803 zweryfikowane bloby |
| Linux release, wszystkie cele i funkcje, rzeczywisty PostgreSQL 17.6 | 34049229486 / 101529713591 | SUCCESS; build, testy i uruchomienie binariów release |
| Oryginalne testy z instrumentacją LLVM, potem osobne sondy Rust/SQL | 34049958965 / 101531656052 | SUCCESS z ograniczeniami pomiaru pokrycia |
| Windows release, oryginalne testy, rzeczywisty DX12 | 34049958965 / 101531656129 | SUCCESS |
| Rust poza workspace i pełne odkrywanie istniejących wejść testów Python | 34050635585 / 101533492650 | FAILURE: Rust PASS, 26 poleceń Python PASS, jedna seria FAIL |

Źródło: `execution-evidence.json`, `EVIDENCE.md` i pełne logi w pakiecie właściciela. Wszystkie pięć pobranych artefaktów porównano z SHA256 zwróconymi przez GitHub.

### Linux i PostgreSQL

Wykonano `cargo +1.94.0 build --locked --release --workspace --all-targets --all-features` oraz `cargo +1.94.0 test --locked --release --workspace --all-features` z izolowanym PostgreSQL 17.6. Log obejmuje 52 bloki celów i **718 zaliczonych wykonań testów** po niewliczeniu ponownie podsumowań procesów potomnych. To nie 718 unikalnych scenariuszy: te same źródła mogą być dołączane do różnych celów. Biblioteka serwera zawierała 364 testy, cel durability_postgres 124, a doctesty serwera 29. Wszystkie te liczby należy interpretować wraz z nazwą celu i konfiguracją bazy.

Uruchomiono dokładne binaria `target/release`, nie tylko `cargo run` w innym profilu. Serwer w smoke przeszedł; poza smoke zwrócił oczekiwany kod 2 i odmowę niezintegrowanego gameplay. Nie jest to pozytywny test działającego świata. Linuxowy binarny klient nadal ma znaczenie platformowego stuba.

### Windows i DX12

Zbudowano klienta MSVC release i uruchomiono ten sam plik `.exe --smoke`. Osobno wykonano 33 oryginalne testy komponentów Windows, renderera, wejścia i deterministycznej symulacji. Audytowa sonda utworzyła prawdziwe okno, wywołała produkcyjny `WindowsRenderer`, zaprezentowała jedną klatkę DX12 i zamknęła renderer. Log zawiera `AUDIT_REAL_DX12_PRESENT: frames=1`.

To istotnie więcej niż wcześniejsza analiza kodu, lecz **nie** test rysowania mapy, wszystkich sterowników, sprzętowych GPU, utraty urządzenia, długiej sesji ani klatkowania gry. Typ adaptera nie został zarejestrowany; nie przypisujemy wyniku konkretnemu fizycznemu GPU.

### Sondy negatywne

W rzeczywistym Rust potwierdzono, że 97 bajtów wejścia PKCE produkuje zaakceptowany verifier o długości 130 znaków. W PowerShell 7.6.5 sekwencja natywnych exit 7, następnie exit 0 zakończyła zewnętrzną sesję kodem 0. Jest to reprodukcja mechanizmu maskowania, nie twierdzenie, że ostatni kanoniczny build faktycznie zawierał ukrytą porażkę.

Dla parserów wykonano 20 000 deterministycznych buforów przez dwa dekodery, 1024 podstawienia bajtów prawidłowego ziarna oraz 6286 odrzuceń skróceń i jednobitowych zmian dwóch skompilowanych artefaktów content. Nie zaobserwowano paniki. Jest to ograniczony mutation smoke; integralność hasha odrzuca znaczną część zmian przed głębszą semantyką. Nie zastępuje kampanii fuzzingu sterowanej pokryciem.

W PostgreSQL wykonano wyekstrahowany predykat ochrony w tabeli tymczasowej i wycofano transakcję: niepusta aktywacja z pustym czasem wygaśnięcia została przyjęta. Dowód dotyczy semantyki `CHECK`, nie wykorzystania całej ścieżki autoryzacji.

### Python i program Rust poza Cargo

Na GitHub-hosted Python 3.12.14 wykonano 24 istniejące wejścia testowe i trzy walidatory. 26 poleceń przeszło. `tools/agents/tests/test_validate_governance_lifecycle.py` ma trzy błędy w `setUp`: wrapper nie udostępnia `ROOT`. To samo zachowanie odtworzono lokalnie. Kanoniczny validator może przejść, mimo że jego testowa rodzina regresyjna jest uszkodzona. Run audytowy prawidłowo pozostał czerwony.

`tools/next-wave-limit-evidence/main.rs`, nieobjęty workspace Cargo, skompilowano oddzielnie. Wszystkie 11 testów przeszło. Osiem iteracji modelowego stress dało 192 przyjęcia i 192 odrzucenia. Nie jest to benchmark przepustowości GameNode ani źródło nowych limitów produktu.

## 4. Architektura serwera

**Zachować:** jeden logiczny właściciel mutacji na kanał/instancję, jawne generacje i rewizje, odrzucanie spóźnionych wyników, asynchroniczne granice I/O oraz zewnętrzną orkiestrację. ADR-0009 i FND-03 opisują spójny modularny GameNode; nie ma dowodu uzasadniającego teraz przepisanie go na mikroserwisy.

**Poprawić:** graf pakietów nie kontroluje większości zależności wewnątrz `apps/game-server`. `foundation/mod.rs` tekstowo łączy admission i recovery przez `include!`, z szeroką fasadą eksportów. Rozdział plików nie jest tożsamy z rozdziałem uprawnień do stanu. Walidator architektury sprawdza liczność zadeklarowanych ścieżek, lecz nie ich przypisanie do manifestów; aktualne przypisanie jest poprawne, luka dotyczy przyszłego wykrywania regresji.

**Rozróżnić model od wykonania:** Ability, AI i Interaction mają wartościowe testy dołączające źródła, ale to nie dowodzi produkcyjnej kompozycji. `AbilityEngine` operuje na `fixture_health`, kopiuje całą mapę dla atomowości i zatrzymuje plany bez polityki retencji w tym module. To rozsądny model semantyki do przetestowania, nie zatwierdzona produkcyjna pętla walki. Należy zachować idempotencję i tożsamość operacji, lecz atomowy zapis oprzeć na dotkniętym stanie rzeczywistego właściciela; czyszczenie historii wymaga kontraktu replay.

**Trwałość:** rzeczywiste testy PostgreSQL są mocną stroną. Problemem integracji są granice reprezentacji, kolejność konfliktów i niezależność aktualnych źródeł od zapisanych receiptów. Każdy nowy stan musi przejść bezstratnie przez domenę, operację, zapis, restart, retry i reconciliation. PR358 dostarczył most pełnej ciągłości owning-loss; wcześniejsze „tylko alokacja #353” jest nieaktualne. Nie dostarczył automatycznie B codec/SQL, driver accounting ani producentów Child C.

**Nieudowodnione:** całościowy scheduler, odporność jednego kanału na przeciążenie sąsiedniego, limity rzeczywistych sterowników i buforów, wszystkie wyścigi oraz recovery procesu pod obciążeniem. Jedna linia „multichannel” nie zastępuje tych dowodów. Szczegóły mają statusy C05–C10, C27–C28.

## 5. Architektura klienta

Najważniejszą luką jest rozdzielona kompozycja: testowany `ClientBootstrap` posiada własne elementy runtime i wejścia, podczas gdy shell Windows konstruuje osobne `Application`, okno i renderer. Biblioteki mają sensowne granice, ale aplikacja i jej test nie powinny składać dwóch różnych systemów.

Rekomendowany jeden kontroler aplikacji posiada cykl życia, zadania, input, stan UI i projekcję. Shell platformowy powinien tłumaczyć zdarzenia, a nie być konkurencyjną kompozycją. Stan autorytatywny serwera, predykcja i UI muszą pozostać rozdzielone. Współdzielony codec wymaga jawnego zakończenia ograniczeń pre-native, nie kopiowania serwerowych typów pod nową nazwą.

Model `SurfaceState` jest oddzielony od backendu i kontroluje generacje. Nowa próba potwierdziła działanie minimalnego DX12, lecz bieżąca prezentacja to clear pass. `ResourceCache` nie stanowi jeszcze kompletnej polityki pamięci i odtwarzania zasobów. Runtime posiada cancellation, ale `spawn` sam nie gwarantuje nadzorowania i zamknięcia każdego zadania. `diagnostics` definiuje zdarzenia, nie kompletny system zapisu telemetrycznego.

Shell gubi część przyczyn błędów; smoke wychodzi przed utworzeniem renderera. Sonda modelu potwierdziła odrzucenie prezentacji po zerowym resize, ale pełne fizyczne odtworzenie minimalizacji i wznowienia całej aplikacji pozostaje otwarte. Nie należy opisywać tego jako potwierdzonej awarii u wszystkich graczy.

## 6. Testy, pokrycie, CI i buildy

Testy niezmienników są wartościowe i nie należy ich usuwać dla szybkości. Jednocześnie test przez `#[path]` może omijać fasadę produkcyjną. Potrzebne są oba rodzaje kontroli: white-box i scenariusz używający API kompozycji produkcyjnej. Syntetyczny harness tworzy niewielką projekcję, obraz i routing akcji; nie jest testem działającego serwera ani GPU. Nie każda drukowana liczba jest asercją.

Surowy eksport LLVM pokazuje 47 992/52 882 linii, czyli **90,75% w swoim własnym zbiorze pomiarowym**. Zawiera kod testów i powtórzone ścieżki source-inclusion; 101 rekordów nazw normalizuje się do 93 fizycznych ścieżek. LLVM zgłosił **38 funkcji z niezgodnymi danymi**. Gałęzie nie zostały zmierzone. Brakuje platformowych i niewyinstancjonowanych ścieżek. Wynik nie nadaje się jako jeden próg „pokrycia całego produktu”; log ostrzeżenia i pełne zastrzeżenia zachowano. Nowe sondy wykonano dopiero po eksporcie oryginalnego pokrycia.

Przejrzano wszystkie 17 workflow. Akcje i reusable workflow mają przypięte SHA, a bezpośrednio wskazane lokalne skrypty istnieją. Klasyfikator PR używa chronionej bazy i konserwatywnego FULL; to należy zachować. Jego snapshoty wejść są jednak kosztowne w utrzymaniu, bo zwykła zmiana konsumenta unieważnia optymalizację dokumentacji. Nie mierzono rzeczywistych oszczędności netto ani rachunków Actions. Historyczne dane #308 są cudzym dowodem i nie zostały potraktowane jako nowe własne pomiary.

`semantic_contract_audit.py` wybiera tylko trzy dokładne zestawy ścieżek. W kontrolowanej próbie oryginalny zestaw wywołuje regułę, lecz dodanie README albo zmiana pojedynczego runtime daje `NOT_APPLICABLE`. To wąski walidator historycznych dostaw, nie ogólny semantyczny audyt architektury.

Kanoniczne PR/MQ/push utrzymują ręcznie podobne procedury. Oddzielne uruchomienia mają uzasadnienie, bo kwalifikują inne konteksty; nie są automatycznie zbędnymi duplikatami. Zgrupowane komendy Windows wymagają jednak kontroli każdego wyniku. Współdzielić można wykonanie techniczne, bez przekazywania protected-base authority do skryptu kontrolowanego wyłącznie przez kandydata.

Nowe próby wykazały możliwość uruchomienia release, ale kanoniczny smoke nadal buduje `--release`, po czym wywołuje `cargo run` w innym profilu. Brakuje dowodu pełnego pakietu wydania, aktualizacji, rollbacku i powtarzalności bitowej. Cache kompilacji warto ocenić po pomiarze zimnych/ciepłych buildów, nie przez usuwanie negatywnych testów. CodeQL w badanych workflow analizuje Python i Actions, nie jest dodatkowym wynikiem Rust SAST.

## 7. Dokumentacja, prompty, narzędzia i granice zewnętrzne

Wszystkie 557 Markdownów objęto pełnotekstowym parsowaniem, badaniem odwołań i duplikacji. Nie oznacza to semantycznego zatwierdzenia każdego kontraktu. 79 rozpoznanych linków Markdown nie wykazało brakujących lokalnych plików; odrębny skan zapisów w backtickach wytypował 105 odwołań do sprawdzenia, nie 105 błędów. Część to historia, przykłady i obce repozytoria. Konkretny błąd: obowiązkowy start reusable koordynatora kieruje do zarchiwizowanego taska pod nieistniejącą ścieżką active.

Zidentyfikowano 57 grup identycznych długich akapitów i 130 801 nadmiarowych bajtów powtórzeń. Nie jest to liczba tokenów ani koszt z rachunku. Registry ma 48 wpisów: 47 reusable i jeden retired. Standard promptów już preferuje task-specific delta, ale część promptów zachowuje wycofane procedury review. Należy dokończyć adopcję istniejącego standardu i usunąć sprzeczne instrukcje wykonawcze zamiast dopisywać następny standard. Nie wykonano eksperymentu porównawczego zachowania agentów ani modeli.

Nie każdy plik ze starym tytułem lub nazwą candidate jest błędem: zaakceptowany overlay może celowo zachowywać historyczny dokument. Podobnie różnica pinned polityki Game względem nowszej META jest długiem adopcyjnym, nie automatycznie sprzecznością obowiązującej hierarchii.

Tooling zawiera parsowanie i walidację wejść content, eksportów Atlas, rekonstrukcji map i modelowych limitów. Testy Python i dodatkowy Rust spoza Cargo wykonano. Wspólny loader indeksów appearances nie wykonuje pełnej walidacji digestów, którą ma osobny verifier: synthetic charakterystyka pokazuje, że bez obowiązkowego wywołania verifiera zmienione dane mogą wejść do helpera. Bieżący sprawdzony pipeline weryfikuje je wcześniej; nie jest to dowód aktualnego incydentu. Należy preferować API przyjmujące już zweryfikowany produkt i wspólną implementację parserów granic.

Sprawdzono dwa lokalne przypięte digests kontraktów i schemat manifestu Reference. Nie uruchomiono całego korpusu legacy ani nie potwierdzono praw do wszystkich assetów. Reference parity pozostaje evidence-gated. Przeczytano przypięte reusable workflow Platform i odświeżono jej main `3b2ea1c7392187d5d22488673073dc8f8305a374`; wyszukiwanie bez wyniku nie dowodzi nieistnienia producenta. Pełna interoperacyjność Platform/Game/Atlas pozostaje częściowo niezweryfikowana, a te repozytoria nie otrzymały żadnych zapisów.

Szczegóły warunkowych ryzyk administracyjnych przygotowano w osobnym aneksie dla właściciela, zgodnie z SECURITY.md. Nie opublikowano instrukcji obejścia ani danych sekretów. Testy izolowały sieć i nie zmieniały żadnych ustawień GitHub. Brak dostępu do efektywnej konfiguracji sekretów i uprawnień nie został przedstawiony jako potwierdzona podatność produkcyjna.

## 8. Rejestr ustaleń i kolejność działań

Szczegółowy `findings.json` zawiera dowód, zakres pewności, wpływ, naprawę i warunek odbioru każdego ustalenia.

| ID | Priorytet / charakter | Ustalenie |
|---|---|---|
| F01 | P1, wiarygodność CI | Maskowanie wcześniejszego błędu natywnego w zgrupowanym PowerShell |
| F02 | P2, błąd testów | Trzy błędy setUp testów lifecycle przy zielonym głównym validatorze |
| F03 | P2, luka kwalifikacji | Semantic audit ograniczony do trzech exact path sets |
| F04 | P2, konstrukcja klienta | Bootstrap testowy i wykonywalna aplikacja mają różne kompozycje |
| F05 | P2, obserwowalność klienta | Smoke bez renderera i utrata przyczyn wybranych błędów |
| F06 | P2, zgodność protokołu logowania | Brak maksymalnego ograniczenia długości verifiera PKCE |
| F07 | P2, constraint | Predykat CHECK dopuszcza NULL dla wygaśnięcia przy aktywacji |
| F08 | P2, bezpieczeństwo testów | Różna siła walidacji URL w dwóch helperach PostgreSQL |
| F09 | P2, instrukcje | Obowiązkowa nieistniejąca ścieżka active taska w reusable koordynatorze |
| F10 | P2, koszt i spójność | Wycofane procedury w promptach i wielokrotne kopie instrukcji |
| F11 | P2, walidacja architektury | Brak kontroli przypisania paths do rzeczywistych manifestów |
| F12 | P2, routing | Niepełne zależności filtrów specialized exporter CI; znane też #308 |
| F13 | P2, release evidence | Kanoniczny smoke nie uruchamia wcześniej zbudowanego artefaktu release |
| F14 | P2, pomiar audytora | Surowe LLVM coverage nie jest pokryciem całego produktu |
| F15 | P3, defensive API | Loader/validator eksportów i ręczne parsery wymagają wspólnego kontraktu |
| F16 | P2, dokumentacja kontroli | Komentarz CODEOWNERS nie opisuje rzeczywistej polityki wymagania review |
| F17 | P2, gotowość integracji | Modele/fasady/komponenty nie są jeszcze kompletną ścieżką gameplay |

Najpierw naprawić F01/F02/F09 i precyzję nazw dowodów. F06–F08 skierować do właścicieli właściwych ścieżek, bez konkurencyjnego writera i bez przepisywania wydanej migracji 0001. Następnie ujednolicić kompozycję klienta i zintegrować obecne komponenty serwera z rzeczywistymi źródłami i trwałością. Refaktor Foundation powinien nastąpić po stabilizacji aktywnych kontraktów, nie jako równoległe przemeblowanie. Optymalizację CI i promptów odbierać porównaniem wyników i kosztów, nie liczbą usuniętych linii.

## 9. Końcowa kontrola pominięć

Nie pozostał plik snapshotu bez wpisu w odtwarzalnej ewidencji ani obszar z 32-punktowego rejestru bez statusu. **Pozostały niewykonane lub częściowo wykonane kontrole**, wyszczególnione w `controls.json`: pełny semantyczny przegląd wszystkich źródeł i historii, rzeczywiste producent–konsument E2E, długotrwały fuzzing i testy współbieżności, benchmarki/noisy-neighbor/soak, cała macierz GPU i awarii aplikacji, backup/restore i utrata procesu, pełny release/update/rollback, wewnętrzna analiza wszystkich 348 zależności zewnętrznych i behawioralne testy promptów. Stan sekretów, realne koszty i część infrastruktury pozostają nieznane.

To ograniczenia audytu, nie automatyczne stwierdzenie, że danych mechanizmów nie ma. Nie można rozliczyć ich jako PASS, ponieważ coś przeszło w sąsiedniej warstwie. Wpis `NOT_EXECUTED` nie jest również zakończonym testem negatywnym.

Poprzednie raporty należy czytać jako historyczne i częściowe. Nowe własne wykonania zastępują ich ograniczenia braku Rust/Windows/PG; samo środowisko lokalne nadal tych możliwości nie miało. #353 jest na badanym main zaimplementowane przez #358. Dzisiejsze 803 pliki i rzeczywista metadana Cargo zastępują deklaratywne lub niepełne wcześniejsze pomiary. Żaden nowszy nieprzeczytany commit nie jest automatycznie objęty tym raportem.

**Werdykt końcowy:** istnieją realne, budujące się i testowane fundamenty. Nie ma podstaw do pełnego zatwierdzenia produkcji ani bezwarunkowego „100% audytu wszystkiego”. Publikacja zapisuje zarówno nowe dowody i defekty, jak i konkretną resztę do sprawdzenia. Issue #359 pozostaje rejestrem niedomkniętych kontroli, a PR #360 dostawą tego raportu; scalenie dokumentacji nie może samo zamknąć pozostałych zobowiązań.

## 10. Źródła techniczne i pochodzenie

Źródłem dla stanu produktu są ścieżki i bloby badanego commitu w `findings.json`, dla wykonania — trzy wymienione runy i pięć zachowanych artefaktów w `execution-evidence.json`. Bazowe decyzje: ADR-0009, FND-03, root i nearer AGENTS, BUILD_TEST_MATRIX, workspace-boundaries i rzeczywisty Cargo metadata. Nie powielamy pełnego źródła produktu w raporcie.

Normy wykorzystane do interpretacji: RFC 7636 §4.1 (https://www.rfc-editor.org/rfc/rfc7636.html#section-4.1), dokumentacja PostgreSQL 17 dotycząca CHECK (https://www.postgresql.org/docs/17/ddl-constraints.html), specyfikacja TOML 1.1 (https://toml.io/en/v1.1.0) i dokumentacja rustc instrument-coverage (https://doc.rust-lang.org/rustc/instrument-coverage.html). Wyniki sond nie zależą wyłącznie od tych opisów: dokładne wykonania zostały zachowane.

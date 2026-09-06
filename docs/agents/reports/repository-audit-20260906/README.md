# Oteryn Game — końcowy raport audytu programu, architektury i weryfikacji

Data: 2026-09-06. Repozytorium: `Oteryn/Oteryn-Game`. Zadanie: #359. Publikacja: PR #360. Raport zastępuje wcześniejszy ogólny werdykt, nie usuwa historycznych dowodów. Status dowodowy: **ocena zakończona z zastrzeżeniami zakresu; wynik negatywny dla przejścia do kolejnej bramki integracji**.

## 1. Executive verdict

**AT_RISK. PROGRAMME_AUDIT = FAIL.**

Nie kwalifikuję obecnej podstawy do aktywacji zależnego Server Seam. W scalonym moście #358 pozostaje P1 niezgodności PREPARE/COMMIT; naprawa #361 jest nieukończona i wstrzymana. Natywny klient release zakończył się po minimalizacji kodem 0. Konstrukcja części testów i CI pozwala na zbyt szeroką interpretację zielonego wyniku. Model własności stanu, rozdział autoryzacji od transportu i wersjonowana trwałość są wartościowe: nie rekomenduję restartu projektu, zmiany języka ani wyłączania testów. Potrzebne są naprawy określonych niezmienników i kwalifikacja najmniejszej rzeczywistej integracji. Werdykt wynika z bieżących usterek, **nie z nieistnienia przyszłej kompletnej gry**. Nie jest to formalny dowód poprawności ani deklaracja semantycznego przeczytania każdej linii wszystkich zależności.

## 2. Snapshot i zakres dowodów

| Przedmiot | Dokładna rewizja / identyfikator |
|---|---|
| Produkt użyty we wszystkich opisanych testach natywnych | `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3` |
| Drzewo produktu | `359a52348bfdf8088a7cd456f4015b05279721b6` |
| Odczyt protected Game main i instrukcje publikacji | `b008614881fcc74f09e55e4d1b9e6c64ece04ce9` |
| META / aktualna polityka AI review | `d0d5a54c5f06db9423d14b17e7f8eadefd15c6fb` |
| Platform: źródła historycznej próby / odczyt bieżący | `3b2ea1c7392187d5d22488673073dc8f8305a374` / `294e18909b8319695021011ccbeb1386cac32ced` |
| Atlas: źródła zachowane w artefakcie integracyjnym | `51623c7dab2346cee39cd51e3caa845bf4b65426`; nie przedstawiam ich jako odczytu najnowszego wdrożenia |
| Audyt / jedyna gałąź publikacji | #359 / #360 / `audit/repository-coverage-20260906` |

Porównanie Game 7ce→b008 obejmuje wyłącznie trzy pliki instrukcji agentów, nie kod produktu, Cargo, migracje lub workflow. Platform 3b2→294e obejmuje pięć plików governance; odczytane mechanizmy GameAuth nie zmieniły się. SHA publikacji i jej CI należy odczytywać z PR, nie mylić z SHA badanego produktu.

**Pokrycie i ograniczenia.** Zweryfikowano bajty i tożsamość wszystkich 803 śledzonych plików: 557 Markdownów, 102 plików Rust, 53 Python oraz pozostałe konfiguracje/dane. Pełnotekstowy spis i kontrole statyczne nie są semantycznym zatwierdzeniem każdej linii. Historyczny rejestr wskazuje 97 ścieżek analizy ukierunkowanej; dodatkowe zakresy i odczytane fragmenty dokumentuje `review-scope.json`. Przegląd pozostał oparty na ryzyku, bieżącym etapie i rozpoznanych rodzinach błędów. Historia 457 commitów została zebrana jako metadane i numstat, nie jako niezależny przegląd wszystkich historycznych patchy.

Podstawą zadania jest pierwotne polecenie użytkownika. Repozytoryjny `OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` został użyty jako pełna lista kontrolna i struktura raportu, nie jako nowe uprawnienie. Jego §5.4–5.6, §6 i §31–34 wymagają oceny adekwatnej do etapu, a nie maksymalizacji liczby odczytów lub formalnej weryfikacji całego przyszłego produktu. Późniejsze polecenie użytkownika i #359 osobno autoryzują zapis raportu oraz izolowane próby; nie autoryzują napraw produktu ani zewnętrznych wdrożeń. Rozliczenie wymagań znajduje się w `prompt-traceability.json`.

## 3. Bieżący kamień milowy

Program #162 zmierza do natywnego wycinka rozgrywki, lecz najbliższą bezpieczną bramką jest **poprawna, trwała i bezstratna integracja admission/reconnect z rzeczywistymi źródłami autoryzacji**. Nie jest nią ukończenie rynku, PvP, całej progresji lub produkcyjnego klastra.

Wizja w `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` wiąże ruch, intencję, koszty i ryzyko z trwałym wynikiem, zachowuje sens solo i party oraz oddziela Reference od Evolved. Ocena: wystarcza do kierowania pierwszym wycinkiem, nie zastępuje formuł balansu, dowodów Reference ani implementacji. Priorytet ochrony wartości przed strojeniem ekonomii należy zachować. Nie ma podstaw do procentowej oceny „ukończenia gry” z LOC lub liczby scalonych ADR-ów.

**Bieżące warunki:** F20 w #353/#361; implementacja i integracja B #329/#335; rozliczenie zasobów SQLx/TLS #351/#356; rzeczywiste źródła #319; następnie kwalifikacja #247. Najnowszy komentarz #162 `5560996518` zapisuje **OWNER STOP** dla wykonawców produktu. Audyt nie wznawia ich pracy ani nie przejmuje ścieżek. Stare liczniki 60/120 minut nie są po b008 obowiązującym powodem zatrzymania; rzeczywiste polecenie właściciela pozostaje nim.

**Istotna korekta poprzedniego raportu:** „#353 dostarczone przez #358” opisywało jedynie scalenie, nie ukończenie semantyki. Komentarze `5560921418`, `5560950482` i PR #361 jednoznacznie dokumentują otwarte P1. Starszy zielony CI nie jest dowodem naprawy.

## 4. Programme evidence matrix

| Obszar / faza | Stan i rzeczywisty dowód | Werdykt |
|---|---|---|
| Tożsamość źródeł — REQUIRED_NOW | 803/803 blobów i rozmiarów; rzeczywiste Cargo: 21 pakietów workspace, 43 zależności wewnętrzne, 369 rozwiązanych pakietów | PASS dla przypiętego źródła, nie wnętrza wszystkich bibliotek |
| Linux release — REQUIRED_NOW | Run `34049229486`, job `101529713591`: build all-target/all-feature, testy z PG17.6; 718 zaliczonych wykonań, 52 podsumowania, zero failed/ignored | PASS w wykonanym zakresie; powielone testy nie są unikalnymi scenariuszami |
| Windows release — REQUIRED_NOW | `34049958965` / `101531656129`: dokładny EXE, 33 testy komponentów, produkcyjny DX12 i jedna klatka | PASS uruchomienia/backendu; nie pełnego klienta |
| Cykl życia okna — REQUIRED_BEFORE_NEXT_GATE | `34054707605` / `101544372599`: prawdziwa minimalizacja, następnie proces kończy się kodem 0 | FAIL, F05 |
| Trwałość/role/restore — REQUIRED_BEFORE_NEXT_GATE | `34054707605` / `101544372476`: wykonana migracja, odtworzony rekord syntetyczny i ledger, ponowna migracja; `connect_runtime` działa rolą bez CREATE TABLE | PASS ograniczonej próby; nie produkcyjnego DR |
| Pokrycie — REQUIRED_NOW dla interpretacji | Osobny profil każdego binarium: server-lib 25600/28877 linii; PG 11241/20760; brak ostrzeżeń | Pomiar poprawiony; test-inclusive, bez gałęzi/MC/DC, mianowników nie sumować |
| Python/governance — REQUIRED_NOW | `34050635585`: 26/27 poleceń PASS, trzy błędy setUp jednej serii | FAIL, F02; wynik nie został ukryty |
| Parsery — REQUIRED_BEFORE_NEXT_GATE | `34056706385`: libFuzzer/ASan, wire 69627322 i content 4757388 wykonań, po 301 sekund, bez crasha | PASS ograniczonej kampanii, nie wszystkich semantyk i transportu |
| Rust SAST — REQUIRED_NOW dla oceny | CodeQL security-extended: 55 wyników, 54 lokalizacje w testach, jedna nierozstrzygnięta lokalizacja include; diagnostyka ekstraktora | QUALIFIED; brak podstaw do „zero podatności” |
| AI/Ability — REQUIRED_BEFORE_NEXT_GATE | `34059391572` / `101557071717`: oryginalny Rust odtwarza F18 i F19 wraz z poprawnymi kontrolami | FAIL właściwości; PASS reprodukcji nie oznacza naprawy |
| Pełna sesja natywna — REQUIRED_BEFORE_NEXT_GATE | Binarium serwera poza smoke odmawia gameplay (exit 2); #247 zależy od poprawnych A/B/C | NOT_QUALIFIED; nie wolno awansować fixture do produkcji |
| Load/soak, fizyczne GPU, instalator, wdrożeniowe DR — FUTURE_REQUIRED | Istnieją założenia i część dowodów komponentowych, brak takiej kwalifikacji produktu | Nie są obecnym warunkiem zakończenia raportu; będą warunkiem właściwego wydania |

Szczegółowe identyfikatory, sumy kontrolne i interpretacje: `execution-evidence.json`, `EVIDENCE.md`, `CONTINUATION.md` oraz `closeout-evidence.json`. Zachowano nieudane próby narzędziowe, zamiast usuwać je z historii.

## 5. Architecture consistency matrix i ocena konstrukcji

| Granica | Ocena | Decyzja audytowa |
|---|---|---|
| Native Rust / protocol-oteryn / zakres Game | PASS kierunku | Nie wracać do Canary jako docelowego runtime bez nowej decyzji; Rust nie eliminuje błędów semantyki i konfiguracji |
| World, Channel, Instance, Node | PASS modelu / PARTIAL wykonania | Nie utożsamiać placementu procesu z tożsamością świata lub generacją właściciela |
| Jeden właściciel mutacji na scope | PASS kontraktu / PARTIAL złożenia | Zachować kolejność wejść i walidację opóźnionych wyników, I/O nie może samodzielnie mutować gameplay |
| Admission/recovery → trwałość | FAIL dla pełnego early-terminal bridge | F20: kontrakt wymaga zmiany kotwicy w PREPARE, kod odkłada ją do COMMIT |
| Transport → autoryzacja | PASS rozdziału / NOT_QUALIFIED integracji | Poprawna ramka, podpis lub TLS nie stanowi admission; brak rzeczywistych źródeł nie może być wypełniony fixture |
| Komponenty gameplay → produkcyjne API | PARTIAL | API i testy istnieją, ale source-included Ability/AI/Interaction nie tworzą jeszcze ścieżki wykonywanej przez serwer |
| Shell klienta → stan aplikacji | PARTIAL / FAIL cyklu życia | Osobne kompozycje biblioteki i binarium oraz błąd minimalizacji; potrzebny wspólny kontroler |
| Content → bezpieczna projekcja | PASS sprawdzonych ograniczeń / PARTIAL rzeczywistego produktu | Zachować allowlistę i integralność; corpus/provenance nie są zastępowane przez udany fuzz parsera |
| Workspace → granice wewnętrzne | PARTIAL | Graf bez cykli nie egzekwuje architektury wewnątrz jednego dużego crate; F11 |
| PR → MQ → wydanie | PARTIAL | Dokładne rewizje i fail-closed routing są dobre; F01/F13 ograniczają dowód Windows |
| Markdown → aktualna prawda | PARTIAL | Historia, kontrakt i bieżący stan wymagają oddzielenia; martwe odwołania i stare bloki review nie są nową authority |

### Serwer: co zachować i co poprawić

Kontrakt FND-03 rozdziela NodeRuntime, WorldServices, ChannelRuntime i InstanceRuntime. Fencing dotyczy właściciela scope, a NodeId procesu. Dzięki temu asynchroniczny callback, stary timer lub rezultat obliczeń nie musi stawać się niejawnie nową authority. To dobra baza deterministycznej i odpornej symulacji. Nie oznacza jeszcze, że istnieje wydajny scheduler, pełna izolacja przeciążonych kanałów lub wielowęzłowy failover. Jeden logiczny właściciel pozostawia koszt autorytatywnej pracy najbardziej obciążonego scope; liczba kanałów sama nie dowodzi skalowania. Nie zmieniałbym topologii przed reprezentatywnym pomiarem.

ADR-0015 traktuje modularny monolit jako roboczą hipotezę, nie zatwierdza dowolnej przyszłej granicy procesu. Ocenę „dobry modularny monolit” trzeba oddzielić od faktycznej widoczności API. `foundation/mod.rs` dołącza admission i recovery przez `include!`, szeroko re-eksportuje rodziny typów, a większość domen pozostaje w jednym `game-server`. Graf pakietów nie wykryje zależności pomiędzy tymi modułami. Zalecam ograniczone fasady i egzekwowanie dostępu do stanu, nie automatyczne tworzenie crate dla każdej ramki diagramu. Refaktor nie może wyprzedzać naprawy P1 ani naruszać aktywnych właścicieli ścieżek.

Typowane identyfikatory, sprawdzane rewizje, explicit context i oddzielenie proposed effect od aktualnego stanu są wartościowe. Ale sam typ danych nie jest źródłem prawdy: `CharacterRecord`, obecność konta, przekazany grant scope i katalog Platform nie tworzą aktualnej authority. Sealed API utrudnia nieuprawnione konstruowanie capability; nie naprawi błędnego momentu zmiany stanu. F20 jest konkretnym przykładem tej granicy.

W Ability atomowy model kopiuje mapę fixture-health i przechowuje plany wykonanych operacji. To użyteczny test semantyki, nie dowód właściwej retencji lub kosztu realnego świata. Przed aktywacją efekty powinny działać na dotkniętych danych rzeczywistego właściciela, z kontraktem idempotencji i retencji. F19 pokazuje ponadto, że walidowany konstruktor helpera nie chroni publicznego wariantu enum przed ręcznym utworzeniem niedozwolonego efektu. AI zachowuje deterministyczny ranking, lecz F18 łamie unikalność kandydatów przy różnych priorytetach. Te usterki są istotne przed integracją, a nie dowodem działającego ataku na nieaktywny serwer.

Warstwa trwałości słusznie rozdziela migrator, kompatybilność ledgeru i runtime bez DDL. Izolowane transakcje, klasyfikacja niejednoznacznego zakończenia i ponowny odczyt chronią ważniejsze własności niż sama liczba testów. Nie wolno jednak uznać testu reconnect za dowód fresh admission lub jednego testowego dump/restore za odtworzenie aktualnych źródeł security i generacji. Zmiana wydanej migracji 0001 nie jest zalecaną naprawą F07; potrzebna jest migracja addytywna w zakresie właściciela Durability.

### Klient: konstrukcja i cykl życia

Rozdzielenie identity, klienta Platform, input, runtime i renderera jest sensowne. Produkcyjny shell powinien jednak używać tej samej kompozycji, którą badają scenariusze biblioteki. Obecnie testowany `ClientBootstrap` nie jest instancją uruchamianą przez Windows Application. Samo istnienie Tokio, cancellation token i JoinHandle nie dowodzi nadzorowania wszystkich zadań ani ograniczonego zamykania przy zawieszonych zależnościach.

Odczytany renderer realizuje prawdziwy backend DX12 i clear/present, nie pełny świat/sprite/UI. `SurfaceState` stanowi dobry czysty model. Przejście modelu do Suspended przy zerowym rozmiarze musi jednak zatrzymać redraw w shellu. Zebrany release wyszedł po minimalizacji kodem 0; źródło łączy bezwarunkowy redraw z odrzuceniem renderu poza Configured i utratą błędu przy wyjściu z pętli. Zaobserwowane zakończenie jest faktem, ta rekonstrukcja przyczyny jest wnioskiem o wysokiej pewności.

Cache renderera ma fencing generacji przy insert, nie ma jeszcze pełnej retencji, usuwania i budżetu tekstur. To zakres do zakończenia przed renderowaniem realnej zawartości, nie powód do wdrażania teraz nowego silnika. Input, stan UI, predykcja i stan autorytatywny muszą pozostać odrębne; renderer powinien konsumować projekcję, nie decydować o stanie gameplay lub wykonywać blokujące requesty.

Klient Platform ma ograniczony odczyt strumienia, timeouty, brak redirectów i allowlistę modelu katalogowego. Tego nie należy osłabiać dla obejścia nieistniejącego native admission. Odczytane PHP Platform nadal wykorzystuje integer Identity/Canary binding; potwierdza użyteczne istniejące mechanizmy odwołania, ale nie kompatybilne cztery native source-observation operations z #319. Wcześniejsze archiwum pominęło PHP: poprawiono dowód bezpośrednimi odczytami i sprawdzeniem, że zmiany Platform nie dotyczyły runtime. Nie wyprowadzam z wyszukiwania kodu stanu wdrożonych endpointów.

### Testy, buildy, CI i supply chain

Trzeba zachować testy lokalnych niezmienników, ale dodać dowód przez publiczne API używane przez aplikację. Test dołączający źródła przez `#[path]` może legalnie badać sealed internals; nie dowodzi, że fasada produkcyjna faktycznie składa ten komponent. Syntetyczny harness 2×2 i pojedyncza klatka DX12 należą do innych warstw niż natywna sesja klient–serwer. W liczbach nie sumuję powielonych testów jako nowych scenariuszy.

Canonical CI właściwie rozstrzyga ryzyko z chronionej bazy, stosuje fallback FULL, przypina akcje i kwalifikuje wspólnego kandydata MQ. Jednak powielone sekwencje PR/push/MQ ułatwiają dryf. Zgrupowane natywne polecenia PowerShell muszą propagować każdy kod błędu. Release build nie jest testowany przez późniejszy `cargo run` bez release. Udało się osobno uruchomić dokładne release EXE, ale nie naprawia to przyszłej canonical weryfikacji.

Snapshoty konsumentów pozwalające pomijać testy dokumentacji są konserwatywne, lecz wymagają utrzymania. Nie wolno automatycznie odnawiać hashy tylko dla zachowania skip. Cache i współdzielenie procedur warto oceniać przez porównywalne pomiary bez przenoszenia authority do kodu kontrolowanego przez PR. Istniejący program #308 i historyczna próbka 100 kandydatów/507 runów nie dają podstaw do obiecania nowych miesięcznych oszczędności lub kosztowej optymalności.

Lockfile, MSRV, feature selection, zakaz unsafe w kodzie workspace i kontrola licencji są dobrymi podstawami. Nie rozciągają się na dowód poprawności wszystkich 348 zewnętrznych implementacji ani ich skryptów build. CodeQL Rust uruchomiono osobno: 54 lokalizacje wskazują testowe wartości, lecz jedna lokalizacja makra oraz diagnostyka ekstraktora pozostają nierozstrzygnięte. Nie kwalifikuję ich zbiorczo jako false positive i nie stwierdzam „braku podatności”.

### Markdown, prompty i organizacja pracy

Podział na architecture/contracts/agents/migration jest użyteczny. Problem stanowi dryf aktualności i koszt ustalania authority, nie sam Markdown. Obowiązkowy odczyt starego „current status”, aktywnego zadania już przeniesionego do archive i powielonych bloków review może skierować agenta do fałszywego blokera. Current GitHub i przyjęte kontrakty mają pierwszeństwo; historia powinna być czytelnie nieautorytatywna.

Rejestr zawiera 48 promptów: 47 reusable i jeden retired. Dotychczasowe walidatory sprawdzają dziedziczenie i spójność strukturalną, nie rzeczywiste zachowanie każdego modelu. Deduplikacja długich paragrafów wykazała nadmiarowe bajty, nie tokeny lub pieniądze. Obowiązuje już standard task-specific delta i oceny przez ablation: rekomenduję jego wdrożenie w istniejących promptach, nie kolejny system instrukcji. Nie wykonano porównawczego benchmarku modeli/effort ani kontroli każdego historycznego argumentu we wszystkich 557 plikach.

## 6. Material findings

Lista niżej jest końcową kwalifikacją; pełne historyczne opisy F01–F17 zachowano w `findings.json`. Bieżące aktualizacje i nowe reprodukcje definiuje `assessment.json`, który ma pierwszeństwo przy różnicy stanu. Żadna usterka produktu nie została naprawiona przez tę publikację.

| ID | Waga / faza | Ustalenie i skutek |
|---|---|---|
| F20 | P1 / CURRENT_GATE | Early-terminal bridge zmienia kotwicę i claims dopiero przy COMMIT zamiast PREPARE; modyfikuje pola oryginalnego FreshAdmissionCommit. #361 nie stanowi jeszcze naprawy |
| F01 | P1 / CURRENT_GATE | Zgrupowane polecenia Windows mogą zamaskować wcześniejszy native exit; odtworzono mechanizm, nie ukryty błąd konkretnego historycznego builda |
| F05 | P2 / NEXT_GATE | Rzeczywisty klient release wychodzi po minimalizacji z exit 0; błędy renderera są gubione, canonical smoke omija renderer |
| F18 | P2 / NEXT_GATE | AI przyjmuje rozdzielone priorytetem powtórzenia CandidateId; odtworzone dla sześciu permutacji |
| F19 | P2 / NEXT_GATE | Publiczny wariant Damage(-7) omija helper i przechodzi plan/commit, zwiększając fixture-health o 7 |
| F02 | P2 / CURRENT_GATE | Trzy testy lifecycle nie docierają do asercji; suite pominięta w canonical wykonaniu |
| F03 | P2 / CURRENT_GATE | Nazwa semantic audit sugeruje więcej niż trzy historyczne dokładne zestawy ścieżek; dodatkowy plik może zmienić wybór na NOT_APPLICABLE |
| F04 | P2 / NEXT_GATE | Dwie kompozycje klienta rozdzielają testy biblioteki od wykonywalnej aplikacji |
| F06 | P2 / NEXT_GATE | PKCE przyjmuje wejście tworzące verifier ponad limit specyfikacji; to zgodność, nie wykazany takeover |
| F07 | P2 / NEXT_GATE | Predykat SQL dopuszcza niepoprawną kombinację NULL; rzeczywista izolowana próba PG |
| F08 | P2 / CURRENT_GATE | Dwa harnessy bazodanowe mają różną jakość ograniczenia lokalnego połączenia; nie wykonano połączenia poza loopback |
| F09 | P2 / CURRENT_GATE | Reusable coordinator wymaga nieistniejącej aktywnej ścieżki zadania już w archive |
| F11 | P2 / NEXT_GATE | Checker sprawdza liczność paths, nie zgodność z manifest_path; nie dowodzi granic intra-crate |
| F12 | P2 / NEXT_GATE | Workflow semantic-search pomija zależny exporter w paths; istniejący zakres #308 |
| F13 | P2 / NEXT_GATE | Canonical smoke nie uruchamia poprzedzającego artefaktu release |
| F10 | P3 / CURRENT_GATE | Pozostał dług starych bloków review; current root wyraźnie je superseduje, więc nie są odrębną authority |
| F15 | P3 / NEXT_GATE | Bezpośredni helper appearance nie wymusza sprawdzenia companion; obecny nadrzędny pipeline je wykonuje |
| F16 | P3 / CURRENT_GATE | Komentarz CODEOWNERS przecenia wymuszane approvals; nie oznacza to potrzeby przywrócenia starej bramki AI |
| F14 | NOTE / CURRENT_GATE | Ostrzeżenia metody pomiaru usunięto przez izolowane profile; pełny produkcyjny mianownik pozostaje nieustalony |
| F17 | NOTE / NEXT_GATE | Niepołączone modele i niewielki cache są jawną granicą gotowości, nie same w sobie błędem etapu projektu |

**F20 — dowód i naprawa.** Kontrakt `docs/architecture/reviews/OTERYN_GAME_TERMINAL_SESSION_REPLACEMENT_COLLISION_RECONCILIATION_DECISION_2026-08-28.md:181–239` wymaga zamiany kotwicy w PREPARE. `foundation/admission_recovery_inner.rs:4281–4338` kopiuje predecessor state/claims i zmienia je dopiero w `if commit`; `admission_authority_publication.rs:2300–2314` nadal oczekuje predecessor claims. Odczytany kod potwierdza rodzinę problemu opisaną przez właściciela w #358/#353. Faktem jest też hold i otwarta, niezweryfikowana naprawa. Nie przypisuję sobie natywnego odtworzenia całej ścieżki recovery. Decyzja potrzebna teraz: **TAK, naprawić według istniejącego kontraktu**, nie wymyślać nową fazę. Blokuje #329/#247; dalsza integracja utrwaliłaby błędny model. Odebranie naprawy wymaga niezależnie odtworzonego prepared successor, niezmienionego oryginalnego receipt, negatywnych przypadków starego predecessor oraz current-source revalidation. Zmienić tę rekomendację może jedynie przyjęta jawna zmiana semantyki lub poprawiona implementacja i dowody; fizyczne storage i przyszła topologia pozostają odrębne.

**F18/F19 — nowe reprodukcje.** `ai/perception.rs` sortuje według priorytetu, potem ID, ale sprawdza duplikaty tylko w sąsiedztwie. `ability/effects.rs`, `plan.rs` i `commit.rs` pozostawiają drogę omijającą dodatni magnitude. Run `34059391572` kompiluje oryginalne moduły bez zmiany produktu; testy kontrolne rozróżniają legalne wejścia i oczekiwane odrzucenia. Wniosek: przed integracją wymagane są globalna unikalność ID oraz walidacja efektu na nieomijalnej granicy. Nie jest to dowód błędu aktywnej ekonomii lub klienta online.

Zmiany wag F10/F16/F17/F14 są jawną korektą kalibracji, nie ukryciem ustaleń. Oddzielono dług, stan gotowości i ograniczenie pomiaru od błędu bezpieczeństwa bieżącej bramki.

## 7. Workstream decisions

| Workstream / Issue | PR i odczytany head | Dyspozycja audytowa / warunek |
|---|---|---|
| Foundation complete bridge #353 | #361 `16e9898b4905f3c5efe8db504a6a865a9f94f564` | **PAUSE**: owner stop i otwarte P1; wznowienie przez właściciela, potem naprawa faz/testy, nie scalenie WIP |
| Durability fresh admission #329 | #335 `834db1d7118d751e31287715d3eaac7780a0c7b9` | **PAUSE** polecenia właściciela; po wznowieniu CONTINUE_WITH_CONDITION: zgodność naprawionego bridge i #351/#319, bez activation |
| SQLx driver budget #351 | #356 `2aecb63f03e01c5e2c3eb8933dbb51d6f8b8c59c` | **PAUSE** polecenia właściciela; po wznowieniu osobne dowody TLS/allocator-lifetime i integracji, nie tylko RAII primitive |
| Owning sources #319 | Brak jednego PR dostarczającego całość | **BLOCKED** gotowości C: osobne owning implementations oraz uprawnienia Platform; dotychczasowy Game-only audit ich nie dostarcza |
| Server Seam #247 | Zachowany częściowy worker checkpoint; brak ukończonego PR implementacji | **BLOCKED** na poprawne A/B/C; nie przyjmować transport-local journal jako obejścia |
| Native Client / QA / Movement / Combat | Zależne od #247 i kolejnych alokacji | **CONTINUE_WITH_CONDITION** wyłącznie odrębne, autoryzowane prace lokalne; brak zgody na gameplay activation |
| CI impact routing #308 | Poprzednie wdrożenia #310/#312; historia pomiarów | **CONTINUE_WITH_CONDITION** istniejącego zakresu i freeze MQ; naprawy F01/F12 bez osłabiania aggregate |
| Dokumentacja i prompty | Current main b008; propozycje audytu w #360 | **CONTINUE_WITH_CONDITION** poza product leases; uporządkowanie punktów wejścia i starych bloków, bez nowej authority |
| Ten audyt #359 | PR #360, dokładny head w GitHub | **CONTINUE** do sprawdzonej publikacji raportu; nie przejmuje lifecycle innych workerów |

Runy PR335 `34046878916` i PR356 `34048206585` są green dla wskazanych headów, ale nie scalonego bieżącego produktu lub wznowienia po owner stop. PR361 ma nieudaną kwalifikację `34049443139`. Opisy PR335/356 cytują starsze checkpointy niż ich obecny kod; compare PR335 wykazuje już 14 zmienionych plików, w tym nowy adapter, guards i migrację. Nie powtarzam fałszywego opisu „tylko początkowy RED” jako stanu aktualnego headu.

## 8. Cross-workstream conflicts

**FND↔DUR:** właścicielem semantyki faz jest Foundation/przyjęty kontrakt, a wykonania trwałego CAS — Durability. F20 nie może zostać naprawiony przez transport lub odgadnięcie brakującego stanu w SQL. B jest 5 commitów za odczytanym main według compare; sama różnica ancestry nie oznacza błędu, ale relewantne zmiany bridge wymagają rekwalifikacji.

**Cargo/SQLx↔B:** fork drivera i limit jego zasobów należy do #351. B nie może uznać niezależnego testu drivera za pełny dowód swojej transakcji/TLS. Wspólne Cargo i publiczne kontrakty muszą zachować istniejące serializowane leases. Nie stwierdzam konfliktu każdego z 237 plików vendor PR356 — tego diffu nie przeglądałem w całości.

**Platform↔Game:** Canary integer-ID redemption i katalog nie są native source authority. Rzeczywiste źródła security/trust, owner store i assignment/readiness pozostają wymagane dla C. Nie można ich stworzyć samym złożeniem aktualnych DTO.

**Queue↔lifecycle:** właściciel zarejestrował, że konwersja #358 do draft nie usunęła oczekującego wpisu MQ, a wadliwy kandydat został scalony. To dowód konkretnego incydentu koordynacji, nie uniwersalny opis każdej wersji GitHub. Reakcja hold musi sprawdzać rzeczywisty stan kolejki i merge; dodatkowy niezależny kontroler approvals nie jest automatycznie rozwiązaniem.

## 9. Legacy contamination review

| Element | Klasyfikacja | Ocena |
|---|---|---|
| Importer Tibia/Canary i Reference fixtures | JUSTIFIED_REFERENCE / ACCEPTED_MIGRATION | Dopuszczalne jako jawne, przypięte źródła migracji; nie jako niejawna autoryzacja lub potwierdzenie Evolved balance |
| Integer/Canary binding w Platform GameAuth | ACCEPTED_MIGRATION; nie gotowy target native | Obecna funkcja może być prawidłowa dla legacy, lecz nie spełnia native AccountId/FND04 kontraktu |
| `blakinio/Oteryn-v2` w historycznych dokumentach | ACCEPTED_MIGRATION | To provenance, nie uprawnienie do nowych zapisów w starym repo |
| Stare bloki Sol/Codex/standing review | UNJUSTIFIED_INHERITANCE w aktywnym powieleniu procedury | Current root superseduje, ale fałszywe procedury i zbędny kontekst pozostają długiem |
| Nazwy i podobieństwo mechanik gry | UNKNOWN bez dowodu pochodzenia | Samo podobieństwo do Tibii nie wystarcza, by zarzucać kopiowanie architektury |

Prawa do wszystkich zewnętrznych assetów i pełna Reference parity nie zostały zatwierdzone. Cztery przypadki manifestu Reference pozostają pending. Są to granice przyszłego użycia i dystrybucji, nie powód do fałszywego przypisania im wartości zero lub statusu PASS.

## 10. Missing validation

### Wymagane teraz / przed najbliższą bramką

Przed kwalifikacją integracji: naprawa i testy F20; negatywne canary propagacji każdego native exit F01; działające testy governance F02; zgodność B z aktualnym bridge i realne SQL race/reload/replay; właściwy dowód zasobów drivera/TLS; owning-source initialization/revocation/restart i niezależne current facts; jedna sesja przez produkcyjne API i transport bez fixture authority. Przed odbiorem natywnego klienta: przeżycie minimalizacji, resume z prezentacją, błąd inicjalizacji z niezerowym wynikiem i kontrolowane zamknięcie.

Nie zaliczono naprawy produktu dlatego, że próba odtworzenia wady jest green. Nierozstrzygniętą lokalizację CodeQL/include i diagnostykę ekstraktora trzeba wyjaśnić przed szeroką deklaracją bezpieczeństwa. Brak pełnego branch denominator jest ograniczeniem twierdzeń o coverage, nie dowodem zerowego pokrycia gałęzi.

### Przyszła kwalifikacja / zastrzeżenia

Reprezentatywny load/soak/hardware matrix, wielowęzłowe failover/PITR/RPO/RTO, pełny instalator/updater/podpis/rollback, cały rzeczywisty corpus i browser/fullworld Atlas, legal clearance każdego assetu, audyt wnętrza każdej zależności oraz porównania modeli/tokenów nie zostały wykonane. Ich wyniki są **UNKNOWN/NOT_EXECUTED**, nie PASS. Nie są automatycznie wymagane dla obecnego negatywnego werdyktu audytowego, lecz odpowiednie z nich będą wymagane przed nazwanym etapem produkcyjnym. Nie ukrywam też ograniczonej głębokości analizy każdego historycznego dokumentu i patcha.

## 11. Top programme risks

1. **Zrealizowane:** P1 faz recovery w chronionym main może zostać potraktowane jako „zakończone” tylko dlatego, że PR scalono.
2. **Zrealizowane:** zielona walidacja może oznaczać zamaskowany native exit, pominiętą serię testów lub inny artefakt niż release.
3. **Zrealizowane:** prawdziwy klient nie zachowuje poprawnie podstawowego cyklu życia okna.
4. **Najbliższa bramka:** typed claims, fixture i legacy Platform mogą zostać błędnie uznane za aktualne źródła authority.
5. **Najbliższa bramka:** integracja lokalnych modeli AI/Ability ujawni pomijane niezmienniki oraz nieograniczoną retencję/koszt całych kolekcji.
6. **Najbliższa bramka:** rozbieżne heady, przestarzałe opisy i wspólne leases zwiększają ryzyko błędnej kolejności integracji.
7. **Ograniczenie przyszłości:** brak pomiarów scheduler/driver/GPU uniemożliwia wiarygodne zobowiązania dotyczące pojemności i opóźnień.
8. **Ograniczenie przyszłości:** dowód binarium i małego restore nie kwalifikuje dystrybucji, danych produkcyjnych lub disaster recovery.

## 12. Immediate corrective actions

Najpierw właściciel rozstrzyga wznowienie wstrzymanych workerów. Po wznowieniu: (1) domknąć #361 według istniejącej semantyki PREPARE, bez przejęcia tej naprawy przez audit; (2) naprawić wiarygodność F01/F02 i klienta F05 w rozłącznych, przydzielonych zakresach; (3) kwalifikować B oraz driver na kompatybilnych headach, następnie rzeczywistych producentów C; (4) przed aktywacją AI/Ability naprawić F18/F19 na nieomijalnej granicy; (5) wykonać minimalny natywny przepływ; (6) równolegle uporządkować dokumentacyjne lokatory i procedury bez kolejnego kontrolera governance. Nie wykonywać masowego refaktoru lub zmiany topologii przed naprawą podstawowego kontraktu.

Każda rekomendacja w `assessment.json` wskazuje właściciela, zakres, fazę i odbiór. Nie jest to automatyczna nowa alokacja, zgoda na produkcję lub polecenie zmiany protected settings. Audyt nie naprawiał znalezionych usterek.

## 13. Next evidence-producing milestone

**Pierwszy dowód:** po naprawie #361 rzeczywisty adapter B przeprowadza early-terminal replacement: PREPARE pozostawia dokładny successor RECONNECTABLE bez transportu i bez aktywacji ochrony; COMMIT aktywuje wyłącznie oczekiwaną generację; oryginalny FreshAdmissionCommit pozostaje niezmienny; utrata odpowiedzi, reload i conflicting replay nie tworzą drugiego właściciela. Fakty do przypadków negatywnych pochodzą z niezależnego aktualnego źródła, nie z odtworzenia oczekiwanego receipt. Jest to mniejszy i bardziej rozstrzygający cel niż „dokończyć cały serwer”.

**Następny dowód produktu:** zatwierdzone realne źródła C, produkcyjne binaria klienta i serwera, wejście, jedna komenda, obserwowalny trwały wynik oraz reconnect/restart bez podwójnego efektu. UI ozdobne, pełny balans, rynek, wiele stref i klaster mogą pozostać odroczone. Fixture/synthetic pozostają szybkimi pomocniczymi oraklami, nie źródłami authority.

## 14. Final audit gate

**PROGRAMME_AUDIT = FAIL**

Dowody wystarczają do odrzucenia obecnego przejścia przez bramkę; nie trzeba znać wszystkich przyszłych parametrów gry, aby wykazać sprzeczność F20 lub zaobserwowane zakończenie klienta. Zakończenie niniejszej oceny nie usuwa wad ani zastrzeżeń. Nie używam sformułowania „100% poprawności”, „każda linia zatwierdzona” ani „wszystkie możliwe testy wykonane”.

Odbiór publikacji obejmuje zgodność rejestrów, pełne przypisanie punktów promptu, zachowane dowody pozytywne i negatywne, brak produktu i tymczasowego workflow w końcowym diffie, sumy kontrolne oraz exact-head CI. Wynik tego CI dotyczy raportu; nie zmienia powyższego FAIL produktu. Końcowy commit, scope readback i wynik publikacji są zapisywane na PR #360 po powstaniu commitu. Nie wykonano merge, bypass, restartu workstreamów ani niezależnego drugiego audytu.

### Rejestr źródeł i odtwarzanie

Źródła produktu należy czytać pod wskazanym 7ce SHA; bieżące instrukcje pod b008. Nowe ustalenia i dowody: `assessment.json`, `closeout-evidence.json`, `review-scope.json`. Mapowanie pierwotnego zlecenia i wszystkich rozdziałów promptu: `prompt-traceability.json`. Historyczne dowody: `findings.json`, `coverage-register.json`, `execution-evidence.json`, `EVIDENCE.md`, `CONTINUATION.md`; nowsze jawne korekty mają pierwszeństwo. `verify_closeout.py` sprawdza spójność publikacji, `verify_evidence.py` odtwarza historyczny spis 803 plików. Pełne źródłowe artefakty, SARIF, profile, CSV i logi dołączono do archiwum właściciela; poufnego aneksu administracyjnego nie publikowano.

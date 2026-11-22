> Retained lane report at the pinned baseline. The root [audit status](README.md) records which proposals were applied and the latest validation. Historical scratch overlays and proposal patches are evidence, not additional production changes.

# Audyt wartości testów klienta — checkpoint

Repozytorium: `Oteryn/Oteryn-Game`. Źródło: `8e9f0e16631aab1beb8416f8d5ca707aa3031083`.
Odczyt lokalny: `<baseline-checkout>`.
Zakres: klient, jego runtime/UI/Bevy/domain/simulation/assets, input, renderer, synthetic assets, pre-native-client i synthetic-client-harness.

## Wynik i granice audytu

**FACT / PROVEN:** indeks obejmuje 1022 pliki zawierające testy oraz 3516 literalnych atrybutów testowych. Przeczytałem semantycznie całe 35 z tych plików (100 atrybutów), część jednego dalszego pliku, a także osiem całych plików kontekstu i części trzech innych. Pozostałe **986 plików testowych ma wyłącznie analizę statycznego indeksu**. Liczby atrybutów nie są liczbą skompilowanych ani uruchomionych test cases. Indeks nie wykrywa `#[ignore]` w tym zakresie; nie dowodzi to, że każdy test jest wybierany przez CI.

**FACT / PROVEN:** `git diff --name-only 8e9f0e1 -- <13 zakresów>` był pusty. Nie zmieniałem repozytorium. Nie uruchamiałem testów Rust, kompilacji klienta, Windows ani testów GPU. Nie raportuję oszczędności czasu ani procentu pokrycia zachowania.

Najważniejszy kandydat do małej poprawki: test publicznego wejścia Winit ma węższy `cfg` niż API, które sprawdza. Dedykowana komenda Windows nie wybiera tego testu. Najważniejsza zmiana interpretacji: wiele testów nazwanych `e2e_*` jest pożytecznym replayem komponentu z syntetycznym fallbackiem; ich zielony wynik sam nie stanowi dowodu przejścia przez produkcyjny serwer i okno klienta.

Manifest `./client_test_review_manifest.json` zawiera wszystkie 1022 ścieżki, SHA-256, status odczytu i oddzielny kontekst. Skrót faktycznie odczytanych plików: `./client_test_reviewed_files.tsv`. Generator checkpointu: `./client_review_checkpoint.py`.

## Rejestr zakresu

W kolumnie „całość” liczę wyłącznie pliki testowe przeczytane semantycznie od początku do końca. Dodatkowe manifesty, build scripts i kod wejścia nie zwiększają tej kolumny.

| Zakres | Pliki z testami w indeksie | Literalne atrybuty | Całość | Część |
|---|---:|---:|---:|---:|
| `apps/client` | 649 | 2519 | 7 | 0 |
| `crates/client-runtime` | 34 | 59 | 6 | 0 |
| `crates/client-ui` | 277 | 729 | 4 | 0 |
| `crates/client-ui-bevy` | 3 | 7 | 3 | 0 |
| `crates/client-domain` | 1 | 1 | 1 | 0 |
| `crates/client-simulation` | 4 | 12 | 2 | 0 |
| `crates/client-assets` | 7 | 31 | 2 | 0 |
| `crates/input-actions` | 1 | 11 | 1 | 0 |
| `crates/input-platform` | 5 | 23 | 3 | 0 |
| `crates/renderer` | 8 | 49 | 1 | 0 |
| `crates/synthetic-assets` | 1 | 1 | 1 | 0 |
| `tests/pre-native-client` | 1 | 1 | 1 | 0 |
| `tools/synthetic-client-harness` | 31 | 73 | 3 | 1 |
| **Razem** | **1022** | **3516** | **35** | **1** |

## Konkretne ustalenia

### C1. CHANGE: wyrównać wybór testu Winit z wyborem publicznego API

**FACT / PROVEN:** [native_winit.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/input-platform/tests/native_winit.rs):1 ma `#![cfg(feature = "native-winit")]`. [src/lib.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/input-platform/src/lib.rs):14 włącza adapter przy `any(windows, feature = "native-winit")`. [Cargo.toml](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/input-platform/Cargo.toml):16–17 nie deklaruje domyślnego włączenia feature. [merge-group-gate.yml](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/.github/workflows/merge-group-gate.yml):1389 uruchamia `cargo test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc` bez `--features`.

**INFERENCE / DERIVED, wysoka pewność:** ta dedykowana komenda Windows pomija jeden test integracyjny `public_native_ingress_preserves_button_focus_reset_and_ime_commit`. Wcześniejsza kompilacja innego pakietu nie włącza feature w kolejnej, osobnej komendzie Cargo. Linuxowa komenda `-p oteryn-client --features bevy-shell --lib` nie uruchamia testów pakietu zależności `input-platform`.

**RECOMMENDATION:** zmienić wyłącznie pierwszy wiersz testu na `#![cfg(any(windows, feature = "native-winit"))]`, a następnie potwierdzić jego obecność w istniejącym Windows jobie. To dopasowanie testu do istniejącej dostępności API; nie wymaga zmiany protection ani rozszerzania macierzy. Wariant lokalny Linux z `--features native-winit --test native_winit` może sprawdzić ten sam kontrakt bez rzeczywistego okna. Nie ma tu dowodu wykonania Windows w tej sesji.

Test jest wartościowy: sprawdza publiczne `WindowEvent`/`DeviceEvent`, reset po utracie fokusu oraz IME commit. Zachować także wewnętrzne [winit_reset_tests.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/input-platform/src/winit_reset_tests.rs), które obejmują inne scenariusze zachowania fokusu przy zmianie generacji.

### C2. KEEP + CHANGE/MOVE: rozdzielić component replay i dowód black-box

**FACT / PROVEN:** 75 plików Rust w zakresie zawiera odwołanie do `E2E_ALL_CAPTURE_DIR` — 20 w aplikacji, 8 w runtime i 47 w UI. Pełna lista jest w JSON; to wynik wyszukiwania, a nie 75 semantycznych odczytów. W pięciu w pełni odczytanych plikach replay brak env wybiera syntetyczny fixture: quest log, target move, NPC shop, raid schedule i analyser. Przykłady: [quest e2e_all.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-runtime/src/features/0016-quest-log/tests/e2e_all.rs):8, [raid e2e_w04.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/src/features/0149-raid-schedule/tests/e2e_w04.rs):8–25, [NPC e2e_w05c.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-ui/src/features/0081-npc-shop/tests/e2e_w05c.rs):9–49.

Są w nich rzeczywiste dekodery, reducery i stan klienta, więc obecność fixture nie oznacza braku wartości. Test NPC i raid naprawdę podają błędne dane do mutowalnego panelu i sprawdzają zachowanie poprzedniego stanu. [Analyser e2e_n07c.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/src/features/0010-analyser/tests/e2e_n07c.rs):94–139 dodatkowo odrzuca syntetyczne dane podszywające się pod capture oraz waliduje metadane. To dobry wzorzec rozdziału trybów.

**RECOMMENDATION:** zachować szybki replay jako component test. W osobnej kwalifikacji wymagającej capture traktować brak artefaktów, niezgodny SHA, brak scenariusza i pusty replay jako błąd. Raportować jawnie `synthetic replay` versus `captured replay`; dopiero żywy klient wobec żywego właściciela serwera stanowi osobną warstwę black-box. Nie zamieniać wszystkich unit/component tests na E2E. Osobny audyt black-box właściciela `/root/audit_blackbox` ma reproduktor brakującego plan→verdict w node-boot; nie dubluję jego implementacji ani nie utożsamiam klientowego fallbacku z pustym server-side early return.

### C3. CHANGE: domyślny harness powinien weryfikować to, co ogłasza

**FACT / PROVEN:** [synthetic-client-harness/src/main.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/tools/synthetic-client-harness/src/main.rs):37–42 wybiera live tylko dla `--live`. Domyślny `synthetic()` sprawdza decyzję renderera, ale na liniach 113–119 jedynie wypisuje revision, liczbę entities i actions. Nie porównuje ich z oczekiwaniem. [merge-group-gate.yml](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/.github/workflows/merge-group-gate.yml):734 i 1339 wywołuje ten domyślny tryb.

**INFERENCE / DERIVED:** cichy brak akcji wejściowej lub brak efektu poprawnie zwracającego `Ok(())` w symulacji może nadal zakończyć ten smoke komunikatem `synthetic-ok`. To analiza przepływu, nie wykonana mutacja.

**RECOMMENDATION:** jeśli zachować smoke, przed sukcesem sprawdzić finalny revision, oba entity i oczekiwane semantyczne `move.east`/`Started`; nie ograniczać się do count. Nie nazywać go kwalifikacją native login. [native_login.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/tools/synthetic-client-harness/tests/native_login.rs):31,36 ręcznie podaje `form.finish(Err(...))` i `form.finish(Ok(()))`: wartościowy test formularza i zapisu ustawień, ale bez połączenia. Istniejący `--live` jest punktem wyjścia do prawdziwej kwalifikacji; odczytano tylko początek jego CLI, więc nie potwierdzam kompletności istniejących live scenarios.

### C4. CHANGE: usunąć tautologiczne sprawdzenie lokalnego Stringa

**FACT / PROVEN:** [e2e_m01.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-runtime/src/features/0064-target-move/tests/e2e_m01.rs):47–57 kopiuje `visible` do `retained`, wywołuje `project` wyłącznie na nowym `GatedCommandResult`, a następnie porównuje te dwa niezmienione Stringi. `visible` nie jest argumentem testowanej funkcji ani mutowanym stanem.

**RECOMMENDATION:** usunąć końcową kopię/equality, pozostawiając realne `is_err()` dla niepoprawnego disposition. Jeśli zamierzeniem jest odporność UI na błąd, przenieść assertion do właściciela stanu przyjmującego odpowiedź i porównać jego widok po rzeczywistym wywołaniu. Nie dopisywać testu samego `String::clone`.

### C5. CHANGE / możliwe RETIRE po sprawdzeniu właściciela: syntetyczne helpery

**FACT / PROVEN:** jedyny test w [synthetic-assets/src/lib.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/synthetic-assets/src/lib.rs):155–162 ma nazwę `fixture_is_bounded_and_deterministic`, ale sprawdza tylko szerokość poprawnego fixture i długość tekstu SHA-256 równą 64. Nie sprawdza granic parsera ani stabilności/zmienności digestu. Wyszukanie symbolu `SyntheticAssetRuntime` w materializowanych `apps`, `crates`, `tools`, `tests` znalazło wyłącznie deklarację i impl w tym pliku. Cała biblioteka nie jest martwa: `SyntheticImage` jest używany przez harness i `tools/synthetic-asset-compiler`.

**RECOMMENDATION:** nie mnożyć testów pozbawionego konsumenta runtime cache. Najpierw potwierdzić, czy `SyntheticAssetRuntime` ma utrzymywany cel; jeśli nie — usunąć nieużywany helper w odrębnej małej zmianie. Dla używanego parsera zachować prosty fixture i sprawdzić własne kontrakty: niedozwolony schema/rozmiar/niezgodność długości. Jeśli stabilny digest jest kontraktem narzędzia, użyć jednego niezależnego znanego wektora; nie implementować w teście kopii SHA-256. Brak odniesień statycznych nie rozstrzyga o nieznanych zewnętrznych konsumentach, choć pakiet jest repozytoryjnym narzędziem.

Podobnie [client-simulation/src/lib.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-simulation/src/lib.rs):68–81 `projection_is_deterministic` sprawdza pojedynczy upsert i count. Uczciwa nazwa to test zastosowania upsert; ewentualne wymaganie deterministycznej sekwencji powinno mieć własny oracle. Nie usuwać znaczących osobnych testów scene order tylko dlatego, że używają liczników.

### C6. KEEP częściowo, CHANGE częściowo: teksty i wygenerowane rejestry

**FACT / PROVEN:** [localization_audit.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-ui/tests/localization_audit.rs):145–215 przypina całe opisy i nagłówki jako dokładne Stringi. Następny test :219–246 przechodzi przez importowane wartości i sprawdza `Alice | 100 | 40 | 60`, a inne testy chronią słowa zaklęć, granice liczb, literalne `YES` i instrukcję wylogowania.

**RECOMMENDATION:** zachować te kontrakty. Rozważyć ograniczenie długich, powtarzanych oczekiwań prozy do celowo zatwierdzonych, krytycznych komunikatów i tabeli przypadków. Dokładny tekst może być prawidłowym testem regresji, ale sam nie dowodzi jakości tłumaczenia. Nie usuwać wszystkich equality ani zastępować ich sprawdzeniem „niepuste”. Do decyzji o usunięciu całych golden paragraphs potrzebny jest cel produktu/historyczny bug, którego tu nie ustalono.

**FACT / PROVEN:** [registrations.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-ui/tests/registrations.rs):33–46 przelicza te same funkcje `scan`/`render`, których używa [build.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/crates/client-ui/build.rs):35–47. Taki parity check łapie nieaktualny output/wiring, ale wspólny błąd generatora może zmienić obie strony identycznie. Zachować konkretne niezależne checks rejestracji mechaniki/command ID. Jeśli koszt skanowania jest problemem, przenieść szeroki generator test do właściciela generatora po zebraniu czasu; obecnie koszt **UNKNOWN**.

### C7. KEEP / ewentualna konsolidacja: pre-native nie znaczy automatycznie legacy

**FACT / PROVEN:** [tests/pre-native-client/src/lib.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/tests/pre-native-client/src/lib.rs):10–24 sprawdza publiczne API bootstrapu bez konfiguracji logowania: odmowę wejścia, availability i informację użytkownika. [apps/client/src/tests.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/src/tests.rs):4–12 pokrywa bardzo podobny brak konfiguracji; jest niewielkie nakładanie. Odczyt fragmentu [apps/client/src/lib.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/src/lib.rs):80–210 potwierdził, że to nadal istniejąca gałąź produktu.

**RECOMMENDATION:** zachować fail-closed. Można przenieść komplet asercji do jednego testu publicznego API i zmienić historyczną nazwę pakietu, jeżeli odrębny pakiet nie ma już wartości architektonicznej. Nie uzasadniać dużego PR redukcją dwóch tanich przypadków; czas jest **UNKNOWN**. Sama nazwa pre-native nie dowodzi martwego testu.

### C8. MEASURE przed CHANGE: dwa warianty kompilacji aplikacji

**FACT / PROVEN:** [merge-group-gate.yml](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/.github/workflows/merge-group-gate.yml):664–674 uruchamia workspace `--lib`, a oddzielnie całe `oteryn-client --features bevy-shell --lib`. [apps/client/build.rs](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/build.rs):51–57 zmienia konfigurację kompilacji; [Cargo.toml](https://github.com/Oteryn/Oteryn-Game/blob/8e9f0e16631aab1beb8416f8d5ca707aa3031083/apps/client/Cargo.toml):28 włącza dodatkowe zależności/native input. Poddrzewo `bevy_shell` ma w indeksie 201 plików i 1155 atrybutów; to nadal nie runtime IDs.

**RECOMMENDATION:** zebrać `nextest list` dla obu dokładnych komend i zestawić case IDs z konfiguracją. Potem zdecydować, czy headless i Bevy są oboma wspieranymi wariantami, czy pełny wariant Bevy może być jedynym właścicielem wspólnych testów. Nie traktować powtarzającej się nazwy case pod różnym `cfg` jako udowodnionej zbędnej duplikacji. Nie zmieniono tych jobów.

## Testy, które należy chronić

| Warstwa / przykłady | Dlaczego chronią zachowanie | Czego nie dowodzą |
|---|---|---|
| `input-actions/src/lib.rs`, `input-platform/src/audit_regressions.rs` | Modal/text blokuje gameplay, utrata focus/capture/device czyści held keys, odpuszczenie ostatniego modifier side anuluje akcję; redakcja tekstu w Debug. | Rzeczywisty OS event loop, sterowniki i hardware. |
| `renderer/src/tests.rs`:56,120,199 | Odrzucone generacje, limit recovery i overflow nie zmieniają stanu; close i suspend mają konkretne skutki. | GPU render, shader i pixel correctness. |
| Wszystkie trzy test files `client-ui-bevy/tests` | Prawdziwe systemy Bevy i `ComputedNode`; rozmiary, border/padding, dziewięć pól nine-slice, pickability, reuse po resize, button states. | Font rasterization i wygląd końcowego okna; część metryk glyph jest fixture. |
| `client-assets/tests/real_assets.rs`:65–390 oraz `boss_area_metadata.rs` | Faktyczne pinned dane, hash corruption, patterns/floors/counts, LRU bounds, niewłaściwe IDs. Konkretne asset counts są identyfikatorem wersji fixture, nie automatycznie śmieciowym testem. | Wszystkie warianty sprzętu i cały produkt. Nie wykonano tych testów ani nie sprawdzono kompletności materializacji wszystkich pinned assets. |
| `client-ui/tests/house_furniture_confirmation.rs`:5 | Potwierdzenie jest związane z dokładnym item/destination/revision, a zmiana celu wymaga nowej zgody. | Pełna transakcja serwera; tego nie należy zastępować samym UI screenshot. |
| `client-runtime/tests/queued_retries.rs`, `admission_progress.rs` | Prawdziwa pętla retry, nieprzedłużanie grant expiry, cancellation i publikacja kolejki mimo kontrolowanego peer. | Autentyczność produkcyjnej konfiguracji Platform/Gateway. |
| `apps/client/src/bevy_shell/owner/admission_tests.rs`:12 | Scripted lokalne HTTP/TLS przechodzi przez faktyczny owner i Bevy UI; kolejka widoczna także przy ukrytym loginie; Cancel kończy próbę bez następnego admission. | Produkcyjny game-server; `native_entry/test_support.rs` jawnie buduje scripted stack. |
| `apps/client/src/bevy_shell/login/tests/text_input.rs`:89,112,165 | Ostatni znak trafia do submit, pending paste blokuje submit i nie wraca po zamknięciu, ustawienia zachowują edit intent. | Systemowy schowek/klawiatura poza kontrolowanymi messages. |
| `synthetic-client-harness/src/native_floor_transition.rs`:152 | Real Session + mailbox + play task + widok po scripted wire; oczekiwany floor i sprite count są warunkiem zakończenia, timeout jest błędem. | Pełny serwer i native rendering. |

Ostatni przykład, podobnie jak `local-reference-preferences/character_scope.rs`:116–139 i startup validation w `bevy_shell/mod.rs`, nie musi używać makra `assert!`, żeby sprawdzać wynik. Mechaniczny filtr „brak assert = usunąć test” dałby fałszywe wskazania.

## Zalecany podział black-box i pozostałej piramidy

1. **Unit:** granice wartości, dekodowanie/refusal, kolejność input, stan renderera i expiry. Krótkie, niezależne expected outcomes. Nie zastępować tych przypadków pełnym klientem.
2. **Component:** prawdziwy reducer/owner/Bevy layout z kontrolowanym peer i fixtures. Jawnie nazywać granicę double; utrzymać rollback, queue, focus, paste i user-confirmation.
3. **Capture replay:** komplet scenariuszy rzeczywistego serwera zapisany jako artefakt z SHA, tożsamością scenariusza i wersją. Brak scenariusza lub artefaktu ma nie dawać zielonego „live”. Syntetyczne fallbacki nadal mają własne szybkie miejsce.
4. **Client black-box:** niewielki zestaw uruchamiający zbudowany klient i rzeczywisty server stack; wejście, ruch/floor transition, odmowa i retry, utrata/reconnect, jedna operacja potwierdzana. Sprawdzać user-visible wynik i trwały wynik właściciela, a także brak niezamierzonej zmiany po odmowie. Istniejący live harness i replay powinny dostarczyć część adapterów; zakres native window/OS wymaga oddzielnej decyzji.

Nie ustalono budżetu czasu/flake dla punktu 4; bez pomiaru nie proponuję pełnej macierzy 75 replay features × platformy ani zastępowania nimi wszystkich unit tests.

## Dokładnie brakujące dowody

- 986 statycznie zaindeksowanych plików nie zostało semantycznie przeczytanych, a w jednym testowym odczytano tylko początek. Ich utrzymanie/usunięcie pozostaje otwarte.
- Nie ma case inventory po Cargo `cfg`, targetach i feature unification; bez tego nie można podać liczby realnie wybieranych ani zduplikowanych testów klienta.
- Nie ma wykonania Windows native-winit ani testów Rust z tej lane. C1 jest wnioskiem z konfiguracji, nie raportem z czerwonego/zielonego runu.
- Nie zebrano czasu per-test, historii flake, code coverage ani mutation score klienta. Nie ma podstaw do deklaracji oszczędności godzin/minut.
- Pinned assets i rzeczywisty rendering nie były uruchamiane; plan black-box jest rekomendacją, nie wdrożonym testem.
- Odczytano tylko początek `live/cli.rs` i qualification player trade; pełna kompletność ich scenariuszy pozostaje niezweryfikowana.

**Rekomendacja końcowa:** małe, konkretne poprawki wyboru testów i oracle; zachowanie testów bezpieczeństwa stanu i interakcji; jawne oznaczenie fixture/component/capture/live. Brak podstaw do szerokiego usunięcia testów klienta jako legacy na podstawie tego checkpointu.

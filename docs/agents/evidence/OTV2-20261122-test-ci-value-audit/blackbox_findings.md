> Retained lane report at the pinned baseline. The root [audit status](README.md) records which proposals were applied and the latest validation. Historical scratch overlays and proposal patches are evidence, not additional production changes.

# Audyt node_boot, E2E i przejścia na testy black-box

**Baza audytu:** `Oteryn/Oteryn-Game@8e9f0e16631aab1beb8416f8d5ca707aa3031083` (main, squash #3391). Repozytorium lokalne: `<baseline-checkout>`. Ścieżki kodu poniżej są względem tego repozytorium, a wskazania linii odnoszą się do bazowego commitu, jeśli nie zaznaczono inaczej. Data odczytu: 2026-11-22.

**Wniosek:** warto rozszerzyć testy black-box, ale istniejący projekt ma już większość potrzebnej infrastruktury i kilka mocnych testów rzeczywistej ścieżki produktu. Pierwsze zadanie to przywrócenie wiarygodności wyniku: uszczelnić baseline, egzekwować wykonanie zaplanowanych scenariuszy i odróżnić test żywej sesji od zapisu capture oraz replay. Masowe usuwanie unitów albo zastąpienie testów PostgreSQL pełnym E2E osłabiłoby sprawdzanie generation fencing i trwałości wartości.

## Zakres i poziom dowodów

- **PROVEN / FACT:** inspekcja kodu kwalifikacji, planera, agregatora, callerów CI, scenariuszy klienta, rzeczywistego harnessu Game–Platform i przykładowych testów komponentowych. Uruchomione reproduktory używają prawdziwego `stage()` lub prawdziwego modułu `matrix.py`, z syntetycznymi logami. Nie zastępują ich uproszczonym modelem.
- **PROVEN / FACT:** skan źródeł daje 678 plików w `client_phases`, 157 plików `full_row_*.rs` i 142 `client_row_*.rs` po wyłączeniu `_mount.rs`. W `world.rs` jest 298 wywołań `verdict::indexed`, wszystkie z odrębnym ID: 157 `.full` i 141 `.client`. Planner znajduje **302 aktywne indeksy w zakresie 1–305**, a nie 305 aktywnych testów. `C07.client` jest wywoływany osobno pod indeksem 227, dlatego liczby plików i `indexed` nie są równoważne (`world.rs:1693–1695`). Pełne wyniki i plany: `./blackbox_inventory.json`.
- **DERIVED / INFERENCE:** brak kontroli oczekiwanych ID może ukrywać niewykonane scenariusze w mieszanym shardzie, jeśli pozostały scenariusz daje wymagane niepuste evidence. Reproduktor dowodzi akceptacji takiego wyniku przez agregator; audyt nie ustalił, ile historycznych przebiegów CI rzeczywiście zawierało taki ubytek.
- **UNKNOWN:** brak pomiaru czasu, kosztu i flakiness całej kwalifikacji na badanym SHA; brak pełnego uruchomienia Rust, rzeczywistego PostgreSQL, Platformy i node_boot w tej lane. Nie twierdzę, że którykolwiek produktowy scenariusz jest dziś zielony lub że wykryto utratę banku/inwentarza. Nie audytowałem zmian równoległej gałęzi `claude/happy-davinci-ijz3wy-e2efix`.
- Raport jest audytem lane E2E/harness; nie jest deklaracją semantycznego przeglądu każdej asercji we wszystkich testach całego monorepo. Liczby testów nie są liczbami udowodnionych mechanik.

## Co naprawdę uruchamia CI

Dowody: `tools/qualification/node_boot/matrix.py:43–45, 169–185`, `apps/game-server/src/qualification/client_phases/e2e_row_batch.rs:1–23`, wskazane workflowy.

| Tryb / caller | Faktyczny zakres | Wpływ na interpretację zielonego wyniku |
|---|---|---|
| `plan(reference)` | 1 shard; domyślny batch `(0,15)`, czyli aktywne indeksy 1–14, wraz z normalnymi grupami | Nie pokrywa całej listy indeksowanych wierszy. |
| `plan(pr)` | 7 shardów: basic, 3 partycje grup i batch 0:5, 5:5, 10:5 | Pokrywa te same pierwsze indeksy 1–14, nie pełne 302. |
| `plan(full)` | 37 shardów: basic, 3 grupowe, 31 row batch, spells, game-platform | Uruchamia cały aktywny zakres indeksów; wykonanie wywołania nadal nie gwarantuje emisji właściwego verdictu. |
| `.github/workflows/pr-postgres-harness.yml` | `plan --set pr` (376); node-boot wymaga niedraftowego PR i labela `node-boot` (304–306, 611–613) | Ten lane nie jest automatycznie uruchamiany dla każdego PR. Summary używa `--missing-informational` (661). |
| `.github/workflows/merge-group-gate.yml` | Workflow obsługuje `merge_group` oraz push main (4–7); kwalifikacja planuje `pr` (1491) | Także post-merge nie oznacza pełnego row set. Summary używa `--missing-informational` (1787). |
| `.github/workflows/node-boot-qualification.yml` | Harmonogram i domyślny scheduled set `full` (39–40, 81, 90) | Pełny nocny zestaw; summary nie używa `--missing-informational`. |

**PROVEN:** `known_fail_rows.txt` jest czytany przez wspólny `run.sh::stage()`. Wyjątek baseline nie jest ograniczony do nocnej kwalifikacji — wpływa również na callerów PR i post-merge. Workflowy dodatkowo kontrolują sukces samych shard jobs; nie należy sugerować, że samo `summarize()` jest jedyną bramką.

## Ustalenia

### BB-01 — baseline odpuszczał dowolną nową awarię znanego scenariusza

**PROVEN, konkretny defekt bramki:** w bazowym `tools/qualification/node_boot/run.sh:993–1040` funkcja `known_rows()` zwraca tylko pierwszą kolumnę. `stage()` porównuje `status=FAIL` wyłącznie po scenario ID. Druga i dalsze kolumny istniejącego baseline nie ograniczają wyjątku.

Plik ma **46 unikalnych wpisów**. Każdy ma postać `<scenario> <cause> at <root>[; uwagi]`; uwagi po pierwszym średniku są opisem/evidence, a nie częścią root. Rodzaje przyczyn obejmują `assertion`, `transport`, `no-progress`, `blocked-by`, `deadline`. Rzeczywisty producent suffixu to `client_phases/verdict.rs:69–70`: ` cause=<kind> root=<root>`.

Konkretny wpis i syntetyczny nowy wynik:

```text
baseline: E01.full assertion at E01/greet; source:apps/game-server/src/qualification/client_phases/full_e2ef4.rs:1
log:      SEAM_EVIDENCE e2e_all scenario=E01.full status=FAIL seconds=1 evidence=synthetic-new-regression cause=assertion root=E01/relog-bank-balance
```

Reproduktor `./blackbox_repros.py --mode baseline` wyciąga rzeczywiste `stage_rows`, `known_rows`, `stage()` z badanego pliku, stubuje wyłącznie diagnostykę i podaje ten log. **Oryginał: `AUDIT_STAGE_STATUS=PASS` i `NODE_BOOT_KNOWN_FAIL ... E01.full`. Propozycja: `AUDIT_STAGE_STATUS=FAIL(rows)`.** Kod procesu reproduktora jest 0, ponieważ kontrolnie odczytuje końcowy stan stage; nie uruchamia końcowej sekcji całego runnera.

**RECOMMENDATION / przygotowana poprawka:** wyjątek dotyczy dokładnego `(scenario, cause, root)`. Brak suffixu, zmieniony root/cause, niejednoznaczny suffix lub połączenie starej i nowej awarii tego samego ID nie jest znanym błędem. Zachować wszystkie 46 wpisów i komentarze, nie dopisywać wyjątków. Historyczne `PASS` dla wpisu pozostaje sygnałem `NODE_BOOT_KNOWN_FAIL_STALE` do usunięcia długu, nie automatyczną zmianą pliku.

Patch w scratch: `./blackbox_baseline_signature.patch`; obejmuje tylko helper i testy w `run.sh` / `test_diagnostics.py`. Osiem testów helpera obejmuje także round-trip wszystkich 46 rzeczywistych sygnatur. Root poinformował o integracji poprawki oraz dodatkowym teście rzeczywistego caller `stage()`; weryfikacja finalnego drafta należy do roota.

### BB-02 — właściwy artifact sharda nie gwarantuje właściwych scenariuszy

**PROVEN:** `matrix.py::extract` (266) zapisuje wyemitowane ID; `shard_passed` (284) wymaga jedynie exit=0 i terminalnego `NODE_BOOT_PASS`. `summarize` (297–358) kontroluje tożsamość artifactu, niepustą listę World scenarios i politykę `MISSING`; nie porównuje jej z planowanymi ID. Duplikaty tylko wypisuje w sekcji overlaps.

Rzeczywisty moduł uruchomiono z `plan("custom", rows="206:207")`. Plan wybiera `OTERYN_E2E_ROW_BATCH=206:1`, którego dispatcher w `world.rs` wskazuje `S09.client`. Jedyny syntetyczny rezultat zawierał `I01.full PASS` i `NODE_BOOT_RESULT=NODE_BOOT_PASS`. Wynik: **`shard_passed=True`, `summary_problems=[]`**, mimo całkowitego braku `S09.client`. Dowód: `./blackbox_repros_baseline.json`, reproduktor tryb `aggregate`.

**RECOMMENDATION:** zapisać oczekiwane scenario ID w planie i sprawdzić je po wykonaniu; wymagany wynik każdego zaplanowanego scenariusza dokładnie raz, jawny zakres dopuszczonych dodatkowych wyników, błąd dla brakującego/nieznanego/zdublowanego ID. Capture i replay potrzebują odrębnej reprezentacji, żeby `CAPTURED` nie udawało zakończonego testu. To pozostawione otwarte: dopisanie tylko licznika nie rozwiąże selekcji grup, wierszy, capture i dodatkowych bounded slices.

### BB-03 — część client rows nie wykonuje nic w obecnych workflowach

**PROVEN:** `rg` w całym `.github/workflows` nie znalazł `E2E_ALL_CAPTURE_DIR` ani wywołania `tools/qualification/e2e_all/run.sh`. Sam `node_boot/run.sh` nie ustawia tej zmiennej i nie uruchamia replay. W 31 plikach początek `run_packet` od razu zwraca bez capture directory. Dokładna lista w inventory; sufiksy plików `apps/game-server/src/qualification/client_phases/client_row_<id>.rs`:

```text
c01 c12 c14 c16a c16c c16d c17b c20a c20b e01 i08 m06 m07
s04c s05 s06 s07 s08 s09 s10 s13a s13b s13c s15b s16a s16b s18
w01a w01b w01c w05c
```

Przykład: `client_row_s09.rs:50–53` kończy wywołanie przed logout/relogin, jeśli capture dir nie istnieje. `indexed_verdict.rs:79–108` zapewnia verdict dla błędu zaobserwowanego w future, lecz przy poprawnym `Ok(())` bez evidence nie wymusza żadnego wyniku. W połączeniu z BB-02 taki brak może zniknąć w shardzie z innymi wynikami.

**Istotne rozróżnienia:**

- Nie wszystkie `.client` są no-op. `client_row_c03.rs:91–132` wykonuje rzeczywiste query Offence/Defence i stosuje wynik w shipping panelu; `client_row_c11.rs:5–51` wykonuje realny odczyt i dekodowanie Battle Instinct. Mogą emitować `PASS` bez capture. To dowód ograniczonej integracji, nie replay ani całego UI.
- `client_row_c07.rs:29–143` ma silny rzeczywisty scenariusz: zakup runy, odmowa w PZ bez ubytku, skuteczny debit i pole poza PZ, damage, expiry i zachowanie custody po relogu. To wartościowy kandydat do zachowania w nocnym lane. Limit 240 s i oczekiwanie na expiry do 75 s są konfiguracją kodu, nie zmierzonym czasem przebiegu.
- Normalizer `apps/game-server/src/qualification.rs:172–215` **nie konwertuje `CAPTURED` na `PASS`**: przenosi je do `e2e_capture`. Problemem jest brak spójnego sprawdzenia oczekiwanego zakończenia, nie błędne twierdzenie o takim mapowaniu.
- `tools/qualification/e2e_all/run.sh:10–29` rzeczywiście tworzy capture dir, uruchamia sześć client library targets i strict report. Nie jest jednak wywołane przez workflowy. Domyślnie dziedziczy row batch `(0,15)`; pełny row sweep wymaga jawnego innego `OTERYN_E2E_ROW_BATCH`. README uczciwie nazywa komendę incomplete i celowo czerwoną.
- Bez capture env test `crates/session/src/tests/e2e_c01.rs::e2e_c01_replays_server_combat_state_and_rejects_invalid_delta` nadal bada shipping Session przez `tokio::io::duplex`, stosuje delta i sprawdza odmowę malformed delta bez mutacji. `payloads()` generuje wtedy dane codecami, a przy ustawionej zmiennej czyta pliki z tego przebiegu. To wartościowy test komponentu; jego zwykły cargo PASS nie jest dowodem działającego serwera.

**RECOMMENDATION:** rozdzielić trzy dowody: live query/command, poprawnie zapisany capture oraz replay dokładnie tego capture. Dla planowanego scenariusza brak wymaganego drivera/artifactu musi być jawnym ubytkiem coverage. Samo dodanie zmiennej środowiskowej nie ustanowi proof replay.

### BB-04 — co najmniej 12 full rows nie ma ścieżki do PASS

**PROVEN przez inspekcję emitera i gałęzi funkcji, nie przez nazwę pliku.** To dolna granica potwierdzonych przypadków, nie twierdzenie o kompletnej liczbie wszystkich nieosiągalnych sukcesów w helperach i złożonych journeys. Wszystkie poniższe pliki są w `apps/game-server/src/qualification/client_phases/`.

| Scenariusz i plik | Co rzeczywiście sprawdza | Dlaczego nie jest gotowym full acceptance test |
|---|---|---|
| `G05.full`, `full_row_g05.rs:16–33` | Lokalny `reference_preferences::open_key(&[])` | Obie gałęzie `verdict(bool)` zwracają FAIL; brak drivera settings/input. |
| `G31.full`, `full_row_g31.rs:11–28` | Ten sam brak skrótu bez inputu | Obie gałęzie FAIL; brak pełnego drivera options/arcs/minimap/actionbar. |
| `G35.full`, `full_row_g35.rs:20–33` | Próba boss lever | Literalny FAIL także po powodzeniu helpera; brakuje pełnej wyprawy/kill/arena. |
| `G36.full`, `full_row_g36.rs:22–35` | Ograniczona odmowa wejścia do Soulpit | Literalny FAIL; nie dowodzi udanego core/custody flow. |
| `M10.full`, `full_row_m10.rs:42–58` | Pięć kroków drowning może przejść | Full emitter zawsze FAIL, bo swimming/traps i pozostałe wymagania są nieudowodnione. |
| `W05b.full`, `full_row_w05b.rs:24–38` | Odczyt durable daily decision i relog | Literalny FAIL; efekty dla gracza nie są prowadzone przez pełny driver. |
| `W06.full`, `full_row_w06.rs:18–32` | Do 3 kill, naturalny los portalu 0,05%, budżet do 420 s | FAIL także po znalezieniu portalu; raid/charm nieukończone. |
| `W07.full`, `full_row_w07.rs:20–32` | Odmowa item action w ograniczonej konfiguracji | Literalny FAIL; brak pełnego Soulpit flow. |
| `W09.full`, `full_row_w09.rs:23–35` | Ograniczona odmowa Drome/concoction | Literalny FAIL także dla gałęzi z wejściową rzeczą; brakuje drivera. |
| `W19b.full`, `full_row_w19b.rs:22–34` | Odmowa seeds/pot | Literalny FAIL; brak pełnego cyklu. |
| `W19d.full`, `full_row_w19d.rs:22–34` | Odmowa outfit/mount item | Literalny FAIL; brak pełnej mechaniki. |
| `W19e.full`, `full_row_w19e.rs:22–34` | Odmowa lottery/rust item | Literalny FAIL; brak pełnego cyklu. |

**RECOMMENDATION:** zachować wartościowe negative/bounded slices jako osobno nazwane testy z rzeczywistą asercją. Brak pełnego drivera powinien pozostawać jawny w macierzy kompletności. Nie zmieniać tych wierszy na PASS i nie usuwać oczekiwań akceptacyjnych. Dla G05/G31 pełna topologia serwera nie wnosi dowodu do samego lokalnego pustego inputu. Dla W06 wielominutowa naturalna loteria nie jest dobrym warunkiem deterministycznego PR gate; ważne reguły prawdopodobieństwa testować deterministycznie, a pełny naturalny przebieg kwalifikować osobno.

Testy nazwane `source_token_has_no_spaces` / `verdict_names_the_needed_driver_without_spaces` chronią format metadanych, nie zachowanie gracza. Można scalić ich wspólny kontrakt przy zmianie raportowania, ale same takie testy nie uzasadniają istotnych oszczędności bez pomiaru.

Przykłady niezgodnego właściciela testu: `full_row_g06.rs` po udanej admission dalej opisuje brak native challenge; `docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md:4` przypisuje implementację challenge Platformie. `full_row_g28.rs` (Bazaar/transfer) oraz `full_row_g38.rs` (Windows touch) pokazują wymagania dodatkowego drivera. Nie należy na podstawie braku drivera Game orzekać braku produktu lub przenosić odpowiedzialności Platformy do Game.

### BB-05 — test diagnostyki przechodził mimo przerwania obu podprocesów

**PROVEN przez uruchomienie oryginalnego testu z audytowanego commitu:** `tools/qualification/node_boot/test_node_boot_tools.py:288–310`, `StageDumpTests.run_stage`, zwraca jedynie stdout. Nie ustawia `GAME_SOURCE`, nie wczytuje row helpers i nie kontroluje returncode ani końcowego `STATUS`. `test_each_stage_end_dumps_postgres_and_a_failed_stage_is_verbose` bada wcześniejsze linie PG dump.

Reproduktor `./blackbox_oracle_repro.py` uruchamia dokładnie oryginalny unittest i przechwytuje ignorowane przez niego wyniki. Rezultat: **1 test PASS**, podczas gdy gałąź `true` kończy się rc=1 z `GAME_SOURCE: unbound variable`, a gałąź `false` rc=127 z `report_character_authority_failures: command not found`. Dowód `./blackbox_oracle_repro.json`.

**RECOMMENDATION:** prawdziwe row helpers i jawne środowisko, stub tylko nieistotnej diagnostyki, asercja rc i końcowego `STATUS`, test nowej awarii w znanym scenariuszu przez rzeczywisty caller `stage()`. Root prowadzi tę naprawę w swoim draftcie; child nie modyfikował repozytorium.

### BB-06 — kilka deklaracji dokumentacji jest starszych niż obecny kod

`tools/qualification/e2e_all/README.md` opisuje Journey bez grants i brak zaimplementowanych full rows. Obecne `world.rs:3325` wywołuje provisioning w core profile, a dispatcher zawiera 157 `.full` wywołań `indexed`. Sama ich obecność nie dowodzi ukończenia pełnych mechanik, ale opis nie jest wystarczającym źródłem aktualnej macierzy wykonania. Dodatkowo mechanizm isolation opisany jako praca na gałęzi nie jest dowodem, że każdy dzisiejszy row ma świeżą postać.

`e2e_row_batch.rs:29–43::release_row_characters` czeka na zwolnienie sesji, **nie resetuje durable charakteru**. `world.rs` używa wspólnego `packet_clients`; część rows przygotowuje własne preconditions. **INFERENCE:** przy reorderyzacji lub łączeniu batchów należy osobno sprawdzić zależności od inwentarza/pozycji/progresji. Nie dowiedziono, że wszystkie rows są zależne od kolejności.

## Co zachować i jaki dowód już istnieje

| Warstwa / istniejące pliki | Co udowadnia | Decyzja |
|---|---|---|
| `tools/qualification/node_boot/run.sh`, `apps/game-server/src/qualification.rs::node_boot_seam_against_running_node` | Rzeczywisty proces `oteryn-game-server serve`, konfiguracja/operator bootstrap, PostgreSQL 18.6, przypięty portal-rs, World bundle, socket/TLS, shutdown | Zachować jako test fizycznej kompozycji. `#[ignore]` nie jest tu dead code: runner wywołuje konkretny test przez `--ignored --exact`. |
| `client_phases/world.rs::connect_character` (3416–3447) i większość gameplay rows | Rzeczywiste komendy przez shipping `oteryn_dev_client`, serwer i PG; admission materiał podpisuje harness (`sign_grant`) | Zachować jako component/system tests. Nie przypisywać im dowodu OAuth → native ticket → Gateway. |
| `client_phases/provision.rs:22–65` | Kwalifikacyjne preconditions w disposable fixture, ponowne wczytanie przez logout/admission; poziom/items/vitals | Używać jawnie w isolated mechanics. Odróżnić od dowodu organicznego zdobycia przedmiotu i levelowania. |
| `tests/security/auth/tests/native_ticket_and_gateway_fail_closed.rs::native_ticket_gateway_admits` (261) i `admitted_journey` | Realna projekcja Character i readiness, OAuth, ticket, realny Gateway, signed grant, shipping TLS admission; niezależne SELECT: dokładnie jedna Platform admission, jeden Game receipt, zgodny join session | Zachować i użyć jako podstawy pierwszego pełnego black-box pilotu. |
| `tools/qualification/game_platform/{run.sh,lifecycle.sh}`, testy `web_identity_pkce_isolation`, `native_device_rotation_replay` | Rzeczywista Platforma, odmowy, rotacje/replay, account isolation; negative phase wymaga zerowych admissions | Zachować jako test kontraktu i realnego właściciela tożsamości. Test-only TLS relay przekazuje odpowiedzi realnej Platformy; nie jest mockiem sukcesu. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/current_fence_receipt_read.rs::current_fences_read_one_receipt_and_keep_replay_and_history_checks_postgres` | Bieżące fence, replay i historyczne receipt przy realnej bazie | Zachować: black-box nie daje równie precyzyjnej kontroli nad każdą negatywną kombinacją dowodów. |
| `.../postgres_tests/level_up_install_fence.rs::{level_up_vitals_postgres_defers_stale_facts_after_revision_advance,level_up_vitals_postgres_retries_and_installs_latest_level_once}` | Zdezaktualizowane facts i ponowna instalacja najnowszych vitals | Zachować z realnym PG i dokładnym sterowaniem interleaving; nie zastępować probabilistycznym E2E. |
| `.../postgres_tests/fresh_admission_contended_holder.rs::{fresh_admission_completes_while_other_owners_share_the_holder_postgres,fresh_spell_repair_becomes_playable_without_reconnect_postgres}` | Kontencja holdera, post-admission repair i stale-generation refusal | Zachować jako deterministyczną granicę mutation/authority. |
| `tools/qualification/node_boot/legacy_d150.sql` oraz `run.sh::legacy_d150_seed` / `legacy_d150_register_starter` | Retained tag-3 bootstrap/recovery w pustej disposable bazie, z restrykcyjnymi guardami | Nie usuwać z powodu słowa `legacy`. Usunięcie dopiero po jawnej decyzji o zakończeniu zgodności tego formatu. |

Realny Game–Platform lane używa pinu `6d04b52a16145723eedb0c56640ad052dbd26e94` (`game_platform/lifecycle.sh:4`, README) i dwóch disposable PostgreSQL 18.6. `native_ticket_gateway_admits` czeka na autentyczną projekcję i raport readiness; driver nie wstawia produkcyjnej Character projection ani readiness row. To istotnie mocniejsza podstawa niż stawianie nowego mockowanego endpointu pod nazwą E2E.

**Granica wykonania auth:** `tests/security/auth/tests/support/mod.rs:549–556::run` zwraca `Ok(())`, gdy brakuje `GAME_PLATFORM_INPUT`. Sam ogólny cargo test nie ustanawia więc dowodu tej topologii. Właściwy `game_platform/lifecycle.sh:165–218` ustawia input/report, wywołuje `--exact`, a następnie wymaga konkretnego `NODE_BOOT_EVIDENCE`; brak wykonania nie może wtedy przejść tylko dzięki zerowemu exit code. To przykład poprawnego wzorca kontroli evidence, który warto zachować.

## Minimalny pilot black-box bez nowego frameworka

**RECOMMENDATION:** rozwinąć istniejący `game_platform` lane i shipping `Session`, zamiast tworzyć nowy ogólny framework. Zmiany pilota są propozycją, nie zostały zaimplementowane ani uruchomione w tym audycie. Każdy test musi mieć ograniczony deadline, jawne preconditions i oracle niezależny od metody właśnie testowanej.

| Kolejność / zachowanie użytkownika | Punkt zaczepienia w istniejącym kodzie | Oracle i negatywna kontrola |
|---|---|---|
| 1. Konto → ticket → Gateway → join → jeden ruch → logout | `native_ticket_gateway_admits`; shipping `crates/session/src/session_commands.rs::step` (42), `logout` (281), `session_access.rs::world_spatial` | Obecne wiązanie account/Character/World/Channel/session w PG; oczekiwane przesunięcie o jeden tile z niezależnej specyfikacji fixture, `Moved`, zgodny wire delta, `LogoutResult::Accepted`. Zablokowany tile: brak przesunięcia. Nie obliczać expected wyniku tą samą funkcją produkcyjnego pathfindera. |
| 2. Ponowna prawdziwa native admission zachowuje pozycję i inwentarz | Rozwinąć journey z (1), przenieść asercje z `client_row_s09.rs::journey` bez jego capture guard | Nowy session ID, poprawny przyrost dokładnie o jedno admission względem wcześniej odczytanego stanu, retained state z nowego wire snapshotu i read-only durable observera. Potrzebny nowy ticket/grant realnego Gateway, nie `connect_character` z testowym signerem. |
| 3. Jedna mutacja custody: stow i withdrawal wybranej ilości, później relog | Istniejący rzeczywisty flow E07 w `client_phases/full_e2ef4.rs:661–800`; shipping command/UI request adapter | Conservation: pre-state + delta = post-state; dokładnie jedna zmiana zasobu, zachowany wynik po nowej sesji. Żaden sam receipt/Accepted nie wystarcza. Minimalny kwalifikacyjny grant itemu to jawna precondition; nie może być użyty jako dowód nabycia przez gameplay. |
| 4. Replay, niewłaściwy scope, stale generation | Istniejące auth refusal cases + komponentowe fixture’y current-fence i generation; wysyłka przez publiczny native protocol | Odmowa na wire oraz zerowa zmiana Character/inventory/balance/receipt dla odrzuconej operacji. Zmienić jedną przesłankę authority na przypadek; nie liczyć ogólnego disconnect jako dowodu konkretnego refusal. |
| 5. Kontrolowany restart i wznowienie po zatwierdzonej mutacji | Istniejący node_boot lifecycle; `tools/dev-client/src/lib.rs::{disconnect,resume_session}` | Po ponownym starcie zachowane zatwierdzone wartości, brak duplikacji i nieuprawnionego odzyskania starej sesji. Osobna scenariuszowa tożsamość i stan; nie resetować fence, żeby test dało się wznowić. Pierwsza wersja nocna. |

Pilot 1 można zbudować na istniejącym przyjętym `Session`, bez nowej biblioteki klienta. Pilot 2 zmienia oczekiwaną liczbę admission w pozytywnym journey, więc wymaga świadomego dostosowania istniejących assertions/evidence do drugiej legalnej sesji; nie wolno po prostu rozluźnić globalnego `count == 1`. Piloty 3–5 powinny używać świeżej tożsamości/fixture lub jawnego baseline stanu i niezależnych odczytów.

## Docelowe przypisanie testów do warstw

| Warstwa | Zachować tu | Przenieść / ograniczyć |
|---|---|---|
| Szybkie unit/property | Czyste reguły, granice numeryczne, dekodowanie wersji protokołu, malformed input, state machine generation/replay, planner/parser/report contracts | Powtarzalne testy napisów i lokalnych flag tylko wtedy scalić, gdy wspólny kontrakt zachowa kontrolę wszystkich producentów. Nie usuwać negatywnych przypadków na podstawie braku sieci. |
| Komponent + rzeczywisty PostgreSQL | Migration/recovery, transaction rollback, custody conservation, concurrency, idempotence, stale-generation writes i independently current authority | Utrzymywać deterministyczne aranżowanie czasu/wyścigu. Black-box dodaje dowód granicy produktu, nie zastępuje pełnej macierzy operatorów mutacji. |
| Krótki black-box PR / post-merge | Real auth→Gateway→Game, podstawowy command, odmowa bez skutków, logout/relogin i jedna krytyczna durable mutacja; dokładne ID i świeże artifacts | Wybrać reprezentatywne ścieżki o stabilnych preconditions. Bez wielominutowego czekania na losowy drop i bez deklaracji całej mechaniki na podstawie jednego query. |
| Nightly / explicit qualification | Pełny row set, prawdziwe client capture+replay, długie expiry/real clock, restart, cross-node/recovery, dłuższe organic journeys | Otwarte wymagania akceptacyjne pozostają widoczne. Liczyć planned/executed/proven/gaps osobno, nie traktować wykluczeń jako PASS. |

Najpierw zmierzyć wall time per shard/scenario i udział setup/compile, liczbę retries i flakes z realnych artifacts. Dopiero wtedy zaproponować oszczędności runnerów. Obecny audyt wykazuje słabe oracles i błędne przypisywanie dowodu, nie dowodzi procentowej redukcji czasu CI.

## Weryfikacja i przekazanie

1. Oryginalny `stage()` zaakceptował nowy root znanego ID; poprawiony helper dał `FAIL(rows)`. Zachowane JSON-y przed i po: `blackbox_repros_baseline.json`, `blackbox_repros_proposal.json`.
2. Nowe helper tests uruchomione najpierw z oryginalną logiką: siedem testów, cztery failures; po poprawce i dodaniu round-trip wszystkich 46 wpisów: **8 tests, OK**. Ostatnie uruchomienie `python3 test_diagnostics.py KnownFailRowsTests` w scratch proposal: 8 testów w 0,373 s, exit=0. `bash -n` dla proposal `run.sh`: exit=0. Te wyniki dotyczą propozycji childa, nie finalnego drafta roota.
3. Agregator na aktualnym, niezmienionym `matrix.py`: niewłaściwy `I01.full PASS` zamiast zaplanowanego `S09.client`, `problems=[]`; luka pozostaje otwarta.
4. Oryginalny unittest diagnostyki z exact base commit: 1 test PASS mimo rc=1 i rc=127. Reproduktor nie uruchamia Docker/PG/Game ani płatnego review.
5. Child nie zmieniał shared checkout, workflowów ani baseline. Patch był przekazany w scratch; finalna integracja, caller regression test, jego wyniki i publikacja draft PR są własnością roota.

**Artefakty lane:**

- `./blackbox_findings.md` — raport.
- `./blackbox_inventory.json` — pełne plany, 46 baseline i lista 31 guardów.
- `./blackbox_baseline_signature.patch` — wąska propozycja baseline matcher + helper tests.
- `./blackbox_repros.py` — reproduktor baseline i expected-ID gap.
- `./blackbox_repros_baseline.json` oraz `blackbox_repros_proposal.json` — wyniki przed/po.
- `./blackbox_oracle_repro.py` oraz `blackbox_oracle_repro.json` — oryginalny test diagnostyki przechodzący mimo błędów.

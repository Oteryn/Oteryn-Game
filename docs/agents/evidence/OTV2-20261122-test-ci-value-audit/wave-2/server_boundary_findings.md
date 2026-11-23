# Server boundary: admission, recovery, qualification — bounded audit

## Zakres i stan dowodów

Oteryn/Oteryn-Game, bazowy commit `8e9f0e16631aab1beb8416f8d5ca707aa3031083`. Odczyt źródeł i trzy małe próby rzeczywistego shellowego caller oracle z syntetycznym procesem testowym. Bez kompilacji Rust, PostgreSQL, Platformy, serwera i zmian w repo. Raport stanowi następną falę audytu; **nie obejmuje wdrożenia BB-02 ani ponownej kwalifikacji pierwszego draftu**.

**PROVEN** oznacza bezpośrednio przeczytany kod lub wykonany reproduktor; **DERIVED** oznacza konsekwencję konstrukcji testu, bez wykonanej mutacji runtime; **UNKNOWN** wskazuje brak pomiaru lub nieprzeczytany zakres. Dokładne pełne ścieżki, zakresy i SHA-256 plików zapisano w `server_boundary_reviewed_scopes.json`; kolejka pozostałych zakresów jest w tym samym pliku. Skrócone ścieżki `qualification/...`, `client_phases/...`, `node/...` i nazwy testów w tabelach odnoszą się do jednoznacznych pełnych ścieżek tego manifestu. To nie jest pełny audyt wszystkich testów serwera.

## Najważniejszy wynik

Obecny seam sprawdza wiele istotnych zachowań przez rzeczywiste TCP/TLS oraz niezależne od odpowiedzi serwera zapytania SQL. Nie ma podstaw do usuwania całego harnessu ani zastąpienia testów generacji i trwałości jednym przebiegiem klienta. Są trzy konkretne luki w oracle, które warto poprawić przed rozszerzaniem powierzchni E2E: rozpoznawanie zamknięcia połączenia, obserwacja floor przed pierwszym zapisem po restarcie oraz dowód wykonania dokładnie wybranego testu w S3B.

### SB-01 — timeout może zostać opisany jako zamknięcie/refusal

**PROVEN:** `apps/game-server/src/qualification.rs:842–865` (`exchange`) ignoruje wynik zapisu/flush oraz wynik timeoutu `read_to_end`. `frames` w tym samym pliku, wiersze 978–999, zwraca `Reply::Closed`, jeżeli nie znalazł pełnej ramki; niepełny ogon nie jest błędem. Helper celowo toleruje nadal otwarte, milczące połączenie dla scenariusza utraty liveness (komentarz 854–858).

Ten sam helper obsługuje jednak odmowy:

- `qualification/client_phases/negatives.rs:121–154`: `client_command_before_admission`, oversized i truncated sprawdzają tylko brak accepted session; potem log zawiera `oversized=closed truncated=closed`.
- `qualification/client_phases/recovery.rs:93–98`: powtórzone recovery credential jest uznane za odmowę, jeżeli wynik to `Reply::Closed` i licznik fresh admissions pozostał równy 2.
- Transport negatives w `negatives.rs:65–86` akceptują również `Reply::Closed`; sam plaintext odczyt też ignoruje timeout. Pierwszy poprawny handshake jest dobrym positive control, lecz nie zmienia semantyki późniejszego odczytu.

**DERIVED:** serwer, który po poprawnym TLS nie odpowiada i nie zamyka połączenia, spełnia powyższy warunek `Closed` po upływie deadline. Niepełna pierwsza ramka może dać ten sam wynik. Dla recovery sam licznik **fresh** admissions dodatkowo nie rozstrzyga zmiany istniejącej sesji. To luka dowodowa, nie dowód, że obecny serwer tak się zachowuje.

**RECOMMENDATION:** użyć istniejącego `exchange_must_close`/`exchange_must_close_paced` (`qualification.rs:871–976`) tam, gdzie kontrakt wymaga zamknięcia. Ten helper odrzuca timeout i niepełne/trailing bytes. Zachować tolerancyjny helper dla jawnego scenariusza milczącego admitted socket. Dodać jeden kontrolowany peer: utrzymuje socket bez odpowiedzi, a następnie wariant z częściową ramką; oba muszą oblać closed oracle. Dla replay recovery sprawdzić także generation/controller/receipt istniejącej sesji przed/po. Uwaga na deadline: strict helper ma obecnie 20 s, a composed listener deklaruje `entry_deadline: 20 s` (`qualification.rs:1974–1984`); dla truncated-input nie wprowadzać wyścigu równych timeoutów. Deadline obserwatora musi uwzględniać granicę serwera i niewielki zapas. Nie uruchamiano takiej mutacji ani peer testu w tej fali.

### SB-02 — test „floor across restart” zapisuje nową wartość przed odczytem zachowanej

**PROVEN:** `apps/game-server/tests/native_admission_source_postgres.rs:376–476`, `fresh_and_recovery_account_security_share_one_floor_across_restart`, poprawnie sprawdza wspólny floor fresh/recovery 10→11→12 i odrzucenie starszych danych. Po `drop(root)` i otwarciu `restarted` (439–441) pierwszym działaniem jest zaakceptowanie `u64::MAX` (442–454), a dopiero potem odmowa `MAX-1` (455–469).

**DERIVED:** część po restarcie nie wykazuje zachowania wcześniejszego floor 12 przed nowym zapisem. Błąd resetujący tylko floor przy ponownym otwarciu, przy zachowanych pozostałych faktach, może zostać przykryty przez zapis MAX. Test nadal ma wartość jako wspólny floor oraz obsługa granicy `u64`.

**RECOMMENDATION:** zaraz po otwarciu nowego root odczytać floor konkretnego account/namespace niezależnym SQL i wymagać 12, odrzucić observation 11 oraz sprawdzić idempotentne odczytanie/zaakceptowanie istniejącego 12, dopiero potem wykonywać część MAX. Helper `source_state` (838–850) już demonstruje niezależny SQL; dla tego testu lepszy jest dokładny subject niż globalne `max`. Bez potrzeby nowego frameworka. Nie wykonywano PG ani mutacji, więc nie przypisuję temu raportowi wyniku red/green tej poprawki.

### SB-03 — S3B końcowy PASS nie wymaga wykonania wybranego testu

**PROVEN:** `tools/qualification/wp5_s3b/run.sh:124–140` wybiera `--ignored --exact`, a 176–183 po rc=0 ustawia `result="$PASS_RESULT"`. Cleanup emituje ten wynik (90–102). Nie ma sprawdzenia wykonanego case ani końcowego dowodu z jego ciała. Obecna nazwa `qualification::server_seam_real_owners_over_tcp_tls` istnieje i ma `#[ignore]` (`qualification.rs:1587–1589`); obecny composition test też istnieje (`tests/wp5_s3b_composition.rs:718–720`). **Nie stwierdzono, że bieżący main uruchamia zero testów.**

Ważne zawężenie: ścieżka prebuilt ma guard nazwy w `prebuilt.py:143–151` poprzez `--list`, wiązanie tożsamości checkoutu i hasha pliku w 152–160, 193–202. Chroni to m.in. przed samym zniknięciem nazwy podczas budowania artefaktu. Listing nie sprawdza jednak eligibility pod `--ignored`; direct Cargo path ma tylko `--no-run`. Inne joby mogą niezależnie wykryć zmianę atrybutu testu — raport dotyczy oracle S3B, nie dowodzi zielonego całego merge gate.

**Wykonany reproduktor:** `python3 server_boundary_repros.py Oteryn-Game` wycina dokładny cleanup oraz tail rzeczywistego skryptu; podmienia wyłącznie proces testowy i pomija setup/cleanup topologii. Wyniki zapisane w `server_boundary_repros.json`:

| Syntetyczny proces testowy | Rzeczywisty shell rc | Końcowy marker |
| --- | --- | --- |
| `0 passed`, rc 0 | 0 | `S3B_RESULT=SEAM_PASS` |
| `1 passed`, rc 0 — kontrola | 0 | `S3B_RESULT=SEAM_PASS` |
| `1 failed`, rc 1 — kontrola | 1 | `S3B_RESULT=COMPOSED_FAIL` |

To dowodzi tolerancji caller oracle na zerowe wykonanie. Nie jest to uruchomienie libtest, Game ani Platformy. Reproducer zachowuje oryginalny tekst cleanup evidence; pole `topology_started=false` jawnie określa zakres próby.

**RECOMMENDATION:** sprawdzić exact nazwę przy tych samych flagach selection oraz wymagać wykonania jednego wybranego case i jego końcowego evidence; sam fixed count ogólnych wierszy E2E nie jest rozwiązaniem. Test regression powinien uruchamiać realną funkcję caller z zero-test output, a nie tylko osobny parser. Dotyczy również `s3b`/`spell-seam` wariantów; treść ich końcowego dowodu musi być osobno określona. Nie przygotowano ani nie zastosowano patcha.

## Co zachować — poziom dowodu i ograniczenia

| Istniejące zakresy | Co sprawdzają | Granica |
| --- | --- | --- |
| `qualification/client_phases/negatives.rs:104–249` i `admission.rs:1–222` | Złe capability i credential dają konkretne publiczne error frames; SQL receipts pozostają 0; ten sam grant później działa po naprawieniu capabilities; dokładne sekwencje i jedna wygrana konkurencyjnego admission. | Grants podpisuje harness, nie publiczny Gateway. Część negatywów ma SB-01. |
| `client_phases/recovery.rs:1–200`, `disconnect_resume.rs:1–81` | Ta sama GameSession, generacje 2 i 3, CommandId/sequence, stan i epoch/grace czytane SQL; reconnect nie tworzy fresh admission. Pending-step test dopuszcza stare położenie lub dokładnie jeden krok i wymaga ciągłości komendy. | Nie dowodzą restartu procesu przez samo odtworzenie TCP. Pending-step jawnie zamyka poprzedni transport, więc nie jest próbą komendy ze starego, nadal żywego socketu. Recovery token podpisuje harness. |
| `client_phases/use_wire.rs:1–144`, `dev_client.rs:1–291`, `readmission.rs:1–53` | Realny klient/protokół: door open/close, occupied/stale/unknown/replay dispositions, content/position/revision w join snapshot, trwały control loss i terminal release, fresh readmission. | Fixture entry-room. W tej fali nie przejrzano wszystkich podwywołań mechanik ani helperów `dev_client.rs:292–603`. |
| `native_admission_source_postgres.rs:750–1035` | Pending publication po ponownym otwarciu ma wcześniejszy checkpoint, sloty mają limity i exact clear; stale/unregistered/superseded custody odrzuca operacje, a niezależny SQL przed/po jest równy; odtworzenie starej custody w DB nie przywraca uprawnień. | PG component, nie login przez Gateway. Keep; SB-02 nie unieważnia tych innych testów trwałości. |
| `native_admission_source_transport.rs:704–810` | Pozytywna kontrola TLS, zła nazwa/root/client/chain; timeout oraz cancellation sprawdzają rzeczywisty EOF po stronie peer, nie tylko błąd klienta. | Kontrolowany lokalny producer, nie rzeczywisty Platform producer. Dobry wzorzec oracle dla SB-01. |
| `node/serve/postgres_tests/readiness.rs:1–39`, `node/serve.rs:4174–4237` | Prawdziwy trigger DB fault, niezależny `is_called` licznika, bounded failure; prawdziwe odebranie SQL privilege, sprawdzenie braku privilege, definite error bez retry. | PG component; bez env przypadki wracają `Ok(())`, więc trzeba liczyć je jako dowód tylko w PG lane. |
| `lifted_tests/foundation_recovery_tests.rs:509–832` | 12 konkretnych przypadków reconciliation, terminal/lease/scope fencing oraz race przy claim; sprawdzają zwrócony stan i disposition. | `RecoveryJournal` jest `Rc<RefCell<...>>` (1–110); „process restart” to nowy obiekt authority, nie restart procesu ani PG. Keep unit-level state machine, nie etykietować physical E2E. Ciało journal 111–482 pozostaje w kolejce. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/preadmission.rs:34–137` | Dwie handshake próby mają dokładnie jednego zwycięzcę, receipts SQL=1; po logout powtórzony nadal ważny grant jest odrzucony jako `AdmissionGrantReplayed` i receipts dalej=1. | Duplex zamiast TCP/TLS, kontrolowany clock/source oraz owner logout. To istniejący silny lower-level test replay po zwolnieniu character; proponowany publiczny pilot go uzupełnia. |

Źródło SQL oracle seam: `qualification.rs:645–656` liczy trwałe receipts; 3074–3089 łączy konkretną reconnect session z jej receipt `owning-loss-v1`, kontrolując epoch i długość grace. Finał in-process seam dodatkowo sprawdza actor/reservation counts i marks (2098–2142). `node_boot_seam_against_running_node` ma osobne ścieżki: World w 1558–1579, basic `seam_clients` w 1580–1582. Nie wolno sumować ich jako identycznego pokrycia.

## Prawdziwa ścieżka produktu i capture-only kontrakty

**PROVEN — granica kompozycji:** `server_seam_real_owners_over_tcp_tls` uruchamia prawdziwe `serve_gameplay` w tym samym procesie testowym; `qualification.rs:1954–2048` jawnie konfiguruje entry-room, `platform_enforcement_enabled: false`, brak NPC talk i części opcjonalnych ownerów. To silny test skomponowanych ownerów i wire boundary, lecz nie pełne uruchomienie shipping node ze wszystkimi funkcjami. `node_boot_seam_against_running_node` łączy się natomiast z osobno uruchomionym node; granty/recovery w tym seam nadal pochodzą z harnessu.

**PROVEN:** publiczny login już istnieje w `tests/security/auth/tests/native_ticket_and_gateway_fail_closed.rs:97–264`: prawdziwe OAuth → native ticket → Gateway login → `oteryn_session_tcp::connection::admit`, potem niezależne SQL w obu DB oraz porównanie publicznego GameSessionId z durable receipt. Test czeka na rzeczywisty outbox/runtime publisher (125–155), nie wstawia jego projekcji. Kończy się `drop(session)` (255), zatem **nie dowodzi publicznego movement→logout→relogin/recovery**.

`tools/qualification/game_platform/lifecycle.sh:151–223` ustawia środowisko, uruchamia exact named tests, sprawdza proces node i konkretne `NODE_BOOT_EVIDENCE`, w tym `identity_character_scope=exact`. To istotny, poprawniejszy guard niż samo S3B rc. Zwykły Cargo run bez `GAME_PLATFORM_INPUT` wraca `Ok(())` w `tests/security/auth/tests/support/mod.rs:549–555`; nie należy przypisywać temu zwykłemu pass wykonania publicznego journey.

`qualification/client_phases/client_row_s09.rs:18–47` już zawiera wartościowy bounded logout→fresh login: accepted logout, inne GameSessionId, to samo położenie. Jednak 50–62 pomija cały journey bez `E2E_ALL_CAPTURE_DIR`, a z env emituje **CAPTURED**, nie PASS. W tej konkretnej capture ścieżce `s09-expected.txt` jest obserwacją wyniku; komentarz 1–5 jawnie mówi, że raw logout bytes nie są zachowane. Replay koduje wynik ponownie. To nie jest dowód odtworzenia surowej ramki. Nie zmieniać automatycznie CAPTURED w PASS ani nie wpinać tego jako kompletnej pozycji macierzy bez osobnego kontraktu producent→konsument→terminalny wynik. Pełny problem completeness opisano wcześniej w BB-02; ta fala go nie implementuje.

## Obecni callerzy CI

| Zakres | Caller i ograniczenie |
| --- | --- |
| Real owner TCP/TLS seam | `.github/workflows/merge-group-gate.yml:1792–1810,1871–1882`: merge_group oraz push main (`3–7`), gdy `server_qualification != false` i oba buildy udane; real PostgreSQL 18.6 i Platform pin `6d04b52a16145723eedb0c56640ad052dbd26e94`. `.github/workflows/gameplay-server-seam.yml:1–78` jest manual-only. |
| S3B composition | `.github/workflows/wp5-s3b-composition.yml:1–19,79–90`: PR paths, manual oraz push dawnego task branch; uruchamia `--test wp5_s3b_composition` i wymaga `COMPOSED_PASS`. Nie oznacza szerokiej kwalifikacji każdego PR. |
| Native admission PG target | `.github/workflows/postgres-test-partitions.yml:318–333` i `merge-group-gate.yml:947–977` rejestrują `native_admission_source_postgres` jako rzeczywisty target, weryfikują source/metadata i budują archiwum. Wykonanie nextest: odpowiednio 777–802 oraz 1161–1187. Env PG: 505/629 oraz 1069. PR caller `pr-postgres-harness.yml:48–58,256–296` jest scoped i wymaga `heavy-tests`, nie jest dowodem wykonania na każdym PR. |
| Ordinary library i integration tests | `merge-group-gate.yml:660–665,701–713`, planner `tools/repository/plan_rust_test_shards.py:42–83`: biblioteki oraz wszystkie aktywne integration targets podzielone po Cargo metadata. `native_admission_source_transport` nie ma `test=false` w całym `apps/game-server/Cargo.toml` i nie należy do heavy names (planner 26–34), więc trafia do integration-rest przez autodiscovery — **DERIVED**, bez nowego Cargo inventory run. Unit mount recovery: `apps/game-server/src/lib.rs:190–192`; readiness mount: `node/serve.rs:15–17`. PG case bodies w zwykłej lane bez env mogą wracać sukces bez wykonania. |
| Real auth/Gateway journey | `game_platform/lifecycle.sh:151–223`, uruchamiany przez game-platform entry node-boot matrix; szczegółowy routing macierzy pozostaje w poprzednim `blackbox_findings.md` i raporcie CI. W tej fali nie odpytywano GitHub run history ani branch protection. |

**Legacy candidate wymagający decyzji, nie usunięcia:** `.github/workflows/wp5-s3a-real-interop.yml:1–90` utrzymuje osobny dawny Platform pin `623435ec1b907d6d9770b767806c90300252a71c`; `tools/qualification/wp5_s3a/run.sh:5–45` wymaga ancestry Game admission commit `cf5c5f...` i dokładnej tożsamości czterech dawnych source blobs przeniesionych do crate paths. To historyczna kwalifikacja kompatybilności, nie obecny portal-rs journey. **UNKNOWN:** czy nadal ma właściciela i wymagany accepted contract, czy przechodzi na obecnym HEAD; nie uruchamiano porównań brakujących starych obiektów ani topologii. Nie usuwać na podstawie samego wieku.

## Minimalny publiczny pilot bez nowego frameworka

**RECOMMENDATION — oddzielny następny PR:** rozszerzyć istniejący `admitted_journey`, z zachowaniem current topology/lifecycle. Najpierw mały, deterministic happy path i jeden dokładny replay negative; recovery z realnym issuerem w drugim kroku.

1. **Join przez rzeczywisty Gateway.** Zachować istniejące weryfikacje route/owner/session/receipts w obu DB. Setup synthetic accounts/fixture world pozostaje jawny; wszystkie akcje gracza od ticket w dół idą publiczną ścieżką.
2. **Jedna komenda movement, logout, ponowny Gateway login.** Oprzeć klienta/oracle o istniejące `dev_client.rs` i bounded `client_row_s09::journey`, ale wywołać zwykłe API journey bez capture guard. Wymagać określonego command outcome, przewidzianego położenia i revision, accepted logout, nowego GameSessionId po ponownym loginie oraz tej samej zapisanej pozycji. SQL odczytuje Character-owned stan i licznik receipts; setup nie zapisuje rezultatu ruchu za produkt.
3. **Zużyty fresh grant po logout, przed drugim loginem.** Token nadal ważny, character już zwolniony, pozostałe binding/currentness prawidłowe. Oczekiwać dokładnie publicznego `AdmissionGrantReplayed` = 1105, generation 0, zamknięcia socketu i niezmienionych durable receipts/controller state. Enum jest w `crates/protocol-oteryn/src/lib.rs:216–224`; mapowanie trwałego replay w `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/admission.rs:290–306`; lower-level wzorzec już istnieje w `preadmission.rs:108–119`. Sam replay pod aktywnym holderem z `client_phases/admission.rs:134–150` kończy wcześniej na AccountCharacterConflict, co kod wyraźnie dokumentuje.
4. **Osobny kolejny etap recovery.** Obecne `disconnect_resume` zachowuje dobre oracle session/sequence/no-new-admission; zastąpić harness-signed credential rzeczywistą publiczną recovery ścieżką dopiero po zweryfikowaniu issuer/device-binding i dostępności endpointu. `crates/platform-client/src/native_login.rs:480–494` ma encoder recovery request; samo to nie dowodzi działającego pełnego issuer flow. **UNKNOWN w tej fali:** kompletny aktualny publiczny recovery caller. Dodać próbę starego generation/transport oraz niezależną obserwację braku mutacji, a nie polegać wyłącznie na braku fresh receipt.

**Gdzie wykonywać:** cheap pure parsing/state-machine cases pozostają unit; PG stale holder/rollback/outcome-unknown pozostają component; mały real public join→step→logout→relogin pilot w obecnej relevant PR / merge lane; process replacement, dłuższe grace windows, lossy transport i wielokrotne recovery w nightly. Nie usuwać lower-level generation/durable-value invariantów tylko dlatego, że pojawił się happy path E2E. Żadnego arbitralnego target count jako substytutu listy scenariuszy i ich oracle.

## Source-shape testy i pozostała kolejka

`qualification.rs:3124–3168` testuje równość literalnych pinów, format SHA oraz wartości wyjęte z run.sh, a `node/serve.rs:5330–5349` porządek tekstu loaderów względem `readiness.publish`. `native_admission_source_postgres.rs:276–301` szuka literalnych constraintów w migracji. To małe guardy driftu konfiguracji/source, nie dowód gotowości node ani działania SQL constraint. `let _ = CharacterId::decode(...)` w qualification 3143 nie sprawdza wyniku. Można później skonsolidować sanity checks; nie rekomenduję usunięcia jako warunku pilota ani przedstawiania ich jako physical coverage.

Pozostałe zakresy, bez przypisanego werdyktu: szczegóły mocked RecoveryJournal 111–482; pozostałe case bodies native admission PG/transport; pełne `wp5_s3b_composition::composition_flow`; pozostałe node startup/config/channel_boot tests i podwywołania mechanik `dev_client`; rzeczywisty publiczny recovery lifecycle; pozostałe capture-only rows i ich consumers. Pełna kolejka ze ścieżkami jest w `server_boundary_reviewed_scopes.json`. W tej fali żadnego z tych obszarów nie oznaczono jako przejrzany na podstawie samego grep/inventory.

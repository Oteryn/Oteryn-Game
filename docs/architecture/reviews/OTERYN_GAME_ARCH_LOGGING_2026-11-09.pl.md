# Logowanie i diagnostyka błędów w Oteryn: projekt i architektura

Status: zaakceptowany przez właściciela 2026-11-09 (wszystkie punkty z §8; backend metryk
później). Zakres rozszerzony na cały serwer (§3.0). Stan kodu sprawdzony na głowie #3057 (`a44b7ec`).
Projekt rozbudowuje istniejącą decyzję ARCH-ERROR-CODES-0
(`docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`). Nie zastępuje jej.

## 0. Skąd ten projekt

Node boot na main jest czerwony. Klient testowy dostaje „transport I/O failed: early eof” około
15 s po tym, jak serwer zalogował `event=connection_admitted ms=5557…6610`. Serwer nie zapisuje,
dlaczego zamknął połączenie, więc przyczyny nie widać ani w logu CI, ani lokalnie. Ustalenie, że
winna jest jedna z siedmiu cichych ścieżek wyjścia w `serve_admitted_actor`
(`crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs`), w tym `hold_refused`,
które zamyka połączenie po 15 s bez żadnej ramki, wymagało czytania kodu i kilku 45-minutowych
przebiegów.

Cel projektu: **każda awaria da się zdiagnozować z jednego przebiegu, bez czytania kodu i bez
ponownego uruchamiania.**

## 1. Stan obecny (fakty z kodu)

Co już jest (ERR-NODE-1, częściowo):
- crate `oteryn-error-codes`: jeden format linii (`Line`, §1.5), kody `E####` z kategorią
  i progresją, rejestr `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`;
- panic hook, linia `process_start` z `build`, ślad (trace) całego procesu, filtr poziomów
  `OTERYN_LOG` (`apps/game-server/src/node/serve.rs:233-265`);
- zasady prywatności: w linii nie ma identyfikatorów gracza, tokenów ani grantów; `detail`
  jest escapowany i ma najwyżej 512 B.

Czego brakuje:

| Obszar | Stan |
|---|---|
| Ślad połączenia (`trace` na każde przyjęte połączenie) | nie ma; listener loguje bez śladu |
| Koniec połączenia po admisji | nie jest logowany; `hold_refused` i 6 innych wyjść są ciche |
| Powód odmowy admisji | tylko `refused_classified`, bez kodu |
| Wspólny writer | `oteryn_error_codes` używa 11 plików; `eprintln!` występuje ok. 920 razy (668 w `gameplay/dispatch`) |
| `oteryn-game-ops diagnose` (ERR-DIAG-4) | nie zaimplementowane |
| Ślad na wire (`CONNECTION_TRACE_V1`, ERR-TRACE-5) | numer 20 zarezerwowany, capability nie zarejestrowana |
| Metryki (liczniki, histogramy) | brak; backend celowo nierozstrzygnięty (§1.9) |
| Raport CI | `run.sh` nie streszcza logu węzła; trzeba ściągać pełne logi jobu |

## 2. Zasady

1. **Jeden model zdarzeń.** Każda linia diagnostyczna przechodzi przez `oteryn_error_codes::Line`.
   Nowy kod nie dodaje `eprintln!`.
2. **Każdy koniec jest logowany dokładnie raz.** Każda operacja z cyklem życia (połączenie,
   admisja, zapis trwały, wywołanie Platform, etap kwalifikacji) kończy się jedną linią
   `*_end` z wynikiem, kodem i czasem. Ciche zamknięcie jest błędem.
3. **Korelacja przez ślad, nie przez tożsamość.** Linie jednego połączenia łączy `trace`
   (UUIDv7). Identyfikatory gracza nigdy nie trafiają do logu (ANL-01 §18).
4. **Kod zamiast tekstu.** Każdy nieudany wynik ma zarejestrowany kod. Tekst idzie wyłącznie
   do `detail`.
5. **Tanio, gdy wszystko działa.** Szczegóły na poziomie debug są zbierane w pamięci
   i wypisywane tylko przy awarii (rejestrator lotu, §3.4).
6. **Testowalne.** Test potwierdza, że każda ścieżka wyjścia emituje swoją linię końcową.

## 3. Architektura

```
 zdarzenie ──► Span / Line (oteryn-error-codes) ──► filtr OTERYN_LOG ──► stderr (key=value)
                  │                                                  │
                  ├─► rejestrator lotu (pierścień na połączenie) ──────┘ wypisywany przy awarii
                  └─► liczniki i histogramy w procesie ──► `oteryn-game-ops stats` (gniazdo sterujące)

 stderr węzła ──► node.log ──► `oteryn-game-ops diagnose` (--trace / --report / --summary)
                           └─► run.sh: streszczenie w $GITHUB_STEP_SUMMARY przy porażce
```

### 3.0 Zakres: cały serwer, nie tylko połączenie

Każdy podsystem serwera ma ten sam kontrakt: zarejestrowany moduł, zarejestrowane zdarzenia,
`Span` na każdą operację z cyklem życia, kod przy każdej porażce i ślad, który wiąże pracę
z jej przyczyną.

**Hierarchia śladów.** Każda praca ma rodzica:

```
boot trace (proces)
 ├─ startup/<etap>            ładowanie konfiguracji, treści, świata, migracje, rejestracja węzła
 ├─ job trace                 jedno uruchomienie zadania okresowego (world pass, harmonogram)
 │   └─ durability / platform operacje tego uruchomienia
 ├─ task trace                każde długo żyjące zadanie tokio (pętla tick, kolektor, listener)
 ├─ connection trace          jedno połączenie gracza
 │   └─ command (seq)         jedna komenda i jej wynik
 │       └─ durability / platform / channel-owner work item
 └─ ops trace                 jedno polecenie z gniazda sterującego (oteryn-game-ops)
```

Linia dziecka niesie własny `trace` i `parent=<trace rodzica>`, więc `diagnose --trace` potrafi
pokazać całe drzewo.

**Katalog podsystemów i obowiązkowe zdarzenia.** Nazwy modułów to tokeny z rejestru zdarzeń
(§3.9):

| Moduł | Gdzie w kodzie | Obowiązkowe zdarzenia (Span → `*_end`) |
|---|---|---|
| `node` | `apps/game-server/src/node/serve.rs`, `config.rs`, `cancellation.rs` | `process_start`, `startup_stage` (każdy etap z czasem), `readiness`, `drain`, `shutdown`, `config_rejected` |
| `content` | `crates/gameplay/content`, `content-promotion`, `quest_log_content.rs` | `content_load` (rewizja, liczby, odrzucone wpisy z kodem), `content_activation` |
| `world` | `crates/gameplay/world`, `world-bundle`, `world-change`, `weather` | `world_bundle_load`, `world_change_apply`, `tick` (tylko przekroczenia, §3.10) |
| `durability` | `crates/gameplay/durability` | `db_tx` (operacja, wynik, czas, ponowienia), `db_pool` (wyczerpanie, czas oczekiwania), `migration`, `fence_conflict` |
| `platform` | `crates/platform-client` | `platform_call` (endpoint, status, czas, ponowienia), `platform_unavailable` |
| `admission` | `native-admission-source`, `admission-evidence`, `character-recovery-fence` | `admission` (wynik, kod, czas), `character_bootstrap`, `recovery_fence` |
| `gameplay_transport` | `crates/gameplay/dispatch` | `connection`, `command` (typ, wynik, kod, czas; sukces na debug), `liveness_lost`, `handoff` |
| `channel` | właściciel kanału i aktorzy w `dispatch`, `entities` | `work_item` (przekroczenia), `actor_spawn`/`actor_despawn` (debug), `channel_owner_change` |
| `combat`, `spell`, `items`, `economy`, `housing`, `social`, `quests`, `npc`, `progression`, `encounters` | odpowiednie crate'y `gameplay/*` | odmowa reguły z kodem (`*_refused`), niespójność stanu (`invariant_violation`, zawsze error) |
| `monster_ai` | `crates/gameplay/monster-ai` | `ai_budget_exceeded`, `pathfinding_failed` (zliczane, próbkowane) |
| `scheduler` | `node/*_scheduler.rs`, `guild_jobs.rs`, `raid_scheduler`, `highscores` | `job` (nazwa, wynik, liczby, czas), `job_skipped` (powód), `job_overrun` |
| `ops` | `bin/oteryn-game-ops.rs`, gniazdo sterujące | `ops_command` (polecenie, wynik, kod, czas) |
| `runtime` | nadzorca zadań (§3.11) | `task_start`, `task_end`, `task_panic`, `task_stalled` |
| `stats` | §3.5 | `stats` co 60 s |

**Reguły dla całego serwera:**
- Porażka na granicy podsystemu (błąd zwracany z crate'u na zewnątrz) ma kod z rejestru.
  Enumy błędów granicznych dostają `fn code()` przy pierwszym dotknięciu modułu (§1.4 ARCH-ERROR-CODES-0).
- Odmowa reguły gry z powodu gracza (np. za mało many) to wynik, nie błąd serwera:
  poziom debug plus licznik. Niespójność stanu serwera to zawsze `error` z kodem.
- Ponowienie (retry) loguje się jako jedno `*_end` z `attempts=N`, nie N linii.
- Zdarzenia o wysokiej częstotliwości (tick, AI, pojedyncze ramki) nigdy na info. Raportują
  tylko przekroczenia progów i liczniki.

### 3.1 Span: cykl życia z gwarancją linii końcowej

Mały typ RAII w `oteryn-error-codes`, bez zależności zewnętrznych:

```rust
let mut span = Span::start(Level::Info, "gameplay_transport", "connection", trace);
// ... praca ...
span.outcome("admitted_disconnected").code(E3xxx).field("reason", "not_playable");
// drop(span) zawsze wypisuje:
// event=connection_end outcome=admitted_disconnected code=E3xxx ms=21043 trace=… detail="reason=not_playable first_entry=RefusedUnavailable"
```

- `drop` wypisuje linię końcową **zawsze**: przy `return`, przy `?` i przy anulowaniu future.
  Gdy nikt nie ustawił wyniku, outcome to `unset`. W teście `unset` jest błędem, więc nowa
  ścieżka wyjścia bez wyniku nie przejdzie testów.
- Linia startowa (`*_start`) jest na poziomie debug. Linia końcowa: `info` przy sukcesie,
  `warn` lub `error` z kodem przy porażce, a więc zawsze zapisywana (§1.10 pkt 5).
- Span dziecka dziedziczy `trace` rodzica, np. admisja, zapis trwały i wywołanie Platform
  w obrębie połączenia.

Dlaczego nie crate `tracing`: ARCH-ERROR-CODES-0 §3 go odrzuca, bo przepisałby wszystkie wywołania
i format linii. Span daje to samo w zakresie, którego potrzebujemy (start, koniec, czas, wynik,
ślad), w ok. 150 liniach.

### 3.2 Ślad połączenia (strona serwera, bez zmiany protokołu)

- Listener nadaje UUIDv7 każdemu przyjętemu połączeniu (`uuid_v7()` już istnieje) i przekazuje
  go przez `admit_frame`, `serve_admitted` i wywołania autorytetu.
- Wszystkie linie tego połączenia mają `trace=…`.
- Wysłanie śladu do klienta to osobny krok (ERR-TRACE-5, §3.7), bo zmienia protokół.

### 3.3 Kody końców i odmów

- `ConnectionEnd` i `AdmissionRefusal` dostają `fn code(&self) -> ErrorCode` (rejestr gry,
  zakres 3xxx dla transportu). Każdy wariant ma kod; test wyczerpujący (`match` bez `_`) tego pilnuje.
- Siedem wyjść `serve_admitted_actor` ustawia `reason` w spanie: `resolve_native_scope`,
  `staff_refresh`, `remember_activity`, `quest_callback`, `not_playable(first_entry)`,
  `observe_none`, `setup_prepare`.
- `hold_refused` przestaje być ciche po stronie serwera: przed zamknięciem zapisuje
  `connection_end outcome=hold_expired` z powodem.

### 3.4 Rejestrator lotu (flight recorder)

- Ograniczony pierścień ostatnich N (np. 64) zdarzeń debug na połączenie, trzymany w pamięci.
- Przy nieprawidłowym końcu (kod błędu, `ControlLost`, `hold_expired`, panika zadania) pierścień
  jest wypisywany jako linie `event=flight trace=… seq=…`. Przy normalnym końcu jest porzucany.
- Daje szczegóły debug dokładnie dla awarii, bez kosztu logowania debug w całym ruchu.
- Pierścień przechodzi te same reguły prywatności: tylko tokeny, liczby i zredagowany `detail`.

### 3.5 Metryki w procesie

- Liczniki atomowe i proste histogramy (kubełki stałe) w `oteryn-error-codes::metrics`:
  - `connection_end_total{outcome}`;
  - `admission_refused_total{code}`;
  - `admission_ms` (p50/p95/p99);
  - `durability_write_ms`;
  - `platform_call_ms{endpoint}`;
  - `liveness_missed_total`.
- Odczyt: `oteryn-game-ops stats` przez istniejące gniazdo sterujące (unix socket). Port
  sieciowy nie jest otwierany.
- Okresowa linia `event=stats` co 60 s, żeby trend był widoczny w samym logu.
- Eksport do Prometheus/OpenTelemetry: decyzja właściciela (§1.9 nadal otwarte). Model danych
  jest zgodny z formatem tekstowym Prometheusa, więc późniejszy eksport to jeden adapter.

### 3.6 Narzędzia

- **`oteryn-game-ops diagnose`** (ERR-DIAG-4, już zaprojektowany) plus nowy tryb `--summary`:
  - zlicza końce według wyniku i kodu;
  - pokazuje rozkład czasów;
  - wypisuje 5 śladów z najgorszym wynikiem wraz z ich liniami.
- **`run.sh` node boot:** po porażce wywołuje `diagnose --summary` na `node.log` i dopisuje wynik
  do `$GITHUB_STEP_SUMMARY`. Wtedy w podsumowaniu jobu widać np. „12× admitted_disconnected
  reason=not_playable(RefusedUnavailable)” bez otwierania logów.
- **Artefakt CI:** `node.log` (zredagowany) wgrywany przy porażce, retencja 7 dni. To edycja
  workflow, więc wymaga zgody właściciela.
- **Klient testowy (`tools/dev-client`, kwalifikacja):** błąd zawiera fazę (`connect`,
  `join_snapshot`, `command`), czas od połączenia, ostatnią odebraną ramkę i `trace`, gdy jest
  znany. Zamiast „early eof” widać np. „early eof w fazie join_snapshot po 21.0 s, ostatnia
  ramka ServerAccepted”.

### 3.7 Ślad i powód zamknięcia na wire (zmiana protokołu)

- ERR-TRACE-5 bez zmian: capability `CONNECTION_TRACE_V1` (id 20), pole `connection_trace`
  w `ServerAccepted`, `ServerResumeAccepted` i `ProtocolError`.
- Rozszerzenie: zamiast cichego zamknięcia po admisji serwer wysyła `ProtocolError` z kodem
  (np. `NOT_PLAYABLE`, `HOLD_EXPIRED`), jeśli klient wynegocjował capability. Starszy klient
  nic nie zauważy.
- Wymaga przeglądu protokołu i zgody właściciela; robione jako osobny pakiet.

### 3.8 Zapadka (ratchet) na `eprintln!`

- `architecture-check` liczy `eprintln!` i `println!` w `crates/**` i `apps/**/src` (poza testami
  i przykładami) i porównuje z wartością bazową w pliku.
- Liczba może tylko maleć. Nowe wywołanie bez wpisu wyjątku oblewa CI.
- Migracja przy okazji: gdy moduł jest zmieniany, jego linie przechodzą na `Line`/`Span`
  (zgodnie z §1.10 pkt 5).

### 3.9 Rejestr zdarzeń

- Fragmenty rejestru, jak dla capabilities i kodów: `docs/contracts/logging/events/<moduł>.json`.
  Każdy wpis to nazwa zdarzenia, poziom, pola (tokeny), czy ma `Span`, kod(y) porażki
  i właściciel.
- `architecture-check` sprawdza trzy rzeczy:
  - każde `event=` w kodzie jest w rejestrze;
  - każdy wpis rejestru jest emitowany;
  - każdy podsystem z tabeli §3.0 ma swoje obowiązkowe zdarzenia.
- Raport pokrycia (`tools/…/log-coverage`) pokazuje procent podsystemów i wyjść objętych
  spanami. Rośnie z każdym pakietem.

### 3.10 Progi wolnych operacji i tick

- `Span` może mieć próg: `Span::start(...).slow_after(Duration)`. Przekroczenie podnosi linię
  końcową do `warn` z `slow=true`, nawet gdy wynik to sukces.
- Domyślne progi w jednym pliku konfiguracji (nie w kodzie):
  - transakcja DB: 250 ms;
  - wywołanie Platform: 1 s;
  - admisja: 2 s (dziś 5,6–6,6 s, więc od razu widać problem);
  - zadanie okresowe: 30 s;
  - work item kanału: 50 ms.
- Tick świata: licznik ticków i histogram czasu. Linia `warn tick_overrun` tylko gdy tick
  przekroczy budżet, najwyżej raz na 10 s z liczbą przekroczeń w tym czasie.

### 3.11 Nadzorca zadań w tle

- W kodzie jest ok. 160 miejsc `tokio::spawn`/`spawn_blocking`. Zastępuje je jedna funkcja
  `spawn_supervised(name, trace, future)`.
- Funkcja loguje `task_start` (debug) i `task_end` z wynikiem. Panika zadania daje
  `task_panic` z kodem `E4001`, nazwą zadania i śladem, zamiast cichego zniknięcia.
- Zadania długożyjące (pętle) zgłaszają heartbeat. Brak heartbeat dłużej niż próg daje
  `task_stalled` (wykrywa zakleszczenia i zawieszone pętle).
- `oteryn-game-ops stats` pokazuje listę żywych zadań z czasem ostatniego heartbeat.

### 3.12 Stan zdrowia węzła

- `oteryn-game-ops health` (gniazdo sterujące) zwraca:
  - gotowość;
  - stan puli DB;
  - dostępność Platform;
  - opóźnienie ticku;
  - liczbę połączeń;
  - zadania zawieszone;
  - ostatnie 10 błędów z kodami.
- Ten sam zestaw okresowo trafia do linii `event=stats`.

## 4. Prywatność i bezpieczeństwo

- Bez zmian względem §1.5: żadnych AccountId, CharacterId, GameSessionId, grantów, tokenów,
  haseł ani payloadów; `detail` zredagowany i ograniczony do 512 B.
- Ślad nie pochodzi od klienta i niczego nie autoryzuje.
- Rejestrator lotu i metryki podlegają tym samym regułom; etykiety metryk to wyłącznie
  zarejestrowane tokeny (bez nieograniczonej kardynalności).
- Brak wysyłki logów do zewnętrznych usług (Sentry itp.) bez decyzji właściciela.

## 5. Wydajność

- Filtr poziomu sprawdzany przed formatowaniem (`format!` tylko gdy linia przejdzie).
- Span na połączenie: jedna linia końcowa, linie startowe tylko w debug.
- Pierścień ograniczony rozmiarem; liczniki atomowe bez blokad.
- Żadnych linii info na pojedynczą ramkę lub tick.

## 6. Plan wdrożenia

| Pakiet | Zakres | Protokół/workflow | Rozmiar |
|---|---|---|---|
| **LOG-0** (razem z poprawką #3057) | ślad połączenia; `connection_end` z wynikiem, `ms`, powodem; kod przy odmowie admisji; streszczenie w `run.sh` | nie | mały |
| **LOG-1 Span** | `Span` w `oteryn-error-codes`; admisja, serwowanie, zapis trwały, Platform; test „każde wyjście ma wynik” | nie | średni |
| **LOG-2 diagnose** | ERR-DIAG-4 + `--summary`; `run.sh` używa go w CI | nie | średni |
| **LOG-3 zapadka** | licznik `eprintln!` w `architecture-check`; migracja przy dotknięciu | nie | mały |
| **LOG-4 rejestrator lotu** | pierścień na połączenie, zrzut przy awarii | nie | średni |
| **LOG-5 metryki** | liczniki i histogramy, `oteryn-game-ops stats`, linia `event=stats` | nie | średni |
| **LOG-6 kontekst klienta** | faza, czas i ostatnia ramka w błędach dev-clienta i kwalifikacji | nie | mały |
| **LOG-7 wire** | ERR-TRACE-5 + `ProtocolError` z kodem zamiast cichego zamknięcia | **tak** (przegląd protokołu) | średni |
| **LOG-8 artefakt CI** | upload zredagowanego `node.log` przy porażce | **tak** (workflow) | mały |

| **LOG-9 rejestr zdarzeń** | fragmenty `docs/contracts/logging/events/*.json`, check w `architecture-check`, raport pokrycia | nie | średni |
| **LOG-10 nadzorca zadań** | `spawn_supervised` zamiast ok. 160 `spawn`; `task_panic`, `task_stalled`; `oteryn-game-ops health` | nie | średni |
| **LOG-11 node + scheduler + ops** | etapy startu, harmonogramy i world passes, polecenia ops jako spany z kodami | nie | średni |
| **LOG-12 durability + platform** | spany transakcji i wywołań, pula DB, ponowienia, progi wolnych operacji | nie | średni |
| **LOG-13 admission + transport** | pełna migracja `dispatch` (668 `eprintln!`) i źródła admisji na `Line`/`Span` | nie | duży (dzielony na moduły) |
| **LOG-14 świat, kanał, AI** | tick, work items, AI: liczniki i przekroczenia | nie | średni |
| **LOG-15 reguły gry** | `fn code()` dla enumów granicznych w `gameplay/*`, `invariant_violation` | nie | duży (po crate'ach) |

Kolejność:
1. LOG-0 od razu, bo naprawia bieżący main.
2. LOG-1, LOG-9 i LOG-10 (fundament: Span, rejestr, nadzorca zadań).
3. LOG-2 (diagnose) i LOG-8 (artefakt CI), żeby CI od razu z tego korzystało.
4. Podsystemy: LOG-11, LOG-12, LOG-13, LOG-14, LOG-15. Każdy pakiet podnosi licznik pokrycia
   i obniża licznik zapadki (LOG-3).
5. LOG-4, LOG-5, LOG-6, LOG-7.

Podsystemy idą równolegle na liniach Codex, po jednym module na linię, bez wspólnych plików.

## 7. Jak to by zadziałało w obecnym incydencie

Bez projektu: „early eof”, potem kilka godzin czytania kodu i przebiegów.

Z LOG-0 do LOG-2 podsumowanie jobu CI pokazałoby od razu coś w tym rodzaju (przykład
ilustracyjny; prawdziwy powód da dopiero trwający przebieg diagnostyczny):

```
connection_end outcomes (12):
  12× admitted_disconnected  reason=hold_expired  cause=not_playable first_entry=RefusedUnavailable  ms≈21000
admission_ms p50=5.8s p99=6.6s
```

Taka linia wskazuje wprost ścieżkę i przyczynę. Widać też drugi problem, już zmierzony:
admisja trwa 5,6–6,6 s.

## 8. Decyzje dla właściciela

1. Akceptacja LOG-0 do LOG-6 jako kierunku (bez zmian protokołu i workflow).
2. LOG-7: zgoda na zmianę protokołu (ślad i kod zamiast cichego zamknięcia).
3. LOG-8: zgoda na edycję workflow (artefakt `node.log` przy porażce, 7 dni).
4. Backend metryk (Prometheus/OpenTelemetry) teraz czy później; rekomendacja: później, najpierw
   `oteryn-game-ops stats`.

## 9. Odrzucone opcje

- **`tracing` + `tracing-subscriber` w całym kodzie:** przepisanie wszystkich wywołań i zmiana
  formatu (spójne z ARCH-ERROR-CODES-0 §3). Do ponownej oceny razem z backendem obserwowalności.
- **Logi JSON:** format key=value jest ustalony (node-boot D6) i czytelny przy grepie.
- **Logowanie debug zawsze włączone:** koszt i ryzyko wycieku; rejestrator lotu daje to samo
  tylko dla awarii.
- **Zewnętrzny SaaS błędów (Sentry):** wysyłka danych na zewnątrz wymaga decyzji o prywatności
  (baseline diagnostyki klienta).

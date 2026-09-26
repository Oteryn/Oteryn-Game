# WP3 — audyt customizacji i decyzja o przejściu na upstream

Data: 2026-09-16. Repozytorium: Oteryn/Oteryn-Game.

**Status: retained audit evidence; wykonany przegląd wybranych źródeł i modelu ich warunków; pełna kwalifikacja wykonawcza NIEUKOŃCZONA.**
**Decyzja: NO-GO dla natychmiastowego usunięcia wszystkich czterech patchy z deklaracją równoważnej wydajności i bezpieczeństwa. Rekomendowana jest odwracalna redukcja, nie bezwarunkowe utrzymywanie wszystkich forków.**

Ten raport nie jest zaakceptowaną architekturą, nie zastępuje wymaganego niezależnego przeglądu całego diffu, kwalifikacji, aktywacji prac ani zgody na integrację.

## 1. Tożsamość materiału i wykonane działania

- PR #356: OPEN/DRAFT, merged=false, head `b6f6282798078443876ba16462a5462692378c77`.
- Drzewo badanego kandydata z zachowanego checkpointu: `388be23b1af7e9088f4f3b0f3384d1f25ce7920d`.
- Odczytany podczas audytu protected `main`: `82534b2d33550b2a5ff1ef923526cb6dd8d30673`.
- Badany `apps/game-server/src/durability/db.rs`: blob `e2647d7d7ba5204d4210e83c00ea2e4f649e7d48`.
- Badany `apps/game-server/src/durability/mod.rs`: blob `93fd44b5ed4b4ea8d2bce544fefacca6e1b072c8`.
- Zawężone zlecenie Codex wyłącznie dla porównania oficjalnych archiwów: komentarz `5693517919`; odpowiedź `5693530716` zawiera ogólny błąd wykonania, bez wyników etapów. To nie jest błąd badanego testu.
- Przegląd COMMENT przypięty do badanego commitu: review `5219653616`.
- Nie zmieniono źródeł WP3, Cargo, vendorów, workflowów, ochrony ani produkcji w ramach audytu.

Repozytoryjne źródła dyskusji:
- https://github.com/Oteryn/Oteryn-Game/pull/356
- https://github.com/Oteryn/Oteryn-Game/pull/356#issuecomment-5693517919
- https://github.com/Oteryn/Oteryn-Game/pull/356#issuecomment-5693530716
- https://github.com/Oteryn/Oteryn-Game/pull/356#pullrequestreview-5219653616

## 2. Stan dowodów — bez zamiany braków na PASS

| Dowód | Status | Znaczenie |
|---|---|---|
| Aktualne metadane i wybrane przypięte źródła | ODCZYTANE | Podstawa ustaleń statycznych |
| Lokalny model wybranych warunków lifecycle | 9 kontroli PASS; w jednej 48 kombinacji | Potwierdza tylko zachowanie modelu opisanego poniżej |
| Wykonanie oryginalnego Rust/SQLx/Tokio/rustls | NIE WYKONANO w tej sesji | Brak nowej kwalifikacji produktu |
| Pełny diff czterech vendorów z pobranymi i zweryfikowanymi `.crate` | BRAK WYNIKU | Brak pełnego policzonego porównania |
| Skompilowany/działający zamiennik upstream | NIE WYKONANO | Nie potwierdzono równoważności implementacji |
| Porównywalne A/B całego konsumenta | NIE WYKONANO | Brak podstaw do liczb CPU/latency/pamięć/throughput |
| Kwalifikacja awarii zamiennika | NIE WYKONANO | Brak podstaw do zatwierdzenia migracji |

Lokalne środowisko audytowe nie udostępniło rustc, cargo, psql ani docker, a próby pobrania toolchainu/archiwów zakończyły się błędem DNS. Dwie próby Codex — szeroka i zawężona do samych archiwów — zakończyły się ogólnym błędem bez dowodów wykonania. Przyczyna jest UNKNOWN.

## 3. AUDIT-LIVE-RECONCILE

**Klasyfikacja: potencjalny problem dostępności; wysoka pewność w zakresie odczytanych warunków; wpływ produkcyjny wymaga testu Rust/PG.**

Źródła na badanym commicie:
- `db.rs::WorkCustody::resume_deadline` zachowuje istniejący deadline; nowy dostaje tylko stan z `deadline=None`.
- `db.rs::WorkCustody::checkpoint_retry` odrzuca `persisted=true`, aktywne wykonanie i acknowledgement.
- `db.rs::RuntimeBackend::begin` odrzuca `now >= pass.deadline` przed zapytaniem SQL.
- `db.rs::WorkCustody::enqueue` nie tworzy nowej aktywnej tożsamości dla tego samego oryginału.
- `db.rs::WorkCustody::begin_acknowledgement` wymaga definitywnego wyniku i braku in-flight.
- `mod.rs::AdmissionRuntime` udostępnia resume i retry delegowane do tego backendu.

Konkretna ścieżka:
1. Oryginalna operacja zostaje zachowana w trwałym checkpoincie.
2. Jej termin to `t0+2000 ms`; wynik pozostaje nierozstrzygnięty, a wykonanie nie jest już in-flight.
3. Baza ponownie działa w `t0+3000 ms`.
4. Resume zachowuje `t0+2000 ms`; `begin` odmawia przed SQL.
5. Retry establishment odmawia, bo checkpoint już jest zapisany.
6. Acknowledgement odmawia, bo brak definitywnego wyniku; ponowne enqueue tego samego oryginału też odmawia.

Model potwierdza tę kombinację. Nie dowodzi nieodwracalnej utraty danych, braku każdej innej drogi lifecycle ani konieczności restartu całego procesu. Rekonstrukcja runtime pozostaje poza modelem. Dwa takie stany mogą zająć oba logiczne sloty; faktyczny wpływ wymaga odtworzenia.

Istniejące testy zachowania deadline nie dowodzą odzyskania sprawności po jego wygaśnięciu.

**Wymagany test:** po trwałym checkpoincie wywołać niejednoznaczność, poczekać na upływ terminu, przywrócić bazę, rozstrzygnąć ten sam oryginał w tym samym runtime, bez nowej tożsamości, bez podwójnego skutku i bez przedłużenia dawnej autoryzacji.

**Rekomendacja:** rozdzielić ważność pierwotnej autoryzacji od terminu nowej, jawnie dopuszczonej próby reconciliation-only. Nie resetować starego deadline w ciemno i nie zwalniać niejednoznacznej operacji. To propozycja wymagająca kwalifikacji, nie wykonana naprawa.

## 4. AUDIT-ROOT-DEMAND

**Klasyfikacja: rozbieżność z odczytaną Revision 3, wymagająca rozstrzygnięcia wobec ewentualnej późniejszej zaakceptowanej zmiany. Nie potwierdzono automatycznej pętli reconnect.**

`RuntimeBackend::maintain_root_ready` konsumuje żądanie, próbuje odzyskać połączenie i po dowolnym błędzie ponownie ustawia żądanie. Drugie jawne wywołanie może więc rozpocząć nowe okno bez nowego zdarzenia zewnętrznego.

Odczytana architektura Revision 3 wymaga, aby po nieudanym oknie kolejne wynikało z nowego/skumulowanego zdarzenia; samo niepowodzenie nie ma uruchamiać autonomicznego ponawiania. Model wykazał samouzbrojenie flagi przy dwóch jawnych wywołaniach. Nie zawiera schedulera ani sieci, więc nie wykazuje rzeczywistego stormu, obciążenia CPU czy ruchu sieciowego.

**Wymagany test:** jedno żądanie, błąd połączenia, pełne zakończenie sprzątania, drugie wywołanie bez nowego zdarzenia. Osobna kontrola musi zachować prawdziwe nowe zdarzenie przychodzące podczas pierwszego okna.

## 5. AUDIT-ERROR-CLASSIFICATION

FAKT: `From<sqlx::Error> for DurabilityError` niszczy oryginalny błąd i zwraca bezpolowy `BoundedDatabaseError`. Na tej granicy znika także rozróżnienie IO/TLS/config/SQLSTATE.

Redakcja tekstów nieufnych jest wartościowa. Rekomendowanym kandydatem jest ograniczony enum, zweryfikowany pięciobajtowy SQLSTATE i identyfikator operacji, bez zachowywania opisów peer ani sekretów. Nie jest to stwierdzenie luki kryptograficznej ani zrealizowana zmiana.

## 6. Ocena potrzeby poszczególnych customizacji

Poniższa dyspozycja nie jest pełnym policzonym diffem plik po pliku. `KEEP` oznacza zachowanie właściwości, a nie bezwarunkowe zachowanie aktualnej implementacji.

| Rodzina | Dyspozycja | Warunek usunięcia patcha |
|---|---|---|
| Original identity, durable checkpoint, replay, fencing | KEEP | Nie usuwać razem z bibliotekami |
| Limity własnych danych i dopuszczonej pracy | KEEP | Równoważne ograniczenia w warstwie Oteryn |
| Własność dokończenia po anulowaniu | KEEP / REPAIR | Wykonawca zachowuje operację po odejściu wywołującego |
| Finalizacja transakcji/puli | REPLACE_ABOVE_UPSTREAM — kandydat | Borrowed transaction + właściciel połączenia, testy ambiguity i cleanup |
| Blokujący loader plików certyfikatów | REMOVE_AFTER_EQUIVALENCE — wybrany profil | Inline CA nie używa file loadera; odrębnie uwzględnić innych konsumentów |
| Tokio I/O registration/custody | UNRESOLVED | Usunięcie file loadera nie usuwa zależności transportu od custom ownera |
| Pełna instrumentacja rustls | REDUCE — kandydat | Rzeczywista osiągalność, także komunikatów odrzucanych, i pokrycie zasobów |
| TLS tickets mimo disabled resumption | NIE UZNAWAĆ ZA MARTWE | Sprawdzić tworzenie wartości przed przekazaniem do magazynu |
| Frame/count/status/metadata gates PG | KEEP właściwość / MINIMIZE | Kontrola musi działać przed niebezpieczną alokacją |
| Vec-based statement cache | A/B WYMAGANE | Ta sama pojemność i korpus SQL; ocenić także owner-free path |
| Jawny profil TLS/auth/config | KEEP właściwość | Nie wracać po cichu do ambient `PG*`, innego TLS ani innej metody auth |
| AWS-LC provider i bezpieczeństwo TLS | KEEP profil do osobnej decyzji | Nie przypisywać accountingowi automatycznej poprawy kryptografii |
| Provenance i historia forków | ZACHOWAĆ / UJEDNOZNACZNIĆ | Jednoznaczna baza archiwum i rozdzielenie historycznych checkpointów |

Istotne źródła badanego specimen:
- `vendor/sqlx-core-0.9.0/src/net/tls/mod.rs::read_certificate_input_owned`
- `vendor/sqlx-core-0.9.0/src/net/mod.rs::ConnectionOwner / connect_tcp_owned`
- `vendor/sqlx-core-0.9.0/src/rt/resource_owner.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`
- `vendor/sqlx-postgres-0.9.0/src/connection/mod.rs::PgStatementCache`
- `vendor/sqlx-postgres-0.9.0/src/connection/stream.rs::recv_unchecked`
- `vendor/sqlx-postgres-0.9.0/src/options/oteryn.rs`
- `vendor/rustls-0.23.45/src/client/tls13.rs::ExpectTraffic::handle_new_ticket_impl`
- `vendor/rustls-0.23.45/OTERYN_PROVENANCE.md`

## 7. Ważne wcześniejsze ustalenia zachowane w decyzji

- `#356` zawiera głęboką instrumentację SQLx/Tokio/rustls, wynikającą z wymogu same-ledger preallocation/lifetime accounting; nie była to przypadkowa customizacja.
- Nie znaleziono wiarygodnego benchmarku dowodzącego poprawy normalnego throughput/latency przez customizacje. Obecne testy kwalifikują głównie poprawność, denial-before-allocation, cleanup, TLS/PostgreSQL i restart.
- Upstream SQLx/Tokio/rustls dostarcza wiele użytecznych limitów i mechanizmów, ale nie daje automatycznie identycznej per-operation preallocation/lifetime gwarancji.
- Wyłączenie TLS resumption nie dowodzi braku przetwarzania `NewSessionTicket`; osiągalność trzeba sprawdzać przed usuwaniem odpowiednich patchy.
- Inline CA w wybranym profilu oznacza, że file-loader jest silnym kandydatem do redukcji, ale custom Tokio owner nadal uczestniczy w transport/I/O path.
- Customowy `PgStatementCache` zmienia algorytm względem upstreamowego `LruCache`; wpływ wydajnościowy wymaga A/B zamiast założenia.
- Jawny no-ambient profil PG/TLS/auth ma wartość niezależną od szczegółowego accounting i powinien pozostać właściwością nawet po redukcji forków.
- Odczyt oficjalnego crates.io index rozstrzygnął oczekiwany checksum rustls 0.23.45 jako `0d41d731c7d2f962d1ccc364cec258de3c0e93b38c2fb3ba97ac74513048d634`; historyczny opening provenance zawierał niespójny wpis.
- Sprawdzono obecność kodu poprawki bezpieczeństwa rustls `f9d4ee92b20da535ef5982ffcaf6f62385f79641` w badanym forku. To nie zastępuje pełnej regresji wykonawczej.

## 8. Wydajność, bezpieczeństwo i koszt migracji

Nie istnieje w dowodach audytu porównawczy wynik pozwalający stwierdzić przyspieszenie lub spowolnienie całego systemu. Przesłanki z algorytmów i liczników są hipotezami, nie pomiarami.

Ocena bezpieczeństwa musi osobno obejmować integralność danych, TLS/auth, dostępność przy wyczerpaniu zasobów oraz ryzyko zmian i aktualizacji. cgroup/proces nie jest automatycznie równoważnym zamiennikiem odmowy pojedynczej alokacji: zakończenie workera zmienia model dostępności.

REKOMENDACJA: redukcję robić etapami, zachowując dane, semantykę i testy. Nie łączyć pierwszej migracji z nową topologią procesów, zmianą bazy, liczby połączeń, blokad, providera lub PKI. Zmiana zarządzania połączeniem jest ograniczonym, ale nietrywialnym zakresem; usunięcie wszystkich forków przy tych samych ścisłych gwarancjach może być dużą pracą. Bez prototypu nie podaje się dni ani procentowej przewagi.

## 9. Zachowanie wykonanej pracy

Nic z badań #356 nie powinno być kasowane tylko dlatego, że aktywny build przejdzie później na upstream. Rekomendowany punkt odniesienia to exact commit/tree, właściwe paczki upstream, lock, toolchain, feature graph i testy. Usunięcie patcha z aktywnego builda nie usuwa historii.

Testy właściwości należy zachować po zmianie implementacji. Archiwalny fork nie jest automatycznie produkcyjnie bezpiecznym rollbackiem: ma otwarte kwestie i wymaga aktualizacji bezpieczeństwa.

## 10. Macierz wymaganej kwalifikacji

Przed dopuszczeniem upstreamowego zamiennika potrzebne są rzeczywiste wyniki:

- kompletny i niezależnie zweryfikowany diff archiwów wszystkich czterech paczek;
- budowa obu wariantów z dokładnymi feature graphami i jednakowym toolchainem;
- ten sam workload, SQL, blokady, TLS/auth, dane, limity i warunki sukcesu;
- raw repetitions: offered/completed/rejected, p50/p95/p99, CPU, peak heap/RSS z metodą pomiaru; warm i reconnect oddzielnie;
- anulowanie przed skutkiem, w transakcji, przy COMMIT i przed acknowledgement;
- utrata odpowiedzi i rzeczywisty restart z nierozstrzygniętymi oryginałami;
- wygasła próba + ponownie zdrowa baza + reconciliation w tym samym runtime;
- zły CA/hostname/TLS/auth oraz hostile frame/count/status/ticket;
- zakończenie sprzątania starego połączenia przed rozpoczęciem nowej generacji;
- jedno zdarzenie recovery i brak samoczynnie tworzonego drugiego okna;
- brak podwójnych skutków, nieuprawnionego odnowienia authority, utraty obowiązku i nieograniczonego wzrostu;
- aktualne regresje bezpieczeństwa, bez cofnięcia do historycznej podatnej bazy.

## 11. Konkluzja

Audyt źródłowy uzasadnia odwracalną redukcję zależności, ale **NIE zatwierdza całkowitego przełączenia już teraz**. Ujawnione problemy pokazują również, że aktualnego forka nie należy bezkrytycznie traktować jako wzorca prawidłowego zachowania.

Najlepszy kierunek to: **oryginalne biblioteki wszędzie, gdzie równoważność zostanie wykazana, własne trwałe invarianty Oteryn ponad upstreamem oraz tylko najmniejsze pozostałe patche tam, gdzie publiczne API rzeczywiście nie potrafi zachować wymaganej właściwości.**

Pełnej kwalifikacji wykonawczej nie oznacza się jako zakończonej, dopóki nie istnieje realny zamiennik, porównywalne A/B oraz kwalifikacja jego zachowania w błędach i awariach.
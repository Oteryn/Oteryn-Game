# OTV2 WP3 — Canonical Qualification Matrix Q01–Q75

- Date: 2026-09-12
- Repository: `Oteryn/Oteryn-Game`
- Scope: WP3 SQLx/TLS resource accounting and exact Child B consumer qualification
- Evidence status: retained acceptance criteria; **not a claim that these tests were executed**
- Canonical audit branch at reconstruction: `agent/wp3-architecture-audit-20260912`
- Canonical report revision at reconstruction: Revision 3
- Purpose: make the full qualification matrix self-contained without requiring Git-history reconstruction.

## Provenance

`WP3-Q01`–`WP3-Q46` are preserved from the original verification matrix produced during the first complete audit. Their original wording is retained below except Q32, which is clarified without narrowing its original SASL obligation so that the complete PostgreSQL startup/authentication phase is explicit.

`WP3-Q47`–`WP3-Q70` are the Revision-3 refinements added by the later consumer/pool/source review.

`WP3-Q71`–`WP3-Q75` are closure refinements added after independent review of the consolidated audit package. They make explicit five coverage classes that were implied across the report but were not previously standalone qualification cells: complete SQL-corpus closure, exact lock-footprint closure, producer-to-completion custody transfer, executor restart/takeover state, and retained credential/startup-configuration lifetime.

Every item remains an acceptance obligation until exact-head evidence explicitly proves it. An identifier does not imply that a test function with that name exists.

## Full matrix

| ID | Area | Acceptance obligation | Required result / observation | Status |
|---|---|---|---|---|
| WP3-Q01 | PROFILE | Graf features rzeczywistego konsumenta | Pozytywna operacja przez właściwy server/B graph, zapis exact features i providera. | Wymaga dowodu exact-head |
| WP3-Q02 | PROFILE | Sześć trybów PgSslMode | Każdy tryb zachowuje zaakceptowaną semantykę; odmowa zasobów nie jest milczącym fallback do plaintext. | Wymaga dowodu exact-head |
| WP3-Q03 | TEST | Równość mechanizmu cfg(test) i produkcyjnego | Ta sama propagacja właściciela Message; test wykrywa brak ownera na konsumencie. | Wymaga dowodu exact-head |
| WP3-Q04 | TEST | Niezależność obserwatora alokacji | Obserwator odróżnia pominięty hook od poprawnego zera; jego własne alokacje mają jawny zakres. | Wymaga dowodu exact-head |
| WP3-Q05 | OWNER | Odmowa przed konstrukcją właściciela | Max-minus-one odrzuca bez utworzenia rozliczanego kontrolnego Arc i bez wycieku debitu. | Wymaga dowodu exact-head |
| WP3-Q06 | OWNER | Właściciel SQLx znika przed referencjami Tokio | Rezerwacja kontrolna pozostaje do końca wszystkich handle, a nie tylko wrappera SQLx. | Wymaga dowodu exact-head |
| WP3-Q07 | OWNER | Współbieżny ostatni drop | Dokładnie jeden zwrot po fizycznej deallokacji; brak raw Arc/Weak omijającego protokół. | Wymaga dowodu exact-head |
| WP3-Q08 | OWNER | Jedna tożsamość przez wszystkie fazy | DNS, reaktor, TLS i loadery nie tworzą drugiego runtime-owner dla tej samej operacji. | Wymaga dowodu exact-head |
| WP3-Q09 | THREAD | Normalne join i limit fizycznych wątków | Limit i rezerwacja nie wracają przed zewnętrznym join; zachować naprawione działanie. | Wymaga dowodu exact-head |
| WP3-Q10 | THREAD | Panika callbacku startu przy unwind | Rezerwacja nie kończy się wewnątrz workera; poprawne sprzątanie i wynik błędu. | Wymaga dowodu exact-head |
| WP3-Q11 | THREAD | Panika callbacku zatrzymania przy unwind | Ok i Err z join kończą życie zasobów we właściwym porządku. | Wymaga dowodu exact-head |
| WP3-Q12 | THREAD | Błąd tworzenia wątku | Częściowo utworzone backing kończy życie przed zwrotem debitu; brak uszkodzonej kolejki. | Wymaga dowodu exact-head |
| WP3-Q13 | THREAD | Shutdown timeout i późniejsze zakończenie | Jawny właściciel późniejszego sprzątania lub wyraźnie zaakceptowany kontrakt; brak deklaracji GREEN na podstawie samego timeoutu. | Wymaga dowodu exact-head |
| WP3-Q14 | THREAD | Uchwyt std::thread::Thread przeżywa join | Metadane pozostają rozliczone do ostatniej rzeczywistej referencji; nie zakładać join-finality dla całości. | Wymaga dowodu exact-head |
| WP3-Q15 | NETWORK | TCP literal IP | Poprawna rejestracja I/O bez sztucznego kosztu DNS; odmowa przed rozliczaną alokacją. | Wymaga dowodu exact-head |
| WP3-Q16 | NETWORK | UDS | Właściwa wspólna rejestracja i teardown; bez wykluczania UDS jako obejścia. | Wymaga dowodu exact-head |
| WP3-Q17 | NETWORK | DNS nazwy hosta | Znany koszt prospektywny i finalizacja wszystkich objętych kontraktem zasobów; nierozstrzygnięte internals pozostają BLOCKED. | Wymaga dowodu exact-head |
| WP3-Q18 | NETWORK | Reaktor zachowuje referencję po drop socketu | Debit utrzymany przez pending-release aż do faktycznej deallokacji. | Wymaga dowodu exact-head |
| WP3-Q19 | BUFFER | Początkowe bufory i socket Box | Osobne funded i max-minus-one przed alokacją; zewnętrzna rezerwacja Box. | Wymaga dowodu exact-head |
| WP3-Q20 | BUFFER | Wzrost, zastąpienie i shrink | Pełny overlap starego/nowego backing; odmowa nie niszczy poprawnego starego stanu. | Wymaga dowodu exact-head |
| WP3-Q21 | BUFFER | Split / freeze / slice / clone | Współdzielony blok liczony raz i do ostatniego potomka. | Wymaga dowodu exact-head |
| WP3-Q22 | BUFFER | Niedozwolony unowned overflow | Budżetowe TLS nie przechodzi przez unowned_tail lub równoważny bypass. | Wymaga dowodu exact-head |
| WP3-Q23 | TLS | TLS1.3 VerifyFull – obecny profil pozytywny | Zachować istniejący rzeczywisty test PostgreSQL/TLS, nie uznawać go za dowód wszystkich komórek. | Wymaga dowodu exact-head |
| WP3-Q24 | TLS | TLS1.2 pełne i resumed | Rezerwacje oraz pełny lifecycle wszystkich zaakceptowanych ścieżek; bez przemianowania braku wykonania na N/A. | Wymaga dowodu exact-head |
| WP3-Q25 | TLS | TLS1.3 HRR / resumption / binder | Przeniesienia i tymczasowe kodowanie nie opuszczają tej samej księgi. | Wymaga dowodu exact-head |
| WP3-Q26 | TLS | Certyfikat skompresowany i błędna druga faza decode | Overlap dekompresji/AST/wyniku oraz rollback po zniszczeniu częściowego AST. | Wymaga dowodu exact-head |
| WP3-Q27 | TLS | ALPN wybierany przez serwer | Obie zaakceptowane wersje TLS: brak zwykłego clone rozliczanego payloadu i brak panic. | Wymaga dowodu exact-head |
| WP3-Q28 | TLS | Stany handshake i ich Box | Przedalokacyjne dopuszczenie każdego osiągalnego Box oraz zwrot po jego deallokacji. | Wymaga dowodu exact-head |
| WP3-Q29 | TLS | Root i client cert/key: plik oraz dane w pamięci | Źródło/DER/key/provider mają oddzielne lifetimes; poprawna rotacja i błędy zgodne z kontraktem. | Wymaga dowodu exact-head |
| WP3-Q30 | TLS | Walidacja certyfikatu i hosta | Bez osłabienia bezpieczeństwa podczas zmian właściciela; zachować semantykę każdego dopuszczonego trybu. | Wymaga dowodu exact-head |
| WP3-Q31 | PG | DataRow / RowDescription | Liczności i rozmiary sprawdzane przed rezerwacją/collection; poprawne dane graniczne nadal obsługiwane. | Wymaga dowodu exact-head |
| WP3-Q32 | PG | PostgreSQL startup/authentication lifecycle, w tym SASL | Pełny dozwolony profil od `Startup` do `ReadyForQuery` jest jawny; dla osiągalnych CleartextPassword, MD5Password, SASL/SCRAM, channel-binding, BackendKeyData i odpowiedzi/błędów wszystkie kontrolowane kopie, bufory i wyniki pozostają w tym samym finansowanym connect/root budget, a nieosiągalne mechanizmy mają dowód unreachable. | Wymaga dowodu exact-head |
| WP3-Q33 | PG | ParameterStatus / Notice / Notification / Error | Kopie, zagnieżdżone obiekty i zachowane Bytes nie są wolnym kosztem. | Wymaga dowodu exact-head |
| WP3-Q34 | PG | Statement/type metadata i cache | Pojemność, wymiana, wyrzucenie i retencja dzielonych metadanych mają jawne rezerwacje. | Wymaga dowodu exact-head |
| WP3-Q35 | PG | Wiersz/wartość zachowane po drop połączenia | Ostatni konsument kończy wspólne obciążenie; nie zwraca go sam socket. | Wymaga dowodu exact-head |
| WP3-Q36 | ERROR | Odmowa w kolejnych fazach, nie tylko przy zerze | Kontrolowana odmowa po wcześniejszych udanych rezerwacjach zachowuje typ błędu i wszystkie lifetimes. | Wymaga dowodu exact-head |
| WP3-Q37 | ERROR | Sync / rollback / Drop bez pamięci | Brak panic i brak drugiej puli; bezpieczny protokół sterujący albo zamknięcie/wycofanie połączenia. | Wymaga dowodu exact-head |
| WP3-Q38 | ERROR | Odmowa po wysłaniu / utracony COMMIT response | Nie zgłaszać niezatwierdzenia bez dowodu; zachować oryginalną tożsamość do reconciliation. | Wymaga dowodu exact-head |
| WP3-Q39 | ENVELOPE | Dwa sloty aktywne i pełna kolejka | Bilans wszystkich jednoczesnych zasobów mieści się w zaakceptowanym root; nie dodawać 12 MiB osobno na połączenie. | Wymaga dowodu exact-head |
| WP3-Q40 | ENVELOPE | Maksima semantyczne i limity wyników | Brak ukrytego obcięcia: m.in. 64 pending commands i 256 domain revisions nie są zastępowane limitem 32 zwracanych wierszy. | Wymaga dowodu exact-head |
| WP3-Q41 | HARNESS | Niebezpieczna konfiguracja adresu bazy | Odrzucenie przed pierwszym połączeniem; nic nie modyfikuje bazy spoza dozwolonego celu. | Wymaga dowodu exact-head |
| WP3-Q42 | HARNESS | Tożsamość kontenera i cleanup | Dokładny URL wskazuje tę samą izolowaną instancję; częściowy błąd nie pozostawia niekontrolowanych zmian. | Wymaga dowodu exact-head |
| WP3-Q43 | REGRESSION | Owner-free zachowanie i portability | Właściwe profile Linux/Windows/no_std są budowane i testowane według zakresu, bez rozciągania wyników na inne profile. | Wymaga dowodu exact-head |
| WP3-Q44 | PERFORMANCE | Porównanie i churn | Pomiar setup/steady-state/p99/CPU/alokacji oraz connect-query-drop w porównywalnym profilu upstream; bez wymyślonych progów. | Wymaga dowodu exact-head |
| WP3-Q45 | SUPPLY | Własny delta vs paczki upstream | Pełne sumy kontrolne, licencje, pliki poza manifestem zmian, własne unsafe/FFI i skutki aktualizacji toolchain. | Wymaga dowodu exact-head |
| WP3-Q46 | INTEGRATION | Końcowy exact-head konsument i review | Jedna spójna rewizja: rzeczywiste B/SQLx/TLS, niezależny whole-diff review, wymagane CI, MQ i readback. | Wymaga dowodu exact-head |
| WP3-Q47 | BOOTSTRAP | Revision-3 refinement | Fund configuration, pool bootstrap and custody restoration before work admission. | Requires exact-head evidence |
| WP3-Q48 | BOOTSTRAP | Revision-3 refinement | Bound migration-ledger count/checksum transfer before client materialization. | Requires exact-head evidence |
| WP3-Q49 | CONFIG | Revision-3 refinement | Bound/authorize PG environment, passfile and username configuration inputs. | Requires exact-head evidence |
| WP3-Q50 | CONFIG | Revision-3 refinement | Redact diagnostics before library sinks, including malformed credential records. | Requires exact-head evidence |
| WP3-Q51 | POOL | Revision-3 refinement | Exercise concurrent eager/maintenance/checkout connect attempts under one admission bound. | Requires exact-head evidence |
| WP3-Q52 | POOL | Revision-3 refinement | Observe maintenance after explicit return/wrapper Drop under selected pool policy. | Requires exact-head evidence |
| WP3-Q53 | POOL | Revision-3 refinement | Establish post-ping residency, not only after_release state. | Requires exact-head evidence |
| WP3-Q54 | CLOSE | Revision-3 refinement | Close during stalled TLS I/O while retaining truthful ownership. | Requires exact-head evidence |
| WP3-Q55 | PG | Revision-3 refinement | Verify all reachable retained connection maps, not only statements/buffers. | Requires exact-head evidence |
| WP3-Q56 | CUSTODY | Revision-3 refinement | Complete/acknowledge more than two sequential operations without runtime restart. | Requires exact-head evidence |
| WP3-Q57 | CUSTODY | Revision-3 refinement | Reconcile two occupied slots without a third slot/replacement identity. | Requires exact-head evidence |
| WP3-Q58 | CUSTODY | Revision-3 refinement | Classify pre-effect initialization failure versus uncertain takeover without blind reset. | Requires exact-head evidence |
| WP3-Q59 | TEST | Revision-3 refinement | Exercise registered production B construction separately from LegacyFixture. | Requires exact-head evidence |
| WP3-Q60 | OWNER | Revision-3 refinement | Observe release order/reuse while runtime remains alive, then audit shutdown separately. | Requires exact-head evidence |
| WP3-Q61 | PROFILE | Revision-3 refinement | Record resolved production features and prove excluded-family reachability claims. | Requires exact-head evidence |
| WP3-Q62 | PROFILE | Revision-3 refinement | Bind layout/phase evidence to compiler, allocator, target, panic and optimization profile. | Requires exact-head evidence |
| WP3-Q63 | BOUND | Revision-3 refinement | Prove each phase entry, overlap, escaping descendants and failure exits. | Requires exact-head evidence |
| WP3-Q64 | ENVELOPE | Revision-3 refinement | Show simultaneous root/sub-budget fit without double counting. | Requires exact-head evidence |
| WP3-Q65 | ERROR | Revision-3 refinement | Reserve completion/cleanup before COMMIT and retain ambiguity if delivery fails. | Requires exact-head evidence |
| WP3-Q66 | PROVIDER | Revision-3 refinement | Separate provider cold/warm initialization and lifetime crypto-thread churn. | Requires exact-head evidence |
| WP3-Q67 | ROTATION | Revision-3 refinement | Charge old/new configuration/connection overlap during permitted rotation/failover. | Requires exact-head evidence |
| WP3-Q68 | RUNTIME | Revision-3 refinement | Prove runtime driving/scheduling under crypto/parser load for selected topology. | Requires exact-head evidence |
| WP3-Q69 | REGRESSION | Revision-3 refinement | Run affected vendored and ordinary-consumer tests in exact supported profiles. | Requires exact-head evidence |
| WP3-Q70 | EVIDENCE | Revision-3 refinement | Retain source identities, test names/skips, independent observations and cleanup outcomes. | Requires exact-head evidence |
| WP3-Q71 | SQL | Complete production SQL corpus closure | Enumerate every production B SQL statement/query family on the exact consumer head and record for each: cardinality, variable-length transfer, same-snapshot size guard, lock dependency, deadline coverage, statement-cache behavior, returned/error lifetime and max/max+1 evidence; no unclassified statement may remain reachable. | Requires exact-head evidence |
| WP3-Q72 | LOCK | Exact 64-key / 16-relation footprint closure | Publish the deduplicated typed logical-key and relation-class inventory for every operation family, prove all sibling writers participate in the same pre-L fencing discipline, and exercise exact maximum plus first-overflow rejection without dropping required locks, FK/cascade coverage or phantom protection. | Requires exact-head evidence |
| WP3-Q73 | CUSTODY | Producer → queue → active → SQL → completion ownership graph | For every copy/move/share boundary, identify physical backing, reservation owner, temporary overlap, transfer event and final release. Include caller/producer storage where copied, queue retention, queue-to-active overlap, submission clones, encoder/SQL args, driver rows/errors, completion and acknowledgement. Demonstrate no uncharged duplicate peak and no early debit release. | Requires exact-head evidence |
| WP3-Q74 | RECOVERY | Executor initialization/restart/takeover state machine | Qualify `Empty -> Starting -> Ready` and all reachable failure transitions: definite pre-effect failure, cancellation while Starting, panic/process loss, stale Starting, predecessor executor still live, restart/replacement and fencing. No path may mint fresh two-slot capacity or reset ambiguous custody until predecessor work is proven fenced/final. | Requires exact-head evidence |
| WP3-Q75 | CONFIG | Retained startup configuration and credential lifetime | Inventory which startup/configuration values survive beyond connect, including the retained runtime registration identity. If a database URL or equivalent may contain credentials, bound and redact its copies and prove the chosen long-lived identity does not retain unnecessary secret-bearing material. Cover permitted username/database/application/options/password inputs and rotation overlap; forbidden sources/mechanisms require proven unreachable evidence. | Requires exact-head evidence |

## Closure rule

The matrix is complete only when every Q01–Q75 item is mapped to one of: `PROVEN_EXACT_HEAD`, `NOT_APPLICABLE_BY_PROVEN_UNREACHABILITY`, or an explicit blocking state. `NOT_APPLICABLE` may not be inferred from a manifest edge, a missing helper feature, a passing smoke test, or an unmeasured implementation assumption.

The full matrix must remain versioned with the audit package so future agents never need to reconstruct mandatory qualification obligations from historical commits.

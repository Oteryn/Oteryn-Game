# Oteryn Game — niezależny audyt programu i architektury

## 1. Executive verdict

`AT_RISK`

- Zamrożony `main` jest bezpieczny, fail-closed i przechodzi pełną lokalną walidację; kierunek native Rust / `protocol-oteryn` / multichannel pozostaje właściwy.
- Aktywny FOUNDATION PR `#59@9d5a251adb16076c3b0ebc50ae023677bf571894` ma jednak trzy otwarte P1 i dwa P2 z niezależnego review dokładnie tego SHA.
- QA `@1e763f4d24baaf42ecff1081fcd4660ecc4fd84f` potrafi nadać `PASS` niekompletnemu lub zduplikowanemu dowodowi i myli awarię infrastruktury z błędem produktu.
- DOMAIN i CONTENT mają zasadniczo prawidłowy kierunek, lecz są zablokowane przez kolejność shared lease; CONTENT dodatkowo nie może zostać zaakceptowany bez kanonicznych limitów DUR-04/VSL.
- Operacyjny rejestr alokacji i PR koordynatora są nieaktualne względem rzeczywistych headów.
- Zmergowany producent Game Catalog ma nieobsłużony przypadek ekstremalnego zagnieżdżenia JSON, ale produkcyjna publikacja i konsument Platform pozostają wyłączone.
- Nie stwierdzono P0 ani aktywnego legacy runtime; ryzyko dotyczy przede wszystkim następnej bramki integracyjnej, nie zmergowanego gameplayu.
- Program nie może bezpiecznie przejść do kolejnych zależnych lane’ów, dopóki P1 nie zostaną naprawione i ponownie niezależnie sprawdzone.

## 2. Audit snapshot

| Pole | Zamrożona wartość |
|---|---|
| Czas końcowego snapshotu | `2026-08-22T21:05:45.4281428Z` |
| Koniec weryfikacji | `2026-08-22T21:22:23.2046571Z` |
| Game repository | `Oteryn/Oteryn-Game` |
| Branch | `main` |
| Game SHA | `0240f9586bff579aca58cdf5686b96886a76cc23` |
| Game tree | `bda690a863c0a4ffaf9fe87cf121c2b686e2f96c` |
| META SHA | `c0dbad93f791953d5efcc6b556e6be73693f0a4f` |
| Platform contract revision inspected | `Oteryn/Oteryn-Platform@20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4` |
| Platform contract SHA-256 | `8c5b60e518a63f667865a108468fbe8b60b5cf3942c188ef0260eeb9185a1f93` |
| Prompt source | PR `#47`, `OTERYN-GAME-INDEPENDENT-PROGRAMME-AUDIT`, wersja `1.1` |

Pierwszy snapshot (`Game@a2a5da955dd8f580c9e768c8ac6a741db388cb22`, `2026-08-22T19:38:13Z`) został porzucony po materialnym przesunięciu `main`: PR `#57` i closeout `#60` zmergowały Game Catalog. Ustalenia raportu odnoszą się wyłącznie do powyższego końcowego snapshotu i wymienionych niżej dokładnych headów.

### Program Issues inspected

- `#53` — FOUNDATION;
- `#54` — VSL CONTENT;
- `#55` — DOMAIN;
- `#18` — historyczne retirement źródła;
- zamknięte `#52` — Game → Platform Catalog, ponieważ jego kod jest już częścią zamrożonego `main`.

### Open PRs and exact heads

| PR / branch | Exact head | Stan w snapshotcie |
|---|---|---|
| `#50` coordinator | `cec89c25fe778b1ba6b700addaeac8865df66daf` | OPEN, non-draft, `BEHIND` |
| `#56` DOMAIN | `674d1ccd637f3565c25750e5d5fe6c56df6fde32` | OPEN, draft, `BEHIND` |
| `#58` CONTENT | `ec68df7a461a011a6480898c9a6d9ee60703189e` | OPEN, draft, `BEHIND` |
| `#59` FOUNDATION | `9d5a251adb16076c3b0ebc50ae023677bf571894` | OPEN, ready, `BEHIND`, review findings open |
| QA branch | `1e763f4d24baaf42ecff1081fcd4660ecc4fd84f` | brak Issue i PR |
| Dependabot `#2` | `c4dfb85b75e9cf1e1df039b5fe5cb511d1a9a48a` | niezwiązany z aktualną bramką, stary i niezielony |

### Relevant checks and direct validation

**Merged main `0240f958…`:**

- `python tools/agents/validate_governance.py` — PASS;
- `python tools/repository/validate_repository_policy.py` — PASS;
- Game Catalog unit tests — PASS, `19/19`;
- `cargo fmt --all -- --check` — PASS;
- `cargo test --workspace --all-targets --locked --offline` — PASS;
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — PASS;
- `oteryn-architecture-check workspace .` — `workspace-boundaries: PASS`;
- zwykłe uruchomienie `oteryn-game-server` — kontrolowane fail-closed, exit `2`;
- tracked status przed i po walidacji — czysty.

**FOUNDATION `9d5a251…`:**

- lokalnie `33/33` testów game-server — PASS;
- format, Clippy `-D warnings`, governance — PASS;
- protected exact-head CI, Linux, Windows, supply chain i aggregate gate — PASS;
- niezależny exact-head review — **3 × P1, 2 × P2**.

**DOMAIN `674d1cc…`:**

- standalone `rustc` tests — PASS, `10/10`;
- standalone production compile `-D warnings` — PASS;
- workspace test/Clippy — PASS;
- exact-head CI — PASS;
- finalne połączenie z composition root jeszcze nie istnieje.

**CONTENT `ec68df7…`:**

- exact-head workflow evidence — PASS;
- standalone test suite deklarowana i sprawdzona przez exact-head CI — `14/14`;
- pozostaje celowo Draft z powodu shared lease i braku kanonicznych limitów DUR-04/VSL.

**QA `1e763f4…`:**

- testy gałęzi — PASS, `8/8`;
- workspace test i Clippy — PASS;
- brak exact-head PR CI i brak lifecycle Issue;
- niezależne próby negatywne audytu obalają wiarygodność klasyfikacji `PASS`.

### Repository protection observed at `2026-08-22T21:21:15Z`

Ruleset wymusza liniową historię, squash-only, CODEOWNERS, rozwiązywanie review threads, strict `Merge gate / validate`, zakaz usuwania i non-fast-forward. Secret scanning, push protection i Dependabot security updates są włączone. Otwarte alerty code scanning, Dependabot i secret scanning: `0 / 0 / 0`.

## 3. Current milestone

### Intended outcome

Aktualny milestone to post-SIM **Wave 1**: dostarczyć najmniejsze prawdziwe, rozłączne seamy FOUNDATION, DOMAIN, CONTENT i QA, które umożliwią później bezpieczne uruchomienie DURABILITY, ABILITY, INTERACTION, AI, CLIENT i pierwszego Movement slice.

Zmergowane i zakończone są:

- Bootstrap — PR `#10`, merge `0809004252db228e8f3fac3cdb6638c3c2a7fbda`;
- SIM — PR `#14`, merge `66619daf5837f31f7c54676e9f8351ed4ae220b0`;
- Wave 1 allocation — PR `#45`, merge `33cec30b8075c73290d7d76e9f59df4701771650`;
- exact-base bind — PR `#46`, merge `fd39c6aa026e82062a8b29af24811d467c115f19`.

### Authoritative acceptance criteria

- FOUNDATION: FND-ID/FND-02/FND-03/FND-04, bounded framing/schema, CommandId, sequencing/snapshot, authority generations, fresh admission, CharacterLease i reconnect, bez gameplay IDs i bez listener side effects.
- DOMAIN: protocol/persistence-neutral Character/Item semantics, revisions, equipment/container legality i fixture-only policy.
- CONTENT: typed canonical graph, deterministic compiler, osobne server/client projections, disposable bounded evidence artifact, staged atomic activation, bez zamrażania finalnego formatu.
- QA: wiarygodny evidence shell, który nie może uznać mocka, direct mutation lub niepełnego przebiegu za terminalne Tier 1/2/3.
- Każda lane: finalny dokładny SHA, self-review, wymagane independent review, exact-head CI i dopiero potem merge.

### Dependencies

- FOUNDATION posiada obecny serialized shared composition lease.
- DOMAIN, CONTENT i QA nie mogą wejść do `apps/game-server/src/lib.rs` przed legalnym przekazaniem lease.
- CONTENT wymaga osobnej alokacji do `RESOURCE_LIMITS_REGISTRY.json`.
- CLIENT pozostaje zablokowany do stabilnego produkcyjnego protocol seam.
- Tier 1 Foundation journey wymaga prawdziwego transport/listener seam; Tier 2 wymaga późniejszego native client networking.

### Current blockers

- otwarte P1 na FOUNDATION `#59`;
- niewiarygodna semantyka dowodów QA;
- brak QA Issue/PR;
- brak kanonicznych limitów CONTENT;
- stale coordinator/live-allocation truth;
- wszystkie aktywne PR-y są za bieżącym `main`.

### Explicitly not required yet

PostgreSQL/persistence, durable item transactions, gameplay abilities, movement/combat, final World Bundle format, szeroki content, native client Tier 2, channel orchestration, PERF/OPS, analytics, production deployment i Reference parity.

## 4. Programme evidence matrix

| Area | Phase requirement | Implementation state | Validation state | Evidence freshness | Verdict |
|---|---|---|---|---|---|
| Repo authority/governance | `REQUIRED_NOW` | `MERGED_STATE` | PASS | exact final main + live settings | PASS |
| Bootstrap composition root | `REQUIRED_NOW` | `MERGED_STATE` | PASS; fail-closed proven | exact final main | PASS |
| SIM deterministic kernel | `REQUIRED_NOW` | `MERGED_STATE` | PASS in bounded declared scope | merged code + current workspace tests | PASS |
| FOUNDATION protocol/runtime/admission | `REQUIRED_NOW` | `PROPOSED_STATE` | CI PASS, independent review FAIL | exact `9d5a251…` | FAIL |
| DOMAIN Character/Item core | `REQUIRED_NOW` | `PROPOSED_STATE` | focused/CI PASS; composition absent | exact `674d1cc…` | PARTIAL |
| CONTENT VSL seam | `REQUIRED_NOW` | `PROPOSED_STATE` | code/CI PASS; limits and composition blocked | exact `ec68df7…` | PARTIAL |
| QA evidence shell | `REQUIRED_NOW` | `PROPOSED_STATE` | unit tests PASS; adversarial audit FAIL | exact `1e763f4…` | FAIL |
| Coordinator/control-plane truth | `REQUIRED_NOW` | `DOCUMENTED_ONLY` | stale vs live heads | current main + `#50@cec89c2…` | FAIL |
| Game Catalog producer | `REQUIRED_BEFORE_NEXT_GATE` | `MERGED_STATE` | 19 tests PASS; deep-parser probe FAIL | exact main `0240f958…` | PARTIAL |
| Native protocol/client seam | `REQUIRED_BEFORE_NEXT_GATE` | `DOCUMENTED_ONLY` | no real listener/client transport | current | PARTIAL |
| Native client gameplay | `FUTURE_REQUIRED` | `DEFERRED` | pre-native shell fail-closed | exact main | NOT_YET_REQUIRED |
| Persistence/economy | `FUTURE_REQUIRED` | `DEFERRED` | architecture only | current docs | NOT_YET_REQUIRED |
| Multichannel runtime | `FUTURE_REQUIRED` | `DOCUMENTED_ONLY` | generation primitives proposed; no real scopes | current | NOT_YET_REQUIRED |
| Security/trust boundaries | `REQUIRED_NOW` | `PARTIAL` | repo baseline PASS; reconnect P1 | exact | FAIL |
| Resource safety | `REQUIRED_NOW` | `PARTIAL` | main bounded; Foundation/catalog gaps | exact | FAIL |
| Observability/operability | `REQUIRED_BEFORE_NEXT_GATE` | `PARTIAL` | safe typed errors; no production ops claim | current | PARTIAL |
| Supply chain/licensing | `REQUIRED_NOW` | `MERGED_STATE` | CI/supply-chain/security settings PASS | exact/live | PASS |
| Legacy/provenance | `REQUIRED_NOW` | `MERGED_STATE` | archived source; no runtime legacy hits | current/live | PASS |
| Player-visible vertical slice | `FUTURE_REQUIRED` | `DEFERRED` | no E2E claim, correctly withheld | current | NOT_YET_REQUIRED |

## 5. Architecture consistency matrix

| Subsystem | Intended architecture | Observed state | Status |
|---|---|---|---|
| Repository authority | Game owns runtime/domain/content; META coordinates | zgodne | PASS |
| Runtime protocol | native `protocol-oteryn`, no Canary fallback | Foundation proposed; main still fail-closed | PARTIAL |
| World/channel identity | `WorldId` and `ChannelId` distinct | contracts preserve distinction; runtime not yet present | PARTIAL |
| Command ordering | monotonic GameSession-scoped `CommandId`; ordered terminal effects | ordered terminalization naprawiona, lecz całe Foundation ma inne P1 | PARTIAL |
| Admission/reconnect | exact transport binding, one-time attempt, generation fencing | transport binding missing; committed attempt reusable | FAIL |
| Snapshot/reconciliation | monotonic snapshot/revision boundaries | snapshot ID replay boundary missing | FAIL |
| Character/Item semantics | protocol/persistence-neutral typed model | primary-path code sound in inspected scope; not composed | PARTIAL |
| Content model | typed canonical graph and explicit client allowlist | now typed, separate record families; blocked by limits | PARTIAL |
| QA evidence | complete canonical phases, tier-specific binary boundary, truthful infra classification | current shell can false-PASS | FAIL |
| Client | pre-native shell; no fake gameplay compatibility | fail-closed and uncoupled | PASS |
| Persistence/economy | later DUR implementation with fencing/conservation | absent by design | NOT_YET_REQUIRED |
| Cross-repo catalog | Game producer, Platform inactive consumer, exact lock, production disabled | contract locked; consumer pending; parser edge case open | PARTIAL |
| Legacy boundary | reference/import only | no material runtime contamination found | PASS |

## 6. Material findings

### F-01

```text
ID: OTERYN-AUDIT-F01
Severity: P1
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@9d5a251… / apps/game-server/src/foundation/protocol.rs / WireEnvelope decode
Exact evidence: independent PR #59 exact-head review, discussion r3836999411, protocol.rs lines 289-293; RESOURCE_LIMITS_REGISTRY `FND02-BOOTSTRAP-PAYLOAD-BYTES = 65,536`.
Current-phase relevance: NEXT_GATE
Why it matters: ClientBootstrap/ClientResume payload 65,537..1,048,576 bytes przechodzi envelope decoder bez message-specific odrzucenia przed credential/admission work. To omija zaakceptowaną granicę unauthenticated work.
Affected workstreams: FOUNDATION; QA Tier 1; CLIENT; wszystkie lane’y zależne od Foundation.
Required correction: po rozpoznaniu MessageType wymusić właściwy payload limit przed kopiowaniem/dekodowaniem/credential work; dodać 65,536 accepted / 65,537 rejected dla Bootstrap i Resume. Następnie nowy exact-head independent review i CI.
Must decide now?: YES. Blokuje merge FOUNDATION i realny transport seam. Supersession wymaga testu i review na nowym SHA; finalny gameplay payload schema pozostaje celowo niezdecydowany.
```

### F-02

```text
ID: OTERYN-AUDIT-F02
Severity: P1
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@9d5a251… / apps/game-server/src/foundation/admission.rs / reconnect PREPARE-COMMIT
Exact evidence: independent PR #59 exact-head review, discussion r3836999414, admission.rs lines 176-182.
Current-phase relevance: NEXT_GATE
Why it matters: PreparedReconnect nie przechowuje tożsamości konkretnego uwierzytelnionego transportu. COMMIT może przełączyć authority wyłącznie po attempt ref i jawnych generacjach, więc konkurencyjne połączenie może próbować przejąć przygotowaną generację.
Affected workstreams: FOUNDATION, CLIENT, QA, przyszły Gateway/admission.
Required correction: wprowadzić nieprzenoszalne candidate-transport binding tworzone po uwierzytelnieniu; ten sam binding musi być wymagany w PREPARE i COMMIT oraz unieważniany przy porażce/replace. Powtórzyć race/replay/competing-transport tests i niezależny review.
Must decide now?: YES. Blokuje bezpieczny reconnect. Fizyczny token/handle może pozostać implementacyjny, ale jego nieprzenoszalna semantyka musi być wymuszona teraz.
```

### F-03

```text
ID: OTERYN-AUDIT-F03
Severity: P1
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@9d5a251… / apps/game-server/src/foundation/admission.rs / ReconnectAttemptRef idempotency
Exact evidence: independent PR #59 exact-head review, discussion r3836999417, admission.rs lines 293-296.
Current-phase relevance: NEXT_GATE
Why it matters: wcześniej zakończony attempt A może zostać ponownie przyjęty przez PREPARE po kolejnym control loss. COMMIT zwraca starą generację, pozostawiając nową generację tylko jako prepared i niespójny stan sesji.
Affected workstreams: FOUNDATION, QA reconnect/replay, CLIENT.
Required correction: zakończone attempt refs muszą pozostać terminalne i nieprzyjmowalne przez nowy PREPARE; zachować bounded history/tombstone wystarczające dla idempotencji. Dodać dokładny scenariusz reuse-after-commit.
Must decide now?: YES. Blokuje merge i wszystkie dalsze semantics same-session reconnect.
```

### F-04

```text
ID: OTERYN-AUDIT-F04
Severity: P1
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@1e763f4… / apps/game-server/tests/support/evidence.rs / QA-E2E evidence model
Exact evidence: evidence.rs lines 172-183, 218-287; audit probes on exact head:
  one_phase_validate=Ok(()) / one_phase_class=Pass;
  tier3_same_as_tier2_validate=Ok(());
  duplicate_population=Pass;
  infra_validate=Ok(()) / infra_class=Fail;
  failure_without_divergence_validate=Ok(()).
Current-phase relevance: NEXT_GATE
Why it matters: shell może wystawić terminalny PASS dla jednej fazy, zduplikowanego attemptu lub Tier 3 bez dowodu exact production binary. Awaria infrastruktury jest błędnie klasyfikowana jako FAIL produktu. Zielone E2E z takiego modelu nie byłyby wiarygodnym dowodem.
Affected workstreams: QA, FOUNDATION, wszystkie późniejsze VSL/client/persistence lane’y.
Required correction: wymagać pełnego kanonicznego phase ledger z jawnie uzasadnionym N/A, unikalnych attempt IDs, tier-specific artifact/binary identity, spójnego failure class + first divergence i osobnej klasyfikacji infrastructure/BLOCKED/NOT_EVALUATED. Utworzyć Issue/PR i przeprowadzić negatywny review.
Must decide now?: YES. Blokuje użycie QA jako bramki dowodowej. Konkretna technologia runnera pozostaje niezdecydowana; prawdziwość modelu dowodu nie.
```

### F-05

```text
ID: OTERYN-AUDIT-F05
Severity: P2
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@9d5a251… / protocol.rs / snapshot barrier
Exact evidence: independent PR #59 exact-head review, discussion r3836999420, protocol.rs lines 443-447.
Current-phase relevance: NEXT_GATE
Why it matters: po commit `active` jest czyszczone i późniejszy taki sam lub niższy snapshot_id może zostać ponownie zaakceptowany. Unsequenced replay może nadpisać nowszy stan, a boundary może cofnąć server sequence.
Affected workstreams: FOUNDATION, CLIENT reconciliation, QA.
Required correction: zachować ostatni accepted/committed snapshot ID i server-sequence boundary; odrzucać non-increasing IDs/boundaries; dodać replay/backward-boundary tests.
Must decide now?: YES przed merge FOUNDATION.
```

### F-06

```text
ID: OTERYN-AUDIT-F06
Severity: P2
Truth: FACT
Repository/path/component: Oteryn/Oteryn-Game@9d5a251… / admission.rs / GrantNonce
Exact evidence: independent PR #59 exact-head review, discussion r3836999423, admission.rs lines 53-55.
Current-phase relevance: NEXT_GATE
Why it matters: kanoniczny FND-04 grant używa 32-byte jti, a implementacja reprezentuje replay key jako 16 bytes. Adapter musiałby skrócić/remapować credential identity, tworząc aliasing risk.
Affected workstreams: FOUNDATION, Platform/Gateway admission contract, QA.
Required correction: przechowywać pełne 32 bytes wraz z wymaganym trust scope i testami rozróżniającymi wspólne prefiksy.
Must decide now?: YES przed merge FOUNDATION.
```

### F-07

```text
ID: OTERYN-AUDIT-F07
Severity: P2
Truth: CONFLICT
Repository/path/component: main operational programme records / PR #50
Exact evidence:
  main docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md nadal deklaruje WAVE1_EXACT_BASE_BIND_PENDING;
  FOUNDATION_PROGRAMME_CURRENT_STATUS.md deklaruje Wave 1 implementing;
  PR #50@cec89c2… zapisuje stare heads Foundation 5dd9c…, Domain 28aa204…, Content 7b07a8c…, QA 63350b3…;
  finalne heads to odpowiednio 9d5a251…, 674d1cc…, ec68df7…, 1e763f4….
Current-phase relevance: CURRENT_GATE
Why it matters: coordinator/control-plane może przyznać lease, ocenić blockers lub sekwencję merge na podstawie nieaktualnego stanu. QA dodatkowo nie ma lifecycle Issue ani PR.
Affected workstreams: wszystkie Wave 1.
Required correction: nie merge’ować obecnego #50. Odtworzyć reconciliation z finalnego main i dokładnych headów, zanotować otwarte review findings, jednoznacznie nazwać shared-lease owner i utworzyć QA Issue/PR.
Must decide now?: YES przed jakimkolwiek lease transfer lub merge Wave 1.
```

### F-08

```text
ID: OTERYN-AUDIT-F08
Severity: P2
Truth: FACT
Repository/path/component: merged main@0240f958… / tools/game-platform-catalog/producer.py::load_json_file
Exact evidence: producer.py lines 509-519; audit probe: 40,001-byte valid JSON, depth 20,000 -> exit 1, uncaught RecursionError and full traceback with local filesystem paths. Declared GAMECAT-V1-NESTING-DEPTH is 16, but validation runs after json.loads.
Current-phase relevance: NEXT_GATE
Why it matters: bounded plik może zakończyć narzędzie niekontrolowanym wyjątkiem przed semantic depth check i ujawnić operatorowi/CI lokalne ścieżki. Nie jest to obecnie publiczny endpoint, a production_enabled=false, więc nie podnosi się do P1.
Affected workstreams: merged Game Catalog producer; przyszły Platform consumer/publication pipeline.
Required correction: zastosować decoder/prescan z limitowanym nesting przed pełnym materialization albo przechwycić parser RecursionError i zwrócić stabilny, zredagowany CatalogValidationError; dodać test znacznie ponad Python recursion limit.
Must decide now?: YES przed consumer fixture exchange lub aktywacją; nie blokuje Wave 1 runtime.
```

### P3 / NOTE

- Issue `#18` pozostaje otwarte mimo zmergowanego retirement PR `#23@19debbba…`, zarchiwizowanego `blakinio/Oteryn-v2` i braku otwartych Issue/PR w źródle.
- Dependabot PR `#2` jest oparty o stary stan i ma historycznie niezielony governance/aggregate gate; powinien zostać odświeżony albo zamknięty.
- Game Catalog nazywa dostępność producenta `server-first-safe`, podczas gdy Platform opisuje rollout jako `consumer-first`; operacyjny lock rozstrzyga właściwą kolejność jako inactive consumer → cross-repo proof → adapters → activation, więc nie stwierdzono materialnej niezgodności wykonawczej.

## 7. Workstream decisions

```text
Workstream: OTV2-IMPL-FOUNDATION
Issue: #53
PR/head: #59 / 9d5a251adb16076c3b0ebc50ae023677bf571894
Disposition: PAUSE
Dependencies: merged Bootstrap/SIM; current shared lease; FND contracts
Reason: trzy otwarte P1 i dwa P2 na exact independent review; PR jest dodatkowo behind main.
Condition to continue: naprawić F-01/F-02/F-03/F-05/F-06, zrebase’ować, ponownie przeprowadzić full diff self-review, independent exact-head review i required CI.
```

```text
Workstream: OTV2-IMPL-DOMAIN
Issue: #55
PR/head: #56 / 674d1ccd637f3565c25750e5d5fe6c56df6fde32
Disposition: BLOCKED
Dependencies: FOUNDATION merge/correction; lawful shared-lease transfer
Reason: primary-path code nie wykazał materialnego błędu w badanym zakresie, lecz nie jest częścią composition root i PR jest behind.
Condition to continue: po naprawie Foundation koordynator przekazuje lease; DOMAIN integruje minimalnie, uzgadnia wspólnego ownera canonical IDs, rozstrzyga review-risk classification i ponawia exact-head validation.
```

```text
Workstream: OTV2-IMPL-CONTENT
Issue: #54
PR/head: #58 / ec68df7a461a011a6480898c9a6d9ee60703189e
Disposition: BLOCKED
Dependencies: shared-lease transfer; osobna alokacja do RESOURCE_LIMITS_REGISTRY
Reason: obecna przebudowana implementacja ma właściwe typed client allowlist i sprawdza count/length przed allocation, lecz kanoniczne DUR-04/VSL hard maxima nadal nie istnieją; PR świadomie pozostaje Draft.
Condition to continue: zaakceptować limity poprzez właściwego ownera, zrebase’ować, podłączyć composition po lease i ponowić pełne testy/review.
```

```text
Workstream: OTV2-IMPL-QA
Issue: none
PR/head: no PR / 1e763f4d24baaf42ecff1081fcd4660ecc4fd84f
Disposition: REDIRECT
Dependencies: real merged product seams for terminal journeys
Reason: aktualny model dowodu może generować fałszywe PASS i nie odróżnia Tier 3 ani infrastruktury.
Condition to continue: utworzyć lifecycle Issue/PR; przeprojektować validator/classifier zgodnie z ADR-0007; negatywne exact-head tests muszą obalać wszystkie próby z F-04.
```

```text
Workstream: OTV2 implementation coordinator
Issue: coordinator task record
PR/head: #50 / cec89c25fe778b1ba6b700addaeac8865df66daf
Disposition: REDIRECT
Dependencies: exact live repository/PR state
Reason: diff jest historycznie poprawny, ale obecnie fałszywy względem wszystkich czterech worker heads i review findings.
Condition to continue: zastąpić/rebuild PR z current main; nie próbować rebase starej treści bez pełnego ponownego odczytu stanu.
```

Game Catalog `#52/#57/#60` jest już zmergowanym, zamkniętym workstreamem, nie aktywną lane. Wymaga osobnego małego corrective issue dla F-08 przed aktywacją konsumenta. Issue `#18` jest administracyjnie terminalne i powinno zostać zamknięte.

## 8. Cross-workstream conflicts

1. **Shared composition lease:** Foundation jest legalnym pierwszym ownerem. Jego otwarte P1 blokują DOMAIN, CONTENT i późniejsze przekazanie lease. Canonical owner: implementation coordinator.
2. **Stale coordination truth:** PR `#50` i main live-allocation record nie odzwierciedlają nowych headów ani review findings. Canonical owner: coordinator.
3. **QA lifecycle gap:** kod istnieje bez Issue/PR/CI, więc ownership, acceptance i closeout nie są adresowalne. Canonical owner: QA lane pod koordynatorem.
4. **Canonical identifier integration:** DOMAIN ma lokalne typed Character/World IDs, FOUNDATION implementuje FND identifiers. Obecnie nie ma zduplikowanego zmergowanego typu, ale przed composition trzeba wskazać jednego publicznego ownera i adapter boundary.
5. **Review-risk classification:** maintained status wymaga independent review dla high-risk item/loot/value changes, podczas gdy DOMAIN i CONTENT deklarują brak takiego wymogu dla fixture/semantic-only scope. Koordynator musi jawnie sklasyfikować finalny integrated diff; nie można pozostawić sprzecznych interpretacji.
6. **Cross-repo Catalog:** brak materialnego konfliktu aktywacyjnego. `CROSS_REPOSITORY_CONTRACT_LOCK.json` jednoznacznie utrzymuje Platform consumer jako `NOT_IMPLEMENTED_INACTIVE_CONSUMER_PENDING`, `production_enabled=false` i consumer-first operational sequence.

## 9. Legacy contamination review

| Przypadek | Klasyfikacja | Wynik |
|---|---|---|
| `blakinio/Oteryn-v2` | `ACCEPTED_MIGRATION` | repo archived/disabled, 0 open Issues, 0 open PRs; brak write authority |
| `blakinio/Otheryn` / historical content evidence | `JUSTIFIED_REFERENCE` | dopuszczalne wyłącznie jako przypięta provenance/reference evidence |
| `blakinio/otclient` | `JUSTIFIED_REFERENCE` | nie jest target client architecture; brak produkcyjnego dependency |
| Canary/Tibia wire/runtime | `UNKNOWN` dla przyszłych importerów; brak obecnego przypadku | w 37 Rust files zmergowanego workspace nie znaleziono Canary/Tibia/Otheryn/OTBM; production dependency tree nie zawiera legacy protocol |
| Game Atlas import/export tooling | `ACCEPTED_MIGRATION` | Game zachowuje source/projection authority; Atlas nie staje się Game truth |
| Proprietary assets | `UNKNOWN` dla przyszłego importu | zamrożony repo nie zawiera materialnego binarnego asset payload; każdy przyszły reuse nadal wymaga license/provenance review |

`NO MATERIAL UNJUSTIFIED LEGACY INHERITANCE FOUND IN INSPECTED CURRENT/NEXT-GATE RUNTIME SCOPE`.

## 10. Missing validation

### Required now / before next gate

- Foundation:
  - test limitu 64 KiB na Bootstrap i Resume;
  - authenticated candidate transport binding race tests;
  - committed reconnect-attempt reuse tests;
  - monotonic snapshot ID/boundary replay tests;
  - pełny 32-byte GrantNonce/trust-scope fixtures;
  - ponowny independent exact-head review bez otwartych material findings;
  - rebase na finalny main i cały protected gate ponownie.
- QA:
  - kompletność kanonicznych faz z uzasadnionym N/A;
  - unikalność attempt IDs;
  - Tier 3 exact release-binary identity;
  - spójność outcome/failure class/first divergence;
  - infrastructure/BLOCKED/NOT_EVALUATED classification;
  - Issue, PR, exact-head CI i review.
- CONTENT:
  - zaakceptowane registry limits dla artifact/section/record/key/string/definitions/cells/references;
  - boundary tests na finalnych wartościach;
  - composition validation po lease.
- DOMAIN:
  - workspace composition po lease;
  - finalne shared-ID ownership;
  - finalny risk/review determination;
  - rebase i exact-head CI.
- Coordinator:
  - aktualny status/heads/lease/review findings.
- Merged Catalog:
  - parser-depth test ponad runtime recursion limit i stabilny redacted error.
- Najmniejszy Tier 1 wire/admission/reconnect journey dopiero po realnym transport seam; obecny brak jest prawidłowo `BLOCKED/NOT_EVALUATED`, nie PASS.

### Future validation

- Tier 2 native-client networking/input/reconciliation/rendering;
- Tier 3 release binary smoke;
- PostgreSQL migrations, rollback, concurrency i crash recovery;
- durable item conservation/anti-duplication;
- real two-channel/multichannel recovery;
- measured PERF/OPS ceilings;
- final content-format spike, decompression/fuzz/property campaigns;
- Reference parity i legal provenance;
- production deployment, activation i rollback.

## 11. Top programme risks

1. **Current realised / next gate:** reconnect authority może zostać przejęte przez transport niebędący przygotowanym kandydatem.
2. **Current realised / next gate:** zakończony reconnect attempt może zostać ponownie użyty i rozszczepić stan generacji.
3. **Current realised / next gate:** unauthenticated bootstrap work omija limit 64 KiB.
4. **Current realised / next gate:** QA może wyprodukować fałszywy terminalny PASS.
5. **Imminent next gate:** snapshot replay/non-monotonic boundary może nadpisać nowszy stan.
6. **Imminent next gate:** skrócenie GrantNonce może aliasować credential replay identity.
7. **Current coordination risk:** stale coordinator record może spowodować błędny lease/merge order.
8. **Imminent content risk:** brak authoritative hard maxima blokuje bezpieczną akceptację parser/loader.
9. **Merged bounded tool risk:** Game Catalog może zakończyć się uncaught recursion tracebackiem na małym, głębokim JSON.
10. **Delivery risk:** brak jeszcze jednego prawdziwego client/server wire proof; zielone unit/CI nie są grywalnym postępem.

## 12. Immediate corrective actions

1. **Zatrzymać merge PR `#59`.** Naprawić wszystkie trzy P1 i dwa P2 z review; każdy repair tworzy nowy SHA i unieważnia obecne review/CI.
2. **Zastąpić PR `#50` aktualnym reconciliation.** Zapisać finalne heads, otwarte findings, lease owner, dependencies i QA lifecycle gap.
3. **Przeprojektować QA przed dodaniem kolejnych journeys.** Najpierw evidence truth model i negatywne testy; dopiero potem realne scenariusze.
4. **Utworzyć osobną bounded alokację dla CONTENT limits.** Nie pozwalać workerowi samodzielnie rozszerzać `docs/contracts/**`; po akceptacji limitów zrebase’ować `#58`.
5. **Po naprawie Foundation przekazywać shared lease kolejno.** DOMAIN → CONTENT → QA, z minimalną composition zmianą i exact-head validation każdego kroku.
6. **Naprawić merged Catalog parser jako mały corrective PR.** Depth-aware decode/catch + test + stable error.
7. **Po zmergowaniu poprawionego Foundation i naprawieniu QA uruchomić najmniejszy Tier 1 proof**, nie kolejną szeroką implementację.

## 13. Next evidence-producing milestone

### Exact observable outcome

Jeden deterministyczny, lokalny, produkcyjny server/protocol journey na exact buildzie:

1. bounded registered frame i envelope są przyjęte;
2. oversized Bootstrap/Resume są odrzucone przed credential work;
3. fresh admission tworzy `GameSessionId`, CharacterLease i generation `1`;
4. reconnect PREPARE wiąże dokładnie jeden authenticated candidate transport;
5. COMMIT tworzy generation `2`, stary transport zostaje fenced;
6. duplicate/reused attempt i stale generation są deterministycznie odrzucone;
7. CommandId nie wykonuje się drugi raz i terminal outcomes publikują się w kolejności;
8. snapshot replay/non-increasing boundary jest odrzucony;
9. nieznany gameplay pozostaje fail-closed;
10. QA zachowuje pełny phase ledger, exact artifacts, first divergence i cleanup.

### Prerequisites

- F-01/F-02/F-03/F-04/F-05/F-06 zamknięte;
- aktualny coordinator record;
- poprawiony Foundation exact-head independent review i CI;
- poprawiony QA validator oraz Issue/PR/CI;
- realny bounded transport listener/test client seam;
- brak potrzeby oczekiwania na finalny gameplay content lub persistence.

### Minimum evidence

- exact client/server build IDs i protocol revision;
- exact scenario/seed/clock/topology;
- byte-level golden/negative fixtures;
- replay, competing transport, stale generation i reused attempt tests;
- canonical phases z jawnie uzasadnionym N/A;
- artifacts z digestami;
- cleanup complete;
- independent review zero material findings.

### Safely deferred

PostgreSQL, durable economy, native-client Tier 2, Movement/Combat, permanent World Bundle encoding, real Reference content/formulas, multichannel deployment, PERF/OPS, analytics i production activation.

## 14. Final audit gate

`PROGRAMME_AUDIT = FAIL`

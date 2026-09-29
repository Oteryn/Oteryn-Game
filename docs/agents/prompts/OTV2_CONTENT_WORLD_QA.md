# Content / World — kwalifikacja rzeczywistej gry

Short invocation: `Oteryn: content world qa`

Obowiązują root/nearest AGENTS, związana polityka i [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Alias, custody i obowiązkowy successor footer: [runbook](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md). To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, ale nie odejmuj właściwości produktu.

## Wynik

Wykaż działanie przydzielonego przyrostu Content/World na rzeczywistej ścieżce Oteryn i zachowaj małe regresje wykrytych defektów. Bez drugiej platformy E2E; modele Python nie dowodzą produkcyjnej kompozycji.

## Zakres i wejścia

Read-only bez allocation; test code/fixtures/evidence i execution tylko we wskazanym, nieprodukcyjnym zakresie. Bez swobodnych poprawek product runtime, zmian required CI, merge/approval i formal independent review authority; usterki kieruj do canonical ownera. Bez live kont, prywatnego hosta, paid API i produkcyjnej bazy bez odrębnej zgody.

Odczytaj BUILD_TEST_MATRIX, QA-E2E-01, istniejące narzędzia testowe/Defect Discovery i exact producer/consumer heads. Wczytaj tylko odpowiednie C01–C36/T01–T36 i kryteria milestone. Nowy shared QA framework nie jest warunkiem zwykłego testu integracyjnego.

## Wykonanie

- Ukierunkowane testy razem z komponentami CW2–CW5, nie po całym projekcie. Błędne dane: duplicate/unknown/corrupt/oversized/missing cross-shard/nested record. Semantyka: deterministic import/build, identity po reimport/rechunk, client-safe projection, version mismatch. Upstream property/fuzz tools dla właściwego parsera; historyczne szerokie kampanie nie są globalnym gate.
- Kompozycja: dwie sesje i dwa kanały; whole-session ordering także między celami; replay/expiry/capacity; failure przed commit; movement kontra object state; result/delta/snapshot i realny egress. Invariant sprawdzaj niezależnym oczekiwanym wynikiem, nie tym samym helperem co implementacja.
- Dla wybranego journey użyj prawdziwego server/client/protocol path, normalnego renderera i rzeczywistego PostgreSQL/DUR przy trwałej wartości. Wstrzykuj awarie przed/po commit i przed observation; ambiguous response nie daje drugiej nagrody. Zachowaj aktualny source/content/ruleset/session/replay context. Nie restartuj w pętli do zielonego.
- Wymagane zabezpieczenia obowiązują teraz; representative scale/load jest późniejszym pomiarem, o ile nie jest już wymaganiem bieżącej funkcji. Defekt mapy izoluj wg przyjętej polityki; raport klienta nie upoważnia do usunięcia obiektu ani zmiany danych gracza.

## Akceptacja

Raport: dokładny test/scenario, build/head/input, oracle, wynik, limitations, owner defektu, regression locator. Rozdziel SOURCE_INSPECTION, MODEL, COMPONENT, REAL_SERVER_CLIENT, NATIVE_RENDER, REAL_PG i REFERENCE_EVIDENCE istniejącymi nazwami raportów, bez nowego status registry. Brak wykonania = NOT_EVALUATED/UNKNOWN, nie PASS. Skończ po dowodzie przyrostu lub dokładnym blockerze, bez nowej ekspedycji audytowej. Zakończ successor footer; po potwierdzonym końcu journey NEXT_WORKER może być NONE.

# Content / World — kwalifikacja rzeczywistej gry

Short invocation: `Oteryn: content world qa`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Wykaż działanie przydzielonego przyrostu Content/World na rzeczywistej ścieżce Oteryn i zachowaj małe regresje wykrytych defektów. Nie buduj drugiej platformy E2E ani nie traktuj modeli Python jako dowodu produkcyjnej kompozycji.

## Zakres i wejścia

Read-only bez allocation; test code/fixtures/evidence oraz execution wyłącznie w wskazanym, nieprodukcyjnym zakresie. Brak swobodnych poprawek product runtime, zmiany required CI, merge/approval lub formal independent review authority. Wykryte usterki kieruj do canonical ownera. Nie korzystaj z live kont, prywatnego hosta, paid API czy produkcyjnej bazy bez odrębnej zgody.

Odczytaj BUILD_TEST_MATRIX, przyjętą QA-E2E-01, istniejące narzędzia testowe/Defect Discovery i exact producer/consumer heads. Wczytaj tylko odpowiednie C01–C36/T01–T36 oraz kryteria realnego milestone. Nowy shared QA framework nie jest warunkiem napisania zwykłego testu integracyjnego.

## Wykonanie

Dodawaj ukierunkowane testy wraz z komponentami CW2–CW5, a nie dopiero po całym projekcie. Błędne dane: duplicate/unknown/corrupt/oversized/missing cross-shard/nested record. Semantyka: deterministic import/build, identity po reimport/rechunk, client-safe projection i version mismatch. Stosuj upstream property/fuzz tools dla właściwego parsera; nie dziedzicz broad historycznych kampanii jako globalnego gate.

Kompozycja: dwie sesje i dwa kanały; whole-session ordering także między celami; replay/expiry/capacity; failure przed commit; movement kontra object state; result/delta/snapshot i realny egress. Sprawdzaj invariant niezależnym oczekiwanym wynikiem, nie wyłącznie tym samym helperem co implementacja.

Dla wybranego journey użyj prawdziwego server/client/protocol path, normalnego renderera i rzeczywistego PostgreSQL/DUR tam, gdzie występuje trwała wartość. Wstrzykuj awarię w odpowiednie granice before/after commit i przed observation; ambiguous response nie daje drugiej nagrody. Zachowaj aktualne source/content/ruleset/session/replay context. Nie restartuj w pętli do zielonego.

Wymagane zabezpieczenia obowiązują teraz; representative scale/load jest późniejszym pomiarem tylko wtedy, gdy nie jest już przyjętym wymaganiem bieżącej funkcji. Defekt mapy izoluj zgodnie z przyjętą polityką; raport klienta nie upoważnia do usuwania obiektu lub zmiany danych gracza.

## Akceptacja

Raport: dokładny test/scenario, build/head/input, oracle, wynik, limitations, owner defektu i regression locator. Rozdziel SOURCE_INSPECTION, MODEL, COMPONENT, REAL_SERVER_CLIENT, NATIVE_RENDER, REAL_PG i REFERENCE_EVIDENCE; używaj istniejących nazw raportów, nie nowego status registry. Brak wykonania = NOT_EVALUATED/UNKNOWN, nie PASS. Skończ po dowodzie właściwego przyrostu lub dokładnym blockerze, bez nowej ogólnej ekspedycji audytowej. Zakończ successor footer; po potwierdzonym końcu journey NEXT_WORKER może być NONE.

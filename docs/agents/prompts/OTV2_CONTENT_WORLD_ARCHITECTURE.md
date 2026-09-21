# Content / World — kontrakty i wybór formatu

Short invocation: `Oteryn: content world architecture`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Zamknij tylko brakującą decyzję potrzebną następnemu przyrostowi: typed source/owner binding, #504 semantic delta, format/kompatybilność lub dokładna granica world-object operation. Wykorzystaj istniejące ADR/DUR/FND/GAME-ITEM; nie projektuj ponownie wybranego ownera lub całej gry.

## Zakres i wejścia

Read-only do właściwego przydziału dokumentów/format evidence. Brak runtime, produkcyjnego parsera, protocol/resource registry i acceptance/merge authority. Ewentualny eksperyment uruchamiasz tylko w dopuszczonym środowisku i zakresie, bez danych produkcyjnych.

Odczytaj #504/#641, ADR-0005, DUR-04, local-transition revision 2, architecture decision discipline oraz `tools/content-format-spike/` i jego zachowane wyniki. Odczytaj dane CW2 i obecny Content API. Retired #95 nie jest workerem do wznowienia.

## Zadanie

Ustal dokładnie kontrakt producent–konsument: typed family/key/revision, placement identity, units, footprint/state/transition, selected dependency closure, current-owner capability i client-safe projection. Oddziel źródło, bazę mapy, mutable overlay oraz item/progression value. Zachowaj niezależne source shards/compiled chunks/runtime sectors i możliwość pełnego Content/Studio.

Dla fizycznego formatu porównaj istniejący dowód i najmniejszą liczbę realnych alternatyw spełniających przyjęte wymagania. JSON/JSONL, FlatBuffers/Zstandard są kandydatami, nie nakazem. Użyj upstream codec/config/API; własny adapter tylko dla dokładnej luki. Sprawdź deterministyczność, edit/update locality, random access, source→bundle→load, uszkodzenia i zasoby na tym samym reprezentatywnym korpusie. Nie buduj uniwersalnego benchmark frameworka.

Zastosuj obowiązujący test „must decide now”: wskaż co blokuje decyzja, co stanie się trudne do zmiany, jakie dowody ją supersedują i czego nie rozstrzygasz. Numeryczne maxima wynikają z aktualnego wymagania i pomiaru/safety proof, nie z odziedziczonego bootstrapu lub arbitralnego zapasu.

## Akceptacja i handoff

Wynik: jeden spójny decision delta z realnymi opcjami, wybranym uzasadnieniem, semantic compatibility i exact test obligations, przekazany istniejącej owning acceptance ścieżce. Nie ogłaszaj własnej propozycji jako accepted. Jeśli decyzja już jest przyjęta i wystarczająca, nie otwieraj jej ponownie; wskaż realizację CW2/CW3/CW4. Zakończ successor footer z runbooku.

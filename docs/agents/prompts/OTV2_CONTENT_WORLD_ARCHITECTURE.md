# Content / World — kontrakty i wybór formatu

Short invocation: `Oteryn: content world architecture`

Obowiązują root/nearest AGENTS, związana polityka i [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Alias, custody i obowiązkowy successor footer: [runbook](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md). To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, ale nie odejmuj właściwości produktu.

## Wynik

Zamknij tylko brakującą decyzję potrzebną następnemu przyrostowi: typed source/owner binding, #504 semantic delta, format/kompatybilność albo dokładną granicę world-object operation. Wykorzystaj istniejące ADR/DUR/FND/GAME-ITEM; nie projektuj ponownie wybranego ownera ani całej gry.

## Zakres i wejścia

Read-only do przydzielonych dokumentów/format evidence. Brak runtime, produkcyjnego parsera, protocol/resource registry i acceptance/merge authority. Eksperyment tylko w dopuszczonym środowisku i zakresie, bez danych produkcyjnych.

Odczytaj #504/#641, ADR-0005, DUR-04, local-transition revision 2, architecture decision discipline, `tools/content-format-spike/` z wynikami, dane CW2 i obecny Content API. Retired #95 nie jest workerem do wznowienia.

## Zadanie

- Ustal kontrakt producent–konsument: typed family/key/revision, placement identity, units, footprint/state/transition, selected dependency closure, current-owner capability, client-safe projection. Oddziel źródło, bazę mapy, mutable overlay i item/progression value. Zachowaj niezależne source shards/compiled chunks/runtime sectors i możliwość pełnego Content/Studio.
- Dla formatu fizycznego porównaj istniejący dowód i najmniejszą liczbę realnych alternatyw spełniających wymagania. JSON/JSONL i FlatBuffers/Zstandard są kandydatami, nie nakazem. Użyj upstream codec/config/API; własny adapter tylko dla dokładnej luki. Na tym samym reprezentatywnym korpusie sprawdź deterministyczność, edit/update locality, random access, source→bundle→load, uszkodzenia i zasoby. Bez uniwersalnego benchmark frameworka.
- Zastosuj test „must decide now”: co decyzja blokuje, co stanie się trudne do zmiany, jakie dowody ją supersedują, czego nie rozstrzygasz. Numeryczne maxima wynikają z aktualnego wymagania i pomiaru/safety proof, nie z bootstrapu ani arbitralnego zapasu.

## Akceptacja i handoff

Jeden spójny decision delta: realne opcje, uzasadniony wybór, semantic compatibility i exact test obligations, przekazany istniejącej owning acceptance ścieżce. Nie ogłaszaj własnej propozycji jako accepted. Jeśli decyzja jest już przyjęta i wystarczająca, nie otwieraj jej ponownie; wskaż realizację CW2/CW3/CW4. Zakończ successor footer.

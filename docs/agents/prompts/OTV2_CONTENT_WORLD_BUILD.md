# Content / World — model, kompilator i pakiety

Short invocation: `Oteryn: content world build`

Obowiązują root/nearest AGENTS, związana polityka i [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Alias, custody i obowiązkowy successor footer: [runbook](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md). To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, ale nie odejmuj właściwości produktu.

## Wynik

Rozszerz obecną ścieżkę Content o przyjęty wspólny model, source validation/linking, profil następczy, indeksowane pakiety i bezpieczny loader/activation, dla tej samej docelowej gry i Studio, bez równoległego systemu.

## Zakres i warunki

Read-only do exact live allocation i wymaganych decyzji dla używanej granicy. Odczytaj #504/#641, `apps/game-server/src/content/`, source data CW2, decyzje CW1, ADR-0005/DUR-04 i build matrix. Publiczne schema, Cargo/lock, protocol/resource registries wymagają jawnej custody, nie samego aliasu.

Jesteś pojedynczym writerem przydzielonego wspólnego modelu; CW2 go używa i nie tworzy konkurencyjnych typów. Jeśli canonical `impl content` już prowadzi przyrost, kontynuuj jego lineage.

Bootstrap pojedynczego rekordu jest zakończony. Po protected D3 i CW2 B1–B5 normalna native/executable promotion w CW3 obejmuje **ograniczoną reprezentatywną partię albo całą gotową partycję rodziny**, nie pojedynczy item/creature/NPC. Pojedynczy rekord tylko jako fixture/regression/diagnostyka albo przy świeżym konkretnym blockerze; taki wyjątek nie liczy się jako postęp katalogowy i podaje blocker oraz warunek przejścia do batcha. Historyczne single-item PR-y są evidence, nie precedensem.

## Item batch promotion

Promocja nie jest osobnym celem. Dla tego samego Item batcha przyjmij exact eligible set z field-level evidence i continuity, zaktualizuj istniejące `ProjectReferenceRecord::Item` / `ReferenceItemSemantics` i od razu uruchom istniejący compile/test path.

- Partial promotion jest normalnym wynikiem: zweryfikowane pola stają się `Known`, reszta `Unknown/Conflict`. Nie czekaj na kompletny Item, jeśli typed model #749 reprezentuje pole.
- `promotable_fields == 0`: bez no-op promotion PR i dodatkowego schema/rule wrappera; zwróć batch do najbliższego source/identity/continuity blockera. `> 0`: promocja zwiększa liczbę canonical-promoted fields, a batch od razu przechodzi artifact v4 / compile-load qualification.
- Nowy parser, model, codec, rule engine lub schema widening dopiero po wykazaniu, że co najmniej jedno dokładne eligible pole nie ma reprezentacji w chronionym modelu. Raport, manifest lub przepakowanie tego samego payloadu nie jest postępem.

## Zadanie

- Wprowadź tylko przyjętą semantic delta #504: typowane definicje, ordered placements i domain bindings wymagane przez wybrany journey, bez ograniczania architektury do fixture. Source keys są stabilne, runtime numeric IDs revision-scoped; podział plików i stref nie zmienia identity/authority. Missing footprint data nie jest void.
- Użyj istniejących modułów i dojrzałych upstream parserów/serializerów. Strictness, limity, duplicates, units, reference families, cycles/expansion i client allowlist to walidacja Oteryn; brak jednego zachowania w API uzasadnia mały adapter, nie fork biblioteki. Bez permanentnego codec przed format decision.
- Buduj deterministic closed release i server/client projections z zachowaniem indexed chunk access, integralności i version compatibility. Indeksy z jednego źródła; lokalne palety/reuse chunków nie uzasadniają własnego CDN. Weryfikuj używane bajty przed publikacją; granice decompression/allocation/work są skończone i sprawdzane przed drogą alokacją.
- Użyj istniejącego staging/activation z jego quiescence/authorization. Nie zmieniaj interpretacji first-production v1, nie mieszaj generacji, nie przemycaj durable migration; LKG bytes nie cofają danych graczy. Bez niezależnego hot-reload managera.

## Akceptacja

Rzeczywiste source→compile→load przez obecny pipeline daje przyjętą semantykę; powtórny build jest deterministyczny; zmiana enumeracji lub shardowania nie psuje identity; negatywne referencje/bounds/corruption/version/client leakage są odrzucane. Zmierz random access i małą aktualizację na realnej partii; pełnej skali nie deklaruj po fixture. Wynik i adapter contract przekaż CW4/CW5 i CW6; zakończ successor footer.

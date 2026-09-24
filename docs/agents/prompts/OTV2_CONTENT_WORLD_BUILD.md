# Content / World — model, kompilator i pakiety

Short invocation: `Oteryn: content world build`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Rozszerz obecną ścieżkę Content o przyjęty wspólny model, source validation/linking, profil następczy, indeksowane pakiety i bezpieczny loader/activation. Dane mają służyć tej samej docelowej grze i Studio, bez równoległego systemu.

## Zakres i warunki

Read-only do exact live allocation oraz wymaganych decyzji dla używanej granicy. Odczytaj #504/#641, aktualne `apps/game-server/src/content/`, source data CW2, przyjęte decyzje CW1, ADR-0005/DUR-04 i build matrix. Publiczne schema, Cargo/lock, protocol/resource registries wymagają jawnej custody; nie przejmuj ich z samego aliasu.

Jesteś pojedynczym writerem przydzielonego wspólnego modelu. CW2 używa tego modelu, nie tworzy konkurencyjnych typów. Jeśli canonical `impl content` już prowadzi przyrost, kontynuuj jego lineage zamiast tworzyć nowe.

Bootstrap pojedynczego rekordu jest zakończony jako zwykła ścieżka rozwoju katalogu. Po protected D3 i istniejącym CW2 B1–B5 normalna native/executable promotion w CW3 ma obejmować **ograniczoną reprezentatywną partię albo całą technicznie i semantycznie gotową partycję rodziny**, nie kolejny pojedynczy item/creature/NPC. Pojedynczy rekord wolno użyć tylko jako fixture, regression, diagnostykę albo gdy świeży konkretny blocker uniemożliwia bezpieczny batch. Taki wyjątek nie liczy się jako postęp katalogowy i musi jawnie podać blocker oraz warunek przejścia do batcha. Historyczne single-item PR-y są bootstrap/evidence, nie precedensem do seryjnej promocji rekord-po-rekordzie.


## Item batch promotion

CW3 nie tworzy osobnej „promotion phase” jako celu. Dla tego samego Item batcha przyjmij dokładny eligible set z field-level evidence i continuity, zaktualizuj istniejące canonical `ProjectReferenceRecord::Item` / `ReferenceItemSemantics`, a następnie od razu wykonaj istniejący compile/test path.

Partial promotion jest poprawnym normalnym wynikiem: pojedyncze zweryfikowane pola mogą stać się `Known`, podczas gdy pozostałe pola tego samego Itemu pozostają `Unknown/Conflict`. Nie czekaj na kompletny Item, jeżeli typed model #749 już reprezentuje promowane pole.

Jeżeli `promotable_fields == 0`, nie twórz no-op promotion PR ani dodatkowego schema/rule wrappera; zwróć batch do najbliższego source/identity/continuity blockera. Jeżeli `promotable_fields > 0`, promotion musi zwiększyć liczbę canonical-promoted fields i ten sam batch powinien od razu przejść przez istniejący artifact v4 / compile-load qualification.

Nowy parser, model, codec, rule engine lub schema widening jest dopuszczalny tylko po wykazaniu, że co najmniej jedno dokładne eligible pole nie ma reprezentacji w chronionym modelu. Sam raport, manifest lub ponowne opakowanie tego samego semantic payload nie jest postępem katalogowym.

## Zadanie

Wprowadź tylko przyjętą semantic delta #504: typowane definicje, ordered placements i domain bindings wymagane przez wybrany journey, bez ograniczania pełnej architektury do fixture. Source keys są stabilne; runtime numeric IDs są revision-scoped. Podział plików i stref nie zmienia identity/authority. Missing footprint data nie jest void.

Użyj istniejących modułów i dojrzałych upstream parserów/serializerów. Wymagana strictness, limity, duplicates, units, reference families, cycles/expansion i client allowlist są walidacją Oteryn; brak jednego zachowania w API uzasadnia mały adapter, nie fork całej biblioteki. Nie wdrażaj permanentnego codec przed właściwą format decision.

Buduj deterministic closed release i server/client projections. Zachowaj indexed chunk access, integralność i version compatibility. Wyprowadzaj indeksy z jednego źródła; potrzebne lokalne palety/reuse chunków nie uzasadniają własnego CDN. Weryfikuj używane bajty przed publikacją. Decompression/allocation/work boundaries są skończone i sprawdzane przed drogą alokacją.

Wykorzystaj istniejący staging/activation i jego quiescence/authorization. Nie zmieniaj interpretacji first-production v1, nie mieszaj generacji i nie przemycaj durable migration. LKG bytes nie cofają danych graczy. Nie dodawaj niezależnego hot-reload managera.

## Akceptacja

Rzeczywiste source→compile→load przez obecny pipeline daje przyjętą semantykę; powtórny build jest deterministyczny; zmiana enumeracji lub shardowania nie psuje identity; negatywne referencje/bounds/corruption/version/client leakage są odrzucane. Zmierz wymagany random access i małą aktualizację na realnej partii, nie deklaruj pełnej skali po fixture. Wynik i adapter contract przekaż CW4/CW5 oraz CW6; zakończ successor footer.

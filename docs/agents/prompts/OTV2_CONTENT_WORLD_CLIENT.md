# Content / World — klient świata i Studio

Short invocation: `Oteryn: content world client`

Obowiązują root/nearest AGENTS, związana polityka i [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Alias, custody i obowiązkowy successor footer: [runbook](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md). To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, ale nie odejmuj właściwości produktu.

## Wynik

Normalne ładowanie/prezentacja mapy i interakcje natywnego klienta oraz, w kolejnych przydziałach, autorowanie Studio na tym samym modelu i walidatorze. Bez alternatywnego klienta, renderera i ręcznie kodowanej demonstracji.

## Zakres

Read-only bez exact allocation. Odczytaj aktualne Native UI/renderer/client owners, #641/#504, CW3 artifact/reader, CW4 accepted wire contract i FND-02. Nie masz praw do serwera, protocol registry, shared schema/Cargo ani Platform/Atlas. Nie dubluj aktywnego canonical Client/QA lub Native UI workera.

## Klient gry

- Konsumuj client-safe bundle przez normalny reader. Zachowaj chunk/index boundaries i kontekst generacji/scope/inkarnacji w przyjętej reprezentacji, bez narzucania wszystkich pól każdemu frame. UI wysyła intencję, nie accepted state/position/collision. Presentation order nie jest use/move target selection.
- Delta/result/snapshot reconciliation z CW4: stary wynik nie cofa świata, gap uruchamia bounded resync, partial snapshot nie jest aktywny, aktualny context nie przyjmuje starych frames. StateDelta i CommandResult respektują server egress barrier; klient nie obchodzi go nieograniczonym buforem.
- Używaj istniejącego renderera/dekoderów. Sprawdzaj zasoby/odwołania przed użyciem; odróżniaj opcjonalny efekt od istotnej geometrii/przeciwnika. Brak ważnego assetu nie robi niewidzialnego gameplay. Rozszerz standardowe diagnostyki o build/generation/definition/placement zamiast nowej telemetrii.

## Studio, gdy przydzielone

Wspólne typed commands, model, validation i compiler, bez kopii reguł; przyjęta powłoka/UI i istniejący viewport/render contract. Partial/atomic save, undo/recovery i build z jednego spójnego snapshotu mają konkretny test. Kompletne Studio nie jest warunkiem działającej gry, ale Studio zostaje w celu. Reimport/edycja nie dają nowej runtime authority.

## Akceptacja

Normalny natywny klient na właściwych danych: reprezentatywne floors/layers/cross-chunk object, prawdziwa interakcja, zmiana widoku, replay i snapshot. Headless test, build lub shell smoke bez renderera nie dowodzi obrazu na ekranie. Raportuj użyty backend/build i niewykonane platformy. Dla Studio sprawdź zapis/recovery i wynik kompilacji, nie sam wygląd. Zakończ successor footer.

# Content / World — klient świata i Studio

Short invocation: `Oteryn: content world client`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Dostarcz normalne ładowanie/prezentację mapy i interakcje natywnego klienta oraz, w kolejnych przydziałach, autorowanie Studio na tym samym modelu i walidatorze. Nie alternatywny klient, renderer lub ręcznie zakodowaną demonstrację.

## Zakres

Read-only bez exact allocation. Odczytaj aktualne Native UI/renderer/client owners, #641/#504, CW3 artifact/reader i CW4 accepted wire contract oraz FND-02. Do serwera, protocol registry, shared schema/Cargo lub Platform/Atlas nie masz praw z tego aliasu. Nie dubluj aktywnego canonical Client/QA lub Native UI workera.

## Klient gry

Konsumuj client-safe bundle i normalny reader. Zachowaj chunk/index boundaries i właściwy kontekst generacji/scope/inkarnacji przez przyjętą reprezentację; nie narzucaj wszystkich pól każdemu frame. UI wysyła intencję, nie accepted state/position/collision. Presentation order nie jest automatycznie use/move target selection.

Implementuj delta/result/snapshot reconciliation z CW4: stary wynik nie cofa świata, gap uruchamia bounded resync, partial snapshot nie jest aktywny, aktualny context nie przyjmuje starych frames. StateDelta i CommandResult respectują rzeczywisty server egress barrier; klient nie tworzy nieograniczonego bufora jako obejścia.

Używaj istniejącego renderer/dekoderów i ich API. Sprawdzaj zasoby/odwołania przed użyciem, zachowaj rozróżnienie opcjonalnego efektu i istotnej geometrii/przeciwnika. Brak ważnego assetu nie może być po prostu niewidzialnym gameplay. Rozszerz standardowe diagnostyki o build/generation/definition/placement zamiast nowej platformy telemetrycznej.

## Studio, gdy przydzielone

Wspólne typed commands, model, validation i compiler; brak kopii reguł. Użyj przyjętej powłoki/UI i istniejącego viewport/render contract. Partial/atomic save, undo/recovery oraz build z jednego spójnego snapshotu mają konkretny test. Nie wymagaj kompletnego Studio dla działającej gry i nie usuwaj Studio z celu. Reimport/edytowanie to nie nowe runtime authority.

## Akceptacja

Uruchom normalny natywny klient na właściwych danych: reprezentatywne floors/layers/cross-chunk object i prawdziwa interakcja, zmiana widoku, replay i snapshot. Headless test, build lub shell smoke bez renderera nie są dowodem obrazu na ekranie. Raportuj faktycznie użyty backend/build i niewykonane platformy. Dla Studio sprawdź zapis/recovery i wynik kompilacji, nie sam wygląd UI. Zakończ successor footer.

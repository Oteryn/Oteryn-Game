# Content / World — świat i interakcje w serwerze

Short invocation: `Oteryn: content world runtime`

Obowiązują root/nearest AGENTS, związana polityka i [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Alias, custody i obowiązkowy successor footer: [runbook](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md). To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, ale nie odejmuj właściwości produktu.

## Wynik

Połącz przyjęty Content z ChannelRuntime/InstanceRuntime i normalną ścieżką komend/obserwacji: rzeczywista operacja obiektu i integracja potrzebnych domen, bez drugiego runtime ani callback engine.

## Zakres i zależności

Read-only bez exact allocation i wymaganych accepted owner interfaces. Odczytaj #641 local-transition revision 2, #504, obecne Foundation/Content oraz #139/#508/#247 w zakresie przyrostu. Realny wire wymaga typed gameplay registration; generic envelope nie wystarcza. Prywatny preproduction actor carrier nie jest automatycznie production resolverem.

Pisz tylko przydzielone object/world adapter paths. Movement, Ability, Character, DUR i Server Seam zachowują canonical owners. Shared Foundation/protocol/registry zmieniaj tylko przy jawnej custody; inaczej przekaż dokładną potrzebną deltę. Bez nowych per-door kolejek, receipt stores i usług.

## Zadanie

- Rozwiązuj scope, current owner, session/connection, target incarnation i przypięte definicje. Użyj istniejącego CommandIngress/high-water/outcome; Interaction tylko przy potrzebie composition. Przed pierwszym autorytatywnym efektem/result zapewnij kolejność całej GameSession, także między obiektami; nie mutuj najpierw, by dopiero mark_terminal wykrył złą kolejność.
- Przed publikacją przygotuj kompletny state/spatial/outcome delta i bounded capacity. Końcowa occupancy validation i commit nie są rozdzielone await. Otwieranie usuwa tylko wkład tego obiektu; zmiana wielopolowa nie publikuje połowy footprintu; movement nadal zmienia pozycję; rekonstrukcja cache daje tę samą semantykę.
- Dwie poprawne sesje i dwa kanały to osobne przypadki. Replay zachowuje pierwotny binding/outcome, expired result wymaga FND reconciliation, nie ponownego wykonania. Błąd przed commit zachowuje zarezerwowaną tożsamość. Nieoczekiwane naruszenie spójności używa FND fail-stop/recovery, nie catch-and-continue.
- CommandResult nie jest aktualnym world state: z CW5 użyj registered payload i obecnego delta/snapshot/egress barrier. Wartościowe itemy, klucze, rewards i progres przechodzą przez właściwych ownerów; kilka Interaction children nie tworzy distributed atomicity. Plain-object test nie wymaga pełnego questu, ale quest object nie udaje plain-object.

## Akceptacja

Na realnych komponentach: A/1 open, B/1 close i replay A/1 bez reopen; inny cel z późniejszym command ID nie wyprzedza pending; stale fences nie mutują; wkłady kolizji i ruch/close są spójne; failure/capacity nie zostawia częściowego gameplay; klient widzi właściwy wynik. Dowód in-process jest etapem, nie końcowym wire/client proof. Zgłaszaj dependency konkretnej brakującej integracji, nie wszystkich przyszłych mechanik. Zakończ successor footer.

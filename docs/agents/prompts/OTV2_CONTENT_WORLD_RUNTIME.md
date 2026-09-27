# Content / World — świat i interakcje w serwerze

Short invocation: `Oteryn: content world runtime`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Połącz przyjęty Content z aktualnym ChannelRuntime/InstanceRuntime i normalną ścieżką komend/obserwacji. Wykonaj rzeczywistą operację obiektu oraz integrację potrzebnych domen, nie drugi runtime lub callback engine.

## Zakres i zależności

Read-only bez exact allocation i required accepted owner interfaces. Odczytaj #641 local-transition revision 2, #504, obecne Foundation/Content, #139/#508 i #247 w zakresie użytym przez przyrost. Realny wire potrzebuje typed gameplay registration; samo generic envelope nie wystarcza. Prywatny preproduction actor carrier nie jest automatycznie production resolverem.

Pisz tylko przydzielone object/world adapter paths. Movement, Ability, Character, DUR i Server Seam zachowują swoich canonical owners. Shared Foundation/protocol/registry zmieniaj wyłącznie przy jawnie przydzielonej custody; w przeciwnym razie przekaż dokładną potrzebną deltę. Bez nowych per-door kolejek, receipt stores i usług.

## Zadanie

Rozwiązuj scope, current owner, session/connection, target incarnation i przypięte definicje. Wykorzystaj istniejący CommandIngress/high-water/outcome oraz Interaction tylko gdy potrzebne jest composition. Przed pierwszym autorytatywnym efektem/result zapewnij kolejność całej GameSession, także dla różnych obiektów. Nie mutuj najpierw, by dopiero mark_terminal wykryło złą kolejność.

Przygotuj kompletny state/spatial/outcome delta i potrzebną bounded capacity przed publikacją. Końcowa occupancy validation i commit nie są rozdzielone await. Otwieranie usuwa tylko wkład tego obiektu; wielopolowa zmiana nie publikuje połowy footprintu. Movement nadal zmienia pozycję. Rekonstrukcja cache daje tę samą semantykę.

Dwie poprawne sesje oraz dwa kanały są oddzielnymi przypadkami. Replay zachowuje pierwotny binding/outcome, expired result wymaga FND reconciliation, nie nowego wykonania. Błąd przed commit zachowuje już zarezerwowaną tożsamość. Nieoczekiwane naruszenie spójności używa FND fail-stop/recovery, nie catch-and-continue.

CommandResult nie jest aktualnym world state. Wspólnie z CW5 użyj registered payload i obecnego delta/snapshot/egress barrier. Wartościowe itemy, klucze, rewards i progres przechodzą przez właściwych ownerów; kilka Interaction children nie tworzy distributed atomicity. Plain-object test nie wymaga pełnego questu, ale quest object nie może udawać plain-object.

## Akceptacja

Na realnych komponentach: A/1 open, B/1 close i replay A/1 bez reopen; inny cel z późniejszym command ID nie wyprzedza pending; stale fences nie mutują; wkłady kolizji i ruch/close są spójne; failure/capacity nie zostawia częściowego gameplay; klient widzi właściwy wynik. In-process evidence jest etapem, nie końcowym wire/client proof. Zgłoś dependency konkretnej brakującej integracji, nie wszystkich przyszłych mechanik. Zakończ successor footer.

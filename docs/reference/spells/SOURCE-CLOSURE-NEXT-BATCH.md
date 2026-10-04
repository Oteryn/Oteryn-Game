# Kontynuacja lokalna: r46 i następne

Kontynuacja planu `BLOCKED-173-PLAN.md`. Liczba 173 opisuje historyczne blokady
projekcji danych, a nie liczbę brakujących funkcji obecnego serwera.
Aktualna kolejka danych `current-source-gap-worklist.json` zawiera osiem referencji:
dwa usunięte Sap Strength, dwa wycofane Expose Weakness i cztery wyłączone przykłady.
Po r59–r62 indeks ma 475 kandydatów danych; pozostałe 121 modeli private source-complete v2
wymagają przyjęcia rozszerzeń kontraktu i implementacji konsumentów, co zachowuje
`source-private-consumer-worklist.json`. Historyczne listy 173/162/129 nie opisują już
bieżących braków projekcji danych. To nie deklaracja domknięcia runtime.

## Sprawdzone istniejące prace

Odczyt 2026-10-04: zdalny main
`eaa401001d5a91cc1cb62385223b6ad4d23111ac`, otwarty PR #1534
`b8597f8579bb09eed600f3a2f1238be392fe3f59`.
Źródła sprawdzono przez zwykłe GitHub API, bez Remote Desktop.
Lokalny `/workspace/Oteryn-Game` jest innym, zmodyfikowanym worktree i nie stanowi
dowodu aktualnej zawartości tych rewizji.

PR #1534 zawiera już `apps/game-server/src/spell/native_house_movement.rs`
oraz `world_execution.rs`: plany Levitate i Magic Rope, kontrole kafli,
kolejność sprawdzania pozycji i efekty. `preflight_relocation` zgłasza
`MissingWorldTileEligibility`; wymagane fakty świata i uprawnienia wejścia
nadal blokują pełne wykonanie. Obecność planera nie kwalifikuje wykonania ruchu.

Main ma podstawowe reguły w `spell/target.rs` i wybór siebie przy leczeniu
w `spell/plan.rs`. PR ma też `native_actor_states.rs`. W tej pracy nie powstaje
drugi executor ruchu, stances, Premium ani pacing. Podłączenie pozostaje
u właścicieli runtime w #1534 i koordynacji #1622.

## Wykonywane zadania danych

- r46: dokładny dowód zgodności helperów Canary Levitate i Magic Rope oraz
  dwa dodatki do istniejących schemów. Nie zmieniać historycznych r28–r45.
- r47: warunki rzucania, stan i wybór celu. Sprawdzać pełne funkcje, ponieważ
  nazwa „prosty guard” może ukrywać Leiden, drugi Combat lub Shared Conservation.
- r48: Forked Glacier, Forked Thorns i Sweeping Takedown — kolejność Combatów,
  losowanie, callbacki i czyszczenie stanu tymczasowego.

R46 został zmaterializowany i niezależnie sprawdzony: dwa nowe kandydaty danych,
16 testów producenta i importera, zgodność wszystkich kopii i sum kontrolnych.
Stan po r46: 483 warianty, 312 zgodnych ze schemami, 171 BLOCKED.
Stan po r52/r53: 321 kandydatów i 162 BLOCKED.
Historyczny stan po r55–r58: 354 kandydatów i 129 BLOCKED, w tym 6 referencji
i 123 ówczesne warianty gameplay do uzupełnienia. Aktualny stan po r59–r62:
475 kandydatów danych i 8 jawnych referencji; zero pozostałych braków danych w tej
129-elementowej kohorcie, lecz 121 oczekujących kontraktów i konsumentów runtime.
To nie liczba czarów zakwalifikowanych do wykonania na serwerze.

Przegląd r47 i r48 potwierdził 17 rekordów częściowych i zero pełnych kandydatów.
Oba importy są już zmaterializowane i niezależnie sprawdzone; wszystkie 36 testów
nowego etapu przeszły razem w jednym procesie.
Find Person zachowuje źródłowe progi donorów `[5,101,275]` w dowodach.
Przyjęta P7 jawnie ustanawia docelowe pasmo 251. Osobne bindings nadal nie
podnoszą źródłowych receiptów: wymagają przyjętej polityki kosztów, poziomów
i tożsamości całego kwalifikowanego profilu. Cancel Magic Shield usuwa warunek przed Combat.
Crystal Nature's Embrace ma dodatkowe leczenie niezależne od wyniku pierwszego
Combatu. Forked Glacier/Thorns w aktualnym Crystal mają jeden Combat z chain
7/6 i dodatkiem Wheel, bez losowania. Sweeping Takedown zachowuje dwa Combaty
i czyszczenie cache po ich wywołaniu. Starszą klasyfikację traktować jako
historyczny punkt startowy, nie aktualny opis tych funkcji.

Dowód składni i częściowy model nie zmieniają statusu pełnego Spell.
Nowe kompletne pakiety muszą przejść walidację schemów, importu i niezależny
przegląd przed aktualizacją indeksu. Dane pozostają lokalne i nieaktywne;
brak PR, commitów, pushów oraz zmian manifestu serwera.

## Zamknięty etap r52–r54

Trzy importy są lokalnie zmaterializowane. R52 dodaje 3 standardowe pakiety
i osobno 3 native templates / 4 niezatwierdzone bindings; r53 dodaje 6 pakietów
Monka zgodnych z S5/S16; r54 dodaje 10 kompletnych projekcji danych slotów
potworów, 62 częściowe i 103 bez bezpiecznej projekcji. Wszystkie flagi runtime
i aktywacji pozostają false. Dokładna pozostała kolejka to
`current-source-gap-worklist.json` i `monster-target-projection-review-index.json`.

Aktualny wynik audytu wszystkich162 i33 domkniętych projekcji:
`UNBLOCK-162-RESULTS.md` / `unblocking-162-review-index.json`.


## Aktualny wynik danych r59–r62

`FINAL-129-DATA-COMPLETION.md` i `final-129-completion-review-index.json` opisują
121 nowych kandydatów prywatnych oraz osiem referencji. Historyczny
`unblocking-162-review-index.json` pozostaje identyczny bajtowo: 33 dodatki i 129
ówczesnych blokad. Źródła są lokalne i przypięte; nie deklarujemy nowej weryfikacji
Wiki. Przyjęte wcześniej fakty canonical Wiki pozostają zachowane. Nie zmieniono
wyboru donora, aktywnego katalogu ani flag native/runtime.

# Plan odblokowania 173 wariantów czarów

Stan bazowy: 483 warianty graczy, 310 pakietów zgodnych ze schemami, 173 BLOCKED.
Źródło: `source-closure-review-index.json`; dokładna lista, nazwy, klucze,
przyczyny i właściciele zadań: `blocked-173-worklist.json`.
To plan pracy, bez zgody na aktywację, zmianę canonical selection lub publikację.
Praca pozostaje lokalna. Plan dotyczy graczy; 175 nierozwiązanych slotów potworów
jest osobnym zbiorem.

## Cel i zasady

Doprowadzić 169 wariantów gameplayowych do kompletnego importu danych, a następnie
kwalifikowanego wykonania na serwerze testowym. Cztery wyłączone przykłady zachować
jako materiał źródłowy, poza aktywnym katalogiem. Nie zerować liczby BLOCKED przez
zmianę etykiet lub pomijanie mechaniki.

Kolejność: lokalne Canary/Crystal → kompletna reprezentacja danych i wymagane
schemy → zgodne wykonanie → zewnętrzne uzupełnienia. Zachować odrębne warianty
obu donorów. Wspólna implementacja może obsłużyć różne parametry, ale nie może
zacierać różnic źródłowych. Real-Tibia 100% nie jest warunkiem zakończenia: brak
nieopublikowanego szczegółu wiki może pozostać oznaczony, jeśli podstawowy przebieg
czaru jest zaimplementowany i przetestowany. Znany brak wymaganej mechaniki nadal
blokuje wykonanie danego wariantu.

## Pełny podział kolejki

Klasyfikacja jest rozłączna: każda rejestracja występuje raz, suma = 173.
Zależności nakładają się: np. Wheel występuje także w części Monka i summonów.
Liczby oznaczają warianty donorów, nie unikalne nazwy czarów.

| Grupa główna | Warianty | Zakres |
| --- | ---: | --- |
| Pozostałe cast/guard/callback | 23 | Cele domyślne, ograniczenia casterów, stan, callbacki, sekwencje |
| Formuły C++ | 6 | Double Jab, Flurry, Uppercut, Repulse i pokrewne |
| Paralyze — wynik Lua | 2 | Rozbieżność kontraktu cast z health/condition result |
| Wheel | 36 | Grade, unlock, modyfikacje obszaru/cooldownu, avatars |
| Stances | 27 | Toggle, buffs, usuwanie warunków, wybór elementu/Combatu |
| Monk / wyposażenie | 25 | Harmony, Virtue, broń, elemental bond |
| Party | 11 | Skład, zasięg, koszty i cele grupowe |
| Summons / familiar | 17 | Tworzenie, limity, corpse, warunki i propagacja |
| Świat / cel | 11 | Tile, spectators, boss guard, target queries |
| Custom/P4 | 10 | Magic Wall, Wild Growth, Challenge, inne custom |
| Chain | 1 | Crystal Lightning — wartości i wybór kolejnych celów |
| Wyłączone przykłady | 4 | Test / test rune; zachować, nie aktywować |

## Kolejność wykonania

### 0. Przygotowanie bez ponownego pobierania

Koordynator utrzymuje jedną listę 173 rejestracji. Dla każdego pakietu zapisujemy
źródłowe SHA, zależności, brakującą reprezentację, stan implementacji i następny
krok. Najpierw sprawdzamy istniejące implementacje i testy — sama stara przyczyna
BLOCKED nie dowodzi, że funkcja nie istnieje dziś na serwerze.

Ponownie wykorzystujemy: r28 headers/receipts, pełne AST r38, condition data
r31/r33, trzy programy Monka r35, Swift Foot r32, Paralyze r45 oraz wiki r40.
Źródła są przypięte w SOURCE-CLOSURE-LOCAL.md. Nie pobieramy ponownie repozytoriów
ani wiki. Nowy research dotyczy tylko konkretnej brakującej informacji po
zamknięciu odpowiedniego przebiegu Canary/Crystal.

### A. Najpierw importer, proste warunki i znane kontrakty

Worker 1 i ja sprawdzamy 23 pozostałe casty w mniejszych wspólnych grupach:

- 2 aktualne helpery Canary: Levitate i Magic Rope — porównać faktyczną closure,
  nie blokować wyłącznie zmianą SHA całego global.lua.
- 3 callbacki Monka: Devastating Knockout, Greater Tiger Clash, Tiger Clash —
  wykorzystać r35; dalsza implementacja zależy od pracy workera Monka.
- 2 sekwencje multi-Combat: Forked Glacier i Forked Thorns — zachować kolejność,
  prawdopodobieństwa i wyniki poszczególnych wykonań.
- 14 wariantów prostych warunków, parametrów, celu domyślnego i własnego stanu —
  najpierw ustalić, które obsługuje istniejący model/server; resztę skierować do
  konkretnego ownera mechaniki.
- 2 wyjątki: Sweeping Takedown i Crystal Heal Friend — cache cleanup/sekwencja
  oraz shared-conservation helper; nie uznawać ich za zwykły pojedynczy Combat.

Architekt #162 rozstrzyga i poprawia kontrakt Paralyze. Worker korzysta z r45:
6000 ms, dokładne współczynniki, source zero-health path i wynik Lua. NUMBER bridge
nie propaguje wyniku doCombat. Implementacja musi zachować tę różnicę; nie wolno
zastąpić UNDEFINEDDAMAGE przez COMBAT_NONE. Do kwalifikacji oba warianty pozostają
BLOCKED.

### B. Wspólne wejścia i formuły Monka

Worker 3 przygotowuje jeden model wymaganych Harmony/Virtue i danych wyposażenia,
po sprawdzeniu obecnych systemów. Obsługuje 25 wariantów Monka/wyposażenia oraz
6 formuł C++, z zależnością od 3 callbacków z fazy A.

Zachować typy i kolejność clamp/round/conversion, zużycie i aktualizację stanu,
wybór Combatu/elementu oraz success/failure. Dane istnieją; wdrożyć tylko brakujące
adaptery i mechaniki. Nie zastępować funkcji C++ przybliżoną krzywą bez oznaczenia.

### C. Stany, Wheel, party i summons równolegle

Worker 2: 27 stances oraz 36 głównych wariantów Wheel. Najpierw stan i toggle,
potem wybór Combatu, grade/unlock, obszar i cooldown. Wykorzystać istniejący Wheel,
jeśli jest dostępny; nie budować drugiego systemu. Stan Wheel jest także zależnością
części czarów z innych grup.

Worker 4: 11 party i 17 summon/familiar. Najpierw wspólne zapytania party/owner,
limity i wybór celów; następnie summon lifecycle, corpse, kopiowanie warunków i
Swift Foot. Sprawdzić leader inclusion, zasięg, death/despawn, koszty i wygaśnięcie.

### D. Mechaniki świata i złożone ataki

Worker 5: 11 world/target, 10 custom/P4 i 1 chain. Zastosować istniejące tile,
collision, spectators i combat rules. Magic Wall/Wild Growth wymagają rzeczywistej
zmiany świata i czasu życia; nie wystarczy efekt wizualny. Challenge/taunt i boss
guards zachowują dozwolone cele. Lightning wymaga pickera, kolejności, limitów
oraz wartości. Delayed/repeated callbacks z innych grup realizować przez obecny
scheduler, z obsługą zniknięcia celu/castera i końca sesji.

## Podział odpowiedzialności i integracja

- Ja: lokalny backlog, dopasowanie danych i authoring schemas, poprawki importera,
  source proofs, effects/sound references, scalanie oraz aktualizacja indeksu.
- Architekt/koordynator #162: zaakceptowane kontrakty wykonania, ownerzy systemów,
  kolejność zależności, canonical selection, podłączenie konsumenta i aktywacja.
- Workerzy: konkretne mechaniki według powyższych grup, w rozłącznych plikach.
- Niezależny reviewer: finalny pakiet danej mechaniki, w tym rozbieżności donorów,
  branch order, nazwy assetów i źródłowe namespace; nie osobny review każdego czaru.

Maksymalnie root + pięciu workerów + reviewer. Workerzy przygotowują tylko zakres
przydzielony przez koordynatora. Wspólne kontrakty i pliki mają jednego właściciela.
Każda gotowa grupa przechodzi do integracji bez czekania na wszystkie 169.
Wcześniejszych sealed imports nie nadpisujemy; nowe overlaye są addytywne.

## Jedno istniejące środowisko testowe

Sprawdzić i wykorzystać istniejące scenariusze `content/test-packs/spells/r25/sidecars/
player-spell-scenarios.json` oraz kwalifikację `docs/reference/spells/r22-audit/
test-environment/`. Wykorzystać znalezioną mapę Thalom i aktualny profil testowy;
historyczna kwalifikacja nie dowodzi dostępności dzisiejszego serwera.
Nie tworzyć osobnego fake runtime ani drugiej mapy/serwera dla każdego czaru.

Dla wspólnej mechaniki: podstawowe wykonanie, odmowa, koszty/soul/charges/cooldown,
cel/zasięg/PZ, status/effect timing, śmierć lub zniknięcie celu, koniec/ponowne
uruchomienie sesji tam, gdzie dotyczy. Parametry i różnice donorów sprawdzamy
przy wszystkich wariantach; szersze replaye grupujemy według rzeczywistej wspólnej
implementacji. Efekty/dźwięki testujemy przy istniejącym renderer/audio bridge,
bez uznawania samych nazw source refs za gotowe native bindings.

## Definicja zakończenia

Osobno odnotowujemy: pełne dane źródłowe, kompletny pakiet schema, działające
wykonanie na testowym runtime oraz znane braki zewnętrznych danych. Schema PASS
nie wystarcza do deklaracji działającego czaru.

Zakończenie kolejki: każdy z 169 wariantów ma kompletny pakiet danych i potwierdzone
podstawowe wykonanie albo konkretny, przypisany ownerowi brak implementacji;
4 przykłady są jawnie wyłączone. Lista przypisanych braków oznacza zakończenie
triage, nie zakończenie implementacji. Pełne zakończenie gameplay oznacza działanie
wszystkich 169 zamierzonych wariantów lub osobną, jawną decyzję o wyłączeniu danego
wariantu z katalogu. Szczegóły nieosiągalnej zgodności z real Tibia pozostają
oflagowane do późniejszego uzupełnienia.

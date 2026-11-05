# Soulpit — przygotowane poprawki i granica wykonania

Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Stan zastany: `Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Inwentaryzacja obejmuje 24 elementy, w tym istniejące reguły walki i konwersji; ten pakiet poprawia tylko trzy przejścia lobby i trigger wyjścia areny. Nie tworzy drugiego silnika Soulpit.

## Wykonane powiązania

| Element | Źródło | Zmiana |
| --- | --- | --- |
| Dwa wejścia do lobby | `data-global/scripts/actions/soulpit/soulpit_entrance.lua:4–8,17–38` | StepIn na `(32349,31030,3)` albo `(32350,31030,3)` wybiera `(32374,31171,8)` przez istniejący właściciel ruchu. |
| Powrót z lobby | `data-global/scripts/actions/soulpit/soulpit_entrance.lua:10–14,40–57` | StepIn na `(32374,31173,8)` wybiera `(32349,31032,3)`. |
| Wyjście z areny | `data-global/scripts/actions/soulpit/soulpit_arena_exit.lua:8–16` | Action/USE na `(32375,31153,8)`; usunięto błędne uruchamianie z samego StepIn. Dodatni powrót nadal należy do wymaganego wspólnego Instance handoff. |

Powyższe ścieżki korzystają z `crates/gameplay/domain/src/domain/soulpit/passages.rs`, `crates/gameplay/dispatch/src/gameplay_transport/soulpit/passages.rs`, rzeczywistego `paced_movement` i `composed_admission/objects`. Potwierdzenie źródłowego pola, obiektu, aktualnego aktora, sesji i revizji mapy pozostaje przy istniejących właścicielach. Brak kwalifikowanego pola lub właściciela powrotu skutkuje odmową. Zmiana współrzędnych między source z i native floor jest jawna w regule; nie nadano nowego WorldId ani Item ID.

## Left out

| Element | Konkretny powód | Źródło i decyzja |
| --- | --- | --- |
| Dodatnie wejście grupy, pełna walka oraz powrót Instance | Brak zaakceptowanej implementacji i fixture `BossActivityHandoff`. Trait i slot nie nadają prawa do nowego destination Instance. W tej partii nie wolno rozszerzać właściciela. | Donor `data-global/scripts/quests/soulpit/soulpit_fight.lua:65–84,100–109`; Oteryn `crates/gameplay/dispatch/src/gameplay_transport/soulpit/entry.rs:229–238`, `soulpit/handoff.rs:38–82`, `soulpit/runtime.rs:90–116`; [QUESTION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589185913), [ANSWER §5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Dwa testy PG atomowości ukończenia/Animus | Wymagają legalnej fixture destination Instance i nie zostały uruchomione. Nie usunięto ani nie osłabiono istniejących testów. | Oteryn `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soulpit_lifecycle/completion_atomicity.rs:36,42`; [wcześniejszy ANSWER](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6543794006), [bieżący QUESTION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589185913). |
| Niezakwalifikowane Item/Creature wyników Soulpit | Istniejące holds zachowane; nie filtrowano puli losowania, nie podmieniono niezakwalifikowanego wyniku na inny. Wybrany brakujący wynik ma odmówić przed debitem. | Donor `data-global/lib/others/soulpit.lua:1–30,143–180`; Oteryn `crates/gameplay/domain/src/domain/soulpit/items.rs`; [Item ANSWER](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6542983549), [Creature ANSWER](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6543121421), [Purple Butterfly](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6543672719), [bieżący §8/9](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Ostrzeżenia i ikony stacków w kliencie | Wymagają wspólnego presentation ownera. Ten pakiet nie dodaje komendy/capability ani nie deklaruje ich widoczności. | Donor `data-global/scripts/quests/soulpit/soulpit_creatureevents.lua`, `data-global/lib/others/soulpit.lua`; [ANSWER prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6543504120), [bieżący §5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Porównanie z Tibia Global | API MediaWiki HTTP403. Bieżący revid i różnice pozostają NIEZNANE. Nie zbudowano niczego tylko z Global. | [QUESTION API](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099), [ANSWER §2](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |

Zachowano wcześniejsze rozstrzygnięcia właściciela: canonical Blue Butterfly oraz pojedynczą natywną aplikację przyjętego buffu Hex50%. To świadome decyzje Oteryn, nie dowód aktualnego Global: [Blue Butterfly](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6542983549), [Hex](https://github.com/Oteryn/Oteryn-Game/pull/2768#issuecomment-6543595950).

## Zweryfikowane wyniki i dalsza bramka

Root uruchomił na komputerze właściciela cztery testy domeny przejść: RED dał3FAIL/1PASS, następnie GREEN całego `domain::soulpit` dał37/37PASS. Potwierdza to rozdzielenie USE/StepIn, dokładne wejścia/wyjście lobby oraz odmowę innego triggera/pola/piętra. To wynik przyrostowej kopii roboczej, nie deklaracja finalnego przetestowanego SHA PR.

W kompilacji dispatch wykryto brak importu `MapFacts`; poprawka jest w końcowym pliku. Bounded review rzeczywistego callera dodał porównanie obu content pinów oraz ponowne pełne resolve_map_target po await na lane pod runtime lock, aby usunięty/ukryty cel nie zachował wczesnego dowodu. Komentarz istniejącego traitu poprawiono z STEP na USE bez zmiany sygnatury. Końcowy wynik pełnego fmt/clippy/test dispatch oraz ewentualnych kwalifikowanych testów PG uzupełnia root. Nie deklaruje się PASS testów niewykonanych ani pełnej grywalności areny.

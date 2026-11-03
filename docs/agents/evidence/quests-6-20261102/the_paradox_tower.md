# Quests 6 — the_paradox_tower

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **31 elementów; AUTO 5, RĘCZNIE 26**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Ustalono 31 elementów oraz istniejący callback czterech skulls. Zachowano istniejącego wykonawcę i źródłowy test PG; ta partia nie deklaruje wyniku PG Paradox. Zakres wykonanych kontroli opisuje S8; nie dodano drugiej tożsamości ani handlera.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L16](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589927833). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- UID25011..25037 nie odnaleziono w przeskanowanym world base; produkcyjne powiązania tych UID i skrzyń pozostają NIEZNANE. Dwóch grup UID i pozycji carvings/exit nie sparowano arbitralnie: donor sprawdza przynależność do obu zbiorów.
- NPC Oldrak/Zoltan/Padreia/Lubo/Hugo/Prisoner/Riddler, pytania, Mathemagics, routing nagród i zmienne sceny wymagają konkretnych placementów i dopuszczonego właściciela. Nie wybierano poprawnych odpowiedzi ani dialogów z pamięci.
- Puzzle owoców/szachów/ghoula, schody, pułapka drzewa i timery wymagają wspólnych world/action contracts, których ANSWER5 nie pozwala rozszerzyć. Pierwotny symbol FirstParadoxAcess pozostaje jawnie rozróżniony od kanonicznego tracku; brak arbitralnego aliasu.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_paradox_tower/movement_sacrifice_skulls.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_paradox_tower/movement_sacrifice_skulls.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_paradox_tower/movement_strange_carvings_in _the_southeast.lua:9](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_paradox_tower/movement_strange_carvings_in%20_the_southeast.lua#L9)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_paradox_tower/move_let_the _quest.lua:10](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_paradox_tower/move_let_the%20_quest.lua#L10)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/riddler.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/riddler.lua#L1)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

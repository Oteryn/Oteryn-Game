# Quests 6 — the_dream_courts

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **153 elementów; AUTO 0, RĘCZNIE 153**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Uzupełniono inwentaryzację źródeł o zależności NPC, fazy bossów, spawn i startup AID. Raport zawiera oba suplementy szczegółowe; ten quest nie otrzymał nowego adaptera runtime.
- Nie importuje się read-only guardów przez set=stała, aby nie cofać progresu. Nie dodano sztucznego storage, nowych fieldów ani drugiej tożsamości questa.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L14](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590084232). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Przejścia Dream Courts z odczytowymi guardami, kierunkiem, FX lub tekstem sukcesu wymagają pól odłożonych przez Q6-C01/ANSWER6589908362. Pozostałe kandydaty nie mają zakwalifikowanych fizycznych AID/UID. Nie dodano sztucznego Set zamiast odczytu; brak tych bindingów pozostaje Left out zgodnie z ANSWER7.
- Brak kwalifikowanych AID23100/23103/23101 i pełnych runtime NPC placements. Same142obecne tiles nie dowodzą binding. ANSWER7: brak tagów/obiektów/binding pozostaje Left out.
- Boss entry, Faceless Bane phase fight, Nightmare curse, DreamScar timers/spawns, sekwencje i złożone nagrody wymagają własnych podłączonych adapterów; ANSWER5 ogranicza rozszerzenia wspólnych właścicieli. Nie wdrożono samych reducerów.
- Undefined Rosebush vs RoseBush, DreamScarAccess, konflikt dwóch Nightmare StepIn oraz zależność custom quest doors od startup AID pozostają dokładnie opisane w raporcie/QUESTION; brak cichej naprawy reguły.
- Andrew/Undal/Vanys/Myzzi i pozostałe NPC: odczytane dialogi nie są działającym adapterem. Global-only dodatki pozostają poza zakresem bez ANSWER.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_courts_entrance.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/movements_courts_entrance.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_access_teleports.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/movements_access_teleports.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_quest_doors.lua:97](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/actions_quest_doors.lua#L97)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_quest_doors.lua:150](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/actions_quest_doors.lua#L150)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/door_quest.lua:846](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/door_quest.lua#L846)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/andrew_lyze.lua:121](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/andrew_lyze.lua#L121)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/andrew_lyze.lua:151](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/andrew_lyze.lua#L151)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/undal.lua:64](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/undal.lua#L64)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vanys.lua:64](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/vanys.lua#L64)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/creaturescripts_faceless_bane.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_dream_courts/creaturescripts_faceless_bane.lua#L1)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/quests/src/quest/world_callbacks.rs:84](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/quests/src/quest/world_callbacks.rs#L84)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

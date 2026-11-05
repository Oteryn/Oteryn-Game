# Quests 6 — the_rookie_guard

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **110 elementów; AUTO 14, RĘCZNIE 96**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dodano13 callbacków w istniejącym formacie. Ich UID/AID nie występują w przeskanowanym world base; to dane wymagające powiązań, a nie13 grywalnych kroków.
- Przejrzano pięć pełnych plików NPC; w czterech (Vascalir/Lily/Hyacinth/Tom) wyodrębniono125 węzłów, w tym36 mutujących. Paulie był przejrzany osobno. Materiał roboczy nie jest publikowany ani ładowany jako dialog runtime.
- Test rzeczywistego loadera/evaluatora sprawdza przejście7→8 i odmowę na etapach6/8. Osobno weryfikuje deklarację zużycia jednego i12784 i nagrody50XP; nie wykonuje transakcji ekwipunku ani przyznania XP.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **13**.

| Klucz | Wyzwalacz i selektor | Skutek | Granica dowodu |
| --- | --- | --- | --- |
| `oteryn:quest-callback/rookie-wolf-found` | `step_in`; `{"unique":25025}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission06","minimum":2,"maximum":2,"set":3}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-wolf-sprint-exit` | `step_in`; `{"unique":25027}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission06","minimum":5,"maximum":5,"set":6}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-mine-enter` | `step_in`; `{"action":50344}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission09","minimum":1,"maximum":1,"set":2}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-mine-exit` | `step_in`; `{"action":50345}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission09","minimum":7,"maximum":7,"set":8}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-wasp-lair` | `step_in`; `{"action":50353}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission11","minimum":1,"maximum":1,"set":2}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-fortress-water-found` | `step_in`; `{"action":50359}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission12","minimum":6,"maximum":6,"set":7}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40058` | `step_in`; `{"unique":40058}` | `{"teleport":{"x":31977,"y":32174,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40059` | `step_in`; `{"unique":40059}` | `{"teleport":{"x":31964,"y":32174,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40060` | `step_in`; `{"unique":40060}` | `{"teleport":{"x":31958,"y":32184,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40061` | `step_in`; `{"unique":40061}` | `{"teleport":{"x":31955,"y":32187,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40062` | `step_in`; `{"unique":40062}` | `{"teleport":{"x":31970,"y":32183,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-energy-barrier-40063` | `step_in`; `{"unique":40063}` | `{"teleport":{"x":31954,"y":32174,"floor":-10}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/rookie-poisoned-water` | `use`; `{"action":40012}` | `{"storage":[{"track":"oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission12","minimum":7,"maximum":7,"set":8}],"item_on_target":{"item":"oteryn:item.tibia.i12784","count":1,"replacement":null,"target_replacement":null},"experience":50}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |

Źródła nowych rekordów:

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:81](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua#L81)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:104](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua#L104)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission09_rock_troll.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission09_rock_troll.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission11_sweet_poison.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission11_sweet_poison.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:386](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua#L386)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:307](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua#L307)

## Left out

[Q6-L18](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589928755). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Brak fizycznych powiązań UID/AID wszystkich13 callbacków w przeskanowanym world base.
- Wymiany NPC obejmujące item/XP/state, premia bankowa Pauliego, nagrody z wieloma przedmiotami i reset13 torów wymagają rozszerzeń wspólnych właścicieli zakazanych przez ANSWER5. Nie rozbito atomowego startu ani resetu.
- Kwalifikacja natywnych przedmiotów dla całego grafu nagród pozostaje niepełna. i12784 po regeneracji ma rzeczywisty profil materializable=true, NonStackable, CharacterInventory+Ground; to nie kwalifikuje pozostałych nagród ani mapbindingu tego kroku.
- Samouczki, party credit dla szczurów, timery, disguise, pokój bossa, chest bits i13 znaczników mapy pozostają bez podłączonego wykonawcy.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:307](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua#L307)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:386](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua#L386)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vascalir.lua:212](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/vascalir.lua#L212)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vascalir.lua:915](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/vascalir.lua#L915)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/lily.lua:121](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/lily.lua#L121)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/hyacinth.lua:78](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/hyacinth.lua#L78)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/tom.lua:117](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tom.lua#L117)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/paulie.lua:161](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/paulie.lua#L161)
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280>

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

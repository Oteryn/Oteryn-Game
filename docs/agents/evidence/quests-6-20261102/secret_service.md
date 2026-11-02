# Quests 6 — secret_service

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **113 elementów; AUTO 1, RĘCZNIE 112**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Ustalono source profile113 wierszy obejmujący3 frakcje, NPC, kroki use/step/death oraz współdzielone chest/rotten-tree/crowbar. Dane i storage writers jawnie odróżniono od native obsługi dialogu.
- Przygotowano niewykonywalny kandydat/test authoring ring349: targetAID12563, TBIMission05==1→2,consume1. Nie opublikowano produkcyjnego JSON, ponieważ rzeczywisty emitted native item artifact nie zawiera i349.
- Nie dodano nowego działającego adaptera Secret Service ani nie oznaczono zadań frakcji jako ukończonych.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L11](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589926009). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Ring349 nie ma profilu w emitted content/items/definitions/spell-native-profiles.json (pełny lookup root HEAD9d2437e2c109ec6abb413ea0104906a85948dd16 po production_binding.target.key). Legacy items-21000-21499.json: materializable=false, stack_class Unknown. Bez nowego kwalifikowanego profilu nie importujemy użycia ani nie zastępujemy itemu (§8).
- Trzy wykluczające się ścieżki TBI/CGB/AVIN, rozmowy i placementy Chester Kahs/Emma/Uncle oraz questowe gałęzie Gamel/Karl/Freezhild wymagają pełnego native owner/identity/placement. Nie wolno traktować writerów z katalogu dialogues jako aktywnej rozmowy ani znieść frakcyjnej wyłączności (§5/§7).
- Współdzielone10 chestów6243–6252, RottenTree9277 i gałąź crowbar3304/AID12566 pozostają w obrębie swoich gałęzi źródłowych, bez drugiego handlera. Potrzebne pełne claim/selector/item semantics i placement; brak kwalifikacji nie jest ukończoną nagrodą (§5/§6/§7/§8).
- Rust bugs350/UID22005 z utworzeniem1069 na4 polach, tool parcels897/898/899, leverAID12574 z19 monster spawns, Amazon disguise397 na20000ms, Amazon/Pirate passages wymagają rzeczywistych obiektów/monsters i istniejących ownerów transform/outfit/encounter. Nie tworzymy pseudo-runtime, nowych wspólnych kontraktów ani zgadywanych wyglądów (§5/§7/§8).
- Black Knight death credit musi zachować onDeathForDamagingPlayers oraz źródłowe frakcyjne stany; nie zamieniamy go na most-damage-only lub ogólne Grizzly kills (§5/§7).
- Source FX/komunikaty i wszelkie nagrody bez rzeczywistego dopuszczenia pozostają Left out. Cap23 nie jest oferowana; brak nowej produkcyjnej ścieżki prezentacji (§5/§7).

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/actions_ring.lua:2–15](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_ring.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/actions_rust_bugs.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_rust_bugs.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/actions_tools.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_tools.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/actions_lever.lua:25](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_lever.lua#L25)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/actions_amazon_disguise_kit.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_amazon_disguise_kit.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/secret_service/creaturescripts_black_knight_kill.lua:3–13](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/creaturescripts_black_knight_kill.lua#L3)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/items/definitions/items-21000-21499.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/items/definitions/items-21000-21499.json#L1)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/world/npc-placements/placements-00000-00032.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/world/npc-placements/placements-00000-00032.json#L1)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

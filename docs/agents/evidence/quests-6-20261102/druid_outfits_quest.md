# Quests 6 — druid_outfits_quest

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **18 elementów; AUTO 1, RĘCZNIE 17**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Potwierdzono istniejącego właściciela CeironsWaterskin: outfit_and_addon_quests. Regeneracja odrzuciła kandydata z powodu niedopuszczonych i5938/i5939; usunięto nowy rekord z aktywnego katalogu bez zmiany holdów ani tożsamości toru.
- Przejrzano trzy pełne źródła NPC, gałęzie wspólnych skrzyń i botanist oraz kwalifikację przedmiotów. Nie dodano nowego adaptera runtime.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L06](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589924150). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Ceiron water sample UID40096/appearance5662 wymaga itemów i5938→i5939. Rzeczywisty generator odrzuca oba: materializable=false, stack_class=Unknown w content/items/definitions/items-29500-29999.json:1; istnieje pojedynczy binding EXACT, ale brakuje native admission. QUEST_CALLBACK_ITEM_NOT_ADMITTED w tools/content-migration/quest_callback_profiles.py:179–182. Zgodnie z ANSWER8 nie rozszerzono admission ani nie podmieniono przedmiotu.
- Brak fizycznego UID40096/appearance5662 w przeskanowanym world base.
- Brakuje kwalifikowanego rozmieszczenia Ceiron/Ustan/A Majestic Warwolf i atomowych wymian składników/addonów. Griffinclaw i wspólna gałąź botanist pozostają odłożone.
- D20 ma źródłowo uzasadniony globalny OutfitQuest.DefaultStart50960. Nie wprowadzono aktywnej poprawki bez pełnego adaptera, lokalnego zastępczego ID ani drugiego toru. Tożsamość Rook Druid jest poza zakresem.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/druid_outfits_quest/actions_ceirons_waterskin.lua:4](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/druid_outfits_quest/actions_ceirons_waterskin.lua#L4)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/druid_outfits_quest/globalevents_druid_outfits_blooming_griffinclaw.lua:10](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/druid_outfits_quest/globalevents_druid_outfits_blooming_griffinclaw.lua#L10)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/ceiron.lua:145](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ceiron.lua#L145)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:213](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/core/storages.lua#L213)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/ustan.lua:75](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ustan.lua#L75)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/a_majestic_warwolf.lua:66](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/a_majestic_warwolf.lua#L66)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_explorer_society/actions_botanist.lua:19](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_explorer_society/actions_botanist.lua#L19)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:2075](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2075)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/quests/missions/quest-state.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/quests/missions/quest-state.json#L1)
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589445484>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589451334>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280>
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:tools/content-migration/quest_callback_profiles.py:179–182](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/tools/content-migration/quest_callback_profiles.py#L179)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

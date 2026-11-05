# Quests 6 — desert_dungeon_quest

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **7 elementów; AUTO 0, RĘCZNIE 7**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dodano test źródłowej nagrody: istniejący claim desert1,3035×100; wire USE i durable once/replay bez nowej reguły. Wynik wykonania dokumentuje główny raport.
- KWALIFIKACJA NATIVE:50181,3264,3084 są materializable=true,NonStackable,CharacterInventory+Ground w finalnym providerze. Legacy definitions materializable=false nie stanowią runtimehold. Kwalifikację dokumentuje wspólny materiał dowodowy.
- Mapa: pięć pozycji startu, pięć ołtarzy i pięć celów obecne; nie implementowano atrapowego group adaptera.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L05](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590082917). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Grupowe przyjęcie pięciu profesji i jednoczesne zużycie pięciu ofiar/przeniesienie grupy wymaga rozszerzenia wspólnych właścicieli. ANSWER5 zabrania tej zmiany w tej partii. Nie dodać fikcyjnego storage/completion ani osobnych jednostkowych teleportów udających atomic group.
- Fizyczna skrzynia2472 istnieje w mapie z kwalifikacji HEAD9d2437e2c109ec6abb413ea0104906a85948dd16. Pełny skan surowej mapy nie znalazł wymaganego UID. Binder wymaga dokładnego UID na top-level entry. Compiled startup overlay/binding NIEZNANE: nie zakończono dowodu jego braku ani obecności. Nie uznawać fixture source_world za produkcyjny WorldBundle. ANSWER7: Left out do kwalifikowanego istniejącego przypięcia; bez rozszerzania WorldBundle/RewardClaimMint placement.
- Desert2 UID6158 nadal ma data_hold ITEM_SEMANTICS_MISSING dla3084, waiting_data i brak native_contents w recordzie RewardClaim. Native3084 jest już admitted; to rozbieżność warstw, nie brak native itemu. ANSWER8: zachować hold; nie przepisać go ręcznie ani nie rozszerzać admission. Raw250 oznacza pełnecharges jednego amuletu, nie250amuletów.
- Hagor/Adrenius w Crystal są stubami FocusModule bez questowego dialogu. Rozszerzenie tylko z Global pozostaje poza zakresem bez ANSWER.
- Donor wymaga pięciu profesji włącznie z Monk; nie zmieniono na Wiki4–5. Pozostałe rozbieżności nagród pozostają w D.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua:23](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua#L23)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua:30](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua#L30)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua:40](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/desert_dungeon_quest/actions_desert_dungeon_lever.lua#L40)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:1806](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1806)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:1813](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1813)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/hagor.lua:26](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/hagor.lua#L26)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/adrenius.lua:21](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/adrenius.lua#L21)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/interactions/reward_claims/reward-claim-variants-00000-00099.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/interactions/reward_claims/reward-claim-variants-00000-00099.json#L1)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/spells.manifest.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/spells.manifest.json#L1)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:tools/content-schema/native-gameplay/build_source_item_profiles.py:132](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/tools/content-schema/native-gameplay/build_source_item_profiles.py#L132)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/quests/src/quest/mod.rs:287](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/quests/src/quest/mod.rs#L287)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/entities/src/carrier/runtime_actor_source_step.rs:20](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/entities/src/carrier/runtime_actor_source_step.rs#L20)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/content/src/content/world_reward_claims.rs:894](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/content/src/content/world_reward_claims.rs#L894)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

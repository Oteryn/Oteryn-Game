# Quests 6 — behemoth

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **2 elementów; AUTO 1, RĘCZNIE 1**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- ISTNIEJĄCY RUNTIME: callback behemoth obsługuje UID30009, nieodwracalne przełączenie2772→2773 i usunięcie5barier1791; bez tworzenia drugiej tożsamości questa.
- Dodano test źródłowych nagród: cztery rzeczywiste przypadki wire USE→RewardClaimMint, dokładne nagrody, once/replay/cold replay. To test istniejących reguł, nie nowy kontrakt. Wynik wykonania dokumentuje główny raport.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L04](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590082222). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Fizyczna skrzynia2472 istnieje w mapie z kwalifikacji HEAD9d2437e2c109ec6abb413ea0104906a85948dd16. Pełny skan surowej mapy nie znalazł wymaganego UID. Binder wymaga dokładnego UID na top-level entry. Compiled startup overlay/binding NIEZNANE: nie zakończono dowodu jego braku ani obecności. Nie uznawać fixture source_world za produkcyjny WorldBundle. ANSWER7: Left out do kwalifikowanego istniejącego przypięcia; bez rozszerzania WorldBundle/RewardClaimMint placement.
- Cztery claimy UID6052,6053,6054,6075 mają dane; bag ma pełne native_contents. Nie oznaczono questa jako playable bez produkcyjnego binding. Testfixture wstawia UID jawnie.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/behemoth/action-lever.lua:16](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/behemoth/action-lever.lua#L16)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:970](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L970)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:977](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L977)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:984](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L984)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:1150](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1150)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/system/quest_reward_common.lua:270](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L270)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/content/src/content/world_reward_claims.rs:894](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/content/src/content/world_reward_claims.rs#L894)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/objects.rs:369](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/dispatch/src/gameplay_transport/composed_admission/objects.rs#L369)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:208](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs#L208)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

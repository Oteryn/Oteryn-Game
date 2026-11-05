# Quests 6 — waterfall

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **5 elementów; AUTO 0, RĘCZNIE 5**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dodano dane: trzy teleporty Waterfall w istniejącym world_callbacks shape, bez nowej gramatyki/fieldów. Dane są częścią istniejącego katalogu callbacków.
- Dodano test źródłowej nagrody: istniejący RewardClaim z korzeniem5926 i dziećmi6096×1,3097×1; realny wire USE, once/replay/cold replay.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **3**.

| Klucz | Wyzwalacz i selektor | Skutek | Granica dowodu |
| --- | --- | --- | --- |
| `oteryn:quest-callback/waterfall-jump` | `step_in`; `{"position":{"x":32966,"y":32626,"floor":-5},"action":50022}` | `{"teleport":{"x":32968,"y":32626,"floor":-5}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/waterfall-cave-entrance` | `step_in`; `{"position":{"x":32968,"y":32629,"floor":-7},"action":50022}` | `{"teleport":{"x":32968,"y":32631,"floor":-8}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/waterfall-cave-exit` | `step_in`; `{"position":{"x":32967,"y":32630,"floor":-8},"action":50022}` | `{"teleport":{"x":32971,"y":32620,"floor":-8}}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |

Źródła nowych rekordów:

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:16-37](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L16)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:23-37](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L23)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:27-37](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L27)

## Left out

[Q6-L21](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590084929). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Trzy pozycje callbacków istnieją, lecz brak AID50022 w surowym skanie. Bez istniejącego kwalifikowanego compiled binding callback zachowuje odmowę; zgodnie ANSWER7 nie dopisywać runtimebinding ani nowych tagów poza zakresem.
- WATERSPLASH oraz source-facing south/east wymagają niedopuszczonego rozszerzenia wspólnego wykonawcy/presentation. Samodzielna gałąź splash-only nie jest importowana jako no-op. Pozostają Left out zgodnie ANSWER5.
- Fizyczna skrzynia2472 istnieje w mapie z kwalifikacji HEAD9d2437e2c109ec6abb413ea0104906a85948dd16. Pełny skan surowej mapy nie znalazł wymaganego UID. Binder wymaga dokładnego UID na top-level entry. Compiled startup overlay/binding NIEZNANE: nie zakończono dowodu jego braku ani obecności. Nie uznawać fixture source_world za produkcyjny WorldBundle. ANSWER7: Left out do kwalifikowanego istniejącego przypięcia; bez rozszerzania WorldBundle/RewardClaimMint placement.
- UID6258: dane i wykonawca claimu istnieją; produkcyjnego przypięcia nie dowiedziono.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:16](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L16)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:20](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L20)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:23](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L23)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:27](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L27)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/waterfall/movements_waterfall.lua:34](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/waterfall/movements_waterfall.lua#L34)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:2591](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2591)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/content/src/content/world_reward_claims.rs:894](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/content/src/content/world_reward_claims.rs#L894)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:208](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs#L208)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

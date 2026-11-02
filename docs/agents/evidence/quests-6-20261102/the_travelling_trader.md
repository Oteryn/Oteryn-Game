# Quests 6 — the_travelling_trader

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **15 elementów; AUTO 1, RĘCZNIE 14**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- 15 źródłowych elementów: siedem dni Rashida i dostawcy. Rozróżniono standardowego Rashida od Rashid of Island. Decyzja ANSWER10 ustala UTC dla os.date.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L20](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589929679). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Brak kwalifikowanych placementów standardowego Rashida i dostawców oraz atomowych wymian przedmiotów/złota/postępu. Nie połączono niezależnego custom NPC ze standardowym questem.
- Istniejące outlaw-camp-door i delivery descriptor nie dowodzą pełnego donora: door nie wykonuje source teleportu, a UID3020 wymaga dokładnego atomowego wydania145. Brak odpowiednich fizycznych UID/AID powiązań w world base.
- Czasowy cheese, fragile vase i transport/konsumpcja wymagają dopuszczonej semantyki itemów i ownera. Nie zmieniono holdów ani nie użyto zastępczych nagród. ANSWER5/7/8.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rashid.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/rashid.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_travelling_trader/actions_outlaw_camp_door.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_travelling_trader/actions_outlaw_camp_door.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/others/quest_system2.lua:20](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/other/others/quest_system2.lua#L20)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:283](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L283)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

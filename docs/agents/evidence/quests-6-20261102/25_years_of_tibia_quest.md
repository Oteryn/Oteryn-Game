# Quests 6 — 25_years_of _tibia_quest

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **6 elementów; AUTO 0, RĘCZNIE 6**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Przypięta inwentaryzacja dwóch lwów, obu obszarów maszyny, BossLever i nagrody Lord Retro. Wycofano 18 kandydatów Use po wykryciu wyboru ordinal0 podłogi zamiast mechanizmu; regresja chroni ich nieobecność.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L01](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589922467). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Maszyna: brak kwalifikowanego powiązania rzeczywistego celu Use; pola istnieją, ale sam position-only selector uruchamiałby tylko podłogę. Nie dodano wymyślonego ograniczenia itemid.
- Lwy: odczyt posiadania 34008 bez konsumpcji wymaga dopuszczonego kontraktu; nie zastąpiono go zużyciem ani sztucznym postępem.
- Lord Retro: grupowy BossLever, pokój, rozliczenie uczestników i outfit1460/1461 wymagają kwalifikowanych wspólnych właścicieli i kosmetyków. ANSWER5/7; globalne cztery fazy nie są implementowane. FX i tekst sukcesu pozostają odłożone.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/25_years_of _tibia_quest/actions_lion_statue.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/25_years_of%20_tibia_quest/actions_lion_statue.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/25_years_of _tibia_quest/actions_time_machine.lua:18](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/25_years_of%20_tibia_quest/actions_time_machine.lua#L18)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/25_years_of _tibia_quest/lever_lord_retro.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/25_years_of%20_tibia_quest/lever_lord_retro.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/25_years_of _tibia_quest/lord_retro_death.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/25_years_of%20_tibia_quest/lord_retro_death.lua#L3)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

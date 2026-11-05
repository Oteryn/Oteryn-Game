# Quests 6 — adventures_of_galthen

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **15 elementów; AUTO 5, RĘCZNIE 10**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dwie definicje StepIn dla wyjść Iksupan z potwierdzonymi fizycznymi polami. Wynik wykonania przez dispatch jest osobno rozliczony w raporcie.
- Zachowano źródłowe selektory i cele; nie oznaczono całego questa jako ukończonego.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **2**.

| Klucz | Wyzwalacz i selektor | Skutek | Granica dowodu |
| --- | --- | --- | --- |
| `oteryn:quest-callback/galthen-iksupan-exit-1` | `step_in`; `{"position":{"x":34015,"y":31892,"floor":-8}}` | `{"teleport":{"x":32728,"y":32880,"floor":-7}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/galthen-iksupan-exit-2` | `step_in`; `{"position":{"x":34041,"y":31726,"floor":-10}}` | `{"teleport":{"x":32728,"y":32880,"floor":-7}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |

Źródła nowych rekordów:

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua#L2)

## Left out

[Q6-L02](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589922884). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Trzy callbacki Use oparte wyłącznie na pozycji (dwoje schodów i wyjście Mitmah) usunięto z aktywnego JSON: ordinal0 wskazuje podłogę, nie źródłowy mechanizm.
- Bramki poziomu150/250 i skrót dostępu wymagają odłożonego kontraktu. Nie zastąpiono odczytu sztucznym zapisem stanu.
- Pokoje bossów Ahau/Megasylvan/Mitmah, satchel z30-dniowym cooldownem i teleportem, Idol40578 oraz stroje1597/1598 wymagają kwalifikowanych wykonawców atomowych zmian i powiązań. ANSWER5/7.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_iksupan_teleports.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_iksupan_teleports.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_galthens_tree.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_galthens_tree.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua#L2)
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589444781>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589445145>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280>

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

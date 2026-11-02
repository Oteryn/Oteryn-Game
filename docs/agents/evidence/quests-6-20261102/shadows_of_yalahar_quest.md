# Quests 6 — shadows_of_yalahar_quest

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **65 elementów; AUTO 4, RĘCZNIE 61**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dodano dwa callbacki: inkwell i strange energy. Oba wymagają brakujących powiązań z mapą.
- Źródłowy licznik książek42557 dodaje istniejący mechanizm extend pod właścicielem Shadows, z dokładnym warunkiem >=8 i resetem do−1. Ten tor nie istniał w bazowym katalogu.
- Przejrzano siedem pełnych plików NPC; ich mutujące gałęzie pozostają poza runtime do kwalifikacji atomowego wykonawcy.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **2**.

| Klucz | Wyzwalacz i selektor | Skutek | Granica dowodu |
| --- | --- | --- | --- |
| `oteryn:quest-callback/shadows-inkwell` | `use`; `{"action":4954}` | `{"storage":[{"track":"oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09","minimum":1,"maximum":1,"set":2},{"track":"oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_books_success","minimum":8,"maximum":9223372036854775807,"set":-1}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |
| `oteryn:quest-callback/shadows-strange-energy` | `step_in`; `{"action":25031}` | `{"storage":[{"track":"oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission11","minimum":1,"maximum":1,"set":2}]}` | Brak pasującego UID/AID w przeskanowanym world base; dostępność produkcyjna niepotwierdzona. |

Źródła nowych rekordów:

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua:59](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua#L59)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/movements_mission11.lua:24](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/movements_mission11.lua#L24)

## Left out

[Q6-L12](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589926429). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Brak fizycznych AID4954/25031 i writera zwiększającego licznik książek. Same callbacki nie zamykają grywalnych kroków.
- Brakuje kwalifikowanych rozmieszczeń siedmiu NPC i pełnych atomowych wymian. Teleskop, folio RNG/timery, biblioteka napraw, disassembler oraz odczytowa bramka Fenrock pozostają odłożone.
- Intencja poprawki D15 dla czerwonych filarów jest źródłowo uzasadniona, lecz pełny adapter teleskopu i jego poprawka nie są aktywne. Donorowe1s, osiem sukcesów folio,120s przygotowania głowy i pięć ticków obrażeń pozostają zgodne z decyzją Crystal.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua:59](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua#L59)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/movements_mission11.lua:24](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/movements_mission11.lua#L24)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:1563](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/core/storages.lua#L1563)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission02.lua:98](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission02.lua#L98)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/telas.lua:465](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/telas.lua#L465)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/golem_disassembler.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/shadows_of_yalahar_quest/golem_disassembler.lua#L1)
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589449455>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589449825>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589450226>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589450612>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589450966>
- <https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280>

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

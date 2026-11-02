# Quests 6 — grimvale

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **41 elementów; AUTO 0, RĘCZNIE 41**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Dodano dane: dziewięć bezwarunkowych wyjść aren w istniejącym shape, selektory pozycyjne, bez sztucznego storage. Źródłowy przegląd potwierdził literalne pozycje i cele.
- ODCZYT ZALEŻNOŚCI: pełne pięć form Feroxy, lib/grimvale, summon, Maeryn, Niral, Cornell, mining/enchants; osobne delty01/02. Potwierdzono exact-case mismatch feroxaDeath/FeroxaDeath.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **9**.

| Klucz | Wyzwalacz i selektor | Skutek | Granica dowodu |
| --- | --- | --- | --- |
| `oteryn:quest-callback/grimvale-curse-exit-6` | `step_in`; `{"position":{"x":33180,"y":32011,"floor":-8}}` | `{"teleport":{"x":33167,"y":31978,"floor":-8}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-curse-exit-7` | `step_in`; `{"position":{"x":33055,"y":31888,"floor":-9}}` | `{"teleport":{"x":33055,"y":31911,"floor":-9}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-curse-exit-8` | `step_in`; `{"position":{"x":33120,"y":31996,"floor":-9}}` | `{"teleport":{"x":33128,"y":31972,"floor":-9}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-curse-exit-9` | `step_in`; `{"position":{"x":33395,"y":32111,"floor":-9}}` | `{"teleport":{"x":33403,"y":32097,"floor":-9}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-curse-exit-10` | `step_in`; `{"position":{"x":33446,"y":32040,"floor":-9}}` | `{"teleport":{"x":33442,"y":32052,"floor":-9}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-ancient-exit-5` | `step_in`; `{"position":{"x":33154,"y":32245,"floor":-12}}` | `{"teleport":{"x":33123,"y":32240,"floor":-12}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-ancient-exit-6` | `step_in`; `{"position":{"x":33119,"y":32278,"floor":-12}}` | `{"teleport":{"x":33130,"y":32252,"floor":-12}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-ancient-exit-7` | `step_in`; `{"position":{"x":33087,"y":32239,"floor":-12}}` | `{"teleport":{"x":33123,"y":32264,"floor":-12}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |
| `oteryn:quest-callback/grimvale-ancient-exit-8` | `step_in`; `{"position":{"x":33148,"y":32283,"floor":-12}}` | `{"teleport":{"x":33115,"y":32252,"floor":-12}}` | Pole pozycyjne potwierdzone w world base; wynik testu wykonawcy tylko w raporcie. |

Źródła nowych rekordów:

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:67-143](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L67)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:71-143](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L71)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:75-143](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L75)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:79-143](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L79)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:83-143](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L83)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:58-135](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua#L58)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:62-135](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua#L62)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:66-135](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua#L66)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:70-135](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua#L70)

## Left out

[Q6-L07](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590083633). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Wejścia do aren, per-player cooldowny, spawn/occupancy, wieloetapowa Feroxa, world schedule/map swap i ich lifecycle nie mają dopuszczonego pełnego adaptera w tej partii. ANSWER5: bez rozszerzeń wspólnych właścicieli Instance/WorldBundle/QuestState.
- FX TELEPORT wyjść pozostają osobnym ownerPR zgodnie ANSWER5; dziewięć przeniesień nie oznacza całego questa DONE.
- Cztery AncientFeud shortcuts: wadliwa sygnatura donora i read-only storage guard; bez nowej gramatyki/read-only rule pozostają Left out.
- NPC mission/exchange/helmet i inne unikalne kroki nie są wykonawcą runtime tylko dlatego, że istnieją definicje. Brak kanonicznych runtime NPC placements / adapterów jest Left out według ANSWER5/7.
- Feroxa case bug jest dowiedziony i dopuszczony do naprawy po ANSWER, lecz nie napisano i nie podłączono fazowego adaptera tylko dla samej korekty nazwy. Nie deklarować poprawki jako wykonanej.
- Wydobycie liquid silver, pułapki, czasowe marks, horn i enchants wymagają właściwych item/condition/craft owners; nie dodano deklaracyjnego pseudo-runtime. Źródłowe cooldowny/efekty zachowane w raporcie.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_ancient_feud_entrances.lua:10](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/actions_ancient_feud_entrances.lua#L10)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/creaturescripts_feroxa_transform.lua:28](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/creaturescripts_feroxa_transform.lua#L28)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/the_curse_spreads/feroxa3.lua:56](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/monster/quests/the_curse_spreads/feroxa3.lua#L56)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/the_curse_spreads/feroxa4.lua:56](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/monster/quests/the_curse_spreads/feroxa4.lua#L56)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/lua/creature/creatureevent.cpp:57](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/creatureevent.cpp#L57)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/lua/creature/creatureevent.hpp:110](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/creatureevent.hpp#L110)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/grimvale.lua:25](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/grimvale.lua#L25)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/spells/monster/feroxa_summon.lua:28](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/spells/monster/feroxa_summon.lua#L28)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:670](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/lib/register_actions.lua#L670)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/maeryn.lua:1](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/maeryn.lua#L1)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

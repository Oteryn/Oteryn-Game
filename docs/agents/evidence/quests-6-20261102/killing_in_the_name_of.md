# Quests 6 — killing_in_the_name_of

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **221 elementów; AUTO 1, RĘCZNIE 220**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Potwierdzono istniejący rzeczywisty native entry Grizzly: aktualna rozmowa z dokładnym NPC oraz podłączony kill route. Zachowano jedną tożsamość i obecne kwalifikowane reguły; ta partia nie dodaje drugiego przebiegu ani nagród.
- Rozdzielono źródłową konfigurację 42 tasków Grizzly/30 pokoi native od 9 zadań innych NPC i 3 records tasks.Extra. Inwentaryzacja221 wierszy jest source evidence, nie221 działających kroków ani dowód pełnej zgodności Grizzly.
- W tej partii nie dodano nowego adaptera poza istniejącą ścieżką Grizzly; dostarczono kwalifikację real caller/source boundaries.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L09](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589925204). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Budrik, Lugri, Gabel, Malor, Raymond i Daniel: brak nowo dopuszczonych placementów oraz pełnego native podłączenia ich odmiennych dialogów, warunków, liczników i nagród. Shared Grizzly action protocol nie nadaje im automatycznie obsługi; wymagane konkretne NPC identity/placement/native rule, nie zmiana nazw lub fikcyjny direct command (§5/§7).
- Budrik zachowuje ToOutfoxAFoxQuest.Questline==2 i własne ograniczenia poziomu. Nie importujemy cudzej zależności ani nie usuwamy jej guardu (§6). Daniel ma rzeczywiste progi i kolejność gałęzi NPC, których nie zastępuje samo lib.level/premium.
- Hide, Esmeralda i Tromphonyte pozostają przy wcześniejszym holdzie kwalifikacji mapy/boss entry; bez arbitralnych pozycji, spawnu lub pominięcia warunków (§7). Nie osłabiono przyjętych wcześniejszych reguł Grizzly (§9).
- Trzy tasks.Extra (Mountain Troll, Muglex Clan Footman, Minotaur Bruisers) mają konfigurację, ale standardowy startup tego questa ich nie rejestruje. Nie oznaczono ich jako aktywnych misji; brak wykazanego entry/writera pozostaje jawny (§3/§7).
- Źródłowe FX portali, NPC i sukcesu/odmowy nie mają podłączonej produkcyjnej publikacji callbacków. Istniejąca cap23 ma wszystkie offers=false; brak nowego outbox/egress w tej partii (§5/§7, Q6-FX-PRESENTATION / ANSWER6589908362).

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/killing_in_the_name_of.lua:28–71](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/killing_in_the_name_of.lua#L28)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/killing_in_the_name_of.lua:931–970](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/killing_in_the_name_of.lua#L931)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/killing_in_the_name_of.lua:1018–1027](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/killing_in_the_name_of.lua#L1018)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/killing_in_the_name_of/creaturescripts_monster_kill.lua:1–111](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/killing_in_the_name_of/creaturescripts_monster_kill.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/killing_in_the_name_of/movements_boss.lua:383–405](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/killing_in_the_name_of/movements_boss.lua#L383)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/budrik.lua:75–116](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/budrik.lua#L75)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/daniel_steelsoul.lua:76–156](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/daniel_steelsoul.lua#L76)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/daniel_steelsoul.lua:174–223](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/daniel_steelsoul.lua#L174)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/dispatch/src/gameplay_transport/npc_command/grizzly.rs:63–121](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/dispatch/src/gameplay_transport/npc_command/grizzly.rs#L63)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs:74–145](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs#L74)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

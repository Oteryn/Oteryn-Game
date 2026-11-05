# Quests 6 — roshamuul_quest

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **19 elementów; AUTO 0, RĘCZNIE 19**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Ustalono source profile19 kroków: bone/chalk/gravel/mixture/mortar/trough, kill credit, cave routes, resonance i rzeczywisty graf Sandomo. Nie dodano nowego działającego adaptera runtime tych mechanizmów.
- Po ANSWER wybrano literalny Crystal dla Q6-D21/D22/D23: chalk data>10 po inkrementacji; losowanie jaskini przy każdym wejściu; donorowe3 fale Silencer i Sight3..8 bez dopisanej wypłaty Global.
- Sandomo zakwalifikowano jako SET Gold_Record=earned, nie ADD do starego salda; earned=100*max(0,mortar)+max(0,frazzlemaws)+max(0,silencers), następnie zerowanie3 liczników. To decyzja source semantics, nie deklaracja zaimportowanego NPC.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L10](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589925586). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Przepisy chalk/gravel/mixture/water/mortar oraz trough, bone-pile losowanie i transform/decay: nadal wymagają dopuszczonych konkretnych item/fluids/target identities, mapbindings i istniejącej atomowej ścieżki transformacji. Nie zastąpiono brakujących obiektów ani itemów fikcyjnymi identyfikatorami (§5/§7/§8).
- Frazzlemaw/Silencer kill credit: trzeba zachować rzeczywiste onDeathForParty i powiązać legacy ROSHAMUUL_KILLED_* z istniejącymi named storage na podstawie dowodu z donora. Podobieństwo nazw nie stanowi aliasu. Brak końcowej kwalifikacji writera/registracji (§3/§7).
- Resonance chamber: plateau guard, brak potworów, wspólny global34380, losowanie, fale60/120/180/240s, cleanup1800000ms i reset muszą pozostać jednym zgodnym mechanizmem. Nie rozszerzamy world/instance/timer ownerów ani nie rozbijamy startu na niezależne zapisy (§5).
- Sandomo: brak dopuszczonej kompletnej ścieżki rozmowy/placementu, atomowego zapisu salda i wymiany cluster item20062. Topic4/5 przechwytuje trade; wymiana wymaga całkowitego n>=1 i salda>=100n, daje item20062*n i odejmuje100n. Literalne SET pozostaje; Q6-SANDOMO-RECORD A został rozstrzygnięty przez ANSWER6589908362 (§3/§4/§5/§7/§8).
- Cave entry/exit nie dostały nowego zakwalifikowanego źródłowego bindingu; brak arbitralnych pozycji. Upper Roshamuul, Morax/Dream Butterfly nie mają ustalonego kompletu source steps w tej części, więc nie dodano gałęzi wyłącznie z Global (§1/§7).
- FX pozostają Left out: cap23 offers=false i brak produkcyjnego callback publishera (§5/§7). Rozstrzygnięte pytania source differences: Q6-D21 https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589451710; Q6-D22 https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589452134; Q6-D23 https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589452594.

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_bone.lua:9](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_bone.lua#L9)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_chalk.lua:10–18](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_chalk.lua#L10)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_gravel.lua:4](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_gravel.lua#L4)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_mixture.lua:8](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_mixture.lua#L8)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_mortar.lua:2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_mortar.lua#L2)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_trough.lua:3](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_trough.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/creaturescripts_lower_roshamuul.lua:1–7](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/creaturescripts_lower_roshamuul.lua#L1)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/movements_lower_roshamuul.lua:3–41](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/movements_lower_roshamuul.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_resonance_chamber.lua:49–97](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest/actions_resonance_chamber.lua#L49)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sandomo.lua:53–59](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/sandomo.lua#L53)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sandomo.lua:97–165](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/sandomo.lua#L97)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

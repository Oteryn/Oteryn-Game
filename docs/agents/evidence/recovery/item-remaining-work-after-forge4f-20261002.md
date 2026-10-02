Stan po opublikowanym Forge1: `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`. Odczytano rzeczywiste Gitbloby oraz zapisane pola właścicieli danych. Publikacja na gałęzi nie ustanawia integracji całego katalogu ani gotowości runtime.

**7171 ID ma co najmniej jeden kwalifikowany fakt.** To nie liczba w pełni zweryfikowanych Itemów. **FullyVerified: NOT_ESTABLISHED; kompletne i niekompletne globalnie: NULL.** Osobna zamrożona klasyfikacja rodzin obejmuje **12252 z 12487 ID**, pozostawia **235**; 22 profile, 23 szablony, 0 brakujących szablonów.

Zapisane obecnie:316 wpisów ItemAuthoring, 147 profili Forge, 39 required_magic_level (9 jawnych zer), 139 wpisów Use/345 obserwacji (111 damage, 138 damage_type, 96 mana_cost), 494 wektory/796 atomów modifierów i 28 relative hit p/100. Relacje:203 źródłowe Itemy/278 relacji. Forge3332 ma niezależną parę 2/2. Use, ML i Forge to dane authoring; brak twierdzenia o aktywacji zachowania.

| Pozostała paczka | Różne ID | Praca do domknięcia |
| --- | ---: | --- |
| Weapon103 |103|78 signed attack_modifier i25 absolute hit: końcowy compiler/owner/glue, aktualne kwalifikatory, atomic guards, generation i serialne testy; teraz AUTHORING,0 policzonych publikacji|
| DefaultFalse8 |8|Source-peer PASS; aktualny następca po Weapon, zamknięty packet8, guards i exact generation;0 applied|
| Physical8 |8|8 całych wektorów/16 atomów: decyzja własnego profilu/źródłowego zbioru, zgodność codec/oracle, seals i kwalifikacja8, finalny exact-head proof|

Weapon103∪DefaultFalse8 daje **109 różnych ID**. Po dodaniu rozłącznego Physical8: **117 ID**. Default8 nakłada się na 2 ID z Weapon oraz 6 z już opublikowanym Use; te 6 nadal ma osobną otwartą pozycję stackfalse. To rozmiar wybranych paczek, nie wszystkich niekompletnych Itemów. Po przyszłych publikacjach trzeba ponownie odczytać rzeczywiste heady i wartości, bez automatycznego odjęcia prognoz.

Większe, nakładające się blokady źródłowe:

| Zakres dowodu | Licznik | Brakujący dowód/decyzja |
| --- | ---: | --- |
| Pozytywne stackable |2493 ID|Niezależne maksimum per Item; brak bezpiecznego default100|
| Diagnostyczne unmove=true |4091 ID|Take jest ABSENT/UNKNOWN; brak afirmacji portable Item-domain;0 bezpiecznych movable=false|
| Oficjalnie dodatnie Forge |870 ID|Niezależne per-Item max_tier; osobny zakres od dawnych upgradeclass865|
| Augments |62 Itemy/77 klauzul/47 etykiet|0 typed bindings; potrzebne własne tożsamości zaklęć i mapping właściwości/efektów|
| Waga |4 ID|39699–39702: konflikt własnych źródeł; potrzebna kwalifikacja wariantu/cyklu|
| weapon.range_cells |3 wpisy|Niepoprawna składnia lub jednostka zachowanego źródła|
| Weapon poza paczką |2+4 ID|8024/8025: konflikt nazwy;25757/35901/25758/35902: wspólne own-ID/warianty|

Tych liczników **nie sumuje się**. Nie każdy UNKNOWN dowodzi brakującego właściwego pola. Dawne3479→3467→3465→3408→**3407** to wyłącznie ta sama zachowana unia list parametrów po mappingu opublikowanych ownerfields; nie globalny unfinished count ani zamiennik historycznych 2521/986.

Physical8: bieżący reference ma **57320 rekordów wszystkich rodzajów/34031 Itemów** i69 wspólnych canonical shards. Chroniony historyczny fixture formatu artifact ma **38157** rekordów własnego zbioru; różnica4126 nie jest listą brakujących Itemów. Zachowane prototype są nieprzetestowane i nie stanowią zaakceptowanego profilu 5. Konkretna lista czterech etapów, codec tests i kwalifikatorów źródłowych jest w `physical8-implementation-work-inventory-20261002.json`. Aktualna ciągłość publicznych rewizji: **UNKNOWN**.

| Rodzina — zamrożona nakładka | ID |
| --- | ---: |
| container | 237 |
| container_equipment | 8 |
| decoration | 3316 |
| document | 446 |
| equipment_armor | 459 |
| equipment_offhand | 337 |
| event_collectible | 192 |
| fluid | 87 |
| food | 195 |
| key | 13 |
| light_source | 166 |
| material_valuable | 3140 |
| plant | 906 |
| progression_material | 11 |
| quest_item | 1108 |
| rune | 41 |
| tool | 354 |
| transformation_item | 30 |
| trash | 206 |
| weapon_distance | 177 |
| weapon_magic | 128 |
| weapon_melee | 695 |

Źródła, ID paczek, przecięcia i SHA w towarzyszącym JSON. Historyczne raporty po Mantra iUse zachowano. Bez Cargo, nowych odczytów publicznych źródeł, edycji repo, Git mutation lub aktualizacji plików postępu Root.

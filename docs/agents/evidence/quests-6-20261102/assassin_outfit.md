# Quests 6 — assassin_outfit

Ta notatka rozlicza zakres jednego questa. Pełna tabela kroków, porównanie z Global i wyniki wykonanych kontroli znajdują się w [raporcie partii](../OTV2-20261102-quests-6-20261102.md).

Crystal: `201189f871d23597b82caf88fd611454a2ddefb8`. Oteryn, baza kwalifikacji: `798e5b1ce27f441d5cbeece333dc7ac45e19d348`. Skan mapy wykonano na pierwszym, dokumentacyjnym head `9d2437e2c109ec6abb413ea0104906a85948dd16`.

Pierwsza inwentaryzacja: **27 elementów; AUTO 0, RĘCZNIE 27**. To historyczna klasyfikacja elementów, a nie liczba dostępnych w grze misji. Późniejsze korekty są jawne w suplementach głównego raportu.

## Dostarczony zakres

- Ustalono rzeczywiste grafy Vescu/Erayo/Atrad, ich material requests i warunki wejścia; data/writers nie są przedstawiane jako aktywna rozmowa lub przyznany outfit.
- Skorygowano kwalifikację storage: Atrad=30036 jest zdefiniowany i odrębny od AssassinSecondAddon40763; donor pisze oba. Nie utworzono aliasu ani nowego ID.
- Potwierdzono zakres płci nagród: Vescu baza156/152, Erayo addon1 obu, Atrad addon2 wyłącznie bieżącej płci. Nie dodano runtime adaptera/grantu Assassin.

## Dane callbacków

Nowych rekordów w istniejącym `content/quests/world-callbacks.json`: **0**.

Ta partia dokumentuje istniejące zachowanie i konkretne braki tego questa; nie deklaruje nowego działającego adaptera.


## Left out

[Q6-L03](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589923733). Stosujemy [ogólne zasady koordynatora](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280) i [rozstrzygnięcie kontraktu oraz prezentacji](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362). [ANSWER6589944418](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589944418) potwierdził pierwsze 16 list; [ANSWER6590303753](https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6590303753) przyjął A dla Q6-L04/L05/L07/L14/L21. Wszystkie 21 list ma odpowiedź koordynatora.

- Brak dopuszczonego produkcyjnego authoringu/mappings i opaque grant keys152/156/addons1/2 z kompatybilnym provenance. Outfits jest READY_UNPOPULATED; synthetic-authoring to fixture. Istniejący quest-grants owner dopuszcza Revenant/DecayingDefender, nie Assassin. Brak nowej gałęzi ownera w tej partii; finalny track nie może zastąpić nagrody (§5/§7; Q6-ASSASSIN-COSMETICS rozstrzygnięte A przez ANSWER6589908362).
- Brak kwalifikowanych native placementów i pełnego dialog owner path Vescu, Erayo, Atrad. Zachowujemy Vescu greeting DRUNK, Atrad greeting FIRE i ich tematy potwierdzenia. Nie zastępujemy kondycji sztucznym storage ani nie osłabiamy warunków (§5/§7).
- Erayo wymaga istniejącego bazowego outfitu bieżącej płci i Shipwrecked==2. Ten cudzy prerequisite pozostaje strzeżony; nie importujemy sąsiedniego questa ani nie pomijamy go (§6).
- Dom Erayo wymaga dokładnego getItemById(3086,deepSearch) albo CONDITION_INVISIBLE, nie ogólnego getItemCount lub samego slotu ring. Helper C++ schodzi do kontenerów tylko dla prawdziwego depthSearch; brak global assignment proof nie jest uprawnieniem do zmiany semantyki. Exit/refusal position jest źródłowa32519,32914,7, bez dopisanego nowego bindingu (§3/§7).
- Material exchanges Vescu/Erayo i Atrad5804+5930 wymagają wszystkich item semantics i atomowej konsumpcji/grantu. Nie przyznajemy obu płci u Atrad, nie wymyślamy nagród i nie oznaczamy końca przez sam storage. Item/placement/grant holds pozostają (§4/§5/§7/§8).
- Źródłowe FX MAGIC_GREEN/MAGIC_BLUE/POFF pozostają reference only; brak produkcyjnego publishera przy cap23 offers=false (§5/§7; Q6-FX-PRESENTATION).

## Źródła kwalifikacji

- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:78–99](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/core/storages.lua#L78)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vescu.lua:150–158](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/vescu.lua#L150)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/erayo.lua:55–63](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/erayo.lua#L55)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/erayo.lua:102–116](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/erayo.lua#L102)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/atrad.lua:53–103](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/atrad.lua#L53)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/assassin_outfit/movements_erayo_house.lua:3–19](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/assassin_outfit/movements_erayo_house.lua#L3)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/lua/functions/creatures/player/player_functions.cpp:2331–2359](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/creatures/player/player_functions.cpp#L2331)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/lua/functions/lua_functions_loader.hpp:118–120](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/lua_functions_loader.hpp#L118)
- [zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/game/game.cpp:2814–2859](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L2814)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/cosmetics/outfits/index.json:1](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/content/cosmetics/outfits/index.json#L1)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs:11–16](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs#L11)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:tools/content-schema/cosmetics-authoring/README.md:3–20](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/tools/content-schema/cosmetics-authoring/README.md#L3)
- [Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:tools/content-schema/cosmetics-authoring/README.md:53–68](https://github.com/Oteryn/Oteryn-Game/blob/798e5b1ce27f441d5cbeece333dc7ac45e19d348/tools/content-schema/cosmetics-authoring/README.md#L53)

## Weryfikacja i granice

Wyniki uruchomienia, dokładne komendy i testowany head są rozliczone w głównym raporcie oraz opisie PR. Zakres i wynik scenariuszy PostgreSQL należy czytać w tej walidacji. Fixture z jawnym UID/AID nie stanowi dowodu powiązania obiektu w dostarczanej mapie.

Skan source world base i kwalifikacja itemów są odrębne od wyniku kompilacji WorldBundle. Nie dodano migracji, protokołu, capability ani rozszerzenia wspólnego właściciela. Pełne teksty NPC pozostają w przypiętych źródłach do czasu dopuszczenia rzeczywistego adaptera; ta notatka nie jest formatem aktywnego dialogu.

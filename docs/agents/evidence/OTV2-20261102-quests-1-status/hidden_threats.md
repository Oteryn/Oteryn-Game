# Hidden Threats — końcowy zakres partii 1

Quest/katalog: `hidden_threats`. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Zastany Oteryn: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f`. Dokument uzupełnia [raport A–D partii 1](../OTV2-20261102-quests-1-20261102.md), nie tworzy dodatkowego wiersza B.

Inwentaryzacja: **57 elementów = 5 AUTO + 52 RĘCZNIE**. Jest to klasyfikacja możliwości importu, a nie liczba aktywnych funkcji. Bieżący zakres po decyzjach opisano poniżej.

Pytania wspólne o właścicieli, mapę i zakres są wskazane jako podstawa zastosowania ogólnych reguł koordynatora; nie oznaczają, że każde z nich pierwotnie dotyczyło tego konkretnego questa. Nie są zadawane ponownie. Wszystkie odpowiedzi uwzględniono do komentarza [ANSWER #6591052733](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591052733) włącznie.

## Dostarczony zakres

Zachowano **44 pełne rekordy tekstowe dziesięciu NPC** oraz sprawdzone kanoniczne tożsamości. Repo generator `tools/content-schema/quest-authoring/hidden_threats_source.py` korzysta z istniejącego parsera literałów Lua, exact Git pin i aktualnych exact NPC bindings. Obsługuje `--crystal-root`, `--repo-root`, `--output`, `--check`. Archiwum jest oznaczone source-only i nie rejestruje NPC executora.

Trzy reguły bez zasobów są jednoznaczne, lecz pozostają **Left out przez brak kwalifikowanego placementu**, zgodnie z [QUESTION — HIDDEN-NPC-PLACEMENT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590445954) oraz [ANSWER §1 — wariant B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985). Nie zostały pominięte z powodu samego braku wygodnego helpera. Kwalifikacja read-only public greeting w innych questach nie tworzy brakującego NPC na mapie.

| Donorowy NPC | Kanoniczna tożsamość | Własna flaga i słowo |
| --- | --- | --- |
| corym_worker_02, typ Corym Worker (2) | oteryn:npc.corym_worker_2 | CorymRescued02=45981, repression, <0→1 |
| corym_worker_04, typ Corym Worker (4) | oteryn:npc.corym_worker_4 | CorymRescued04=45983, repression, <0→1 |
| corym_slave | oteryn:npc.corym_slave | CorymRescued07=45986, pay back, <0→1 |

Źródła: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_02.lua:1–2,57–64,74–80`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_04.lua:1–2,57–64,74–80`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_slave.lua:1–2,57–64,74–80`; flagi: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:2706,2708,2711`. Exact identity bindings są w `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/npcs/definitions/npcs-00000-00499.json:1` (indeksy290/292/288). Nie zamieniono source stemów `_02/_04` na nowe identity; istniejące klucze mają `_2/_4`.

## Mapa: trzy claimy i pięć drzwi

Trzy skrzynie fizycznie istnieją jako item2469 na depth0, lecz mają `attrs=null`, więc nie zawierają source UID. Są to istniejące claimy, a nie trzy nowe wykonane wypłaty. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:1302–1322`; istniejące dane: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/interactions/reward_claims/reward-claims-00100-00199.json:1`.

W końcowej klasyfikacji te trzy elementy są RĘCZNIE z powodu brakującej kwalifikacji mapy; pięć istniejących bramek drzwi pozostaje AUTO. To korekta raportu, bez modyfikacji readiness istniejących claimów.

| UID | Pozycja source x/y/z | Reward / storage | Region na przypiętym Oteryn | Wynik dekodowania |
| --- | --- | --- | --- | --- |
| 6094 | 33078/31980/13 | 27270×1 / Rewards.metalFile | content/world/placements/region-z13-x129-y124.b3 | item2469, depth0, attrs=null |
| 6095 | 33080/32014/13 | 27262×1 / Rewards.keyFragment01 | content/world/placements/region-z13-x129-y125.b3 | item2469, depth0, attrs=null |
| 6096 | 33031/32050/13 | 27261×1 / Rewards.keyFragment02 | content/world/placements/region-z13-x129-y125.b3 | item2469, depth0, attrs=null |

Regiony pochodzą z `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f`; SHA256 dwóch plików to odpowiednio `c1b4d50352dd91a67d9e977cd128622847287a256b6c2b00e851a5c2a0618760` i `cd6d870e934961d3e6dd89e5b4b9ec6922c4d63b79203678bb876370859c83a1`. Binder w `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/content/src/content/world_reward_claims.rs:894–930` przegląda `tile.unique(ordinal)`, wymaga zgodności UID i appearance, a bez takiego wpisu zwraca **NoEntry**. Nie jest to ogólne NIEZNANE mapy.

Zgodnie z [QUESTION — MAP-UID-RESTORATION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590297389) i [ANSWER B — bez ręcznego UID i zmiany readiness](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590319562) claimy pozostają niezmienione. Odtworzenie startup UID narzędziem mapy wymaga osobnej kwalifikacji koordynatora.

Pięć drzwi ma natomiast rzeczywiste source AID na mapie:

| AID / storage | Pozycja source x/y/z | Item | Region |
| --- | --- | --- | --- |
| 45972 / RatterDoor | 33046/32066/12 | 1644 | content/world/placements/region-z12-x129-y125.b3 |
| 45973 / ServantDoor | 33036/32008/12 | 1642 | content/world/placements/region-z12-x129-y125.b3 |
| 45974 / CorymWorksDoor01 | 33025/32008/12 | 1642 | content/world/placements/region-z12-x129-y125.b3 |
| 45975 / CorymWorksDoor02 | 33045/32007/12 | 1642 | content/world/placements/region-z12-x129-y125.b3 |
| 45976 / CorymWorksDoor03 | 33001/32047/12 | 1644 | content/world/placements/region-z12-x128-y125.b3 |

Źródło pozycji i gate references: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/door_quest.lua:740–759`. Regiony Oteryn mają SHA256 `610a677b67117f155bc8ebb49f75f2f8e6325ca538b365fa8e75e11820fcf2f1` i `0a439fea95ddab78f0b30f0e226299fcd261838ed48b857a590e81ed1bc6aa14`. Obecność AID kwalifikuje mapowy selector istniejących drzwi; nie nadaje flag graczowi i nie kończy wstrzymanych gałęzi ich writerów.

## Left out

| Element | Konkretna granica i źródło | QUESTION + ANSWER |
| --- | --- | --- |
| Worker02, Worker04, Slave; powiązane wejścia rozmowy | Jedyny shard ma33 rekordy bez Corym: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/world/npc-placements/placements-00000-00032.json:1`; `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/world/npc-placements/held.json:1` jest pusty; `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/content/src/content/project/native_entry_room.json:1` też ich nie wiąże. To brak world admission, nie brak source reguły <0→1. | [QUESTION — HIDDEN-NPC-PLACEMENT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590445954); [ANSWER §1 — B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985). |
| Trzy skrzynie6094–6096 | Fizyczny item istnieje, lecz bez UID binder zwraca NoEntry; szczegóły i źródła w tabeli wyżej. Nie zmieniono readiness ani attrs. | [QUESTION — MAP-UID-RESTORATION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590297389); [ANSWER B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590319562). |
| CorymRescueMission i flaga06 | Counter nie ma definicji; flaga05 ma writerów Worker05 i Butler, a06 brak writera. Nie przypisano nowego numeru ani arbitralnego aliasu. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:2693–2714`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_05.lua:57–61`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_butler.lua:78–80`. | [QUESTION — HIDDEN-RESCUE](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173997); [ANSWER §3,5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Servant promotion | QuestLine3 i osiem flag oznaczają dziewięć niezależnych guardów. Nie podniesiono limitu8 i nie rozbito atomowego przejścia. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_servant.lua:65–81`. | [QUESTION — HIDDEN-RESCUE](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173997); [ANSWER §5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Ratter accept | Zapisuje również obce TibiaTales.DefaultStart; nie pominięto go po cichu. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_ratter.lua:79–85`. | [QUESTION — OTHERS-SCOPE](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173573); [ANSWER §6](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Granty Worker01/03/05 i wymiana20 rare earth | Itemy, konsumpcja i flagi wymagają atomowego NPC ownera. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_01.lua:54–62`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_03.lua:54–62`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_worker_05.lua:54–62`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/corym_servant.lua:129–145`. | [QUESTION — NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162); [QUESTION — TARGUNA-REWARD-PLACEMENT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274971); [ANSWER §4,5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Wielorazowy pilnik, forge i odblokowanie kluczem | Nie zastąpiono reusable tool operacją consume+mint ani fałszywym zapisem storage. Potrzebne są kwalifikowane przedmioty, target/custody i źródłowe transformacje; obecny ItemOnTarget nie odwzorowuje całego zestawu. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/metal_file.lua:1–21`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/action-key_forge.lua:4–32`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/action-corym_works_doors.lua:5–18`. | [QUESTION — NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162); [QUESTION — QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708); [ANSWER §5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) oraz [ANSWER §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926). |
| StepOut przy skrzyniach | Event wymaga obu flag fragmentów i QuestLine2→3; nie jest StepIn. Nie dodano nowego triggera ani sąsiedniego zastępczego kroku. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/movement-chests_check.lua:3–20`. | [QUESTION — QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708); [ANSWER §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926). |
| Cave Spider room | Oprócz niezdefiniowanego countera wymaga atomowego occupancy/spawn/teleport/timeout300s i cleanup. Sam teleport nie kwalifikuje walki. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:1–63`. | [QUESTION — HIDDEN-RESCUE](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173997); [QUESTION — wspólny owner świata/Instance](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589185913); [ANSWER §3,5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Monster/loot admission | Źródło Cave Spider, Tarantula i rare earth w Stonerefiner nie stanowi samo runtime admission całej kompozycji. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/hidden_threats/cave_spider.lua:69–88`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/vermins/tarantula.lua:1–123`, `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/reptiles/stonerefiner.lua:81–89`. | [QUESTION — wspólny owner świata/Instance](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589185913); [ANSWER §5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |

## Walidacja

Zweryfikowano10 exact NPC blobów i wszystkie44 source spans. Lokalny replay generatora zachowuje identyczne records; wynik18808 bajtów, SHA256 `9e13d588417777a0bbaecc298cadbde53cc8ca9090ed1c358a9f6e18e2a31d19`. Dodatkowo wykonano opisany bounded decode mapy. To dowody tekstu, identity i fizycznych selectorów; nie są testem pełnego gameplay ani potwierdzeniem wypłaty trzech claimów. Końcowych wyników Cargo nie wpisano przed ich wykonaniem.

## Global i status kwalifikacji

Strona identyfikacyjna: **Hidden Threats Quest**. Bieżące porównanie i revid pozostają **NIEZNANE/API403**. Historyczny revid849215 w prompt0:236 jest źródłem wtórnym. Nie pobierano HTML, nie obchodzono blokady i nie dodano danych wyłącznie z Global. Ścieżka decyzji: [QUESTION — GLOBAL-API](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099); [ANSWER §1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Różnica między dwoma plikami Crystal albo między Crystal a dawną receptą Oteryn nie jest deklarowana jako różnica względem Global.

Rejestracje wspólne są kwalifikowane na końcowym SHA partii: [QUESTION — PER-QUEST-COMMITS-ROUTING](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590518467); [ANSWER §2 — wcześniejsze heady AUTHORING](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985). Obecność danych, testu lub source modelu nie oznacza pełnej grywalności ani ukończonego testu zintegrowanego serwera.

Zgodnie z [QUESTION — AUTHORING-PRE-PUSH-CHECKS](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591017221) i [ANSWER A z doprecyzowaniem](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591052733) pełne kontrole są wymagane przed rozpoczęciem sekwencji oraz na zamrożonym końcowym SHA. Prywatny commit zawiera wyłącznie niepodpięte dane/kod i własne wejście generatora; nie zawiera rejestracji callbacku, zmiany wspólnego kodu ani generowanych indeksów. Wynik sprawdzenia roboczego checkoutu nie jest kwalifikacją przyszłego heada.

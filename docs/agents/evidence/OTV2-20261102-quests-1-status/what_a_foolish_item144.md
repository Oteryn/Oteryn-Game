# What a Foolish Quest — kwalifikacja native item144 i P20

Ten dokument dotyczy wyłącznie istniejącego przedmiotu144 oraz czasowej przemiany P20. Nie zwiększa liczby questów w raporcie i nie zalicza tej pracy do katalogu `others`. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Zastany Oteryn: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f`.

## Źródło i rzeczywisty consumer

`zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:195–198` definiuje item144, nazwę `mummy disguise`, primarytype `quest items`, weight900. Źródłowy action nakłada look65 na10000ms bez zużycia itemu i bez trwałego grantu wyglądu: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:1–13`. Fizyczne flagi loadera są jawne w `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/items/items.cpp:226–251`; wykorzystywane standardowe wartości domyślne w `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:src/items/items.hpp:38,292,300,323,348`.

Przygotowany caller `crates/gameplay/dispatch/src/gameplay_transport/item_use/quest_appearance.rs` korzysta z istniejącego odczytu owned inventory, item/native-content/session/actor fences i spell lane. Używa już zakwalifikowanego wyglądu mummy i obecnego ownera tymczasowej appearance; command replay nie przedłuża warunku. Sam P20 nie zużywa144, nie zapisuje flag Questa i nie przyznaje outfitu. Prywatny commit pozostaje AUTHORING do końcowego podpięcia.

## Standardowe dopuszczenie przedmiotu

Zastany canonical item144 istniał, ale native provider nie miał jego rekordu: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/items/definitions/items-04000-04499.json:1` (rekord272) oraz `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/items/definitions/spell-native-profiles.json:1`. Nowy repo tool `tools/content-migration/quest_disguise_item_profiles.py` przechwytuje exact Git pin i wykorzystuje istniejące `build_source_item_profiles.profile` oraz `bind_profiles`. Powstaje tylko `quest-disguise-item-profiles.json` dla144, z materializable=true, NonStackable i destinations CharacterInventory/Ground.

Istniejący binding `oteryn:item.tibia.i144@definition-r1` zachowuje historyczny source_revision `ff7ede593c69d4c658b382c97443e8155926924a`. Niezależne fakty Crystal201189f nie zmieniają accepted identity ani pina partii. Zastany binder już dopuszcza niezależny pin201189f: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:tools/content-schema/native-gameplay/build_spell_item_identities.py:31–32,112–168`. Legacy declaration pozostaje niezmieniona. Pola `weapon`, `protection`, `imbuement`, `use_transform`, `trade_restrictions`, `fluid`, `readable_writeable` nadal są UNKNOWN; nie wpisano nowego consumption ani use_requirements.

Nowy tool regeneruje profil i podpina go przez istniejący register pipeline, bez nowego ownera. Osobny test rzeczywistego artefaktu serwera potwierdził obecność i właściwości item144 po rejestracji; jego wynik jest wskazany poniżej.

## Rzeczywiście wykonana walidacja

Poniższe wyniki pochodzą z wykonanych prób roboczego checkoutu na komputerze właściciela. Nie są przenoszone automatycznie na przyszły końcowy SHA.

| Sprawdzenie | Rzeczywisty wynik |
| --- | --- |
| Source capture item144 przez istniejące profile/bind_profiles | PASS, exit0 |
| RED admission: test oczekujący rekordu przed aktywnym admit | 1 FAIL, rzeczywisty brak rekordu |
| GREEN Python `test_quest_disguise_item_profiles` | 6/6 PASS |
| Powtórzenie generatora `--check` | PASS, exit0 |
| P20 Rust RED | 1 PASS, 2 FAIL |
| P20 Rust GREEN | 3/3 PASS |
| Native artefact RED: compile/decode i wyszukanie item_policy144 | 0 PASS, 1 FAIL — rzeczywisty brak rekordu |
| Rejestracja overlay przez istniejący register tool | PASS |
| Native artefact GREEN po register overlay | 1/1 PASS |
| Pełne fmt/clippy/test/content checks końcowego SHA | Oczekują na finalny zintegrowany head |

Testy admission sprawdzają dokładny zbiór144, zachowanie identity i UNKNOWN, duplikaty, drift źródła/canonical input oraz brak częściowej zmiany planowanych plików. Wykonany test native artefaktu to `quest_mummy_disguise_is_an_exact_carried_native_item_without_invented_transform` w module testowym `content::native_gameplay`. Podane wyniki dotyczą sprawdzonego roboczego checkoutu; pełna kwalifikacja zamrożonego końcowego SHA nadal wymaga osobnego zestawu kontroli.

## Left out

| Element | Konkretna granica i źródło | QUESTION + ANSWER |
| --- | --- | --- |
| NPC grant i podaż144 | Profil nie zastępuje atomowego grant/exchange ownera. Nie stworzono przedmiotu w ekwipunku, mapowego placementu ani alternatywnej nagrody. Źródłowy action jedynie używa już posiadanego144: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:6–13`. | [QUESTION — NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162); [ANSWER §4,5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Publiczny MONSTER_SAY | Donor używa publicznej wypowiedzi; obecny P20 caller wysyła tekst przez prywatny feedback. To ograniczenie prezentacji, odrębne od działającej przemiany. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:8`. | [QUESTION — QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708); [ANSWER §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926) oraz [ANSWER §5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Pozostały runtime NPC Foolish | Pełne377 źródłowych wypowiedzi nie aktywują modelu i adaptera NPC wymagających niezakwalifikowanych wejść/powitań i resource ownerów. Wyników research modelu nie zaliczono jako runtime. | [QUESTION — własność i warunki powitań Foolish](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589180314); [QUESTION — NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162); [ANSWER §5,6,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |

Finalna kwalifikacja wspólnych rejestracji podlega [QUESTION — PER-QUEST-COMMITS-ROUTING](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590518467) i [ANSWER §2](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985). Pierwszy pełny run przed sekwencją i kwalifikacja końcowego SHA podlegają [QUESTION — AUTHORING-PRE-PUSH-CHECKS](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591017221) i [ANSWER A z doprecyzowaniem](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591052733). Porównanie Global pozostaje NIEZNANE/API403: [QUESTION — GLOBAL-API](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099); [ANSWER §1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728).

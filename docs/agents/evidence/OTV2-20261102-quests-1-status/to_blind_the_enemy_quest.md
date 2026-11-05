# To Blind the Enemy — źródłowy dialog i istniejące nagrody

## Pełne dane dialogu

`content/dialogues/quests/to-blind-the-enemy-source.json` zachowuje 51 rekordów, 53 pełne akapity Erotha, łącznie z nieaktywnymi tu ofertami nauki czarów. Każdy rekord ma linię źródła; plik jest jawnie `source_only`. Reprodukcja: `tools/content-schema/quest-authoring/batch_one_dialogue_sources.py --donor <Crystal201189f> --quest to_blind_the_enemy_quest --out content/dialogues/quests/to-blind-the-enemy-source.json --check`.

Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/eroth.lua:58–203`, blob `adc5789d7f85d0310e802a63f8462084d19ec2c1`. Nie zastąpiono istniejącego właściciela nauki czarów ani nie uznano archiwum za aktywną rozmowę questową.

## Left out: start, lustro i zakończenie

Start po `mission`/`yes` musi wydać 3463 i ustawić Questline=1 razem. Zakończenie wymaga Questline=2 i czterech flag nagród równych 1, następnie daje 3082 oraz 3035 ×10 i ustawia Questline=3. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/eroth.lua:58–80`. Wspólny atomowy owner wymiany/grantu NPC jest poza partią: [QUESTION NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162), [ANSWER punkty 4–5,8](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Nie zapisano postępu bez przedmiotu ani sukcesu po odmowie capacity.

Użycie lustra 3463 na AID 4001 i itemie 132 usuwa cel, zużywa lustro i przesuwa Questline 1→2. Obejmuje też `CONST_ME_MAGIC_POWDER` (linia 16), pełną wypowiedź `KABOOM !!` (linia 18) i opóźnienie 60 s (linia 19). Opóźniony kod tylko szuka istniejącego 132 i transformuje go w 132; nie odtwarza usuniętego celu. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:1–31`. [QUESTION BLIND-REVERT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589175767) i [ANSWER punkt 3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) nie uprawniają do wymyślenia nowego writera mapy. Mechanika pozostaje Left out do kwalifikowanego ownera opóźnionego efektu.

## Istniejące skrzynie

| UID | Pozycja Crystal | Zawartość źródłowa |
| --- | --- | --- |
| 6253 | 32591,31647,3 | 3425 ×1 |
| 6254 | 32590,31647,3 | 3282 ×1 |
| 6255 | 32588,31645,3 | 2853 z 237 ×1, 3147 ×1, 3059 ×1 |
| 6256 | 32588,31644,3 | 2853 z 3028 ×2, 3031 ×100 |

Źródło tabeli: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:2552–2582`. Nie zmieniono istniejących claimów i ich holdów. Wszystkie cztery fizyczne obiekty są w `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/world/placements/region-z03-x127-y123.b3`, lecz ich top-level entries mają `attrs=null`. Binder wymaga source UID wraz z pozycją i wyglądem: `crates/gameplay/content/src/content/world_reward_claims.rs:894–930` pod tym samym pinem. Odtworzenie UID jest Left out: [QUESTION MAP-UID-RESTORATION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590297389), [ANSWER B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590319562).

## Global i kontrola

Siedem elementów inwentaryzacji obejmuje trzy interakcje oraz cztery skrzynie. Aktualny diff `To Blind the Enemy Quest` pozostaje NIEZNANY/API403; [QUESTION GLOBAL](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099) i [ANSWER punkt 1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Historyczny revid 1091434 w prompt 0 nie jest nowym odczytem Global.

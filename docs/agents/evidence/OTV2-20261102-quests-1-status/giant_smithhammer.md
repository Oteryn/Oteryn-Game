# Giant Smithhammer — istniejący mechanizm i kwalifikacja mapy

## Istniejący zakres

Oteryn już ma `oteryn:quest-callback/giant-smithhammer`, Use UID 30024, parę dźwigni 2772/2773 i odwracalne usuwanie/tworzenie 389 w pozycji (32780,32231,-8). Nie dodano drugiego callbacku. Źródła: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/quests/world-callbacks.json:112–146`; `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/giant_smithhammer/lever.lua:1–18`.

Dekoder repo potwierdził fizyczny blocker 389 w `content/world/placements/region-z08-x128-y125.b3` pod tym samym pinem Oteryn. To potwierdzenie przeszkody, nie dowód obecności selektora UID 30024 na dźwigni. Bez kwalifikowanego selektora istniejący executor pozostaje zamknięty.

## Trzy nagrody i Left out

| UID | Pozycja Crystal | Nagroda Crystal | Źródło Crystal |
| --- | --- | --- | --- |
| 6077 | 32774,32253,8 | 3034 ×1 | `data-global/startup/tables/chest.lua:1172–1178` |
| 6078 | 32776,32253,8 | 3208 ×1 | `data-global/startup/tables/chest.lua:1179–1185` |
| 6079 | 32778,32253,8 | 3031 ×100 | `data-global/startup/tables/chest.lua:1186–1192` |

Wszystkie źródła w tabeli mają prefiks `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:`. Nagrody należą do źródłowych kluczy `ExplorerSociety.GiantSmithHammer`; pozostałe misje Explorer Society są poza tą partią.

Trzy fizyczne skrzynie 2472 istnieją w `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/world/placements/region-z08-x128-y125.b3`, ale ich top-level entries mają `attrs=null`, bez source UID. Binder wymaga zgodności pozycji, UID i wyglądu: `crates/gameplay/content/src/content/world_reward_claims.rs:894–930` w tym samym pinie. Istniejące claimy oraz ich gotowość zostają bez zmian. Odtworzenie startup UID jest **Left out** zgodnie z [QUESTION MAP-UID-RESTORATION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590297389) i [ANSWER B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590319562).

Nie wywiedziono nowej tożsamości młota z numerycznego suffixu i nie podmieniono istniejącego mapowania itemów. Kwalifikacja źródłowego 3208 i istniejącego claimu musi zachować zasady semantyki itemów i jednej kanonicznej tożsamości: [ANSWER punkty 8–9](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728).

## Global

Historyczne wskazanie strony `Giant Smithhammer Quest`, revid 756268, pochodzi z `Oteryn/Oteryn-Game@0c8b519c78e6c742c11c5c1c8c4c730dac5185d5:docs/agents/evidence/OTV2-20261102-quest-patterns-0.md:223`. Aktualne porównanie pozostaje NIEZNANE/API403; [QUESTION GLOBAL](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099). Cztery elementy inwentaryzacji obejmują istniejący callback i trzy istniejące nagrody, bez deklaracji pełnej grywalności.

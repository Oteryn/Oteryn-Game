# An Uneasy Alliance — pełny tekst Curosa i granice wykonania

## Dane źródłowe

`content/dialogues/quests/an-uneasy-alliance-source.json` zachowuje osiem rekordów wypowiedzi Curosa, łącznie 23 pełne akapity, wraz z liniami i hashem dokładnego bloba. Reprodukcja: `tools/content-schema/quest-authoring/batch_one_dialogue_sources.py --donor <Crystal201189f> --quest an_uneasy_alliance --out content/dialogues/quests/an-uneasy-alliance-source.json --check`. Generator używa istniejącego czytnika literalnych stringów i odmawia przy innych bajtach źródła; nie wykonuje Lua.

Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/curos.lua:58–121`, blob `4fdffd8e90b8de6d2f17ce5c38c45020397c48c9`. Archiwum jest jawnie `source_only`; zapis pełnego tekstu nie przyznaje uprawnienia do zmiany innych questów.

## Left out

| Element | Dokładna przeszkoda | Źródło Crystal |
| --- | --- | --- |
| Curos: przyjęcie i kolejne raporty | Wcześniejsze gałęzie `TheNewFrontier.Questline == 16/18` mają pierwszeństwo nad własnym `mission`; przyjęcie wymaga `Mission06 >= 5` (linia 70). Raporty w liniach 83 i 86 nie powtarzają tego warunku. Nie odłączono cudzych guardów ani nie utworzono drugiego automatu Curosa. | `data-global/npc/curos.lua:58–94` |
| Curos: zakończenie | Przedmiot 10217 i Questline=5 muszą zostać zatwierdzone atomowo. Rozszerzenie wspólnego NPC exchange ownera jest poza partią. | `data-global/npc/curos.lua:95–102` |
| Renegade Orc | Źródło wybiera wszystkich kwalifikujących się damaging players z QuestDoor=0. Istniejący adapter Grizzly kwalifikuje most-damage/shared-XP party; nie jest dowodem równoważnej listy odbiorców. Nie podmieniono tego na last hit ani na samo istnienie `CreatureDeath`. | `data-global/scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua:3–8` |
| Crystal ball AID 40030 | Zmiana 10186→10187 i Questline 3→4 wymaga także przywrócenia po 10 s oraz prezentacji `CONST_ME_ENERGYHIT` (linia 17). Zamknięty callback nie ma trwałego opóźnionego odtworzenia świata; nie wydzielono częściowego, nieodwracanego sukcesu. | `data-global/scripts/quests/an_uneasy_alliance/actions_crystal_ball.lua:1–25` |

Prefiks źródeł w tabeli: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:`. Ograniczenia istniejących ścieżek: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs:74–86,196–245,309–350` oraz `crates/gameplay/quests/src/quest/world_callbacks.rs:84–107`.

Powiązane decyzje: [QUESTION NPC-EXCHANGE-OWNER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162), [QUESTION o granice wspólnego adaptera](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708), [ANSWER punkty 4–7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) i [ANSWER callback](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926). Brak kwalifikacji nie jest nową regułą gry.

## Global i kontrola

Sześć elementów RĘCZNIE ma źródło i konkretną zależność. Aktualna treść `An Uneasy Alliance Quest` pozostaje NIEZNANA/API403; historyczny revid 683776 w raporcie promptu 0 jest materiałem wtórnym. [QUESTION GLOBAL](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099), [ANSWER punkt 1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Nie zadeklarowano pełnej grywalności ani migracji zapisanych torów.

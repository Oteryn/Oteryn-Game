# QMECH — przydział brakujących mechanik (koordynator, 2026-11-08)

Źródło: wszystkie `MISSING-*.md` (241 wierszy) po deduplikacji. Ta tabela jest wiążąca: każda linia robi TYLKO swoje pozycje, w kolejności. Stan zmieniasz w `CLAIMS.md` (CLAIMED -> DONE <sha>). Migracje tylko z własnego zakresu QMECH (2000+).

| id | mechanika | questy | linia | stan |
|---|---|---|---|---|
| M01a | imported-route: natywne wiązanie etapów TALK (`native_dispatch_binding`, `runtime_enabled`) przez `quest_native_binding_preflight.py` + konsument w serwerze | 44 questy z MISSING-questpc-pc2.md (talk) | qengine2 | ASSIGNED |
| M01b | imported-route: wiązanie etapów EXPLORE | 46 questów z MISSING-questpc-pc2.md (explore) | questpc | ASSIGNED |
| M01c | imported-route: wiązanie etapów USE / COLLECT / KILL | 11 questów z MISSING-questpc-pc2.md (use, collect, kill) | questpc3 | ASSIGNED |
| M02 | predykat kolejności stosu Ground (mutable stack/order) | Nomads Land | qengine2 | ASSIGNED |
| M03 | predykaty wejścia do świata: blessing/promotion/Premium | 25 Years of Tibia, Pits of Inferno | qengine2 | ASSIGNED |
| M04 | kanoniczne ukończenie questa przy finalnym oddaniu u NPC (`npc_program.rs` completes=false, brak completed_character_revision) | A Father's Burden, Marlin Trophy, Djinn War Efreet, Djinn War Marid, White Raven Monastery | engnpc | ASSIGNED |
| M05 | receipt NPC: item/points/cooldown (SetNowPlus) | Dangerous Depths | engnpc | ASSIGNED |
| M06 | ukończenie questa-dziecka w transakcji rodzica | Waterskin of Mead | questpc4 | CLAIMED |
| M07 | wiązania etapów lifecycle/trial (offering, repair/burn, puzzle, area-clear) | Lion's Rock, Machinery of War, Dreamer's Challenge, Too Hot to Handle, Realm of Dreams | questpc4 | ASSIGNED |
| M08 | live level/vocation/Premium w programach NPC | Elemental Spheres, Hot Cuisine, Hidden City of Beregar | questpc2 | ASSIGNED |
| M09 | atomowa wielokrotna nagroda (multi-grant MINT) | Rookie Guard, Treasure Hunt, Tower Defence | qready2 | CLAIMED (+Tower Defence) |
| M10 | płatność NPC (carried/bank) + oddanie itemu + efekt magiczny | Koshei, Demon Oak, The Inquisition | qnpcfx | CLAIMED (+Demon Oak, Inquisition) |
| M11 | predykat imienia postaci w dialogu | The Dream Courts | engnpc2 | CLAIMED |
| M12 | repeatable World MINT w epoce questa | Oramond | engitem | CLAIMED |
| M13 | narzędzie questowe na żywym potworze/zwłokach (capture, sample, paint, transform) | Goblin Merchant, Formorgar Mines / Ice Islands (Baby Seal), Targuna (carried tool) | engitem | ASSIGNED |
| M14 | oswajanie obiektu questowego | Targuna | srv7 | CLAIMED |
| M15 | literalny tekst na nagrodzie | Nomads Land | questpc5 | CLAIMED (commit ENGITEM na qmech — sprawdź i oznacz DONE) |
| M16 | osobisty znacznik mapy z dialogu NPC (serwer+klient) | Targuna | questpc5 | ASSIGNED |
| M17 | roczne okno dat dla dialogu NPC + projekcja bieżącego roku | First Dragon, Hot Cuisine | srvwld | CLAIMED (season: RELEASE duplikatu) |
| M18 | czasowy encounter z osobistym cooldownem i sprzątaniem | First Dragon | season | ASSIGNED |
| M19 | zegar transakcji w world callbackach | Bewitched, Threatened Dreams | qready1 | ASSIGNED |
| M20 | crafting: wiele wejść/wyjść, item-na-item, world-tool | Cradle of Monsters, Cursed Crystal, Scatterbrained Sorcerer | qready1 | ASSIGNED |
| M21 | zagnieżdżone kontenery na mapie: pickup/reset + proweniencja klucza | Outlaw Camp, Skeleton Decoration, Small Ruby | npcfix2 | ASSIGNED |
| M22 | przechwycona ilość stosu w transformacjach | The Tainted Souls | npcfix2 | ASSIGNED |
| M23 | przebranie (item-look disguise) + powrót | Wrath of the Emperor, Last Creep Standing | srvgap1108 | CLAIMED |
| M24 | postęp questa ze śmierci bossa/potwora (room recipients, kill counter) | Heart of Destruction, Order of the Stag, Tower Defence, Primal Ordeal (Magma Bubble), A Piece of Cake (Cake Golem), Ancient Tombs (pharaoh) | srvenc | CLAIMED (+Cake Golem, Ancient Tombs) |
| M25 | BossLever Ahau + exit teleporter + zamknięcie callbacków Mitmah Vanguard | Adventures of Galthen | boss2 | ASSIGNED |
| M26 | kwalifikacja pokoi bossów + cooldown Bakragore | Soul War, Rotten Blood | srvgap2 | ASSIGNED |
| M27 | fazy Cults (shared phase controller -> produkcja) | Cults of Tibia | engenc2 | CLAIMED |
| M28 | profil walki Fugue: wyszukaj w internecie (TibiaWiki, Canary `fugue.lua`), zapisz jako source profile wg `docs/reference/content-prep/prey-content-1/creature-source-profiles/` (z URL źródła), potem definicja natywna + rozliczenie śmierci | The Order of the Lion | engenc2 | ASSIGNED (nie OWNER) |
| M29 | Primal Ordeal: etap zwłok z deadline w Instance + odmowa wymagań drużyny | The Primal Ordeal | encx | ASSIGNED |
| M30 | zdarzenia trafienia (spear/magic missile) + śmierci obrońców otwierające drzwi | Child of Destiny, Asura Palace | engenc | ASSIGNED |
| M31 | world change: aktywacja, progi, raidy, journal/recipe lowering | 10 wierszy z MISSING-engworldchg.md | engworldchg | ASSIGNED |
| M32 | world change: liczniki śmierci z warunkiem | Fury Gates, Grimvale | qworld | CLAIMED |
| M33 | wybór world STEP callback po fazie | Feaster of Souls | qworld | ASSIGNED |
| M34 | RewardClaim: rozmieszczenie | Kormarak, Magic Sword, Mastermind Potion, Mystery of the Valley | engclaim | CLAIMED |
| M35 | ukończenie zbiorcze z wielu once-only claimów | Circle Room, Crystal Wand, Demona Ring, Double Hero, Griffin Shield | engclaim | ASSIGNED |
| M36 | nadanie outfitu/addonu z NPC + snapshot outfitu | In Service of Yalahar, Rock Steady / Deepling | engoutfit | ASSIGNED |
| M37 | teleport NPC questa | Paradox Tower, In Service of Yalahar | pgspeed | CLAIMED |
| M38 | obecność NPC warunkowa, woda/stacjonarne, powroty, bilety, Mistrock | MISSING-npcfix.md (5 wierszy) | npcfix | ASSIGNED |
| M39 | oferty NPC `any_topic` + warunek ilości/braku itemu z odmową | Tomes of Knowledge, Into the Bone Pit | questl2 | ASSIGNED |
| M40 | ruch po skosie (serwer+klient) | Unnatural Selection | questl2 | ASSIGNED (po M39) |
| M41 | achievement w transakcji NPC + oddania z proweniencją zwłok | Kilmaresh, The Order of the Stag (finale) | wpkg | ASSIGNED |
| M42 | dispatch planera gotowania UTC | Hot Cuisine | questpc7 | ASSIGNED |
| M43 | USE na stanie questa innego gracza | The Colours of Magic | questpc8 | CLAIMED |
| OWNER | most Cake 48227 typed unavailable decyzją właściciela (W05d.md:18) | A Piece of Cake | — | OWNER |

## PR3056: integracja z main f01f1858 (2026-11-11)

Przydział naprawy migracji: `2740_quest_paid_experience_rewards.sql` jest następcą dla powtarzalnych wymian M39 z potwierdzonym zużyciem przedmiotu i nadaniem XP w tej samej transakcji. Opublikowana migracja1097 pozostaje niezmieniona;2740 musi zachować jej pozostałe ograniczenia i późniejsze rozszerzenia funkcji. Ten przydział nie oznacza zakończonej kwalifikacji PostgreSQL.

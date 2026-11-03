# The Isle of Evil — zakres aktywnej implementacji i Left out

## Callbacki — rejestracja zgodnie z ANSWER6598201888

[QUESTION CALLBACKS-PROOF-HASH](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6592501082) zostało rozstrzygnięte [ANSWER A](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6598201888); wcześniejszy wybór B jest wycofany. Prywatne wejście `tools/content-schema/quest-authoring/batch-one-world-callbacks/the_isle_of_evil_quest.json` zachowuje źródłowe callbacki (liczba rekordów: **1**). Ich rejestracja w `content/quests/world-callbacks.json` należy do końcowego wspólnego commitu. Łączny zakres partii to 28 nowych rekordów z 8 wejść i 20 niezmienionych wcześniejszych rekordów, czyli 48 rejestracji.

Końcowa integracja używa istniejącego `amend_monk_source_receipt.py`, następnie generatorów pochodnych w kolejności narzędzi repo i pełnego `run_checks.py --part all` na zamrożonym SHA. Dowód musi wykazać odtworzenie poprzedniego pliku bajt w bajt po usunięciu wyłącznie 28 dodanych rekordów oraz dozwolony diff pochodnych hashy. Po połączeniu partii koordynator powtórzy tę procedurę na połączonym drzewie; hashy nie scala się ręcznie.

Zgoda na tę rejestrację nie kwalifikuje pełnej mapy, miejsc NPC ani pozostałych mechanik opisanych w Left out. Wcześniejsze wyniki testów nie zastępują walidacji końcowego SHA po tej integracji.

Źródło wszystkich donorowych odnośników poniżej: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Istniejące tory i tożsamości: `Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/quests/missions/quest-state.json:1` oraz istniejące rekordy `content/npcs/definitions/` / `content/dialogues/definitions/` odczytane z tego samego pinu. Nie nadano nowych legacy storage ID.

## Aktywne przejścia

Wykonanie prowadzi istniejący QuestState przez `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs`; selektor questa znajduje się w `crates/gameplay/quests/src/quest/isle_of_evil/` i nie dodaje nowego trwałego właściciela. Każde przejście ma najwyżej trzy efekty i zachowuje limit ośmiu. Nie ma nagród, mintu, zabierania przedmiotów ani pieniędzy.

| Przejście | Warunek źródłowy | Atomowa zmiana | Donor |
| --- | --- | --- | --- |
| accept-fight | Rapanaio, `yes`, topic 7 po pełnej sekwencji rozmowy | Questline = 1; donor nie ponawia guardu stanu w tym topicu | `data-global/npc/rapanaio.lua:183–187` |
| mission-3 | Rapanaio, `mission`, Questline = 3 | Questline = 4, Mission02 = 1 | `data-global/npc/rapanaio.lua:80–87` |
| mission-5 | Rapanaio, `mission`, Questline = 5 | Questline = 6, Mission03 = 1 | `data-global/npc/rapanaio.lua:93–95` |
| mission-7 | Rapanaio, `mission`, Questline = 7 | Questline = 8, Mission04 = 1 | `data-global/npc/rapanaio.lua:101–106` |
| mission-9 | Rapanaio, `mission`, Questline = 9 | Questline = 10, Mission05 = 1 | `data-global/npc/rapanaio.lua:112–124` |
| prepare-departure | Rapanaio, `mission`, Questline = 11 | Questline = 12, Mission06 = 1, Door06 = 1 | `data-global/npc/rapanaio.lua:130–136` |
| mortimer-map | Mortimer, `nautical map`, Mission05 = 1; wykluczona kolizja z wcześniejszą obcą gałęzią | Mission05 = 2, Door05 = 1 | `data-global/npc/mortimer.lua:626–638` |
| begin-final-mission | Rapanaio na wyspie, `mission`, Questline = 13 | Questline = 14, Mission07 = 1, KV `the-isle-of-evil/rapanaio-travel` = true | `data-global/npc/rapanaio_isle_of_evil.lua:84–88` |
| report-evil-master | Rapanaio na wyspie, `mission`, Questline = 14 i Mission07 = 2 | Questline = 15, Mission07 = 3 | `data-global/npc/rapanaio_isle_of_evil.lua:93–99` |

JSON przechowuje pełne wypowiedzi używanych gałęzi, także wszystkie akapity. Selekcja zachowuje literalne MsgContains, kolejność dwóch niezależnych bloków Rapanaio, stan topicu dokładnej rozmowy oraz guards źródła. Dialog nie emituje tekstu sukcesu, jeżeli cała powiązana operacja jest wstrzymana.

## Kwalifikacja istniejących NPC

Aktywowano dokładnie pięć istniejących tożsamości: `rapanaio`, `rapanaio_boat`, `rapanaio_isle_of_evil`, `scutty`, `mortimer`. Ich powitania są bezwarunkowe: odpowiednio `rapanaio.lua:265`, `rapanaio_boat.lua:156`, `rapanaio_isle_of_evil.lua:123`, `scutty.lua:136`, `mortimer.lua:650`. Brak własnego farewell/walkaway jest uzupełniony dokładnym domyślnym źródłem `data/npclib/npc_system/npc_handler.lua:82,90`; podstawienie `|PLAYERNAME|` korzysta z istniejącej kwalifikowanej ścieżki obu callerów. Nie dodano banku ani nowych usług. Istniejące usługi handlowe Mortimera pozostają u dotychczasowego właściciela.

**Kwalifikacja miejsca jest osobna od adaptera.** `Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/world/npc-placements/index.json:1` odsyła do jednego sharda 33 rekordów, a `placements-00000-00032.json:1` nie zawiera pięciu tożsamości Isle. `held.json:1` jest pusty; `crates/gameplay/content/src/content/project/native_entry_room.json:1` też ich nie wiąże. Źródłowe miejsca istnieją w `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/world/world-npc.xml:558–559,1441–1442,3011–3012,3185–3189`. Fizyczne placementy pozostają Left out: [QUESTION TARGUNA-ISLE-NPC-PLACEMENT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590683986) i zaakceptowany zakres [ANSWER §5/7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Testowy `place_npcs` nie jest produkcyjnym dowodem mapy.

King Tibianus nie otrzymuje nowej kwalifikacji powitania, ponieważ jego greet i wcześniejsze reguły dotyczą także obcych zadań. Tekst dwóch akapitów własnej nagrody jest zachowany w źródłowym JSON, lecz gałąź nie jest aktywna.

## Fizyczne placementy — przyjęte Left out

[QUESTION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590683986) otrzymało [ANSWER B, §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985): nie istnieje zaakceptowany mapowy cohort tych jedenastu tożsamości. Ta partia dostarcza podłączone adaptery i testy publicznych callerów; fizyczne placementy i deklaracja pełnej grywalności pozostają do osobnego mapowego PR. Nie powstaje syntetyczny NPC w produkcji, nowy WorldId ani capability.

Źródło każdej współrzędnej poniżej: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/world/world-npc.xml`; x,y,z są źródłowe. Każda instancja jest niewykazana w przyjętym shardzie Oteryn `placements-00000-00032.json` i pustym `held.json`, zgodnie z primary-read bazy `39f104c39e844e523c43158e0c46370fb40fbf5f` opisanym w QUESTION (wcześniejszy odczyt bazy798e daje ten sam brak).

| NPC | Źródłowe x,y,z | world-npc.xml |
| --- | --- | --- |
| Rapanaio — Isle of Evil | `32667,31454,6` | `558–559` |
| Mortimer | `32500,31627,7` | `1441–1442` |
| Scutty | `32574,31915,12` | `3011–3012` |
| Rapanaio | `32683,31987,14` | `3185–3186` |
| Rapanaio — Boat | `32700,31992,14` | `3188–3189` |

## Jawne KV i donor bug fix

Zachowano 11 istniejących torów legacy i ich granice. Dwa dodatkowe tory odwzorowują już istniejące donorowe booleany w jawnej przestrzeni `oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/`: `rapanaio-travel` i `mission04-mustpay`. Nieobecne/false = 0; true = 1. Pierwszy ma źródłowy zapis w `rapanaio_isle_of_evil.lua:88`; drugi jest wyłącznie odczytem potrzebnym do źródłowej gałęzi Scutty'ego, bez nowego writera kary.

**Donor bug fix:** `scutty.lua:81,116–117` wskazuje `Mission04MustPay` / `Mission04WormCount`, których nie ma w tabeli `data-global/lib/core/storages.lua:1573–1585`. Plik `data-global/scripts/quests/the_isle_of_evil_quest/actions_heated_worm_punisher.lua:4–5,34,46,55–56` zapisuje już te same flagi jako KV `mission04-wormcount` i `mission04-mustpay`. Odczyt gałęzi kary został skierowany wyłącznie do dokładnego istniejącego KV, z testem false/true. Płatność i reset są nadal wstrzymane. Podstawa: [QUESTION #6589174504](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589174504) i [ANSWER §3–5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728).

## Left out

| Element | Konkretny brak / decyzja | Donor | QUESTION / ANSWER |
| --- | --- | --- | --- |
| Rapanaio topics 8–13 | Atomowe wydanie/odebranie przedmiotów wraz ze stanem: wędka 9306, ryba 9307, rum 2874 subtype13 ×10, sok subtype14, maszyna 9390, mapa 9308. W tej partii nie wolno rozszerzać wspólnego NPC exchange ownera. | `data-global/npc/rapanaio.lua:190–253` | [NPC exchange](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162), [ANSWER §4–5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Scutty: część i ukończenie maszyny | Wydanie 9234 lub 9390 połączone ze stanem wymaga tego samego atomowego właściciela. Brak częściowego sukcesu. | `data-global/npc/scutty.lua:89–93,108–113` | [NPC exchange](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274162), [ANSWER §5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Scutty: kara 500 gold i reset | Brak zatwierdzonego złożenia odbioru opłaty, resetu liczby robaków i boolean KV. Nie zapisuje się częściowego resetu. | `data-global/npc/scutty.lua:116–124` | [KV](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589174504), [ANSWER §4–5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Pierwsza podróż łodzią i powrót | Teleport z własną zmianą stanu wymaga kwalifikacji istniejącej ścieżki podróży; nie zbudowano nowego połączonego właściciela transportu/QuestState. | `data-global/npc/rapanaio_boat.lua:130–139`; `rapanaio_isle_of_evil.lua:110–115` | [granice wspólnych ownerów](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Powtórna podróż topic9 | Źródłowa gałąź topic9 leży wewnątrz warunku topic1..8; pozostaje nieosiągalna. Nie zgaduje się naprawy zakresu przy brakującej kwalifikacji podróży. | `data-global/npc/rapanaio_boat.lua:102,137–139` | [ANSWER §3,5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| King Tibianus: karta i achievement | Własny finał potrzebuje atomowego item9389 + Questline/Mission07 + achievement; wcześniejsze guards/powitanie dotyczą także obcych questów. Brak fałszywego zakończenia. | `data-global/npc/king_tibianus.lua:220–232,494` | [placement](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274971), [ANSWER §4–6](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Mortimer: mieszana wiadomość lub obcy topic | Dozwolona własna gałąź na topic0 bez wcześniejszego obcego słowa. Wiadomości nakładające się na Explorer Society/New Frontier nie otrzymują przejścia Isle; nie importowano obcego automatu rozmów. | `data-global/npc/mortimer.lua:58–625,626–638,653–690` | [ANSWER §6](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |
| Pozostałe world callbacki, heated worm, kill i pełna mapa | Osobny zakres root; ten pakiet NPC nie deklaruje ich wykonania ani pełnej grywalności całego questa. Nowe pola world callbacków wyłączone dla partii. | sześć plików `data-global/scripts/quests/the_isle_of_evil_quest/`, pełna inwentaryzacja w zbiorczym raporcie | [ANSWER callback](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926) |
| Tibia Global | API MediaWiki HTTP403; brak nowego porównania i nowych elementów Global-only. | historyczny wiersz prompt0; bieżący wynik NIEZNANE | [Global API](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099), [ANSWER §2](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728) |

## Testy i wynik

Rzeczywisty semantyczny RED z odłączonym własnym cohortem dał **6 PASS / 3 FAIL**, exit101; GREEN po przywróceniu rejestracji dał **9 PASS / 0 FAIL**, exit0. Dowód: `integration2-receipts.json`, przypadki `isle-red` oraz `isle-green`. Trzy testy wykrywają brak rejestracji w publicznym `load_embedded_quest_state`; pozostałe sześć sprawdza reguły selektora. Jest to wykonanie na roboczym checkoutcie, nie kwalifikacja zamrożonego końcowego SHA.

Przygotowano również dwa testy rzeczywistego `NpcDataCatalogue` i `NpcTalkBook::from_model` dla pięciu NPC, źródłowego farewell/walkaway oraz braku automatycznego granta przez `yes`. Dostarczone odpisy wyników nie zawierają ich osobnego uruchomienia, więc nie nadają im statusu PASS. Wspólny wykonawca `quest_dialogue` ma **9 PASS / 0 FAIL**, exit0, w przypadku `shared-reply-green` z `caller2-receipts.json`; potwierdza to wspólne zasady publikacji/tekstu, nie fizyczne placementy ani osobny test każdego NPC Isle.

Lokalnie sprawdzono generator, składnię Pythona, poprawność JSON, ścieżki include i aplikowalność prywatnego patcha. Wyniki opisane powyżej nie zastępują końcowych fmt/clippy/test, checks treści i kwalifikacji dokładnego zamrożonego heada, które raportuje root.

## Granica commitów i kwalifikacji

[ANSWER §2](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985) wymaga prywatnego commitu `quests(1): <quest>` jako AUTHORING, bez deklaracji runtime. Wspólny helper ma osobny wcześniejszy commit; minimalne rejestracje modułów, publicznego loadera i NPC routingu trafiają do jednego końcowego commitu z listą bloków per quest. Dopiero zamrożony końcowy SHA otrzymuje pełną kwalifikację. Parser world callbacków nie jest rozszerzany.

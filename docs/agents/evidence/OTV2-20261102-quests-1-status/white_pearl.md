# White Pearl — powrót i warunkowe wejście

## Callbacki — rejestracja zgodnie z ANSWER6598201888

[QUESTION CALLBACKS-PROOF-HASH](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6592501082) zostało rozstrzygnięte [ANSWER A](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6598201888); wcześniejszy wybór B jest wycofany. Prywatne wejście `tools/content-schema/quest-authoring/batch-one-world-callbacks/white_pearl.json` zachowuje źródłowe callbacki (liczba rekordów: **1**). Ich rejestracja w `content/quests/world-callbacks.json` należy do końcowego wspólnego commitu. Łączny zakres partii to 28 nowych rekordów z 8 wejść i 20 niezmienionych wcześniejszych rekordów, czyli 48 rejestracji.

Końcowa integracja używa istniejącego `amend_monk_source_receipt.py`, następnie generatorów pochodnych w kolejności narzędzi repo i pełnego `run_checks.py --part all` na zamrożonym SHA. Dowód musi wykazać odtworzenie poprzedniego pliku bajt w bajt po usunięciu wyłącznie 28 dodanych rekordów oraz dozwolony diff pochodnych hashy. Po połączeniu partii koordynator powtórzy tę procedurę na połączonym drzewie; hashy nie scala się ręcznie.

Zgoda na tę rejestrację nie kwalifikuje pełnej mapy, miejsc NPC ani pozostałych mechanik opisanych w Left out. Wcześniejsze wyniki testów nie zastępują walidacji końcowego SHA po tej integracji.

## Zaimplementowane dane

Dodano `oteryn:quest-callback/white-pearl-exit`: StepIn AID 5631 przenosi gracza do (33147,32864,-7), bez flagi postępu i bez konsumpcji. Jest to odwzorowanie `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/white_pearl/movements_white_pearl.lua:7–11,15–24,35–46` w istniejącym schemacie. Brak kwalifikowanego źródła lub celu mapy nadal powoduje odmowę istniejącego executora.

Identyfikator grupujący `oteryn:quest.white_pearl` w callbacku nie stwierdza istnienia osobnej strony ani osobnego questa Global. Nie dodano nagrody perły, storage ani misji niewystępującej w tym źródle.

## Left out

AID 5630 sprawdza obecność 3465 w (33145,32862,7). Gdy przedmiot istnieje, przenosi do (33151,32864,7); w przeciwnym razie do (33145,32863,7). Garnek nie jest zużywany. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/white_pearl/movements_white_pearl.lua:2–6,26–36`.

Oba warianty wejścia pozostają **Left out**: wymagają odczytu stanu konkretnego pola bez konsumpcji i wyboru dwóch celów. `Offering` konsumuje ofiarę; użycie go zmieniłoby źródłową regułę. Zamknięty callback nie reprezentuje takiego warunku świata: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/quests/src/quest/world_callbacks.rs:47–60,84–107,175–180`. [QUESTION QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708), [ANSWER punkt 3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926) pozostawia rozszerzenia wspólnemu PR.

Źródłowa prezentacja `CONST_ME_MAGIC_GREEN` również nie ma nowego pola w tym schemacie; zachowano ją w inwentaryzacji, bez deklarowania jej wykonania. Źródło: `movements_white_pearl.lua:4,9,30,36` pod pinem Crystal powyżej. Mechanika powrotu i efekt wizualny mają oddzielny status.

## Global i kontrola

Trzy wiersze inwentaryzacji: jeden importowany powrót oraz dwa warunkowe warianty wejścia. Powiązanie z osobnym tytułem Global jest NIEZNANE; [QUESTION GLOBAL](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099), [ANSWER punkt 1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Nie tworzono danych tylko z Global.

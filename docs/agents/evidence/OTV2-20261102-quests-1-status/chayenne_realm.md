# Chayenne Realm — końcowy zakres partii 1

## Callbacki — rejestracja zgodnie z ANSWER6598201888

[QUESTION CALLBACKS-PROOF-HASH](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6592501082) zostało rozstrzygnięte [ANSWER A](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6598201888); wcześniejszy wybór B jest wycofany. Prywatne wejście `tools/content-schema/quest-authoring/batch-one-world-callbacks/chayenne_realm.json` zachowuje źródłowe callbacki (liczba rekordów: **1**). Ich rejestracja w `content/quests/world-callbacks.json` należy do końcowego wspólnego commitu. Łączny zakres partii to 28 nowych rekordów z 8 wejść i 20 niezmienionych wcześniejszych rekordów, czyli 48 rejestracji.

Końcowa integracja używa istniejącego `amend_monk_source_receipt.py`, następnie generatorów pochodnych w kolejności narzędzi repo i pełnego `run_checks.py --part all` na zamrożonym SHA. Dowód musi wykazać odtworzenie poprzedniego pliku bajt w bajt po usunięciu wyłącznie 28 dodanych rekordów oraz dozwolony diff pochodnych hashy. Po połączeniu partii koordynator powtórzy tę procedurę na połączonym drzewie; hashy nie scala się ręcznie.

Zgoda na tę rejestrację nie kwalifikuje pełnej mapy, miejsc NPC ani pozostałych mechanik opisanych w Left out. Wcześniejsze wyniki testów nie zastępują walidacji końcowego SHA po tej integracji.

Quest/katalog: `chayenne_realm`. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Zastany Oteryn: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f`. Dokument uzupełnia [raport A–D partii 1](../OTV2-20261102-quests-1-20261102.md), nie tworzy dodatkowego wiersza B.

Inwentaryzacja: **4 elementów = 0 AUTO + 4 RĘCZNIE**. Jest to klasyfikacja możliwości importu, a nie liczba aktywnych funkcji. Bieżący zakres po decyzjach opisano poniżej.

Pytania wspólne o właścicieli, mapę i zakres są wskazane jako podstawa zastosowania ogólnych reguł koordynatora; nie oznaczają, że każde z nich pierwotnie dotyczyło tego konkretnego questa. Nie są zadawane ponownie. Wszystkie odpowiedzi uwzględniono do komentarza [ANSWER #6591052733](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591052733) włącznie.

## Dostarczony zakres

Do `content/quests/world-callbacks.json` przygotowano **sam teleport wyjścia**: StepIn AID55024 → `(33117,32604,floor=-6)`, bez kosztu, storage i dodatkowego warunku. Komunikat `Goodbye, dream traveller...` nie jest wykonywany przez ten wpis i pozostaje osobnym Left out. Źródło obu części: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/movements_enter_realm.lua:19–23`.

Prywatny commit przechowuje wejście `tools/content-schema/quest-authoring/batch-one-world-callbacks/chayenne_realm.json`. Wpis runtime generuje `batch_one_world_callbacks.py` dopiero podczas końcowej integracji wspólnej.

Wpis wykorzystuje istniejący format oraz kanoniczne `oteryn:quest.chayenne_realm`, bez nowych pól parsera. Zastany rekord: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/quests/definitions/quests-00100-00199.json:1` (rekord20). Dawna nieaktywna recepta `CHOSEN_OTERYN_APPROXIMATION` z 12 zabójstwami nie jest źródłem nowej reguły. Brak kwalifikowanego pola nadal powoduje odmowę wykonawcy; wpis nie przyznaje nowej mapy. Istniejący wykonawca: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:256–264`.

## Left out

| Element | Konkretna granica i źródło | QUESTION + ANSWER |
| --- | --- | --- |
| Tekst po wyjściu AID55024 | `success_message` wymaga nowego pola parsera. Teleport jest osobnym przygotowanym elementem; tekst nie został zastąpiony wymyśloną wiadomością. `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/movements_enter_realm.lua:23`. | [QUESTION — QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708); [ANSWER §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926). |
| Wejście AID55022 | Wymagane są jednocześnie carried14682 i level≥40, bez zużycia klucza, z powrotem na fromPosition po odmowie. StepIn nie ma kwalifikowanego fizycznego carried-only guardu; ItemOnTarget konsumuje/zmienia przedmiot w innej ścieżce. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/movements_enter_realm.lua:9–17`; model: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/quests/src/quest/world_callbacks.rs:62–68,194–205`. | [QUESTION — QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708); [ANSWER §3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926) oraz [ANSWER §5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Dźwignia AID55021 | Wymaga WORLD30056, carried14682, cooldown300s i odtworzenia2129 po60s. Ten stan świata i opóźniony efekt nie są obsługiwane istniejącym callbackiem. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/actions_lever.lua:3–24`. | [QUESTION — CHAYENNE-LEVER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589175311); [QUESTION — wspólny owner świata/Instance](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589185913); [ANSWER §3,5](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |
| Skrzynia AID55023 | Bundle5949 z dziećmi16244,3659,9034,3027,5882,5791,2995,6570 po1; storage30033<1→1. Skrypt daje tylko AID; binder wymaga qualified position+UID+appearance. Nie wymyślono tych danych ani nowej wypłaty. Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/actions_reward.lua:2–19`; binder: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:34–62`. | [QUESTION — kwalifikacja mapy FQ10](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589184566); [QUESTION — TARGUNA-REWARD-PLACEMENT](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589274971); [ANSWER §4,5,7](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). |

Zacieniony `item` dźwigni jest jednoznacznym błędem donora: lokalna zmienna wskazuje usuniętą ścianę, a końcowe `transform(2773)` powinno dotyczyć użytej dźwigni; sąsiednia gałąź odwraca2773→2772. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/chayenne_realm/actions_lever.lua:3–4,15–24`. Zgoda na tę naprawę pochodzi z pytania CHAYENNE-LEVER i ANSWER§3, ale sam fix nie jest deklarowany jako wdrożony, gdy cały wymagany owner pozostaje held.

## Walidacja

Zweryfikowano trzy dokładne bloby źródłowe. Teleport korzysta ze starego schematu; nie ma aktywnego `success_message`, nullable storage ani nowego triggera. Końcowa walidacja loadera i rzeczywistego callbacku należy do zamrożonego SHA partii; ten dokument nie stwierdza jej przyszłego wyniku.

## Global i status kwalifikacji

Strona identyfikacyjna: **Realm of Dreams Quest**. Bieżące porównanie i revid pozostają **NIEZNANE/API403**. Historyczne revid683951/999253 w katalogu są źródłami wtórnymi, bez nowego odczytu ich treści. Nie pobierano HTML, nie obchodzono blokady i nie dodano danych wyłącznie z Global. Ścieżka decyzji: [QUESTION — GLOBAL-API](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099); [ANSWER §1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Różnica między dwoma plikami Crystal albo między Crystal a dawną receptą Oteryn nie jest deklarowana jako różnica względem Global.

Rejestracje wspólne są kwalifikowane na końcowym SHA partii: [QUESTION — PER-QUEST-COMMITS-ROUTING](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590518467); [ANSWER §2 — wcześniejsze heady AUTHORING](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590727985). Obecność danych, testu lub source modelu nie oznacza pełnej grywalności ani ukończonego testu zintegrowanego serwera.

Zgodnie z [QUESTION — AUTHORING-PRE-PUSH-CHECKS](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591017221) i [ANSWER A z doprecyzowaniem](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6591052733) pełne kontrole są wymagane przed rozpoczęciem sekwencji oraz na zamrożonym końcowym SHA. Prywatny commit zawiera wyłącznie niepodpięte dane/kod i własne wejście generatora; nie zawiera rejestracji callbacku, zmiany wspólnego kodu ani generowanych indeksów. Wynik sprawdzenia roboczego checkoutu nie jest kwalifikacją przyszłego heada.

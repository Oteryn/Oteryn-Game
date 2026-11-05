# Deeper Fibula — zakres partii 1

## Callbacki — rejestracja zgodnie z ANSWER6598201888

[QUESTION CALLBACKS-PROOF-HASH](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6592501082) zostało rozstrzygnięte [ANSWER A](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6598201888); wcześniejszy wybór B jest wycofany. Prywatne wejście `tools/content-schema/quest-authoring/batch-one-world-callbacks/deeper_fibula.json` zachowuje źródłowe callbacki (liczba rekordów: **1**). Ich rejestracja w `content/quests/world-callbacks.json` należy do końcowego wspólnego commitu. Łączny zakres partii to 28 nowych rekordów z 8 wejść i 20 niezmienionych wcześniejszych rekordów, czyli 48 rejestracji.

Końcowa integracja używa istniejącego `amend_monk_source_receipt.py`, następnie generatorów pochodnych w kolejności narzędzi repo i pełnego `run_checks.py --part all` na zamrożonym SHA. Dowód musi wykazać odtworzenie poprzedniego pliku bajt w bajt po usunięciu wyłącznie 28 dodanych rekordów oraz dozwolony diff pochodnych hashy. Po połączeniu partii koordynator powtórzy tę procedurę na połączonym drzewie; hashy nie scala się ręcznie.

Zgoda na tę rejestrację nie kwalifikuje pełnej mapy, miejsc NPC ani pozostałych mechanik opisanych w Left out. Wcześniejsze wyniki testów nie zastępują walidacji końcowego SHA po tej integracji.

## Zaimplementowane

Dopisano `oteryn:quest-callback/deeper-fibula-well` do istniejącego formatu `content/quests/world-callbacks.json`: Use AID15005 przenosi do (32172,32439,-8). Dane nie wymagają zmiany parsera ani właściciela trwałego stanu. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/deeper_fibula/action-draw_well.lua:3–9`. Wywołanie nadal odmawia, jeśli bieżąca mapa nie dostarcza prawidłowego źródła lub celu.

## Istniejące dane i Left out

Pięć nagród ma już kanoniczne claimy `oteryn:reward-claim.kv.deeperfibula1`…`deeperfibula5`; obiekt klucza ma `oteryn:reward-claim.quest.key.id3980`. Nie utworzono drugiego claimu ani drugiej ścieżki odbioru.

| UID | Pozycja donora | Nagroda źródłowa | Stan istniejących danych |
| --- | --- | --- | --- |
| 6008 | 32239,32471,10 | 3428 ×1 | ready |
| 6009 | 32239,32478,10 | 3369 ×1 | ready |
| 6010 | 32233,32491,10 | 3097 ×1 | ready |
| 6011 | 32245,32492,10 | 3082 ×1 | waiting_item_semantics; hold zachowany |
| 6012 | 32256,32500,10 | 3318 ×1 | ready |
| 5000 | 32219,32401,10 | 2972 ×1; klucz3980 | authored variant, waiting_implementation |

Źródła tabeli: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:370–376,639–673`; istniejące dane `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:content/interactions/reward_claims/reward-claims-00000-00099.json:1` rekordy28–32 oraz `content/interactions/reward_claims/reward-claim-variants-00000-00099.json:1` rekord47. Komentarz3496 w donorze nie zastępuje wykonanego `Storage.Quest.Key.ID3980`.

Wszystkie sześć fizycznych obiektów istnieje w regionach `content/world/placements/region-z10-x125-y126.b3` i `region-z10-x126-y126.b3`, lecz ich top-level entries mają `attrs=null`, bez `unique`. Odczyt wykonano dekoderem `tools/content-schema/world-authoring/world_region_codec.py` na tym samym pinie Oteryn. Binder wymaga pozycji, UID i wyglądu: `crates/gameplay/content/src/content/world_reward_claims.rs:894–930`; sam zapis readiness=ready nie potwierdza możliwości odbioru.

**Left out:** odtworzenie źródłowych UID mapy, kwalifikacja holdów itemu3082 oraz klucza. [QUESTION MAP-UID-RESTORATION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590297389), [ANSWER B](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6590319562) oraz [ANSWER §7–9](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Nie zmieniono mapy, istniejących stanów postaci ani kontraktów claimów.

## Global i weryfikacja

Aktualna treść TibiaWiki pozostaje NIEZNANA/API403; [QUESTION](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099), [ANSWER §1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Szczegółowy raport partii zachowuje historyczny revid jako źródło wtórne. Wyniki walidacji danego head są w opisie DRAFT PR #2845; sam ten dokument nie jest dowodem grywalności całego questa.

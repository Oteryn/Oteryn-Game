# Uzupełnienie zależności potworów

Batch przygotowuje kandydat do importu z przypiętych Canary/Crystal i zachowuje oryginalne wejścia. Zamyka referencje całego wcześniejszego katalogu 1660 stworzeń, dodaje 36 wymaganych aktorów/form i przygotowuje 83 Encounter. Wynik: 1696 stworzeń, zero wstrzymanych referencji. Nie jest to aktywacja świata ani dowód pełnego gameplayu.

## Jedno polecenie

Z katalogu repozytorium, w przygotowanym workspace:

```bash
/workspace/audit-venv/bin/python tools/content-migration/complete_creature_dependencies.py \
  --canary /workspace/monster-reference-sources/canary \
  --crystal /workspace/monster-reference-sources/crystal \
  --bundles /workspace/monster-round7-output/bundles \
  --index /workspace/monster-round7-output/population-bundles-canary-47dfd51f.json \
  --stage /workspace/monster-round7-output/creature-admission-stage.json \
  --item-map /workspace/monster-round7-output/native-item-map-rust.json \
  --output /workspace/monster-dependency-output/new-completion-run
```

Wymagany nowy/pusty output. Przy ponownym wykorzystaniu już przygotowanych komponentów można dodać `--components /workspace/monster-dependency-output --encounter-components /workspace/monster-dependency-output/encounter-bundles-v3`. To oszczędza ponowną konwersję donorów. Nie zmieniać współdzielonych plików wejściowych; uzupełnienia są niezależnymi kopiami. Niezmienione pliki korzystają z hard linków, gdy system plików pozwala; w pozostałych przypadkach z kopii. Dane bazowe są zabezpieczone w draft #1533.

## Co zostało poprawione

- Przygotowano brakujące Creature z exact registered source names, w tym Bone Bear ze źródłem Crystal; rekurencyjnie domknięto aktorów Encounter i ich summonów.
- Grand Mother Foulscale: źródłowe skrypty obu serwerów potwierdzają `Dragon Hatchling`, więc poprawiono tylko jej błędną referencję `Dragon Hatchlings`; nie utworzono aliasu.
- Knight/Monk/Paladin Familiar: dodano konkretne owned SummonCreature Ability; zachowano oryginalne `is_familiar`, manę i duration. Gating gracza, cooldown, personalizacja i expiry pozostają do integracji.
- Candy Horror: zachowano corpse48267 i loot. Oba źródła odwołują się do nieistniejącego48296; transform w donorach pozostawia zwłoki. Jawna aproksymacja pomija pięciosekundowy timer bez skutecznej transformacji; nie tworzy Itemu ani aliasu.
- Professor Maxxen: pominięto wyłącznie nieistniejący summon `Glooth Smasher`, zachowując trzy pozostałe. Nie utożsamiono go z `Glooth Masher`.
- Seacrest Serpent: pominięto wyłącznie `defense-2` melee odrzucane przez natywny scheduler; zachowano ataki, statystyki, loot i oryginalną definicję Ability. Arena może sprawdzać cały kandydat bez osobnych wariantów `.lab.`.

## Flagi i granice

`completion-quality.json` zachowuje źródła, wszystkie pominięte oryginalne wpisy i różnice. `population-index.json`, `creature-admission-stage.json` oraz inventory/catalog areny przenoszą flagi aproksymacji. Nieobsługiwane callbacki/custom spelle nowych aktorów są jawnie pominięte, zamiast uznawane za wykonane. Wszystkie już zakodowane reguły 83 Encounter pozostają zachowane. Dwa częściowe scenariusze nadal pomijają SW6 tworzenia pól oraz Ferumbras quest/global crystal reset. To kandydaty z oznaczonymi ograniczeniami, nie pełne odwzorowanie Global.

Statystyki i loot wszystkich 1660 oryginalnych stworzeń zachowują poprzednie dane. Braki mitygacji/Bestiary pozostają flagami; nie są zastępowane zerem. Dla nowych aktorów donor jest hipotezą OTS, nie nowym potwierdzeniem Global.

Źródła odczytano z przypiętych publicznych Git caches oraz wcześniejszych zweryfikowanych zapisów wiki. Nowy research wiki był niedostępny: Tavily limit432, BR403, Fandom402; wszystkie urządzenia Remote Desktop były offline. Odpowiedź Tibiopedii była stroną Setup, nie użyto jej jako danych potwora. Nie wykonano żadnych działań projektowych przez Remote Desktop.

Natywny stage jest kompletnym przygotowanym wejściem; obecny produkcyjny materializer ma historyczne przypięte wejście/counts. Kontrolowane przyjęcie nowego snapshotu, wymagane CI/review i runtime activation należą do integracji pod #162. Nie osłabiono jego pinów ani reguł E3/E4. Arena bez klienta bada typowane profile, schedule/retry/replay i HP/obrażenia/śmierć/despawn; pełne efekty spellów, summon installation, loot commit i respawn pozostają niezakwalifikowane.

## Kolejny batch: loot i rdzenie czarów

`monster-source-gaps-20261002` zawiera 190 korekt szans/ilości oraz 325 nowych wpisów lootu w 31 stworzeniach. Wszystkie ItemID mają istniejące zaakceptowane mapowania. 158 dodatków pochodzi z mniej niż dziesięciu zaobserwowanych dropów i zachowuje flagę `LOW_CONFIDENCE_WIKI_LOOT`; prawdopodobieństwa są oszacowaniami wiki. 16 wpisów dzieli szansę między jawnie wskazane warianty według istniejącej reguły. Zachowano źródła, poprzednie wartości, rewizje wiki, liczbę obserwacji i regułę zaokrąglenia.

Przywrócono pięć źródłowych harmonogramów dla czterech czarów: generator, maxxenteleport, time guardian lost time i zamulosh tp. Trzy następcze Encounter zachowują wszystkie poprzednie reguły. Generator wymaga rzeczywistego Glooth-Generatora, więc dodano go z przypiętego Canary. Jego własny opóźniony summon/usunięcie pozostaje oznaczoną pominiętą mechaniką. Z wcześniejszych 49 pominiętych wpisów nowych aktorów przywrócono pięć; pozostaje 44 oraz jeden wpis nowego aktora. Nie są to wszystkie ograniczenia całego katalogu: osobne flagi callbacków, familiarów i encounterów nadal obowiązują.

Kandydat: 1697 stworzeń, 83 Encounter, 17 841 wpisów lootu. Wszystkie 804 stworzenia z Bestiary mają mitygację. Historyczny kandydat 1696 miał 192 braki Bosstiary i 451 braków bez Bestiary/Bosstiary; nowy Glooth-Generator dodaje jeden brak. Nie wpisano zer ani wartości zwykłego potwora do podobnie nazwanego wariantu historycznego. Pancerz i prędkość Dark Merudri pozostają wcześniejszymi wartościami szablonowymi, nie potwierdzonymi wartościami Global.

Powtórzenie na rozpakowanym snapshotcie poprzedniego kandydata (zmienne oznaczają ścieżki przygotowane przez operatora):

```bash
python tools/content-migration/prepare_loot_completion.py \
  --baseline "$prior/bundles" --index "$prior/population-index.json" \
  --packet "$evidence/loot/loot-patch-proposals.json" --output "$work/loot/patched-bundles"
python tools/content-migration/prepare_spell_core_completion.py \
  --repo "$repo" --baseline "$prior" --encounters "$previous_encounters" \
  --canary "$canary" --output "$work/spells"
python tools/content-migration/prepare_glooth_generator.py \
  --repo "$repo" --canary "$canary" --output "$work/spells"
python tools/content-migration/complete_monster_source_gaps.py \
  --baseline "$prior" --loot "$work/loot" --spells "$work/spells" \
  --encounters "$previous_encounters" --item-map "$item_map" --output "$work/population"
```

Każdy output ma być nowy. Niezmienione paczki korzystają z read-only hard linków; zmienione są kopiami. Łączenie zachowuje osobne zmiany lootu i czarów również wtedy, gdy oba dotyczą tego samego potwora. Pakiet poprawek jest związany SHA poprzedniego indeksu, a każda paczka własnym digestem.

Bieżący odczyt źródeł: publiczne przypięte cache Git Canary/Crystal i wcześniej zweryfikowane rewizje Fandom. Tavily zwracał wymaganie ponownego połączenia, Remote Desktop miał wszystkie trzy urządzenia offline; nie deklarujemy nowego odczytu Global/wiki. Arena potwierdza natywne przygotowanie harmonogramów i fixture HP/śmierci, a nie wszystkie efekty czarów i dropy. Produkcyjny snapshot oraz aktywacja nadal wymagają integracji.

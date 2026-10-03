# Wspólne laboratorium potworów

Jedno polecenie sprawdza cały katalog danych i uruchamia arenę **bez klienta, na natywnych komponentach**. Wykorzystuje istniejący `native_entry_room.json`: gracz i jeden potwór zajmują dwa sąsiednie, jawnie przechodnie pola. Kolejne potwory korzystają z tego samego pokoju. To nie jest test pojemności kanału ani przestrzenne sprawdzenie dużych AoE.

## Uruchomienie w przygotowanym workspace

```bash
/workspace/audit-venv/bin/python tools/monster-lab/run.py inventory \
  --config /workspace/monster-lab/lab-config.json \
  --output-dir /workspace/monster-lab/runs/inventory-01

/workspace/audit-venv/bin/python tools/monster-lab/run.py arena \
  --config /workspace/monster-lab/lab-config.json \
  --output-dir /workspace/monster-lab/runs/arena-01 \
  --training --ticks 30
```

Uruchamiać z głównego katalogu repozytorium. Każdy przebieg z `--training` wymaga nowego katalogu wyników; wejścia i poprzednie raporty pozostają zachowane. Bez `--training` testowane są oryginalne przyjęte profile. `--monster rat` wybiera jeden potwór; argument można powtórzyć. Z `--training --monster seacrest_serpent` wybierany jest oznaczony wariant testowy. `--seed` przyjmuje 0–255, `--ticks` 1–600.

`lab-config.json` wskazuje istniejące cache, bundle index/stage, Item map i wspólny target Cargo. Można wskazać inne katalogi bez kopiowania źródeł na każdego workera. Przykład przenośnej konfiguracji znajduje się w `config.example.json`. Same dane do inventory/treningowych profili są w zweryfikowanym archiwum zabezpieczonym w PR #1533; po rozpakowaniu trzeba ustawić odpowiednie ścieżki konfiguracji. Testy natywne wymagają toolchainu repozytorium i jego zależności, Python training wymaga istniejącej venv z jsonschema/lupa.

## Co wykonuje arena

1. Weryfikuje cztery pliki i digest wszystkich 1660 paczek oraz rozłączność przyjętych/wstrzymanych stworzeń.
2. Wczytuje istniejącą mapę, sprawdza jej SHA, pozycje i jawne deklaracje kolizji. Drzwi zależne od stanu nie są miejscem spawnu.
3. Tworzy testowego właściciela `ChannelRuntimeV1`, prawdziwą sesję gracza i uchwyt stworzenia w tym samym kanale. Ustawia aktorów w pozycjach istniejącego pokoju.
4. Dla wybranych profili wykonuje rzeczywisty `ProfileScheduleState` i przygotowanie propozycji Ability/summon/melee przez wskazaną liczbę ticków. Weryfikuje retry tej samej operacji i identyczny replay od początku.
5. Dla każdego wybranego profilu wykonuje natywną próbę właściciela z HP z tego profilu: obrażenia, retry, śmierć i despawn. Pozycje tej próby obrażeń są oddzielną istniejącą fixture (100,100,7); nie udajemy przestrzennego wykonania spellów na mapie. Dodatkowa próba 20 HP pozostaje testem samego harnessu.

Przygotowanie propozycji nie oznacza wykonania wszystkich efektów spellów, instalacji summonów, respawnu i nagród. Nie uruchamiamy listenera `serve`, klienta, PostgreSQL ani produkcyjnego świata. Brak włączenia tych komponentów do żywego cyklu pozostaje osobną pracą integracyjną. Raport nigdy nie oznacza `live_gameplay_tested=true`.

## Uproszczone profile

`--training` generuje osobne, nazwane pod `.lab.` profile. Oryginalne dane pozostają niezmienione. Obecny zakres:

- Wszystkie 47 dotąd wstrzymanych stworzeń: samodzielna baza do testów z wyłączonymi zależnymi mechanikami. Usunięto sześć zależnych łańcuchów Ability i siedem summonów z nieistniejącym lub nieprzyjętym celem; trzy Familiary są oznaczonymi samodzielnymi stworzeniami treningowymi, nie gotowymi companionami. Candy Horror pomija corpse/decay zależny od niezarejestrowanego Itemu. Pełne oryginalne definicje są zachowane.
- Seacrest Serpent: wyłączona jedynie nieobsługiwana przez scheduler akcja `defense-2` typu melee. Wszystkie ataki, HP/statystyki i wpisy lootu zachowują dane dawcy.

Oryginalny Seacrest jest zastąpiony w rosterze testowym swoim wariantem; nie liczymy go podwójnie. Generator sprawdza schemy i rzeczywiste domknięcie natywnych referencji względem dokładnie przypiętej bazy, w tym zaakceptowane tożsamości Itemów. Nie tworzy pustych Encounter ani fikcyjnych Itemów.

## Wyniki i błędy

- `catalog-status.json`: wspólna tabela potworów, wariantów, uproszczeń i wyników natywnych.
- `inventory.json`: kompletna lista i flagi mitygacji/Bestiary/importu; braki nie zatrzymują pozostałych poprawnych rekordów.
- `training-bundles/`: izolowane paczki, oryginalne digests, wyłączone mechaniki, raport schemy i referencji.
- `arena-input.json`, `isolated-lab-stage.json`: dokładne wejście bieżącej próby.
- `native-arena-result.json`: wynik każdego profilu, liczniki przygotowanych akcji, replay oraz jawny zakres próby właściciela.
- `native-arena.log`: pełny log kompilacji/przebiegu.

Kod wyjścia: 0 — wszystkie wybrane natywne scenariusze przeszły; 1 — batch ukończony, ale są blokady komponentów; 2 — błędne wejście, konfiguracja, kompilacja albo niezgodny wynik. Raport częściowy jest zachowany. Poprzedni wynik nie może udawać sukcesu nowego przebiegu.

## Testy narzędzi

```bash
python -m unittest discover -s tools/monster-lab -p 'test_*.py'
cargo test --locked -p oteryn-game-server --lib monster_lab:: --quiet
```

Drugie polecenie bez manifestu wykonuje testy mechaniki harnessu, a nie batch danych. Pełny batch należy uruchamiać przez `run.py`, który wymaga świeżego wyniku i sprawdza zakres, profile, SHA oraz liczniki.

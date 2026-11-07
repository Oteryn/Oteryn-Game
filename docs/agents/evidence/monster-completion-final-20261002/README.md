# Końcowy pakiet uzupełnienia potworów — 2026-10-02

Pełne, niezależne archiwum przygotowanych danych: **1763 Creature, 95 Encounter**. Zawiera wszystkie czteroplikowe paczki, indeks, native stage, klasyfikację, flagi jakości, raporty źródeł, mitygacji i testów. Nie wymaga składania danych z wcześniejszych archiwów. Kod narzędzi pozostaje w stosie PR.

## Wynik

| Sprawdzenie | Wynik |
|---|---:|
| Przyjęte Creature / Encounter | 1763 / 95 |
| Wstrzymane Creature / Encounter | 0 / 0 |
| Native records / profiles | 24893 / 23779 |
| Unikalne powiązania źródeł | 1858 |
| Mitygacja źródłowa / zaakceptowane szacunki Oteryn | 1083 / 680 |
| Brak mitygacji | 0 |
| Arena: profile PASS / blokady | 1763 / 0 |
| Testy natywne areny | 7 PASS |
| Testy migracji | 159 PASS, 1 SKIP (160 razem) |

Dodano 66 rzeczywistych definicji: 53 statyczne ze źródeł, 2 nowe familiary i 11 dynamicznych wariantów Primal. Poprawiono również istniejące familiary i komponenty mechanik. Przywrócono 36 z 50 wcześniej pominiętych komponentów oraz 6 komponentów nowych definicji. Źródłowe wartości i dawne 644 zaakceptowane szacunki zachowano; dodano 36 szacunków. Szacunki nie trafiają do zbioru treningowego jako prawdziwe dane źródłowe.

Role wywnioskowane ze źródeł są oznaczone; 421 ogólnych klasyfikacji nie dowodzi pełnego rozróżnienia wszystkich bossów/helperów. Zachowano niepewność 158 wpisów lootu u 26 stworzeń. Pusta źródłowa tabela lootu, brak corpse/Bestiary lub zerowe doświadczenie mogą być właściwe dla danego rodzaju aktora.

## Użycie i weryfikacja

`qualification.json` zawiera SHA-256 archiwum, każdego pliku i narzędzi, dokładny SHA indeksu oraz stage. Wszystkie 7515 plików archiwum sprawdzono po kompresji. Rozpakować `prepared-population.tar.gz` do nowego katalogu. W `validation/lab-config.json` ustawić bieżące ścieżki repozytorium, Pythona/toolchainu oraz `read_only_inputs`:

- `bundles`: rozpakowane `population/bundles`;
- `index`: `population/population-index.json`;
- `stage`: `population/creature-admission-stage.json`;
- `item_map`: `import/protected-item-map.json`;
- wyjścia: nowe katalogi poza źródłami i rozpakowanymi danymi.

Walidacja: `python tools/monster-lab/validate_full_population.py --config CONFIG --output-dir NOWY_KATALOG`.
Arena: `python tools/monster-lab/run.py arena --config CONFIG --output-dir NOWY_KATALOG --ticks 30`.
Istniejący pokój mapy wystarcza do testu pojedynczego potwora; nie potwierdza dużych obszarów czarów. Raport areny jest związany z dokładnym stage i mapą. Sprawdza przygotowanie harmonogramu, retry/replay oraz fixture śmierci z HP profilu. **Nie wykonuje wszystkich efektów czarów ani dropu/respawnu na działającym świecie.**

## Co pozostaje

Pełna lista z aktorami, źródłami i flagami: `integration/remaining-work.json` w archiwum.

- 14 pominiętych komponentów u 8 wcześniejszych stworzeń: stan questów, Megalomania, debuff Walkera i defensywny melee Seacrest.
- 66 pominiętych komponentów u 47 nowych definicji, jawnie oznaczonych.
- Cykl właściciela 5 familiarów i 2 Challenge; 42 flagi rozmieszczenia/placement wymagają kwalifikacji.
- Wykonanie efektów, summon lifecycle, trwały corpse/drop, respawn i E2E klienta.
- Kwalifikacja 11 pochodnych powiązań `#registered-type`, promocja pełnych SHA/liczników/powiązań źródeł i chroniona integracja.

Liczby nakładają się; nie sumować ich jako liczby brakujących potworów. Odpowiedzialności architektów/koordynatora są w #162. `integration/final-materializer-proposal/materializer-promotion.PROPOSAL.patch` to konkretny przygotowany patch do oceny, **nie został zastosowany**. Odczytany materializer main nadal przypina 1503 Creature / 61 Encounter. Ten pakiet nie uruchamia serwera ani nie deklaruje zgodności z Global.

## Źródła

Canary: publiczny Git, rewizja `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Crystal summer-update: publiczny Git oraz zwykły HTTP GitHub do 50 brakujących plików Lua, rewizja `00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Zakres rejestracji Lua zweryfikowano; 5 dużych zasobów nie-Lua nie było potrzebnych do tego przeglądu. Wcześniejsze próbki/dane wiki zachowano wraz z pochodzeniem i niepewnością; nie oznaczono ich jako świeżo zweryfikowanych na Global. Remote Desktop nie był używany w tej rundzie.

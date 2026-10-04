Aktualizacja: nowsze partie r55–r58 opisuje `UNBLOCK-162-RESULTS.md`;
aktualny indeks ma354 kandydatów i129 BLOCKED. Poniżej zachowano wynik etapu r54.

# Modele źródłowe i rzeczywiste projekcje docelowe

Praca lokalna z 2026-10-04 obejmuje trzy zakresy: 21 wariantów `other_cast`,
sześć formuł Monka oraz 175 zaległych slotów potworów. R49–r51 zachowują
modele źródłowe; r52–r54 dodają rzeczywiste definicje docelowych schemów.
Kompletność capture nie jest utożsamiana z gotowością wykonania.

| Zakres | Zachowany model źródłowy | Nowa projekcja danych |
| --- | --- | --- |
| 21 wariantów cast | r49: pełne programy tego samego pliku | r52: 3 standardowe pakiety; osobno 3 istniejące native templates i 4 niezatwierdzone bindings |
| 6 formuł Monka | r50: dokładne callbacki Canary i helper C++ | r53: 6 standardowych Spell/Ability/Effect/Formula według S5/S16 |
| 175 slotów potworów | r51: dokładne parametry i programy | r54: 10 kompletnych kandydatów danych, 62 częściowe projekcje, 103 bez bezpiecznej projekcji |

R52 daje Canary Nature's Embrace i Crystal Forked Glacier/Forked Thorns.
Pakiety zachowują konkretne pozostałe zależności, m.in. presentation odmowy,
providerów i augments. S23 ustala docelowy chain; różnice od źródła Crystal
pozostają jawne. Cztery bindings native nie podnoszą źródłowych receiptów:
kwalifikowany czytnik porównuje cały profil, a donorowe koszty/poziomy/tożsamości
różnią się od osadzonych wzorców. Nie obniżamy kosztów dla obejścia kontroli.

R53 stosuje już przyjętą S5: `level_base_damage_healing`, zamiast wadliwego
helpera Canary. S16 zachowuje wheel unlock dla dwóch revelation spells.
Oryginały Canary pozostają niezmienione. Pola tylko do wyświetlania nie trafiają
do operacyjnych Requirements; wymagane pola targeting mają dowody źródłowe.

R54 daje 9 jawnych normalizacji D25 z jednoznacznych lokalnych dopasowań wiki
oraz Ratmiral Ball z top creature, trzema nazwanymi sojusznikami i heal 0–1000.
Surowy typ źródłowy pozostaje zachowany. Częściowe Ability/Effect/Formula nie
stają się kompletnymi slotami przez samą walidację schemy.

## Stan po r54 i granice

Po sprawdzonym imporcie r52–r54 indeks obejmował 483 warianty graczy:
**321 kandydatów strukturalnych i 162 BLOCKED**. To wzrost o 9 względem r51.
Nie jest to liczba unikalnych czarów ani czarów działających na serwerze.

Dla historycznych 175 slotów potworów: **10 kompletnych projekcji danych,
165 nadal niekompletnych**, z czego 62 mają częściowe definicje, a 103 nie mają
bezpiecznej projekcji. Wszystkie 175 zachowują niekwalifikowane wykonanie.
Pierwotny r28 nadal ma 20 552 mapped, 175 unresolved i 15 approved omissions;
nowy widok nakłada projekcje, nie przepisuje zapieczętowanego importu.

`source-closure-review-index.json`, `source-schema-progress.json`,
`current-source-gap-worklist.json` i osobny przegląd monster target projections
wskazują rzeczywiste importy, dowody oraz pozostałe blokady.

## Źródła i kwalifikacja

Dane pochodzą z wcześniej pobranych, przypiętych lokalnych Canary/Crystal,
importów r28/r49–r51 oraz istniejącego cache wiki. Przyjęte decyzje S5/S16/S23,
D25 i P7 sprawdzono przez zwykłe GitHub API; lokalne dokumenty zgadzają się
z przypiętym main. Nie pobierano ponownie donorów/wiki ani nie używano Remote
Desktop. Cache wiki pozostaje dowodem o zadeklarowanym wieku, bez deklaracji
świeżej weryfikacji wszystkich czarów.

Wykonanie native, wejścia z silnika, providerzy, wybór aktywnego manifestu
oraz podłączenie serwera pozostają niezakwalifikowane. Praca ta przygotowuje
lokalne dane dla istniejących właścicieli #1534 / #1622. Nie zmieniono aktywnej
zawartości serwera, dzierżawionych plików Rust ani historycznych r28–r51.
Nie utworzono PR, commitów ani pushów.

## Weryfikacja końcowa

- R52: 10 testów producenta PASS; r53: 6 PASS, w tym 42 przypadki wykonane
  przez oryginalny moduł Rust formuł i regresje wymaganej struktury czytnika.
- R54: 6 testów producenta PASS; importer: 5 PASS; builder postępu: 4 PASS.
- Niezależny przegląd odtworzył pakiety i wszystkie rzeczywiste kopie:
  49/49/18 artefaktów, bytes, sourcePath, schemaRefs i SHA256SUMS zgodne.
- Indeks 27 importów i trzy widoki postępu odtworzone identycznie:
  483/321/162 dla graczy oraz 10/62/103 dla projekcji potworów.
- `git diff --check` PASS; brak zmian w `content/`, `apps/`, `crates/`, `server/`.

Testy struktury czytnika nie są wykonaniem całego czaru na serwerze.
Aktualne manifesty importu mają SHA256:

| Import | SHA256 manifestu |
| --- | --- |
| r52 | `422e639c807081a1b6a3ecd1d2f49af0c0666547def91e7b8efe354736e4ab14` |
| r53 | `f476e8c93e34062730eed181b3472172ef970c46575b7a7e7160a84fdc26fac3` |
| r54 | `9bec95b4e71d64b740f69f37f7865083fc0dccb79a921d0712bd7ad926050fc1` |

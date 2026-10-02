# R21 — zakres kandydata i pozostałe zależności

Katalog źródłowy zawiera 246 aktywnych definicji czarów i sześć wpisów usuniętych
na podstawie zapisanych dowodów. Gotowość konwersji i kwalifikacja definicji nie
oznaczają, że cały świat, wszystkie parametry czarów i wszystkie ścieżki klienta
są już dostępne w działającym produkcie.

Aktualny manifest wejściowy ma SHA256
`f4b41d2269a48d8308dacc7fb31e48686fb168042ed20ab074d8c89e574157a2`.
Zawiera 162 rzeczywiste profile Creature: wszystkie 157 znormalizowanych
canonical gatunków dopuszczonych do Summon lub Convince oraz pięć Familiarów.
Rat i Skeleton zachowują wcześniejsze formalne tożsamości. 155 dodatkowych
pełnych profili pozostaje identycznych z przyjętymi canonical deklaracjami.
Rift Fragment ma rzeczywisty wygląd Object2122; Stag ma źródłową selekcję
Crystal Outfit1913 i osobny dowód wszystkich ośmiu domyślnych pól Outfit.
Provider zawiera 130 selekcji wyglądu: 125 stworzeń dopuszczających iluzję oraz
pięć Avatarów. Źródłowe początkowe HP wszystkich przyjętych gatunków są równe
maksymalnym HP i znormalizowanemu health. Nie ma eligible Invisible appearance.
Provider Item obejmuje 118 profili.
85 referencji Item w aktywnym katalogu sprawdzono względem niezależnych bindingów
źródłowych i rzeczywistych definicji produkcyjnych. Istniejące formalne tożsamości Creature z rewizji47 i dokładne canonical
DefinitionRefs pozostają zachowane. Dowód zgodności kompletnych Lua z bieżącym
źródłem nie zmienia tych referencji.

## Weryfikacja bieżącego kandydata

Rzeczywisty loader skompilował, ściśle zdekodował i wystagował dokładnie powyższy
manifest z 162 Creature/Presentation, 246 czarami i 118 Item. Próby podmiany
zewnętrznych pinów obu artefaktów zostały odrzucone. Receipt kwalifikacji ma SHA256
`1a21820d6478972a891b0e31fa3fb227d95eea818e376e33eba02c92df34b251`. Nie wykonano aktywacji runtime.

Końcowy wynik wszystkich testów Rust i kwalifikacji dokładnie tego manifestu
musi pochodzić z raportu koordynatora oraz rzeczywistego
`active-artifact/qualification-proof.json`. Historyczne logi obejmują także
nieudane próby wcześniejszych wersji. Ich obecność nie potwierdza sukcesu
bieżących plików. Wszystkie 60 celów Cargo przeszło końcową walidację na zamrożonych plikach.
Testowy launcher używał jawnego RUST_MIN_STACK=16777216; produkcyjny WP3
2 MiB pozostał bez zmian. Ignorowane przypadki i zakres wyniku są zapisane
w coordinator-validation-report.json. Pełny manifest sprawdzono osobno.
Producent snapshotu sprawdza dokładne odtworzenie patcha i integralność
archiwum; wynik oraz hashe są zapisane w review-manifest.json i SHA256SUMS.

Testy na PostgreSQL 17.6 potwierdziły zabezpieczenia nowego trwałego receipt dla
bezpośredniego Summon/Convince: Named, Target, brak opcjonalnego intent, osiem
podmian lub pominięć oraz niezmienność historii. Były to rzeczywiste testy
ograniczeń SQL z administracyjnie przygotowanymi danymi. Sam ten wynik nie
potwierdza SourceAdmission, dispatch ani fizycznej rezerwacji w działającym
świecie. Dowód COMMIT jest historyczny; instalacja nadal wymaga rzeczywistej,
aktualnej tożsamości aktora, mastera, sesji i generacji scope.

## Pozostałe zależności produktu

| Obszar | Aktualna granica i wymagana praca |
| --- | --- |
| Potwory dzikie | Pełny dispatcher czarów i harmonogram Behavior nie są skomponowane z rzeczywistym runtime. Importowane profile i sidecary nie uruchamiają AI. Potrzebne są aktualna percepcja/targeting, harmonogram interwałów i chance, CreatureIssuer oraz rzeczywiste wykonanie obrażeń, warunków i śmierci. |
| Populacja dla Summon/Convince | Provider obejmuje wszystkie 157 eligible canonical gatunków; 162 rekordy uwzględniają pięć Familiarów. Sześć dodatkowych pełnych konwersji jest wykluczonych przez zatwierdzone normalized summon/convince=false, nie przez brak adaptera. Każde pozyskanie wymaga aktualnego celu, źródłowej polityki i rzeczywistej rezerwacji właściciela. Nie oznacza to aktywacji wszystkich potworów z census. |
| Custom summon challenge | Dwie źródłowe operacje pozostają jawnie nierozwiązane. Ich referencje i harmonogram są zachowane; nie zastępują rzeczywistego ownera callbacku. |
| Soul War | Rzeczywisty owner/progresja questu nie są zaimplementowane. Katalogowa definicja i dane źródłowe nie ustanawiają uprawnienia do questa. |
| Limity i admission | Większe limity artefaktu kandydata wymagają przydziału przez właściwy kontrakt/owner przed produkcyjnym admission. Poprawne skompilowanie i staging nie przydzielają limitów ani nie aktywują wdrożenia. |
| Klient i prezentacja | Kandydackie komunikaty/providery wymagają rzeczywistego konsumenta klienta i renderowania. Dowód outbox/publikacji po receipt nie jest testem wyglądu ani interfejsu użytkownika. |
| VIS2 i ujemne piętra | Wymagany jest zaakceptowany kontrakt współrzędnych oraz właściwy konsument klienta dla pięter poza istniejącą granicą VIS2. Nie ma podstaw do zastępowania ich inną współrzędną. |
| WorldInstance | Pełna integracja musi korzystać z istniejącego rzeczywistego ownera WorldInstance i jego aktualnego admission. Kandydackie dane świata/house nie tworzą zastępczej authority. |
| PvP i black skull | Rzeczywisty producent faktów PvP/black skull nie jest obecny. Zależne operacje muszą pozostać fail closed przy brakujących aktualnych faktach; dane testowe nie mogą stanowić produkcyjnego dowodu. |

`monster-runtime-status.json` zawiera dokładniejszy audyt kodu i źródeł. Zliczone
1640 formalne bundle populacji oraz 1656 przeskanowanych plików potworów opisują
źródła, a nie liczbę aktywnych lub wykonanych potworów. Raport jawnie zaznacza
historyczne sześć nieodczytanych plików oraz brak skomponowanego wild monster
runtime. Nowy audyt ponownie przekonwertował wszystkie te sześć kompletnych
bundle i potwierdził zgodność z formalnym census. Przyjęte captures normalizują
ich summon/convince do false i pomijają mana_cost; surowe flagi Lua nie zastępują
tej polityki. Travelling Merchant zachowuje jawny konflikt raw HP110 kontra
znormalizowane HP100. Wszystkie sześć pozostaje źródłowym audytem poza aktywną
tabelą pozyskania.

`active-artifact/source-closure-report.json` zachowuje dziesięć brakujących
dokładnych deklaracji sidecar i nierozwiązane operacje źródłowe. Ponadto zachowuje 633 deklaracje zależności bez authoring profile. Te dane nie są
usuwane ani przedstawiane jako kwalifikowany pełny monster AI.

## Granica paczki

Paczka zawiera jawnie przydzielone zmiany kodu, dokładny patch, kopie źródeł,
typowane dane, dowody pochodzenia, hashe i referencje do logów. Pełny lokalny
Item audit zawierający `source_rows` z surowym XML pozostaje poza dystrybucją;
dystrybuowany projection zachowuje SHA256 oryginalnego dowodu. Surowe upstream
XML/DAT/OTBM/SPR nie są dystrybuowane. Historyczne r20 pozostaje zachowane
oddzielnie. Nie wykonano aktywacji produkcyjnej.

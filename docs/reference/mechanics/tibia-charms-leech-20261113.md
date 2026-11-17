# Tibia Global: Parry, Dodge, Low Blow, Carnage, leech i Cleanse

> **Najnowszy pełny rejestr dowodów (13.11.2026):** [cztery uzupełnienia, źródła, sprostowania i siedem niewiadomych Leech](tibia-charms-leech-20261113-evidence-rounds.md). Poniższy tekst jest pierwotną wersją; późniejsze korekty w rejestrze mają pierwszeństwo. Oficjalny wyjątek Cleanse dotyczy **Player Summon**.

Stan badania: **2026-11-13**. Typ dokumentu: **materiał badawczy do przeglądu**.

Zakres: aktualnie odnalezione dowody dotyczące Tibia Global, zebrane na prośbę właściciela projektu. Oficjalne wypowiedzi potwierdzają wspólny wynik Low Blow dla odpowiednich celów AoE. Dokumentacja społecznościowa opisuje brak łańcucha Carnage, a relacja z 2026 r. wspiera uwzględnianie overkill przez **Mana Leech**. Nowszy test gracza wskazuje współistnienie odporności Cleanse. Parry wobec właściwego Mana Drain, współaktywacja Dodge z minorami i dokładne odświeżanie Cleanse pozostają nierozstrzygnięte.

## Zakres względem Oteryn

Obowiązująca [decyzja o dacie odniesienia](../../agents/programs/OTERYN_TARGET_DATE_20260927_DECISION.md) wskazuje **2026-09-27**. Data zebrania tego raportu nie przesuwa tej granicy. Materiał późniejszy niż data odniesienia może wspierać badanie ciągłości, ale sam nie dowodzi zachowania gry 27 września. Nie ustalono numeru klienta ani rewizji serwera dla wszystkich relacji graczy.

Dokument zachowuje kandydatów na reguły i luki dowodowe; nie nadaje im statusu zaakceptowanego kontraktu implementacyjnego. Nie przeprowadzono audytu bieżącego kodu Oteryn ani własnego eksperymentu w grze. Powiązany [raport o wzorach crit/leech/skills](../crit-leech-skill-formulas.md) również pozostawia dokładne wzory Global, Damage Prey i zaokrąglenia niezakwalifikowane. Niniejszy materiał uzupełnia jego źródła, bez zastępowania wcześniejszych klasyfikacji.

## Podstawa dowodowa i werdykty

Uwzględniono oficjalne komunikaty CipSoft, opisy TibiaWiki, datowane wypowiedzi graczy i kontekst rozmów na Discordzie Kusniera oraz Tibia Analysers. **Fakt o treści źródła nie jest równoznaczny z własnym testem gry.** Wskazano również sytuacje, w których wiki i Discord prawdopodobnie opisują ten sam eksperyment. W zgromadzonym materiale nie znaleziono potwierdzonej zmiany listopadowego Winter Update rozstrzygającej pozostałe luki; nie jest to dowód braku zmian. Doniesienia z test servera nie są tu traktowane jako dowód wdrożenia na Global.

Klasyfikacje są zgodne z [rejestrem źródeł](../../agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md): `PROVEN` dla dokładnie potwierdzonej treści oficjalnej reguły w jej podanym kontekście, `DERIVED` dla wniosku z opisów lub relacji, `UNKNOWN` dla brakującego rozstrzygnięcia oraz `CONFLICT` dla nieuzgodnionych rozbieżności. `PROVEN` nie oznacza wykonania własnego testu ani automatycznie potwierdzonej ciągłości do daty odniesienia. Opisy wiki bez utrwalonej rewizji, Reddit, TibiaQA, kalkulatory i Discord mają tutaj rolę `COMMUNITY_CORROBORATION`; nie są kwalifikowanym zrzutem `TIBIAWIKI_STRUCTURED` ani własną obserwacją `GLOBAL_BLACKBOX`.

| Pytanie | Najlepiej wsparty wynik | Ograniczenie dowodu |
|---|---|---|
| Parry: Mana Drain i redukcje | Odbicie przed odpornościami gracza; odporności potwora pomijane, armor potwora uwzględniany | Mana Drain oraz pełna kolejność armor/shielding/mitigation nieustalone |
| Dodge + Cleanse/Numb/Adrenaline Burst | Brak dowodu współaktywacji przez ten sam uniknięty atak | „Poof” bez utraty HP nie rozstrzyga Dodge |
| Low Blow AoE | Wspólny dodatkowy krytyk dla trafionych celów objętych charmem, także diamond arrows | Wewnętrzna liczba wywołań RNG niewidoczna; wyjątek potwierdzony bezpośrednio na oficjalnym forum |
| Carnage | Brak rekurencji; dokumentacja wymaga osobistego ostatniego ciosu | Część zasad summonów i przypisanie wtórnych zabójstw wymagają testów |
| Leech | Nowszy opis overkill dotyczy Mana Leech; wiki podaje sumę zaokrąglonych wkładów celów | Rozbieżny współczynnik AoE; starsze obserwacje Life Leech bez świeżej replikacji dokładnej arytmetyki |
| Cleanse | Relacja z września 2026 wskazuje równoczesne odporności | Odświeżanie tej samej odporności pozostaje nieustalone |

Szczegółowe źródła i ich zakres znajdują się przy odpowiednich ustaleniach poniżej.

## 1. Parry pomija odporności, ale pełna kolejność redukcji pozostaje otwarta

**Opis społecznościowy:** podstawą odbicia jest damage sprzed odporności gracza. Ochronne imbuementy i odporności wyposażenia nie powinny więc obniżać tej podstawy. Odbicie ignoruje odporności potwora, również physical resistance, mimo fizycznej prezentacji obrażenia; **armor potwora nadal je zmniejsza**. Taki opis podaje [TibiaWiki: Parry](https://tibia.fandom.com/wiki/Parry).

Historyczny cytat przypisany Cedeko mówi o neutralnym obrażeniu redukowanym wyłącznie przez armor stworzenia. Pochodzi jednak sprzed mechaniki Mitigation. Odczytano indeksowany fragment przedruku na [TibiaBR](https://forums.tibiabr.com/threads/512969-Feedback-Bestiary-Charms?mode=hybrid); pełna strona była niedostępna, a pierwotnego oficjalnego posta nie zweryfikowano. Nowsza [wypowiedź na Reddicie](https://www.reddit.com/r/TibiaMMO/comments/1trziuw/critical_charms_and_parry_question/) jest zgodna z tym opisem, ale nie zawiera kontrolowanego pomiaru.

**Nieustalone:** czy Parry odbija właściwy **Mana Drain**; czy pobiera damage przed armor i shielding gracza; jak traktuje obecną mitigation obu stron. Ubytek many przez Magic Shield nie dowodzi ataku typu Mana Drain, który jest osobną kategorią ([opis Mana Drain](https://tibia.fandom.com/wiki/Mana_Drain)). Nie ma podstaw, aby podać kompletny wzór tych etapów jako zweryfikowany.

**Wniosek, wysoka pewność względem opisów:** implementacja oparta wyłącznie na końcowej utracie HP gracza albo redukująca odbicie przez physical resistance potwora byłaby niezgodna z najlepiej udokumentowanym zachowaniem.

## 2. Dodge z minorami nadal wymaga obserwacji tego samego ataku

**Relacja z testu:** Danashi 8 września 2026 r. podał, że Cleanse i Adrenaline Burst mogą aktywować się bez utraty HP, także przy „poof”. DOT oraz pola poison/fire/energy nie uruchamiały ich w jego próbach ([Discord, pełny wpis](https://discord.com/channels/836592213327609957/1381954407716884630/1546838689164955668)).

**Nieustalone:** wpis nie testuje Dodge i nie obejmuje Numb. Nie znaleziono materiału pokazującego, że jeden konkretny atak jednocześnie wywołał Dodge oraz Cleanse, Numb albo Adrenaline Burst. Same opisy warunku „trafienia” lub „ataku” nie określają kolejności procedur ([katalog charmów](https://tibiadb.wiki/charms)).

**Wniosek:** brak utraty HP nie wystarcza do utożsamienia bloku z Dodge. Minimalny dowód dodatni to pojedynczy przeciwnik, izolowany atak i jednocześnie widoczny efekt Dodge oraz danego minoru. Każdą z trzech par należy sprawdzić osobno. Brak wspólnego proc w krótkiej próbie nie wystarczy do pewnego „nie”, ponieważ aktywacje są losowe.

## 3. Low Blow daje wspólny wynik AoE, a diamond arrows mają istotny wyjątek

**Fakt oficjalny:** CipSoft wyjaśnił 5 listopada 2019 r., że gdy dodatkowa szansa Low Blow powoduje krytyk, otrzymują go wszystkie objęte danym atakiem stworzenia, do których przypisano charm ([komunikat 5268](https://www.tibia.com/news/?id=5268&subtopic=newsarchive)). Dla AoE run i czarów nie wspiera to niezależnego przyznawania bonusu części potworów tego samego przypisanego gatunku.

**Wniosek, wysoka pewność:** należy odwzorować wspólny obserwowalny wynik dla uprawnionych celów. Źródło nie ujawnia liczby wywołań generatora losowego wewnątrz serwera. Przy mieszanych gatunkach dodatkowy bonus Low Blow dotyczy przypisanego gatunku; zwykły krytyk z broni jest odrębną kwestią.

Wprowadzona 16 czerwca 2026 r. ogólna zasada ograniczyła aktywowanie charmów przez autoataki do głównego celu, pozostawiając runy i czary bez tej zmiany ([oficjalny opis](https://www.tibia.com/news/?id=8833&subtopic=newsarchive), [ogłoszenie wdrożenia](https://www.tibia.com/news/?id=8849&subtopic=newsarchive)). **Nie należy automatycznie stosować tego ograniczenia do Low Blow na diamond arrows:** changelog TibiaTools z 8 lipca 2026 r. informuje o działaniu na wszystkich celach i powołuje się na potwierdzenie CipSoft ([TibiaTools](https://www.tibiatools.io/tools/calculator/changelog)).

**Potwierdzenie oficjalne:** 15 lipca 2026 r. Liamas odpowiedział, że działanie Low Blow na wszystkich objętych celach diamond arrow jest zamierzone ([pierwsza odpowiedź](https://www.tibia.com/forum/?action=thread&postid=39596270#post39596270)). Następnie poprawił niefortunne nazwanie Low Blow czarem i wyraźnie potwierdził, że sprawdził zamierzony charakter tego zachowania z programistami ([doprecyzowanie](https://www.tibia.com/forum/?action=thread&postid=39596416#post39596416)). Oba posty odczytano bezpośrednio na tibia.com. Data 8 lipca w changelogu oznacza poprawkę kalkulatora, nie datę zmiany gry. Rozmowa z 31 października 2026 r. także opisuje wspólne krytyki ([Tibia Analysers](https://discord.com/channels/1431612466818977814/1431632652603232397/1566097294812520590)).

## 4. Carnage nie uruchamia kolejnego Carnage; reguły zabicia mają osobne warunki

**Dokumentacja społecznościowa:** obrażenie Carnage nie może wywołać kolejnego Carnage. Do pierwszej aktywacji wymagany jest ostatni cios właściciela charmu; dobicie przez jego summona lub familiara nie wystarcza. Wiki wyklucza także aktywację po zabiciu przywołania potwora ([TibiaWiki: Carnage](https://tibia.fandom.com/wiki/Carnage)). Zakaz łańcuchów występuje również w [cytacie zamieszczonym na Discordzie w listopadzie 2024 r.](https://discord.com/channels/836592213327609957/836593610811768833/1306641105873408022); pierwotnego oficjalnego wpisu nie odnaleziono.

**Konkretny test gracza:** 22 października 2025 r. Picture zgłosił, że Carnage nie działa po przypisaniu do Spidris Elite przywoływanych przez Hive Overseers ([zgłoszenie](https://discord.com/channels/836592213327609957/836593610811768833/1430638436783620228), [dopowiedzenie](https://discord.com/channels/836592213327609957/836593610811768833/1430638487031382158)). Następnie zapowiedział dodanie notatki do wiki. Wiki i ten wpis prawdopodobnie mają więc wspólne pochodzenie; nie liczą się jako dwa niezależne eksperymenty.

**Nieustalone:** kto otrzymuje kill credit, loot i XP za potwora zabitego wtórnym wybuchem; czy stojący obok summon otrzymuje obrażenia eksplozji; jak rozliczane są liczne niezależne zgony spowodowane pierwotnym AoE. Wyłączenie summona jako źródła aktywacji nie dowodzi jego odporności na eksplozję. Zakaz rekurencji również nie rozstrzyga, czy jedno zwykłe AoE może dać kilka niezależnych aktywacji.

## 5. Mana Leech: overkill wsparty nowszą relacją, arytmetyka AoE nadal niezakwalifikowana

**Najdokładniejszy model społecznościowy Mana Leech:**

```math
M=\sum_{i=1}^{N}\left\lceil D_i L\frac{0{,}1N+0{,}9}{N}\right\rceil
```

W tym modelu `N` oznacza liczbę uwzględnionych trafionych stworzeń, a `L` ułamek leech, np. `0,08`. `D_i` to podstawa obrażeń danego celu dla leech: uwzględnia critical i overkill, lecz **nie zawiera dodatkowego bonusu Damage Prey**. Nie należy bezwarunkowo podstawiać końcowego damage po wszystkich bonusach. Dokładne wliczanie do `N` celów immune, zablokowanych, otrzymujących zero obrażeń oraz summonów jest `UNKNOWN`. Wiki podaje **zaokrąglenie w górę osobno dla każdego celu**, następnie sumę. Uproszczenie „pełny leech z głównego celu i 10% z pozostałych” zawodzi przy różnych odpornościach. Według tego samego opisu critical zwiększa leech, Damage Prey go nie zwiększa, a overkill pozostaje uwzględniany ([TibiaWiki: Formulae](https://tibia.fandom.com/wiki/Formulae)).

**Przykład rachunkowy:** przy dwóch celach, 100 i 300 damage oraz 8% leech otrzymujemy `ceil(4,4)+ceil(13,2)=19`. Zaokrąglenie dopiero sumy dałoby `18`. To konsekwencja wzoru, nie pomiar z gry.

**Nowsza relacja wspierająca overkill Mana Leech:** gracz 24 lutego 2026 r. opisał odzyskiwanie 50–60 many przy zabiciu bug z 29 HP i 16% mana leech, wykluczając bonus proficiency ([Reddit](https://www.reddit.com/r/TibiaMMO/comments/1rczq7e/is_this_a_bug/)).

**Starsze dowody i konflikt:** pytanie TibiaQA z 2 kwietnia 2021 r. zawiera obserwacje overkill dla **Life Leech i Mana Leech**. Odpowiedź Anubora z 17 września 2022 r. opisuje testy z Gred i inny współczynnik AoE: `(8% + (N-1) × 1%)/N` dla jednego Powerful Void. Dla dwóch celów daje to 4,5%, a wzór wiki 4,4%. To `CONFLICT`, którego nie rozstrzyga późniejsze podbicie wątku; nie jest ścisłym potwierdzeniem całego wzoru wiki ([TibiaQA](https://www.tibiaqa.com/24514/about-imbuements-vampirism-mana-leech-better-single-target-recover-most)). Nie znaleziono świeżego kontrolowanego testu wszystkich zaokrągleń ani osobnego równie dokładnego potwierdzenia arytmetyki **Life Leech**. Nowszej relacji dotyczącej many nie należy rozszerzać na zdrowie. Oficjalny manual opisuje obie mechaniki ogólnie, bez takich szczegółów ([CipSoft](https://www.tibia.com/gameguides/?section=combat&subtopic=manual)).

## 6. Cleanse może utrzymywać kilka odporności; odświeżanie pozostaje otwarte

**Nowszy test gracza:** Danashi 8 września 2026 r. potwierdził po kolejnych próbach równoczesne działanie kilku odporności. Podał także, że uzyskana odporność chroni przed odpowiednimi polami, choć same pola nie aktywują charmu ([Discord](https://discord.com/channels/836592213327609957/1381954407716884630/1546838689164955668)). To mocniejsza podstawa do opisu obecnego zachowania niż przenoszenie wcześniejszej tezy o pozostawaniu wyłącznie ostatniej odporności. Nadal jest to relacja gracza, bez własnej replikacji.

**Podstawowa reguła w dokumentacji społecznościowej:** proc usuwa jeden losowy aktywny negatywny stan i daje odporność na niego; opis po reworku podaje 11 sekund ([TibiaQA z 29 grudnia 2024](https://www.tibiaqa.com/36769/how-cleanse-works-after-the-2024-charm-overhaul-winter-update)). To opis czasu trwania, nie nowy pomiar odświeżania timerów. **Fakt oficjalny:** komunikat z 8 września 2026 r. obejmuje również odporność Cleanse na hex spells ([CipSoft](https://www.tibia.com/news/?id=8960&subtopic=newsarchive)).

**Nieustalone:** czy ponowne trafienie odświeża tę samą odporność, czy wymagany jest nowy proc, czy trwająca odporność blokuje taki proc i czy dodanie innej odporności wpływa na poprzednie timery. Współistnienie odporności nie rozstrzyga ich czasu wygaśnięcia.

## Minimalne brakujące dowody przed odwzorowaniem w Oteryn

**Rekomendacja:** przeprowadzić sześć krótkich, kontrolowanych prób z zapisanym numerem wersji klienta i rozróżnieniem Global/test server:

1. **Parry:** izolowany Mana Drain; następnie porównanie armor, shielding i mitigation przy stałych pozostałych parametrach.
2. **Dodge:** osobno każdy minor; pojedynczy napastnik i rejestracja wspólnych zdarzeń z jednego ataku.
3. **Low Blow:** mieszane gatunki oraz diamond arrows, także z przypisanym gatunkiem wyłącznie w obszarze dodatkowym.
4. **Carnage:** osobiste dobicie kontra summon, summon jako ofiara i cel eksplozji, rozdzielenie ostatniego ciosu od największego damage w party.
5. **Leech:** dwa cele o różnych odpornościach, niecałkowite wyniki, overkill, Damage Prey, zero damage, immune, blok i summony; osobne odczyty HP oraz many. Rozdzielić test współczynnika od testu etapu zaokrąglenia.
6. **Cleanse:** znany moment pierwszej odporności, ponowne zdarzenie tego samego typu i drugiego typu oraz obserwacja obu momentów wygaśnięcia.

Do czasu tych prób pełnej zgodności Oteryn z Global nie należy deklarować dla nierozstrzygniętych warunków. Przyjęte tymczasowo zachowanie powinno być zapisane jako założenie implementacyjne wraz z testem, który może je obalić.

## Klasyfikacja ustaleń i ciągłość do daty odniesienia

Źródła podlinkowane w odpowiednich sekcjach odczytano podczas badania zakończonego 2026-11-13. Nie utrwalono rewizji wiki ani pełnych zapisów eksperymentów; zapis nie może zastąpić takich dowodów. Ciągłość poniżej dotyczy daty projektu **2026-09-27**, a nie samej daty odczytu.

| Pole / reguła | Datowane źródło lub brak daty | Rola źródła | Klasyfikacja ustalenia | Ciągłość do 2026-09-27 |
|---|---|---|---|---|
| Parry: podstawa przed odpornościami, armor potwora | Wiki bez utrwalonej rewizji; historyczny przedruk Cedeko | `COMMUNITY_CORROBORATION` | `DERIVED`; pełna kolejność redukcji `UNKNOWN` | `UNKNOWN`: brak datowanego testu aktualnej mitigation |
| Parry: właściwy Mana Drain | Brak rozstrzygającego materiału | Brak | `UNKNOWN` | `UNKNOWN` |
| Dodge + każdy z trzech minorów | Danashi, 2026-09-08: tylko poof oraz Cleanse/Adrenaline Burst | `COMMUNITY_CORROBORATION` | `UNKNOWN` dla współaktywacji z Dodge i dla Numb | `UNKNOWN`: relacja nie bada tej pary zdarzeń |
| Low Blow: wspólny dodatkowy krytyk AoE | CipSoft, 2019-11-05 | `PRIMARY_OFFICIAL` | `PROVEN` w zakresie opublikowanej reguły; wewnętrzne RNG `UNKNOWN` | `DERIVED`: późniejsze potwierdzenie konkretnego AoE, bez własnego testu na dacie odniesienia |
| Low Blow: wyjątek diamond arrows | Liamas, 2026-07-15; dwa oryginalne posty | `PRIMARY_OFFICIAL` | `PROVEN` dla zamierzonego działania na odpowiednich celach diamond arrow | `DERIVED`: oficjalne potwierdzenie przed datą odniesienia, brak własnego pomiaru w tej dacie |
| Carnage: brak rekurencji, wymagany własny ostatni cios | Wiki bez rewizji; cytat na Discordzie z 2024-11-14 | `COMMUNITY_CORROBORATION` | `DERIVED`; pierwotnego oficjalnego cytatu nie odnaleziono | `UNKNOWN`: brak utrwalonego testu na dacie odniesienia |
| Carnage: summon gracza nie dobija do proc; summon potwora jako ofiara | Wiki; Picture, 2025-10-22, tylko przywołane Spidris Elite | `COMMUNITY_CORROBORATION` | `DERIVED`; zależność wiki od tej samej relacji możliwa | `UNKNOWN`: nie uogólniać jednego testu na wszystkie przypadki |
| Carnage: wtórny kill credit / loot / XP | Brak rozstrzygającego materiału | Brak | `UNKNOWN` | `UNKNOWN` |
| Mana Leech: overkill | Reddit, 2026-02-24; TibiaQA, 2021-04-02 | `COMMUNITY_CORROBORATION` | `DERIVED` z relacji graczy | `UNKNOWN`: brak testu w dacie odniesienia |
| Life Leech: overkill i dokładna arytmetyka | TibiaQA, 2021-04-02; brak świeżej replikacji | `COMMUNITY_CORROBORATION` | Overkill `DERIVED` historycznie; obecna dokładna arytmetyka `UNKNOWN` | `UNKNOWN` |
| Mana Leech: współczynnik AoE | Wiki bez rewizji kontra TibiaQA, 2022-09-17 | `COMMUNITY_CORROBORATION` | `CONFLICT`: 4,4% kontra 4,5% w opisanym przypadku | `UNKNOWN` |
| Mana Leech: ceil per cel, Damage Prey, zbiór N | Wiki bez rewizji; odczyt z indeksu wyszukiwarki | `COMMUNITY_CORROBORATION` | ceil i wyłączenie Prey: `DERIVED` jako kandydat; dokładny zbiór N: `UNKNOWN` | `UNKNOWN` |
| Cleanse: jednoczesne odporności | Danashi, 2026-09-08 | `COMMUNITY_CORROBORATION` | `DERIVED` z jednej relacji testowej | `UNKNOWN`: brak własnej replikacji i pełnych timerów |
| Cleanse: jeden stan i 11 sekund; odświeżanie | TibiaQA, 2024-12-29; odczytano treść opisu | `COMMUNITY_CORROBORATION` | Reguła bazowa `DERIVED`; dokładne odświeżanie `UNKNOWN` | `UNKNOWN`: starszy opis nie rozstrzyga aktualnych timerów |
| Cleanse: odporność na hex spells | CipSoft, 2026-09-08 | `PRIMARY_OFFICIAL` | `PROVEN` w zakresie komunikatu | `DERIVED`: komunikat przed datą odniesienia |

Dla wszystkich wierszy stan implementacji Oteryn pozostaje **nieaudytowany w ramach tego zadania**. Następne działania dotyczące brakujących dowodów wskazano w sześciu próbach powyżej. Zmiana kontraktu, implementacja lub zamknięcie wpisu w rejestrze mechanik wymagają osobnego zadania.

### Uwagi o pochodzeniu i niezależności

- Posty Liamasa odczytano bezpośrednio na oficjalnym forum; poprawione określenie Low Blow nie jest dowodem wewnętrznej implementacji charmu jako czaru.
- Discordowe relacje odczytano wraz z kontekstem rozmów. Cytowany zakres dotyczy tych wypowiedzi; nie udostępnia się tutaj pełnych historii kanałów.
- Przedruk Cedeko oraz szczegółowy wzór wiki odczytano z indeksowanych fragmentów. Brak pełnej historycznej rewizji ogranicza możliwość odtworzenia dokładnie tego samego źródła.
- Zgodność wiki z wpisem Picture może wynikać z jednego eksperymentu. Cytat z oficjalnej wypowiedzi zamieszczony przez użytkownika Discorda pozostaje źródłem wtórnym do czasu odnalezienia oryginału.
- TibiaTools z 2026-07-08 dokumentuje zmianę kalkulatora. Rozmowa Tibia Analysers z 2026-10-31 jest późniejsza niż data odniesienia i stanowi tylko uzupełniające świadectwo społeczności.

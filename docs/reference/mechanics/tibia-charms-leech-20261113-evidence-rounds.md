# Tibia Global — pełny dziennik badania charmów i Leech, 13 listopada 2026

> **Status: EVIDENCE, nie kontrakt implementacji.** [Raport pierwotny](tibia-charms-leech-20261113.md) oraz trzy uzupełnienia w tym pliku tworzą **całość** pracy badawczej. Zachowano pełne oryginalne treści komentarzy w kolejności ich powstania. **Późniejsze sprostowania są ważniejsze od wcześniejszych przypuszczeń.** Datą odniesienia programu Oteryn pozostaje **2026-09-27**; późniejsze wypowiedzi i testy nie dowodzą automatycznie ciągłości zachowania na ten dzień.

## Sześć tematów: co wiemy, czego nie wiemy

| Temat | Dowód / kandydat | Otwarte pytania |
| --- | --- | --- |
| Parry | [CipSoft 2017](https://www.tibia.com/news/?id=4386&subtopic=newsarchive) oficjalnie: odbicie pomija odporności potwora, armor potwora je redukuje | Mana Drain, kolejność armor/shielding/mitigation, nowe redukcje |
| Dodge + minory | [Danashi 2026](https://discord.com/channels/836592213327609957/1381954407716884630/1546838689164955668) obserwował Cleanse i Adrenaline Burst przy „poof” | Czy dokładnie ten sam atak uniknięty przez Dodge może uruchomić Cleanse/Numb/Adrenaline Burst |
| Low Blow AoE | [CipSoft 2019](https://www.tibia.com/news/?id=5268&subtopic=newsarchive) i [Liamas 2026](https://www.tibia.com/forum/?action=thread&postid=39596416#post39596416): wspólny dodatkowy krytyk dla odpowiednich celów, również diamond arrows | Liczba wewnętrznych wywołań RNG |
| Carnage | Zasięg ortogonalny wokół zabitego celu; brak rekurencji; bazowo 15% max HP i cap 6×level | Wtórny kill credit, XP, loot; summony i kilka pierwotnych zgonów w jednym AoE |
| Leech | Historyczny model AoE, źródła o overkill, critical i wyłączeniu Damage Prey; testy 2026 pokazują rozdzielenie beamów | Siedem szczegółów w tabeli niżej |
| Cleanse | Relacja z 2026: jednoczesne odporności; [CipSoft 2024](https://www.tibia.com/news/?id=7736&subtopic=newsarchive): nie aktywuje się od **Player Summon** | Odświeżanie tej samej odporności, zależność timerów, summon *potwora* |

To są **zakresy dowodów**, nie pomiar procentowej zgodności aktualnego kodu Oteryn.

## Wszystkie siedem obecnych niewiadomych Leech

| ID | Niewiadoma | Co trzeba sprawdzić |
| --- | --- | --- |
| L-01 | Dokładny mnożnik AoE | Wzór wiki `f(N)=(0.1N+0.9)/N` kontra starszy współczynnik TibiaQA. Test 2×1000 damage, 8% mana leech: przewidywania 88 vs 90 MP |
| L-02 | Co jest jedną grupą celów `N` | Oddzielne centralne i boczne trafienia Beam Mastery; inne czary z wieloma komponentami |
| L-03 | Moment zaokrąglania `ceil` | `ceil(D×L×f)` kontra `ceil(ceil(D×L)×f)`, per cel/per źródło oraz suma na końcu |
| L-04 | Immune, zablokowany lub zero-damage cel w `N` | Czy obiekt bez otrzymanych obrażeń zmniejsza leech uzyskany z pozostałych |
| L-05 | Summony | Czy można z nich leechować i czy wchodzą do `N`; osobno player i monster summons |
| L-06 | Overkill oraz kolejność modyfikatorów | Kiedy pobierane są obrażenia względem critical, prey, proficiency, bounty i innych bonusów |
| L-07 | Sumowanie źródeł leech | Czy imbuement, equipment, Wheel oraz minor charm tworzą wspólne `L`, czy osobne naliczania |

**Największy ilościowy wyjątek:** [Porfz, 15 lipca 2026, film od 16:23](https://www.youtube.com/watch?v=YKA_VqNX1eI&t=983s) pokazuje dwa trafienia centralne po 2086 damage/207 MP oraz sześć bocznych po 1621 damage/73 MP. Łącznie odczytano **852 MP**. Model dwóch grup (`N=2` i `N=6`) przy **jawnym założeniu 18% leech i widocznym damage jako podstawie** odtwarza 852 MP; model jednego wspólnego `N=8` daje 538 MP. To **różnica 314 MP w przykładzie, nie ogólna średnia**. Wyposażenia postaci z tego konkretnego testu nie zweryfikowano niezależnie, występuje Expose Weakness, a zachowania w listopadzie nie potwierdzono nowym eksperymentem.

**Najważniejsze sprostowania:**
- Oficjalny zakaz aktywacji Cleanse dotyczy **Player Summon**, nie każdego summona; stwierdzenie z pierwszego komentarza było zbyt szerokie.
- Parry armor/resistances potwora ma **oryginalny oficjalny news z 2017 r.**, nie tylko cytat historyczny.
- Dokładna implementacja `ceil` w 2026, łącznie ze sprawdzoną w komentarzu hipotezą podwójnego zaokrąglenia, pozostaje `UNKNOWN/CONFLICT`.
- Oddzielne części Beam Mastery to **kandydat poparty testem**, ale nie dowód liczby wewnętrznych instancji czaru.
- W Physical Pierce trzeba odróżniać regular pierce, weapon pierce i armor penetration; nie wolno uogólniać ich do jednej flagi.
- Opisy z test servera i Global są osobnymi kontekstami, nawet gdy dotyczą tego samego dnia.

## Archiwum źródeł i obserwacji — wszystkie trzy komentarze

Poniższe komentarze są zachowane w całości, wraz z linkami do źródeł, scenariuszami testów, datami i hipotezami. Treść historyczna może zawierać twierdzenia później skorygowane. Dokument nie zawiera surowych, chronionych plików klienta, prywatnych sesji ani kopii nagrań.

---

### Badanie 1/3 — komentarz 6856504721

[Oryginalny komentarz i data publikacji](https://github.com/Oteryn/Oteryn-Game/pull/3177#issuecomment-6856504721). **Pamiętaj o późniejszych sprostowaniach.**

## Uzupełnienie badania Global — 2026-11-13 (bez zmiany zamrożonego HEAD)

Poniższe informacje **uzupełniają** `docs/reference/mechanics/tibia-charms-leech-20261113.md`. Nie oznaczają, że mechaniki zostały zaimplementowane lub że wynik test servera występuje na produkcji. Aktualny head PR: `07511eee10b27a1283bc23e1340f2fd559142ef9`.

| Dziedzina / atomowe ustalenie | Nowy dowód i data | Klasyfikacja / implikacja |
| --- | --- | --- |
| **Carnage — parametry** | [CipSoft 2024-11-21](https://www.tibia.com/news/?id=8140&subtopic=newsarchive): 10/20/22% proc, 15% HP zabitego potwora, zasięg radius 1, limit 6 × level | `PROVEN` w granicach tekstu oficjalnego; geometria radius 1 niedoprecyzowana przez samą liczbę |
| **Carnage — geometria** | [TibiaQA: test i nagranie](https://www.tibiaqa.com/36586/how-does-charm-carnage-range-work); [wideo wskazane przez autora](https://www.youtube.com/watch?v=1Bs8nL7D8UM&t=2042s): uderzenie obejmuje 4 sąsiadujące pola ortogonalne **wokół zabitego celu**, nie przekątne | `DERIVED` ze świadectwa społeczności; brak nowego niezależnego odtworzenia |
| **Carnage — summony i wtórne zabicia** | [Discord: uwaga o zgonie summona bez XP/zwłok, 2025-10-22](https://discord.com/channels/836592213327609957/836593610811768833/1430639514786857074); [wiki Carnage](https://tibia.fandom.com/wiki/Carnage) | Wtórny kill credit, loot/XP, ofiara-summon i wiele bezpośrednich zgonów jednego AoE nadal `UNKNOWN` — wykluczenie summonów jako *triggera* nie oznacza immunitetu na obrażenia |
| **Cleanse — brak aktywnego statusu** | [TibiaQA: historyczna korespondencja gracza z supportem CipSoft (około 2020)](https://www.tibiaqa.com/19959/what-is-the-duration-of-immunity-granted-by-the-cleanse-rune): brak aktywnego negatywnego stanu => brak działania, 11 s ochrony po skutecznym usunięciu | `DERIVED` / świadectwo wtórne: aktualność 2026 oraz refresh trwającej odporności wciąż `UNKNOWN` |
| **Cleanse — atak summona** | [TibiaWiki: historia patchy 2024-01-30](https://tibia.fandom.com/wiki/Patch_Updates): Cleanse nie aktywuje się od ataków summonów | `DERIVED` ze źródła społecznościowego o historycznym patchu; brak nowszego testu |
| **Parry — odczyt rzeczywistego Mana Drain** | [CipSoft 2026-08-04](https://www.tibia.com/news/?id=8915&subtopic=newsarchive): naprawa Damage Input Analysera, który przy Magic Shield błędnie przedstawiał wszelki incoming damage jako Mana Drain | `PROVEN` dla błędu UI; dawne nagranie samych liczb analysera nie dowodzi, że Parry odbija realny typ Mana Drain. Odbicie prawdziwego Mana Drain nadal `UNKNOWN` |
| **Parry — Mana on Hit** | [CipSoft 2025-08-05](https://www.tibia.com/news/?id=8477&subtopic=newsarchive): Parry/Damage Reflection nie wywołują już regeneracji many przez perk Mana on Hit podczas bycia trafionym | `PROVEN` dla tej kombinacji, NIE dla zachowania Parry przeciw Mana Drain |
| **Pierce w charmach — ostrożnie z uogólnieniem** | [CipSoft 2025-11-25](https://www.tibia.com/news/?id=8610&subtopic=newsarchive) ogólnie wyłącza Physical Pierce z charmami; [CipSoft: pierce z proficiency nie stosuje się do charmów](https://www.tibia.com/news/?id=8598&subtopic=newsarchive); [TibiaTools changelog 2026-07-08](https://www.tibiatools.io/tools/calculator/changelog) rozdziela regular pierce (wg kalkulatora wpływa) od weapon pierce (nie wpływa) | **Różne zakresy terminologii**. Nie implementować jednego przełącznika „każdy pierce off” bez testu konkretnych źródeł pierce. Status per-poziom `CONFLICT/UNKNOWN` |
| **Mana Leech AoE — rozróżnialny test** | [TibiaWiki Formulae](https://tibia.fandom.com/wiki/Formulae) kontra [TibiaQA testy 2022](https://www.tibiaqa.com/24514/about-imbuements-vampirism-mana-leech-better-single-target-recover-most) | 2 cele × 1000 damage, 8% leech: model wiki `2 × ceil(44)=88`, model TibiaQA `2 × ceil(45)=90`. **To przewidywania, nie pomiary Global**. Status `CONFLICT` |
| **Mana Leech — rozbieżność test server/live w maju** | [Reddit 2026-05-12](https://www.reddit.com/r/TibiaMMO/comments/1taytn2/did_mana_leech_get_nerfed_on_the_test_server/) — gracz zgłosił niższą regenerację w test serverze przy podobnych obrażeniach | `UNKNOWN` czy był to bug później naprawiony; nie przenosić bez testu na Global po czerwcu 2026 |

**W dalszym ciągu brak bezpośredniego rozstrzygnięcia**: (1) Parry kontra atak rzeczywistego typu Mana Drain oraz kolejność shielding/armor/mitigation; (2) wspólna aktywacja Dodge i konkretnego minoru **przy tym samym unikniętym ataku**; (3) kto dostaje kill credit za zgon od eksplozji Carnage; (4) produkcyjny leech i jego ceil/floor przy wielu celach, overkill i różnych typach; (5) odświeżanie odporności Cleanse tego samego typu i zależność timerów. Zebrane nowe wyniki wyszukiwarki Discordów nie zawierały rozstrzygającego kontrolowanego testu tych pytań.

**Minimalna następna próba:** Parry na izolowanym realnym Mana Drain po naprawie analizatora; Dodge + jeden minor przy jednym napastniku z logiem zdarzeń; Cleanse z drugim statusem w 11-sekundowym oknie i zapisem momentów wygaśnięcia; Mana Leech na 2 celach ×1000 przy 8%, bez prey/bonusów, odrębnie dla HP i MP; Carnage w układzie dwóch osób + summon i wtórny kill z logiem XP/lootu.

**Uwaga chronologiczna:** dodatkowa [zmiana CipSoft z 2026-10-20](https://www.tibiaxplorer.com/tibia_news/2047) dotyczy Aura of Exposed Weakness: Overpower i Overflux nie są już nią wzmacniane, pozostałe charmy podlegają jej przy pierwszym trafieniu. To jest **po** obowiązującej dacie odniesienia Oteryn 2026-09-27; nie należy automatycznie przyjmować jej do baseline bez nowej decyzji.

Stan istniejącego PR/HEAD bez zmian.


---

### Badanie 2/3 — komentarz 6857122422

[Oryginalny komentarz i data publikacji](https://github.com/Oteryn/Oteryn-Game/pull/3177#issuecomment-6857122422). **Pamiętaj o późniejszych sprostowaniach.**

## Dalsze badanie — 2026-11-13: korekta Cleanse, logi zaokrągleń i rozdzielenie leech beama

Uzupełnienie `docs/reference/mechanics/tibia-charms-leech-20261113.md` oraz [poprzedniego komentarza](https://github.com/Oteryn/Oteryn-Game/pull/3177#issuecomment-6856504721). To materiał referencyjny, nie zmiana runtime ani kwalifikacja zgodności. Head odczytany przed publikacją: `07511eee10b27a1283bc23e1340f2fd559142ef9`; gałęzi nie zmieniam.

### 1. Korekta zakresu oficjalnej poprawki Cleanse: Player Summon, nie dowolny summon

**PROVEN — dokładny zakres oficjalnego komunikatu.** Odnalazłem [oryginał CipSoft, „Fixes and Changes”, 2024-01-30, news 7736](https://www.tibia.com/news/?subtopic=newsarchive&id=7736). Wyłączenie aktywacji Cleanse dotyczy napastnika będącego **Player Summon**.

Poprzedni komentarz, oparty na historii patchy wiki, używał szerszego słowa „summon”. Należy go w tym punkcie skorygować: **ten komunikat nie dowodzi wyłączenia ataków stworzeń przywołanych przez potwory**. Nie dowodzi też, że te ostatnie aktywują Cleanse — to pozostaje `UNKNOWN`.

Treść i datę odczytano także przez [TibiaData news 7736](https://api.tibiadata.com/v4/news/id/7736) 2026-11-13. TibiaData jest kopią oficjalnej publikacji, nie niezależnym eksperymentem. Wiki Cleanse `oldid=1203554` opisuje napastnika ogólniej niż oryginalny komunikat; w zakresie tego historycznego patcha pierwszeństwo ma dokładne brzmienie źródła pierwotnego.

### 2. Parry: odnaleziony pierwotny oficjalny opis armor/resistances

[CipSoft, „Important Changes during the Test Server Phase”, 2017-11-23, news 4386](https://www.tibia.com/news/?id=4386&subtopic=newsarchive) stwierdza, że odbicie pomija odporności potwora i podlega jego armor. Datę oraz pełny tekst odczytano przez [TibiaData 4386](https://api.tibiadata.com/v4/news/id/4386); tę samą regułę odnaleziono w indeksie oficjalnej strony.

**PROVEN dla zadeklarowanej wówczas reguły**, mocniejsze źródło niż przedruk Cedeko lub wiki. Jest to komunikat o zmianach podczas testu z 2017 r., a nie pomiar obecnego klienta. Nie rozstrzyga rzeczywistego Mana Drain, kolejności shielding/armor gracza ani późniejszej mitigation. Nie rozszerzam historycznego „tylko armor” na wszystkie mechanizmy dodane do 2026 r.

### 3. Leech: konkretny log AoE pozwala sprawdzić kandydatów zaokrąglania

**Źródło pierwotne społecznościowe:** [tom, 2025-07-03, log trzech booków i zadeklarowane parametry](https://discord.com/channels/836592213327609957/836593610811768833/1390345009844457595). Autor podał `N=3`, Mana Leech `16%`, Life Leech `50.75%` oraz broń opisaną jako „razor”. Wpis zawiera następujące liczby:

| Cel | Damage | HP odzyskane w logu | Mana odzyskana w logu | Mana: pojedyncze ceil | Mana: hipoteza dwóch ceil |
|---|---:|---:|---:|---:|---:|
| Cursed book | 1202 | 245 | 78 | 77 | 78 |
| Icecold book | 1076 | 219 | 70 | 69 | 70 |
| Icecold book | 1103 | 224 | 71 | 71 | 71 |

`f(N) = (0.1*N + 0.9)/N`, więc dla trzech celów `f=0.4`.

- Model opisany na wiki: `A_i = ceil(D_i * L * f(N))`.
- **Nowa hipoteza rachunkowa, nie ustalona implementacja CipSoft:** `B_i = ceil(ceil(D_i * L) * f(N))`.

Sprawdziłem obliczenia na dokładnych ułamkach wymiernych (`fractions.Fraction`, `math.ceil`). Model A daje dla many `77/69/71`, a model B `78/70/71`. Dla HP oba dają `245/219/224`. Zatem B pasuje do wszystkich sześciu podanych wartości odzysku, lecz próbka nie identyfikuje algorytmu jednoznacznie. **To moje dopasowanie matematyczne do opublikowanych liczb, nie nowy pomiar w grze.**

Ograniczenia: trzy trafienia, brak pełnej konfiguracji postaci/obrażeń i odtworzenia testu; nie wykluczono pośrednich zaokrągleń składowych obrażeń ani nieujawnionego modyfikatora. Autor sam zauważył różnicę w manie. Nie należy promować hipotezy B do wzoru runtime na podstawie tej jednej próbki. Poprawnie zaklasyfikować ją jako `DERIVED` — kandydat o **niskiej pewności co do rzeczywistego algorytmu**; dokładne obecne zaokrąglanie pozostaje `UNKNOWN/CONFLICT`.

W tekście źródłowym występuje literówka współczynnika `1+0.1*N`; [b0ni wskazał potrzebę N−1](https://discord.com/channels/836592213327609957/836593610811768833/1390346223080177836), a [tom ją zaakceptował](https://discord.com/channels/836592213327609957/836593610811768833/1390346272866828359). Przykłady autora już używają 1.2 dla trzech celów. Nie kopiować niepoprawnej wersji z pierwszego zdania.

**Wniosek:** mamy konkretną próbkę do falsyfikacji pojedynczego ceil przy zadeklarowanych parametrach, a nie tylko ogólny opis wiki. Nie rozstrzyga ona sama konfliktu współczynników z odpowiedzią TibiaQA z 2022 r. ani zachowania po zmianach z 2026 r.

### 4. Leech po zmianach z 2026 r.: główny beam i boczne wiązki

[Sabat, 2026-09-13](https://discord.com/channels/836592213327609957/1405864725367095317/1548629834958110781) opisuje **oddzielne naliczanie leech dla głównej wiązki i wiązek bocznych**. Szacuje wynik na około 30–40% więcej odzysku na cast w opisywanym zastosowaniu. To nie jest dowód stałego mnożnika 1.3–1.4: wynik zależy od podziału celów i damage między obszary. Nie ustalono również, czy każda boczna wiązka ma własną grupę, czy boczne cele tworzą jedną wspólną grupę.

To `DERIVED` z relacji konkretnego gracza, nie oficjalne potwierdzenie ani własny test. [Rozmowa z 2026-10-30](https://discord.com/channels/836592213327609957/1381954407716884630/1565847523019591772) nadal określa to jako beam leech bug; nie jest samodzielnym dowodem, że stan ten utrzymał się do 13 listopada. Nie ustalono, czy jest to zachowanie zamierzone.

Dodatkowo sprawdzono początek dyskusji z 2026-07-14. [Autor analizy zaznaczył](https://discord.com/channels/836592213327609957/1405864725367095317/1526678975966085120), że pierwszym materiałem były logi huntu w Gnompronie, **nie kontrolowany test**; później proszono o próbę na ośmiu Ravenous Lava Lurkers przy 16% leech bez Void's Call. Nie przedstawiam samego zaproszenia do testu jako jego wyniku.

**Rekomendacja:** rozdzielić test zwykłego AoE od testu Beam Mastery. Nie zakładać, że jeden cast zawsze odpowiada jednemu wspólnemu zbiorowi `N`. Relacja wrześniowa poprzedza datę odniesienia Oteryn 2026-09-27, ale ciągłość i dokładny sposób grupowania nadal wymagają pomiaru.

### 5. Cele immune / zero damage: pojawiły się konkretne relacje dotyczące N

[Free_Undertaker, Reddit, wątek z października 2020](https://www.reddit.com/r/TibiaMMO/comments/ji85yi/how_is_voids_call_calculated_for_aoe/) opisuje własną próbę Stone Shower na siedmiu Poison Spiderach i jednym orku: obrażenia i odzysk pochodziły od orka, ale obecność odpornych pająków ograniczała odzysk. Podana słownie skala spadku nie jest dokładną weryfikacją współczynnika ani zaokrągleń.

[Cereyon, 2025-09-11](https://discord.com/channels/836592213327609957/836593610811768833/1415696735207952507) wskazuje podobny efekt przy obiektach/przeciwnikach nieotrzymujących damage, takich jak grzyby lub filary w Bakragore. [tom, 2025-07-03](https://discord.com/channels/836592213327609957/836593610811768833/1390345495339204648) także opisuje obniżenie odzysku przez fizycznie odporne cele w obszarze.

Zgodne relacje wzmacniają kandydata **immune/zero-damage cele mogą nadal zwiększać mianownik N** (`DERIVED`), lecz nie stanowią jednego kompletnego, współczesnego kontrolowanego pomiaru. Nie uogólniać na wszystkie obiekty, summony ani osobno na blok/odporność: dokładny zbiór celów pozostaje otwarty.

### 6. Cleanse: sprawdzony starszy trop nie rozstrzyga odświeżania

[Felipe Domina Easy, 2020-06-07](https://forum.portaltibia.com.br/topic/37167-reset-charm-points/) opisuje dwie Plaguesmith i interpretuje ponowne sparaliżowanie jako ochronę tylko przed konkretnym napastnikiem. Brakuje czasów wszystkich proców i kontroli drugiego stanu. Ta obserwacja nie odróżnia pewnie ochrony per-napastnik, podmiany odporności i wygaśnięcia timera. Jest starszą hipotezą, nie rozstrzygnięciem dla 2026 r.

Późniejsza [relacja Danashiego z 2026-09-08](https://discord.com/channels/836592213327609957/1381954407716884630/1546838689164955668) opisuje równoczesne odporności i ochronę przed polami. Nie wolno z żadnego z tych wpisów samodzielnie wywieść dokładnego refreshu tej samej odporności.

### 7. Uzupełniona odtwarzalność wiki

Strony odczytano 2026-11-13 przez MediaWiki API (`prop=revisions`, `rvslots=main`, identyfikator i timestamp), nie tylko z fragmentu wyszukiwarki. Najważniejsze dokładne rewizje:

| Strona | Rewizja | Timestamp UTC |
|---|---:|---|
| [Formulae](https://tibia.fandom.com/index.php?title=Formulae&oldid=1205374) | 1205374 | 2026-09-07T17:28:39Z |
| [Parry](https://tibia.fandom.com/index.php?title=Parry&oldid=1084043) | 1084043 | 2024-11-25T22:37:06Z |
| [Cleanse](https://tibia.fandom.com/index.php?title=Cleanse&oldid=1203554) | 1203554 | 2026-08-25T21:03:44Z |
| [Carnage](https://tibia.fandom.com/index.php?title=Carnage&oldid=1188155) | 1188155 | 2026-06-01T21:06:35Z |

Pin rewizji poprawia odtwarzalność źródła, nie podnosi automatycznie mechaniki do `PROVEN`.

### Nierozstrzygnięte po tym przebiegu

Prawdziwy Mana Drain i pełna kolejność redukcji Parry; Dodge + Cleanse/Numb/Adrenaline Burst z tego samego unikniętego ataku; wtórny kill credit/XP/loot Carnage; dokładne obecne zaokrąglenia oraz grupy i zbiór celów leech; odświeżanie i zależność timerów Cleanse. Nie wykonano własnego testu gry. Nowe dowody zawężają hipotezy i korygują zakres źródeł, ale nie upoważniają do deklaracji pełnej zgodności runtime.


---

### Badanie 3/3 — komentarz 6860789655

[Oryginalny komentarz i data publikacji](https://github.com/Oteryn/Oteryn-Game/pull/3177#issuecomment-6860789655). **Pamiętaj o późniejszych sprostowaniach.**

## Runda 3 — nowy materiał liczbowy leech z 2026 r., oryginalne komunikaty Parry/Cleanse i przypięte rewizje wiki

Kontynuacja badania z 2026-11-13. Odczytano publiczne źródła, kontekst rozmów oraz obrazy/logi w przeglądarce właściciela. **Nie wykonano własnego eksperymentu na Global ani test serverze.** Komentarz uzupełnia dokument i koryguje wskazany niżej zakres poprzedniego komentarza. Nie zmienia kodu, kontraktu, daty odniesienia projektu ani zamrożonego HEAD `07511eee10b27a1283bc23e1340f2fd559142ef9`.

### R3-L1. Beam Mastery: opublikowany test nie pasuje do jednego wspólnego współczynnika leech dla całego czaru

**Źródło pierwotne materiału:** Porfz Tibia, film „QUATRO BUGS BIZARROS PÓS REBALANCEAMENTO (um buffa muito o MS)”, opublikowany **15 lipca 2026**. Odczytano metadane filmu, transkrypcję i pokazane klatki, a nie tylko opis lub komentarze. [Nagranie od 16:23](https://www.youtube.com/watch?v=YKA_VqNX1eI&t=983s).

W pokazanym teście osiem Lava Lurkerów stoi wokół innego gracza. Z logu odczytano:

| Część obszaru | Liczba celów | Damage każdego celu | Odzyskana mana za każdy cel |
|---|---:|---:|---:|
| Środkowa | 2 | 2086 | 207 |
| Boczna, łącznie obie strony | 6 | 1621 | 73 |

Suma widocznych przyrostów: `2 × 207 + 6 × 73 = 852 MP`. Log oznacza zwiększenie damage przez **Expose Weakness**. Nie przedstawiam tego jako próby bez wszystkich modyfikatorów.

**DERIVED — model zgodny z próbą, nie odczyt kodu CipSoft:** rozdzielić środkowe i boczne trafienia na grupy o liczebności `N=2` i `N=6`, a następnie zastosować kandydat wiki `f(N)=(0.1N+0.9)/N`.

Przy **założeniu obliczeniowym** `L=18%` i użyciu widocznego damage jako podstawy leech:

```text
centralne: ceil(2086 × 0.18 × 0.55) = ceil(206.514) = 207
boczne:   ceil(1621 × 0.18 × 0.25) = ceil(72.945)  = 73
razem:    852

jeden wspólny N=8:
centralne: ceil(2086 × 0.18 × 0.2125) = 80
boczne:   ceil(1621 × 0.18 × 0.2125) = 63
razem:    2 × 80 + 6 × 63 = 538, a nie 852
```

**Granice wniosku:** 18% jest podane w kontekście wcześniejszych logów omawianych przez autora, ale nie zweryfikowano niezależnie pełnego wyposażenia osoby wykonującej tę konkretną próbę na ośmiu celach. Dlatego pozostaje jawnie oznaczonym założeniem dla powyższego rachunku. Nawet bez wyboru 18%, jednakowy procent leech i jeden wspólny współczynnik mnożący pokazane obrażenia nie pasują równocześnie do 207/2086 i 73/1621, z uwzględnieniem zaokrąglenia w górę. Różne wewnętrzne podstawy damage lub modyfikatory części czaru wymagają osobnej kontroli.

Ta próba mocno wspiera **różną kalkulację części środkowej i bocznej**, a konkretnie model dwóch grup. Nie dowodzi trzech osobnych kolumn ani dokładnej struktury implementacji serwera. Nie ustalono dokładnego numeru klienta/serwera ani ciągłości tego zachowania do 13 listopada. Określenie „bug” w tytule filmu pochodzi od autora, nie od CipSoft.

**Zaokrąglenia:** wynik 852 sam w sobie nie rozstrzyga `ceil` per cel kontra `ceil` dopiero sumy każdej z dwóch grup, ponieważ tutaj oba warianty dają tę samą sumę. Indywidualne wpisy 207/73 są zgodne z modelem per cel. Nie rozszerzać tego na pełną obecną arytmetykę Life Leech.

**Pochodzenie / brak niezależności:** [kontekst z 14 lipca](https://discord.com/channels/836592213327609957/1405864725367095317/1526678975966085120) wprost mówi, że wcześniejsze dane z Gnomprony były logiem polowania, nie kontrolowaną próbą. [Wpis o 18%](https://discord.com/channels/836592213327609957/1405864725367095317/1526680551216517171) dotyczy tego wcześniejszego kontekstu. [Relacja Sabata z 13 września 2026](https://discord.com/channels/836592213327609957/1405864725367095317/1548629834958110781) także opisuje oddzielne rozliczanie leech części środkowej i bocznej, lecz może opierać się na tym samym odkryciu. Nie liczyć ich automatycznie jako niezależnych eksperymentów.

### R3-L2. Zaokrąglanie: konkretny stary log dla HP i MP, ale nie nowa replikacja

[Wątek z grudnia 2016, komentarz DasFrettchen](https://www.reddit.com/r/TibiaMMO/comments/5gsgz9/mana_leech_aoe_info/) podaje trzy Sea Serpenty, damage `757 / 759 / 754`, każdy z przyrostem **31 HP i 16 MP**, przy deklarowanych Intricate Vampirism 10% i Intricate Void 5%.

Przy współczynniku `f(3)=0.4` z modelu wiki: suma zaokrągleń osobnych wynosi `93 HP / 48 MP`, a zaokrąglenie dopiero sumy `91 HP / 46 MP`. To **historyczne wsparcie** osobnych wkładów, nie potwierdzenie produkcyjnej arytmetyki w listopadzie 2026. Sam wątek zawiera też dyskusję zmian test-serverowych i przybliżony współczynnik `0.07`, więc nie nadaje się do bezwarunkowego zatwierdzenia całego wzoru.

Konflikt `[8%+(N−1)×1%]/N` z [TibiaQA 2022](https://www.tibiaqa.com/24514/about-imbuements-vampirism-mana-leech-better-single-target-recover-most) kontra współczynnik wiki pozostaje jawny. Wynik specjalnego czaru z dwiema częściami nie jest uniwersalnym testem wszystkich zwykłych AoE i wartości leech.

### R3-P1. Parry: odnaleziony oryginalny oficjalny komunikat zamiast przedruku

**PRIMARY_OFFICIAL / PROVEN dla treści historycznej reguły:** [CipSoft, 23 listopada 2017, news 4386](https://www.tibia.com/news/?id=4386&subtopic=newsarchive), „Important Changes during the Test Server Phase”. Oryginał odczytano bezpośrednio w przeglądarce. Komunikat stwierdza, że odbicie Parry pomija odporności potwora i podlega jego pancerzowi.

To zastępuje zależność od wtórnego cytatu Cedeko jako dowodu **tej konkretnej historycznej reguły**. Jest to komunikat podsumowujący test przed ówczesnym Winter Update, a nie nowy pomiar w 2026 r. Nie rozstrzyga graczowego armor/shielding, późniejszej mitigation, rzeczywistego Mana Drain ani pełnej kolejności redukcji.

W historycznym [wątku o Asurach](https://www.reddit.com/r/TibiaMMO/comments/ispetq/ek_asuras_tips/) występuje anegdota o braku obrażeń przy odbiciu Mana Drain. Nie ma izolowanego zapisu ataku; nie wystarcza do zmiany klasyfikacji `UNKNOWN` na „nie odbija”.

### R3-C1. KOREKTA: Cleanse — Player Summon, nie dowolne przywołanie

**PRIMARY_OFFICIAL / PROVEN:** [CipSoft, 30 stycznia 2024, news 7736](https://www.tibia.com/news/?id=7736&subtopic=newsarchive), „Fixes and Changes”, odczytano oryginał. Wyłączenie Cleanse dotyczy napastnika określonego jako **Player Summon**.

Poprzedni komentarz w tym PR mówił zbyt szeroko o atakach „summonów”. **Nie należy na podstawie tej poprawki wyłączać także przywołań potworów/bossów. Ich aktualne zachowanie pozostaje osobnym pytaniem.** Starsza [relacja gracza z wątku o charmach](https://www.reddit.com/r/TibiaMMO/comments/14xydm1/meme_charm_runes_and_where_to_use_them_ive/) opisuje użycie Cleanse na przywołaniach bossa werehyena, ale pochodzi sprzed tej poprawki i nie stanowi replikacji aktualnego zachowania.

Ten sam oficjalny komunikat 7736 mówi, że obrażenia odbite przez gracza do potwora nie mogą zostać odbite ponownie. Jest to zasada **damage reflection**, nie dowód warunków łańcucha Carnage lub przypisania wtórnego zabicia.

### R3-W1. Zamknięta luka pochodzenia: dokładne rewizje wiki

Poniższe rewizje pobrano przez MediaWiki API, indywidualnie dla każdej strony, z granicą odczytu `2026-11-13T23:59:59Z`. Daty są znacznikami rewizji, **nie datami wykonania eksperymentu ani dowodem ciągłości zachowania gry**.

| Strona | Page ID | Revision ID | Znacznik czasu UTC |
|---|---:|---:|---|
| [Parry](https://tibia.fandom.com/wiki/Parry?oldid=1084043) | 87947 | 1084043 | 2024-11-25T22:37:06Z |
| [Cleanse](https://tibia.fandom.com/wiki/Cleanse?oldid=1203554) | 87952 | 1203554 | 2026-08-25T21:03:44Z |
| [Carnage](https://tibia.fandom.com/wiki/Carnage?oldid=1188155) | 107639 | 1188155 | 2026-06-01T21:06:35Z |
| [Formulae](https://tibia.fandom.com/wiki/Formulae?oldid=1205374) | 1092 | 1205374 | 2026-09-07T17:28:39Z |
| [Mana Leech](https://tibia.fandom.com/wiki/Mana_Leech?oldid=1101810) | 78087 | 1101810 | 2025-05-04T17:01:10Z |
| [Life Leech](https://tibia.fandom.com/wiki/Life_Leech?oldid=1101811) | 78085 | 1101811 | 2025-05-04T17:01:12Z |
| [Mitigation](https://tibia.fandom.com/wiki/Mitigation?oldid=995776) | 101538 | 995776 | 2023-03-13T22:51:21Z |

W `Formulae` faktycznie zapisano osobne obliczanie i zaokrąglanie w górę dla każdego celu, wyłączenie Damage Prey, wpływ critical i uwzględnianie overkill. To już odczyt konkretnego źródła, nie tylko fragmentu indeksu wyszukiwarki. Strona Life Leech nie podaje równie kompletnej arytmetyki.

`Cleanse` zawiera Hexed w rewizji z sierpnia 2026; starszy fragment indeksowany w wyszukiwarce tego nie zawierał. `Mitigation` ma rewizję z 2023 i nadal opisuje combat stances: nie jest wystarczającym źródłem aktualnego algorytmu po przebudowie profesji w 2026. `Carnage` utrzymuje opis własnego ostatniego ciosu, wyłączenia własnego summona/familiara, brak aktywacji po zabiciu przywołania potwora i brak rekurencji, ale nie podaje rozliczenia wtórnego kill credit/XP/loot.

### Co nadal pozostaje UNKNOWN i dlaczego

| Atomowy problem | Czego dokładnie brakuje |
|---|---|
| Parry: realny Mana Drain | Izolowanego ataku rozpoznanego jako Mana Drain, efektu Parry i zmiany HP napastnika; Magic Shield ani stary błędny analyzer nie wystarczą. |
| Parry: armor/shielding/mitigation | Porównania z kontrolą pojedynczej redukcji i aktualnym numerem wersji; oficjalny opis z 2017 nie ustala późniejszych etapów. |
| Dodge + Cleanse / Numb / Adrenaline Burst | Wspólnego zdarzenia dla **jednego** unikniętego ataku. Poof bez ubytku HP i samo przypisanie obu charmów do gatunku nie są takim dowodem. |
| Carnage: wtórne zabicie | Logu XP/loot/kill credit w układzie rozdzielającym właściciela charma, ostatni cios, największy damage i summona. Model DPS z Reddita tego nie testuje. |
| Leech: pełny obecny algorytm | Kontroli podstawy damage, Prey, obu rodzajów leech, limitu brakującego HP/MP, immune/zero/summon oraz grupowania złożonych czarów; nie traktować jednej próby beam jako całego silnika. |
| Cleanse: refresh i timery | Momentów uzyskania i wygaśnięcia dwóch odporności oraz ponownego zdarzenia tego samego typu. Relacja o współistnieniu odporności nie dowodzi niezależności timerów. |

**Weryfikacja tej rundy:** rachunki przeliczono dokładną arytmetyką wymierną; odczytano źródłowe logi/obrazy i kontekst ich autorów; błędne uogólnienie Player Summon zostało jawnie skorygowane. Nie podniesiono całej mechaniki do `PROVEN` na podstawie wiki, kalkulatora, modelu OTS lub pojedynczego filmu. Zebrany materiał nie zmienia samodzielnie daty odniesienia Oteryn 2026-09-27.

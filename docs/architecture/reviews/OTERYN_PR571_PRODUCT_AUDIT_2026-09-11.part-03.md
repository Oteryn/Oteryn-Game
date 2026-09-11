
Wynik klasyfikacji wpisów: **31 — BADANIE**, **3 — ODRZUCIĆ**, **25 — PÓŹNIEJ**, **10 — UZGODNIĆ**, **54 — ZACHOWAĆ**, **69 — ZMIENIĆ**. Liczby odnoszą się do powtarzających się wpisów źródłowych, nie liczby unikatowych funkcji.

## Aneks A. Wszystkie 128 propozycji historycznych

Nazwy i przypisania pochodzą z §6 master roadmapy. Oceny i zdania w ostatniej kolumnie są rekomendacjami tego audytu.

| ID | Propozycja źródłowa | Program | Rekomendacja | Uzasadnienie / zmiana |
|---|---|---|---|---|
| 1 | Huntfinder 2.0 | 2 | **ZMIENIĆ** | Zacząć od filtrowanej, redagowanej bazy huntów i wyjaśnienia dopasowania; prognozy XP/profitu oznaczać zakresem i pochodzeniem danych. |
| 2 | Hunting Spot Availability | 2 | **ZMIENIĆ** | Pokazywać przybliżony stan, aktualność i niepewność; nie ustanawiać własności respawnu ani ujawniać tożsamości polujących. |
| 3 | Party Finder 2.0 | 3 | **ZACHOWAĆ** | Jedno lobby z rolami, dostępami i zgodą na zaproszenie; mały wariant natychmiastowy przed rozbudowanym kalendarzem. |
| 4 | Account-wide quest progression | 9 | **ZMIENIĆ** | Współdzielić wybrane dostępy w Account+World, nie kopiować automatycznie ukończenia, wyborów i uprawnień do nagród. |
| 5 | Independent auto attack and spell casting | 15 | **UZGODNIĆ** | Najpierw ustalić zachowanie dla przyjętej granicy Reference; Evolved może zmienić harmonogram walki jako jawny balans, nie domniemaną naprawę błędu. |
| 6 | Character markers on minimap | 2 / 3 | **ZMIENIĆ** | Udostępniać znaczniki według uprawnień i kontekstu World/Channel/Instance; nowe informacje nie są automatycznie neutralnym UX Reference. |
| 7 | Loot System Rework | 4 | **ZMIENIĆ** | Najpierw ręcznie uruchamiane, ograniczone Loot Nearby i routing do pojemników; automatyczny loot wymaga osobnej polityki i oceny wydajności ekonomicznej. |
| 8 | Summon Loot Assistant | 4 | **PÓŹNIEJ** | Nie dublować Loot Nearby dodatkową symulacją pomocnika; wrócić tylko przy odrębnej wartości rozgrywkowej, bez przewagi wymagającej konkretnego summona. |
| 9 | Bank UI 2.0 | 5 | **ZACHOWAĆ** | Wprowadzić podgląd odbiorcy, kwoty, opłat i rezultatu; ponowienie tej samej autoryzowanej operacji nie może wykonać drugiego przelewu. |
| 10 | Transaction History | 5 | **ZACHOWAĆ** | Historia ma wynikać z trwałych operacji, nie klientowego logu; odróżniać zatwierdzone, odrzucone i nierozstrzygnięte wyniki. |
| 11 | Trade System 2.0 | 5 | **ZMIENIĆ** | Wspólny handel przedmiotami i bankowym złotem, z ponowną akceptacją każdej zmiany oraz jawnymi ograniczeniami lokalnej interakcji. |
| 12 | Death System 2.0 | 12 | **UZGODNIĆ** | Powiązać z #295 i aktualną architekturą Evolved; nie tworzyć konkurencyjnego modelu utraty poziomów obok kierunku no-delevel/DeathDebt. |
| 13 | Recovery Pool | 12 | **UZGODNIĆ** | Określić relację do DeathDebt i odzysku z ciała w #295; odzysk i Rested muszą mieć rozłączne cele i księgowanie. |
| 14 | Death Fatigue / Post-Death Weakness | 12 | **UZGODNIĆ** | Skonfrontować z istniejącym kierunkiem exhaustion; rekomendacja audytu: nie dodawać osobnej ogólnej kary do obrażeń, leczenia i obrony bez testu spirali zgonów. |
| 15 | Connection Loss Protection | 12 | **UZGODNIĆ** | Rozwijać przyjęte FND-04 zamiast drugiego systemu ochrony; nie uznawać pojedynczego rozłączenia za dowód winy albo niewinności. |
| 16 | Disconnect Abuse Detection | 18 | **ZMIENIĆ** | Analizować wzorce i korelacje w dłuższym czasie; sam sygnał utraty połączenia nie powinien uruchamiać automatycznej sankcji. |
| 17 | AI Anti-Bot / Anti-Cheat Platform | 18 | **ZMIENIĆ** | Najpierw serwerowa integralność i dane dowodowe; modele wykrywające anomalie później, z kontrolą błędów, odwołaniem i bez automatycznej winy. |
| 18 | PvP System 2.0 | 13 | **PÓŹNIEJ** | Wybrać jeden pierwszy tryb i jawny kontrakt nagród/strat; nie uruchamiać jednocześnie kilku niepowiązanych systemów konkurencji. |
| 19 | PvP rating / prestige / seasons | 13 | **PÓŹNIEJ** | Ranking i nagrody prestiżowe po sprawdzeniu aktywności populacji; unikać rozdrabniania małej społeczności na zbyt wiele kolejek. |
| 20 | High-Risk PvP Zones | 13 | **PÓŹNIEJ** | Ryzyko dobrowolne, z czytelną granicą i zasadami wyjścia; nie przenosić silniejszych strat na zwykłą grę bez zgody gracza. |
| 21 | Adaptive UI / Scalable Client | 1 | **ZACHOWAĆ** | Oddzielić skalę UI, DPI, fonty i kamerę; większy monitor nie powinien niejawnie nadawać większej informacji o świecie. |
| 22 | Layout Presets | 1 | **ZACHOWAĆ** | Wersjonować i ograniczać ustawienia; zapewnić odzysk układu po zmianie monitora i bezpieczny powrót do domyślnej konfiguracji. |
| 23 | Boss System 2.0 | 11 | **ZMIENIĆ** | Jeden model encounteru, uczestnictwa i nagród; odróżniać uprawnienia do walki od uprawnień do otrzymania wartości. |
| 24 | Adventure Guild Boss Hub | 11 | **PÓŹNIEJ** | Hub tylko dla odkrytych i odblokowanych treści; wygoda podróży nie może po cichu unieważniać dostępu, świata i ryzyka przejścia. |
| 25 | Flexible Boss Party Size | 11 | **ZMIENIĆ** | Wybrać encountery i obsługiwane składy; dostosować mechaniki, nie obiecywać dowolnego rozmiaru drużyny dla każdego bossa. |
| 26 | Boss Difficulty Tiers | 11 | **ZMIENIĆ** | Rozdzielić practice bez progresji od trybów z nagrodą; skalować mechaniki i budżet nagród, nie wyłącznie HP. |
| 27 | Personal Loot | 11 | **ZMIENIĆ** | Zachować wkład ról wspierających i wspólny budżet encounteru; samo dodanie uczestnika nie powinno niejawnie mnożyć gwarantowanej podaży. |
| 28 | Bad Luck Protection / Pity | 11 | **ZMIENIĆ** | Określić konkretną nagrodę, zakres licznika i rozkład czasu zdobycia; liczyć razem z personal loot i essence. |
| 29 | Boss Essence / Guaranteed Progression | 11 | **ZMIENIĆ** | Essence ma uzupełniać kontrolowaną podaż, nie być niezależnym dodatkiem do pełnego dropu oraz pity; oddzielić cele kosmetyczne i bojowe. |
| 30 | Threat-Based Monster Collision | 15 | **BADANIE** | Nie zmieniać przechodzenia przez potwory automatycznie według poziomu; wymaga to osobnej analizy topologii, walki i PvP. |
| 31 | Break Free / Emergency Breakthrough | 15 | **BADANIE** | Ocenić jako konkretną zdolność z kosztem i ograniczeniami, nie uniwersalną gwarancję wyjścia z niebezpiecznego ustawienia. |
| 32 | Transition Safety | 15 | **ZMIENIĆ** | Walidować prawidłowe lądowanie i przekazanie kontroli przy przejściach; naprawa błędnego wejścia nie oznacza odporności na legalne zagrożenie. |
| 33 | Guaranteed Escape Path | 15 | **BADANIE** | Zastąpić obietnicę gwarantowanej ucieczki wymaganiem poprawnych przejść; zachować znaczenie pozycji, blokowania i ryzyka. |
| 34 | Fishing System 2.0 | 16 | **PÓŹNIEJ** | Rozwijać różnorodność i aktywną interakcję jako dobrowolną aktywność; nie wymagać wędkowania do standardowego poziomu mocy bojowej. |
| 35 | Fishing Codex / Records / Tournaments | 16 | **PÓŹNIEJ** | Kolekcje i prestiż bez obowiązkowej przewagi bojowej; użyć wspólnych widoków kolekcji i jawnych zasad konkursów. |
| 36 | Cooking 2.0 | 16 | **PÓŹNIEJ** | Ograniczyć liczbę jednocześnie wymaganych buffów i obsługę; gotowanie ma oferować wybór, nie mnożyć przygotowań przed każdą sesją. |
| 37 | House Chef | 16 | **PÓŹNIEJ** | Usługa powiązana z zaakceptowanym housingiem, składnikami i recepturami; bez niejawnego uzależniania podstawowej skuteczności od Premium/domu. |
| 38 | Economy Sink Framework | 17 | **ZACHOWAĆ** | Rozróżniać tworzenie, niszczenie i transfer wartości; mierzyć całość World oraz rozkład majątku, nie tylko sumę złota. |
| 39 | House upgrades / services / luxury sinks | 16 / 17 | **UZGODNIĆ** | Przyjąć EXP-HOUSES-01 i rolę darmowej Residence; płatna wygoda/prestiż nie powinna podważać zaakceptowanej podstawowej funkcjonalności. |
| 40 | Equipment Durability | 6 | **ODRZUCIĆ** | Nie włączać rutynowego zużycia do pierwszego projektu produktu; brak wykazanej decyzji gracza uzasadniającej dodatkową obsługę i naprawy. |
| 41 | Repair All / Auto Repair | 6 | **PÓŹNIEJ** | Nie implementować bez osobno przyjętej trwałości; automatyczne usuwanie nowo stworzonej uciążliwości nie jest samodzielnym uzasadnieniem systemu. |
| 42 | Skill Wheel changes outside temple | 8 | **ZMIENIĆ** | Evolved: jawna klasa bezpiecznej strefy plus ograniczenia walki; Reference zachowuje udowodnione zasady wybranej wersji. |
| 43 | Skill Progression 2.0 | 8 | **ZMIENIĆ** | Najpierw określić relację skilli, treningu i oficjalnej proficjencji; nie dodawać czwartego ogólnego miernika mistrzostwa. |
| 44 | Real Combat Training | 8 | **ZMIENIĆ** | Wzmacniać sens aktywnej gry, ale porównywać role i buildy; szybsze animacje lub liczba trafień nie powinny automatycznie wyznaczać całej progresji. |
| 45 | Threat coefficient for classic skill gain | 8 | **BADANIE** | Wymaga scenariuszy i danych dla różnych klas; nie opierać progresji na niejawnej, zmiennej ocenie mocy gracza. |
| 46 | Diminishing returns on one persistent training target | 8 | **BADANIE** | Sprawdzić fałszywe ograniczenia normalnych huntów; preferować prostą kwalifikację aktywności zanim powstanie złożony system kar za powtarzalność. |
| 47 | Combat Activity Score | 8 | **BADANIE** | Tylko pomocniczy sygnał, nie nieprzejrzysty główny licznik skilli; aktywność wsparcia i dostępność muszą być uwzględnione. |
| 48 | Shielding progression in real combat | 8 | **ZMIENIĆ** | Powiązać rozwój z legalnie obsłużoną presją i rolą defensywną; nie nagradzać wyłącznie liczby bezwartościowych trafień. |
| 49 | Offline training as slower convenience path | 8 | **ZACHOWAĆ** | Zachować prostą przewidywalną ścieżkę; odróżnić ją w UI od treningu Exercise zużywającego zasoby. |
| 50 | Exercise weapons as faster gold-sink path | 8 / 17 | **ZACHOWAĆ** | Zachować zużycie i skuteczność zasobów; w Offline Exercise dopuszczać równoległe postacie konta bez utrzymywania fikcyjnych aktorów online. |
| 51 | Classic skills and Weapon Proficiency remain separate | 8 | **ZACHOWAĆ** | Różne liczniki i reguły, wspólne objaśnienie w panelu postaci; nie scalać ich księgowania wyłącznie dla wygody UI. |
| 52 | Natural hunts advance classic skills and Weapon Proficiency independently | 8 | **ZACHOWAĆ** | Oprzeć na zweryfikowanych zasadach obu systemów; Weapon Proficiency z walki istnieje już w Global, więc nie przedstawiać jej jako nowego wynalazku. |
| 53 | Equipment Presets | 6 | **ZMIENIĆ** | Jeden serwerowy loadout, z legalnością przedmiotów, czasem aktywacji i jawną odmową; atomowość nie oznacza darmowej natychmiastowej zmiany w walce. |
| 54 | Linked Tasks | 9 / 14 | **ZACHOWAĆ** | Wspólny dziennik i jawne zależności etapów; nie tworzyć osobnych sprzecznych flag dla identycznego zadania w kilku panelach. |
| 55 | Better Map UX | 2 | **ZACHOWAĆ** | Spójna minimapa i mapa świata, wyszukiwanie oraz warstwy odkrycia; ograniczyć spoilery według wybranej polityki produktu. |
| 56 | Castle / Battleground | 13 | **PÓŹNIEJ** | Jeden tryb celów z pomiarem udziału i strat, dopiero po grywalnej podstawie oraz wystarczającej populacji. |
| 57 | Prestige Arena | 13 | **PÓŹNIEJ** | Nie konkurować od razu z kilkoma innymi kolejkami PvP; prestiż przed transferowalną mocą i nagrodami za powtarzane zabójstwa. |
| 58 | Crash-aware boss cooldown compensation / Obelisk concept | 11 / 12 | **UZGODNIĆ** | Rozliczać potwierdzony incydent infrastruktury i rezultat encounteru dokładnie raz; utrata odpowiedzi nie dowodzi braku otrzymanej nagrody. |
| 59 | Dynamic Spawn and Hunting Capacity | 2 / 14 | **BADANIE** | Oddzielić dostępność kanałów od zwiększania tempa pozyskiwania nagród; badać łączny wpływ na podaż przedmiotów, XP i obciążenie całego World. |
| 60 | Bounty and Weekly Tasks Rework | 14 | **ZMIENIĆ** | Opisać rzeczywistą różnicę wobec Bounty/Weekly z 2025; dobór aktywności i zrozumiałe nagrody przed kolejną checklistą obowiązków. |
| 61 | Task Suitability and progression-aware Bounty offers | 14 | **ZACHOWAĆ** | Uwzględniać znane dostępy, rolę, oczekiwany wysiłek i realną dostępność treści; wyjaśniać powód rekomendacji. |
| 62 | Dynamic Bounty/Weekly kill-count and reward scaling | 14 | **ZMIENIĆ** | Zamrażać warunki przyjętego zadania; dostosowanie przyszłej oferty nie powinno przesuwać graczowi mety w trakcie realizacji. |
| 63 | Dedicated Bounty Equipment Slot | 14 / 6 | **UZGODNIĆ** | Zweryfikować oficjalny odpowiednik i własność slotu; nie tworzyć ogólnego drugiego ringu ani nieograniczonego dodatkowego budżetu statystyk. |
| 64 | Bounty Talisman Combat/Sustain/Spoils/Knowledge redesign | 14 / 6 | **BADANIE** | Zaprojektować konkurencyjne wybory z ograniczonym budżetem; nie przyznawać pełnego zestawu bonusów do walki, loota i wiedzy jednocześnie. |
| 65 | Weekly Delivery Economy Controller | 17 / 14 | **ZMIENIĆ** | Najpierw deterministyczny limit nowego popytu na przedmiot i World; uwzględnić małą płynność, a nie wyłącznie deklarowane ceny ofert. |
| 66 | Weekly Delivery Price Shock Guard | 17 | **ZMIENIĆ** | Wstrzymywać nowe generowane zamówienia przy niewiarygodnym rynku; nie cofać już zaakceptowanych warunków zadania. |
| 67 | AI-assisted economy forecasting with deterministic caps | 17 | **PÓŹNIEJ** | Najpierw dane oraz prosty model bazowy; prognoza AI doradcza, bez samodzielnego ustalania cen, dropu i nagród. |
| 68 | Bounty Spawn Allowance for overleveled task players | 14 | **PÓŹNIEJ** | Zależne od przyjęcia konkretnego dynamicznego spawnu; nie tworzyć wyjątku od ograniczenia, którego produkt jeszcze nie potrzebuje. |
| 69 | World Map & Discovery System 2.0 | 2 | **ZMIENIĆ** | Rozdzielić prezentację odkryć od nagród gameplayowych; porównać Discovery po aktualizacji 2026 przed definiowaniem różnicy Evolved. |
| 70 | Live Social Map & Permissioned Position Sharing | 2 / 3 | **ZMIENIĆ** | Wycofanie zgody ma usuwać aktualny dostęp; różne kanały/instancje pokazywać jawnie zamiast fałszywej wspólnej lokalizacji. |
| 71 | Shared Discovery Markers | 2 | **ZMIENIĆ** | Zakres odbiorców, wygaśnięcie i ręczna kontrola; nie zamieniać zgłoszeń rzadkich zdarzeń w publiczny system śledzenia. |
| 72 | Friends/VIP Mutual Consent & Privacy 2.0 | 3 | **ZMIENIĆ** | Oddzielić prywatne tagi od udostępniania obecności i pozycji; zachować profile prywatności oraz semantykę Reference. |
| 73 | Party System 2.0 | 3 | **ZMIENIĆ** | Oddzielić członkostwo World od lokalnych efektów walki i XP; uzgadniać role, cele i zakres informacji bez sztywnej obowiązkowej trójcy. |
| 74 | Party Combat Visibility | 3 | **ZMIENIĆ** | Poprawić czytelność legalnie znanych danych; wskaźniki poza ekranem i statusy podlegają polityce informacji, nie tylko ustawieniu klienta. |
| 75 | Shared Hunt Accounting / Party Loot Ledger | 3 / 5 | **ZMIENIĆ** | Rozróżnić wycenę, zrealizowaną sprzedaż i rozrachunek; zamrożony uzgodniony rachunek oraz osobna autoryzacja każdego obciążenia. |
| 76 | Itemization & Build System 2.0 | 6 | **ZMIENIĆ** | Ułożyć wspólny budżet mocy i warianty alternatywne; loadouty nie uzasadniają automatycznego przyjęcia Forge, +N i kolejnej proficjencji razem. |
| 77 | Build Impact & Comparison System | 6 | **ZACHOWAĆ** | Porównywać konkretne scenariusze obrażeń, ochrony i sustainu; nie ukrywać wad builda pod jedną liczbą Gear Score. |
| 78 | Training Arena / Combat Simulation | 6 / 15 | **ZACHOWAĆ** | Standardowe cele bez XP, loota, skilli i kolekcji; rozróżniać kalkulację teoretyczną od wyniku rzeczywistego scenariusza. |
| 79 | Inventory / Depot / Stash 2.0 | 4 | **ZACHOWAĆ** | Wyszukiwanie i porządkowanie przy zachowaniu praw dostępu i tożsamości przedmiotów; nie scalać fizycznego inventory z dowolnym magazynem świata. |
| 80 | Smart Item Reservation & Task Protection | 4 | **ZMIENIĆ** | Określić zakres Keep at least N i pierwszeństwo rezerwacji; chronić przed zbiorczą utratą, nie blokować niejawnie normalnego zużycia potionów. |
| 81 | Market sale-only commission | 5 / 17 | **BADANIE** | Porównać prowizję sprzedaży z limitem ofert i ewentualnym zwrotnym depozytem; darmowe wystawianie bez ograniczeń nie jest wymogiem wygodnego rynku. |
| 82 | Multi-Currency Market & Trade | 5 | **PÓŹNIEJ** | Tożsamość waluty przewidzieć w kontrakcie; wiele aktywnych rynków wdrażać dopiero po wykazaniu potrzeby i wpływu na płynność. |
| 83 | Imbuement System 2.0 | 7 | **ZMIENIĆ** | Najpierw konfiguracja, czytelność kosztów i obsługa; zmiana mocy, sukcesu i czasu to oddzielny zakres z konkretnym baseline Global. |
| 84 | Slot-Based Imbuement Library & Active Channels | 7 | **ZMIENIĆ** | Biblioteka przechowuje konfigurację, nie darmowe efekty; wyposażony przedmiot nadal określa dozwolone kategorie i liczbę aktywnych slotów. |
| 85 | Elemental Attunement Layer | 7 | **BADANIE** | Najpierw rozważyć wygodniejszy wybór w istniejącym budżecie; osobna ochrona obok pełnego crit/leech/skill wymaga kompensacji mocy. |
| 86 | Imbuement Slot / Attunement Progression | 7 | **BADANIE** | Preferować elastyczność konfiguracji nad dodatkową obowiązkową pionową progresją; wybrać konkretny cel przed projektowaniem poziomów. |
| 87 | Imbuement Progression & Maintenance Sink | 7 / 17 | **ZMIENIĆ** | Jednorazowy unlock oddzielić od odnawialnych zasobów; porównywać rzeczywiste systemowe odpływy złota i materiałów. |
| 88 | Effective-Use Imbuement Lifecycle | 7 | **BADANIE** | Ocenić skutki długości użytkowania i definicji aktywności dla każdej kategorii; nie uznawać wybiórczego zatrzymywania czasu za czysto wizualne QoL. |
| 89 | Vocation/Class Identity & Role Framework | 15 | **ZACHOWAĆ** | Opisać mocne strony, ograniczenia i wkład klas w różnych sytuacjach; używać aktualnego zestawu wspieranych vocations, także Monk. |
| 90 | Shared Role Taxonomy & Party Contribution Model | 15 / 3 | **ZMIENIĆ** | Role ułatwiają grupowanie i analizę, ale pojedynczy wynik wkładu nie może redukować leczenia, kontroli i przeżywalności do samego DPS. |
| 91 | Vocation Balance Target Bands & Telemetry | 15 | **ZACHOWAĆ** | Mierzyć obrażenia, leczenie, sustain, przeżywalność i użyteczność w tych samych scenariuszach; bez zmyślonych docelowych procentów. |
| 92 | Solo and Party Viability Balance Principle | 15 | **ZACHOWAĆ** | Zapewnić sens solo/duo/trio i korzyść z zespołu; porównywać również czas organizacji sesji, nie tylko najlepsze XP/h w walce. |
| 93 | Context-Separated PvE/Boss/PvP Balance | 15 | **ZACHOWAĆ** | Rozdzielać strojenie tam, gdzie potrzebne, z czytelnym UI i deterministycznymi regułami; unikać niejawnych wyjątków dla każdego spawnu. |
| 94 | Gem Atelier & Fragment Workshop parity/balance review | 6 | **UZGODNIĆ** | Nie zamrażać szczegółów na podstawie nazw; brakuje w audycie pełnego dowodu ich reguł dla granicy Reference 2026-07-28. |
| 95 | Bestiary staged Charm Point rewards | 10 | **BADANIE** | Niezmieniona suma końcowa zmienia moment dostępu do mocy; ocenić całą krzywą progresji i znaczenie pełnego ukończenia. |
| 96 | Bestiary knowledge-gated effective Charm levels | 10 | **BADANIE** | Nie karać podwójnym grindem za zmianę hunta; jasno pokazać odblokowaną korzyść i koszt wyboru w modelu wiedzy. |
| 97 | Charm Level 3 Mastery and Level 4 Grandmaster | 10 | **BADANIE** | Najpierw rozróżnić obecny oficjalny poziom od nowej warstwy; preferować ograniczone specjalizacje zamiast kolejnych procentów dla każdego. |
| 98 | Creature Family Mastery | 10 | **BADANIE** | Mastery jako opcjonalny cel wiedzy/prestiżu; bonusy rodzinne liczyć razem z Charmami, a nie jako darmową niezależną warstwę. |
| 99 | Persistent Charm assignments and loadouts | 10 | **ZACHOWAĆ** | Zapamiętać konfiguracje z zachowaniem limitów jednoczesnych przypisań i jawnych kosztów aktywacji. |

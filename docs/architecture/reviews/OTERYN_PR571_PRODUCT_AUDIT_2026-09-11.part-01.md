# Oteryn Evolved — końcowy audyt produktowy PR #571

Data audytu: **11 września 2026**. Zakres: projekt produktu, rozgrywki i interfejsu oraz zgodność propozycji z odczytaną architekturą Oteryn. **Nie jest to certyfikat implementacji, pełnej zgodności Reference, bezpieczeństwa produkcyjnego ani gotowości PR do scalenia.**

## 1. Podstawa i granice dowodowe

Odczytano PR `Oteryn/Oteryn-Game#571`, head `b2ef322791520130a2683f3004d26abd5c027ac8`, gałąź `docs/evolved-qol-feature-clusters-20260911`; PR był otwarty i draft. Bieżące odczytane `main`: `c6103b325748fd31268c7b332defcddf76c59a66`. Base SHA zapisany w PR: `90a3f92434e32354ff1aaeac96d038bbc49eba9c`. Zmiany PR obejmują dokładnie dwa dokumenty roadmapy/QoL oraz indeks architektury. [G01–G03]

Audyt obejmuje **wszystkie wpisy źródłowe w sekcjach 6–8 master roadmapy: 128 historycznych + 57 forum/QoL + 7 badawczych = 192 wpisy**. Wiele opisuje tę samą funkcję. Nie oznacza to 192 niezależnych systemów. R01–R07 są lokalnymi etykietami audytu dla siedmiu kandydatów wymienionych w źródle. [G02]

Rozróżnienia: **FAKT** oznacza bezpośrednio odczytaną treść; **WNIOSEK** jest interpretacją tych faktów; **REKOMENDACJA** jest propozycją audytu, nie decyzją właściciela; **NIEUSTALONE** oznacza brak wystarczających dowodów. Wszystkie dyspozycje w aneksach są rekomendacjami, włącznie z „ZACHOWAĆ”. Nie zastępują autorytetu przyjętych dokumentów ani nie nadają prawa wdrożenia.

Nie ma tu dowodu identyfikującego każdy pierwotny wątek forum przypisany do A–D. Przejrzane dyskusje Reddit są niezależnym potwierdzeniem występowania problemów, nie odtworzeniem pełnego pochodzenia inwentarza. Opinie nie są reprezentatywną ankietą, a dane liczbowe z komentarzy nie zostały przyjęte jako statystyki populacji.

## 2. Werdykt

**Zachować PR jako uporządkowany katalog kierunków, ale poprawić jego relacje z istniejącymi decyzjami oraz rozdzielić mocne QoL od nowych mechanik wymagających testów. Nie zatwierdzać 19 programów jako jednego pakietu realizacyjnego.**

Najsilniejsza proponowana przewaga Oteryn to spójny przepływ: znaleźć sensowną aktywność, zrozumieć wymagania, przygotować postać, zebrać drużynę, zagrać i bezpiecznie rozliczyć wynik. Dodatkowe poziomy mocy są zasadne tylko wtedy, gdy tworzą wybór z realnym kosztem alternatywnym.

## 3. Najważniejsze ustalenia wymagające korekty PR

### AUD-01 — Reference ma już ustaloną granicę, nie trzeba wybierać jej ponownie

**FAKT:** `OTERYN_PRODUCT_PROFILE_REFERENCE_TARGET_RECONCILIATION_2026-09-09.md` potwierdza przyjęty pierwszy target: zachowanie Global Tibia po server-save/maintenance **28 lipca 2026**. Jest to niezmienna granica pierwszej wersji. Nieustalone pozostają m.in. dowody poszczególnych mechanik, nie sam wybór daty. [G04]

**REKOMENDACJA:** w §1/§2 i zasadach wdrożenia wpisać tę granicę wprost. Każda propozycja powinna odróżniać: przyjęty Reference, późniejsze obserwacje Global oraz własny projekt Evolved. Aktualna wiki lub najnowszy komunikat nie przesuwają automatycznie Reference.

### AUD-02 — Hideout musi zostać uzgodniony z już przyjętym Residence

**FAKT:** `EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` przyjmuje rzeczywistą Residence bez ograniczonej podaży, dostępną bez Premium. Dla Account+World działa jeden osobisty slot: NONE, RESIDENCE albo PHYSICAL_HOUSE. Residence i zwykły dom fizyczny nie mogą należeć równocześnie do tego samego Account+World. Fizyczny dom ma jedną tożsamość i stan w całym World; gildyjne domy są odrębną klasą. [G05]

**REKOMENDACJA:** §4.16.6 i D15 powinny oznaczać rozwój i prezentację istniejącego Residence, a nie trzeci rodzaj niezależnej nieruchomości. „Uzupełnianie” domów prestiżowych jest poprawne na poziomie oferty produktu, nie jako obietnica dwóch równoczesnych osobistych nieruchomości. Utrzymać przyjęte bezpieczeństwo przejścia Residence → dom i ochronę przedmiotów. Nie przedstawiać zaakceptowanego modelu mieszkaniowego jako całkowicie otwartego pomysłu.

### AUD-03 — Rested, DeathDebt i odzysk z ciała nie mogą zniknąć z konsolidacji

**FAKT:** otwarty draft #295 na `5e74abdee0d55698c20fd7c06adb49ab778a00cd` utrwala kierunek właściciela: dodatnia pula Rested XP per postać, księgowanie EligibleRawXP, no-delevel osiągniętych poziomów, DeathDebt, kierunek exhaustion i odzysk z ciała. Dokładne wartości nie są tam zamrożone. Przyjęte uzgodnienie profili wskazuje ten kierunek jako Evolved-only i nie zatwierdza pakietu liczb. [G04, G06]

**REKOMENDACJA:** §4.8/§4.12 należy powiązać z tym toczącym się procesem zamiast wprowadzać konkurencyjny Recovery Pool i ogólną utratę poziomów. Rested oraz odzysk śmierci to dwa różne mechanizmy, wymagające jawnego sposobu wspólnego rozliczania. Nie nadawać draftowi statusu merged/accepted w całym zakresie.

Rekomendacja produktowa audytu pozostaje ostrożna wobec dodatkowego osłabienia obrażeń/leczenia/obrony po zgonie, ponieważ projektowo może ono utrudniać odzysk i powodować spiralę strat. To **propozycja do istniejącej decyzji o exhaustion**, nie automatyczne skreślenie wcześniejszego kierunku właściciela. Potrzebny jest test rozgrywki, nie deklaracja pewnego skutku.

### AUD-04 — Zmiana wygody może zmienić informacje, tempo akcji lub gospodarkę

**FAKT:** PR rozróżnia SHARED-UX i Evolved, ale etykiety dużych programów nie rozstrzygają wszystkich drobnych zachowań. [G02, G03]

**REKOMENDACJA:** badać trzy granice oddzielnie: informacja dostępna graczowi, wynik/timing komendy oraz wynik ekonomiczny. Większa czcionka zwykle jest prezentacją. Nowe dane o przeciwnikach poza ekranem, publiczny GPS, natychmiastowy pełny swap, dłuższy hunt dzięki większym stackom lub wybiórcze zużycie Prey to odrębne kwestie. Wygodniejszy interfejs nie jest automatycznym dowodem zgodności Reference.

### AUD-05 — Wspólna gospodarka World wymaga wspólnego budżetu skutków

**FAKT:** zakresy multichannel przewidują odrębny runtime spawnu w Channel, wspólny Market World, wspólną trwałą progresję oraz uprawnienia do nagród niezależne od prostego przejścia na drugi kanał. Członkostwo party może przekraczać kanały, ale efekt shared XP wymaga uprawnionych, współobecnych uczestników. [G07]

**REKOMENDACJA:** łącznie badać łatwiejszy loot, większe zapasy, automatyzację sprzedaży, dynamikę spawnu, Prey, personal loot, pity oraz essence. Nie zakładać, że każdy system z osobna można zwiększyć, pozostawiając gospodarkę bez zmian. „Global Market” oznacza tu World, nie automatycznie wszystkie światy. Wartości proponowanych nagród nie są ustalone tym audytem.

### AUD-06 — Priorytet architektury i kolejność dostarczenia funkcji są różne

**FAKT:** program buildów łączy loadouty z Forge Slot Mastery, +N i Equipment Proficiency; podobne połączenia występują w kolekcjach i progresji. Całe programy otrzymują wysokie priorytety. [G02]

**REKOMENDACJA:** obok ważności decyzji architektonicznej dodać osobny etap dostarczenia oraz listę zależności. Przewidzieć rozszerzalne granice przedmiotów/efektów/progresji teraz, lecz nie implementować wszystkich przyszłych mechanik przed grywalną podstawą. Forge Slot Mastery, inwestycja w item z odzyskiem i kolejne proficjencje to warianty wymagające wspólnego wyboru.

### AUD-07 — Obecność kodu fundamentów nie dowodzi grywalnej kompozycji

**FAKT:** `apps/client/src/lib.rs` na odczytanym main zwraca `PreNativeProtocol` i odmowę wejścia do gameplayu. Odczytany serwerowy bootstrap pozostaje `UnavailableBootstrap`. Nie przeczy to obecności domen i fundamentów. [G08, G09]

**REKOMENDACJA:** roadmapa ma kształtować przyszłe funkcje, a nie raportować ich gotowość. Nie tworzyć równoległego klienta ani backendu dla każdego programu. UI jest projekcją i źródłem intencji; Game zachowuje reguły, czas, przedmioty, uprawnienia i trwałe wyniki.

## 4. Co potwierdza badanie zewnętrzne

| Źródło | Bezpośrednio potwierdzony fakt / relacja | Znaczenie dla audytu |
|---|---|---|
| CipSoft, 21.07.2025 [W01] | Weapon Proficiency rozwijana przez wyposażenie broni i pokonywanie potworów; silniejsze cele dają większy postęp. | „Rozwój podczas prawdziwej walki” wymaga odróżnienia klasycznych skilli od istniejącej proficjencji. |
| CipSoft, 24.11.2025 [W02] | Hunting Task System przebudowano na Bounty i Weekly Tasks. | Opisać konkretną poprawę dopasowania i obsługi, nie tylko „Tasks 2.0”. |
| CipSoft, 13.07.2026 [W03] | Nowe bossy mają skalowaną trudność obejmującą mechaniki; rozwinięto personalizację dwóch slotów drzewa broni. | Przewaga Oteryn ma wynikać z jakości encounterów, grupowania i uczciwego postępu, nie samego istnienia tych funkcji. |
| TibiaWiki, Imbuements [W04] | Strona odnotowuje usunięcie niepowodzenia imbuowania w lecie 2025. | Nie opisywać usunięcia starej porażki jako niewątpliwie nowej przewagi Evolved; to źródło wtórne, nie pełny test targetu. |
| TibiaWiki, Prey [W05] | Opis wiąże zużycie z battle mode. Strona jest oznaczona jako wymagająca rewizji. | Zmiana na wyłącznie aktywność wybranego rodzaju potwora jest projektem ekonomicznym; dokładny target wymaga potwierdzenia. |
| TibiaPal [W08] | Autor udostępnia huntfinder, rozliczanie loota, narzędzia questów, skilli i buildów. | To praktyczny punkt odniesienia dla zintegrowanej obsługi we własnym kliencie, nie gotowe dane Oteryn. |

Dyskusja r/TibiaMMO z 2024 o solo/teamhuntach pokazuje zarówno skargi na sztywne terminy, oczekiwanie i przygotowania, jak i argumenty za nagradzaniem współpracy. To materiał jakościowy, powstały przed częścią późniejszych zmian gry, a nie podstawa przejęcia konkretnych procentów bonusu party. Rekomendacja: lepsze formowanie grup i sensowne mniejsze składy, bez automatycznego zrównywania wszystkich wyników. [W06]

Historyczna dyskusja o karze śmierci wskazuje, że część graczy ceni napięcie i ryzyko. Nie uzasadnia ani dowolnie wysokiej straty, ani jej całkowitego usunięcia. Przewodnik klienta i reakcje powracających graczy wspierają testowanie lepszego onboardingu. [W07, W10]

Wikipedia została przeczytana jako tło; artykuł ma ostrzeżenia o nieaktualnych fragmentach i problemach źródłowych. Nie służy tu za autorytet aktualnych wzorów ani parametrów mechanik. [W09]

## 5. Docelowe opisy kluczowych funkcji — proponowane zamienniki

Poniższe brzmienie nadaje się do dalszej redakcji PR. Każdy akapit to **REKOMENDACJA**, nie samodzielna zmiana przyjętej architektury.

**Equipment Loadouts.** Zapisana konfiguracja konkretnych przedmiotów, aktywowana przez autorytatywną operację Game. Zachowuje reguły slotów, dostępność, czas i ograniczenia walki. Wariant przygotowania i dopuszczalny wariant bojowy mają jawne polityki. Odmowa nie pozostawia niejawnego częściowego zestawu. A/B ring jest widokiem tego samego systemu. Bez domyślnego automatycznego wybierania kontr-zestawu przez klienta.

**Build Comparison.** Porównanie przed/po dla wybranych scenariuszy: typ obrażeń, przeciwnik, tryb gry, ważne efekty i koszty utrzymania. Pokazuje również pogorszenia. Liczba slotów imbuementu jest częścią budżetu wartości przedmiotu, nie dodatkiem poza nim. Biblioteka konfiguracji nie usuwa ograniczeń wyposażonego itemu. Jedna liczba Gear Score nie zastępuje wyboru między ochroną, obrażeniami i sustainem.

**Offline Exercise.** Trwałe zlecenie z zarezerwowanymi zasobami, przewidywalnym naliczaniem i jednoznacznym wynikiem start/przerwanie/koniec/awaria. Jedna sesja na postać; wiele postaci tego samego konta może trenować równolegle, każda płaci za własny wynik. To nie wymaga fikcyjnego klienta lub aktora online. Tryb autoryzowanego zapisu offline musi mieć przyjętego właściciela i kontrakt; nie wolno po prostu omijać reguł session/lease postaci. Nie zmieniać zaakceptowanej konkurencji postaci w limit jednej na konto.

**Smart Reservations.** Polityka „zachowaj co najmniej N” określa zakres zasobów i pierwszeństwo ręcznych/zadaniowych rezerwacji. Te same zasady stosują wszystkie właściwe zbiorcze operacje utraty wartości. Widok pokazuje bezpieczną nadwyżkę. Trzeba jawnie określić, czy rezerwacja dotyczy wyłącznie Stash czy większego obszaru; nie zakładać tego niejawnie. Nie blokować zwykłego leczenia dlatego, że UI zarezerwował potion do przyszłego planu.

**Hunt Accounting & Settlement.** Rachunek zawiera uczestników, okres, metodę wyceny, zużycie, opłaty i uzgodnione korekty. Loot oszacowany nie jest automatycznie sprzedanym lootem, a oszacowana należność nie daje prawa obciążyć konta. Przelewy mają osobną zgodę i jednoznaczny wynik. Zakup zapasu na kilka sesji nie jest automatycznie w całości kosztem jednej.

**Huntfinder & Availability.** Filtrowana baza huntów z wytłumaczeniem dopasowania, wymaganiami i informacją o aktualności. Zajętość jest oszacowaniem, nie nadaniem prawa do respawnu. Najpierw koordynacja i jawne kanały, następnie ewentualne eksperymenty pojemności/spawnu. Brak publicznego śledzenia konkretnych postaci, także przez pośrednie sygnały zajętości.

**Quest Access & Collections.** Przedstawienie wspólne, prawa i liczniki właściwe domenom. Dostęp, ukończenie, wybór fabularny i nagroda są rozróżnione. Account+World unlock dotyczy wskazanej treści. Story Mode ma ręcznie dobraną listę z osobnymi regułami nagród, a puzzle actors nie stają się uniwersalną bojową drużyną NPC. Etapowe Charm Points wymagają oceny czasu odblokowania mocy, nawet przy niezmienionej sumie.

**Boss Rewards.** Personal loot, pity i essence są częściami jednego kontrolowanego modelu podaży. Zasługi ról wspierających nie sprowadzają się do ostatniego trafienia ani DPS. Kopie encounteru na różnych kanałach nie nadają nowych uprawnień do nagrody. Kompensacja po awarii rozlicza potwierdzony stan serwera i historię nagrody, nie sam komunikat klienta o rozłączeniu.

**Prey & Imbuements.** Konfiguracja, aktywny bonus, opłacone zasoby i odnowienie są różnymi stanami. Można zapamiętać cel bez darmowego bonusu. Zmiana sposobu zużycia czasu wymaga oceny mieszanych spawnów, wsparcia, summonów i skutku ekonomicznego. Dodatkowej ochrony elementalnej nie dodawać bez uwzględnienia łącznej mocy wyposażenia.

**Residence / Housing.** Używać EXP-HOUSES-01, w tym bezpłatnej dostępności podstawowego Residence bez Premium oraz wyłączności osobistego slotu Account+World. Dom fizyczny pozostaje prestiżową, ograniczoną alternatywą; przejście między modelami nie kopiuje przedmiotów i nie otwiera niejawnego przełączania kanałów.

**Market & Economy.** Dokładne kwoty, trwała historia i prosty rynek jednej głównej waluty najpierw. Wiele walut, COD i logistyka później. Anonimowość publiczna nie usuwa wewnętrznego audytu. Cena oferty nie jest ceną zrealizowanej transakcji. Prowizja tylko od sprzedaży wymaga osobnej ochrony przed nadmiarem ofert, nie domyślnej sztywnej blokady pięciu minut.

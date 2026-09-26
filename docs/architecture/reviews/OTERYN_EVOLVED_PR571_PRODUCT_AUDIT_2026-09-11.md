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

**Safety, Anti-bot & Premium.** Najpierw serwerowa legalność i dowody integralności. Dopuszczone własne QoL nie może samo powodować uznania zachowania za botowanie. Modele ryzyka są pomocnicze, z analizą fałszywych trafień i procedurą odwołania. Premium i uprawnienia komercyjne pozostają w granicach Platform; brak mocy za przyszły dopiero opłacony staż. Nie automatyzować „AI uznało → ban”.

## 6. Końcowa dyspozycja 19 programów

„Wcześnie” oznacza pierwszeństwo produktu **po gotowości potrzebnych istniejących domen i alokacji**, a nie prawo do ominięcia Reference lub uruchomienia wszystkich zależności naraz.

| Program | Dyspozycja programu | Pierwszy sensowny zakres | Poza pierwszym zakresem |
|---|---|---|---|
| 1. UX/input/workspace/effects | Zachować wysoko | Skalowanie, trwały layout, obsługa wejścia, czytelne efekty | Nieudowodnione rozszerzenie FOV i wiedzy o świecie |
| 2. Mapa/discovery/huntfinder/capacity | Rozdzielić | Mapa, znane wymagania, wyszukiwanie i przybliżona zajętość | Dynamiczny spawn zależny od mocy |
| 3. Party/social/finder/accounting | Zachować wysoko, etapować | Lobby, role, ready check, zgodność dostępów | Złożony kalendarz i mapy taktyczne na start |
| 4. Inventory/loot/depot/stash | Zachować wysoko | Search, rezerwacje, post-hunt flow, bezpieczny batch | Osobny summon-looter i nieograniczony auto-loot |
| 5. Bank/trade/market/postal | Rozdzielić podstawę i dodatki | Dokładne kwoty, zwykły handel, historia, prosty rynek | Wielowalutowość, COD, ubezpieczenia |
| 6. Build/loadout/itemization/forge | Obowiązkowo rozdzielić | Loadouty, porównanie i kontrolowany test builda | Łączne przyjęcie +N, slot mastery i equipment proficiency |
| 7. Imbuement/attunement | Zachować cel, zbadać mechanikę | UI, konfiguracja i czytelność kosztów | Darmowa dodatkowa warstwa ochrony |
| 8. Skills/training/proficiency/wheel | Rozdzielić | Offline Exercise i proste objaśnienie progresji | Główny rozwój zależny od Activity Score |
| 9. Quest/unlocks/legacy | Zachować wysoko | Dziennik i jasne blokady; wybrany pilot dostępu | Automatyczne kopiowanie questów i solo całego endgame |
| 10. Collections/charms/echo/prey | Rozdzielić UI od mocy | Wspólna przeglądarka, presety trackerów | Bezwarunkowe nowe mastery i wzmacniacze |
| 11. Boss/encounters | Zachować, ograniczyć pierwszy zakres | Jeden encounter i wspólnie policzone nagrody | Dowolne składy dla wszystkich bossów |
| 12. Death/recovery/reconnect | Uzgodnić | Przyjęte FND-04 i kontynuacja #295 | Drugi konkurencyjny system długu/odzysku |
| 13. PvP/arenas/wars | Później | Jeden dopracowany tryb | Wiele kolejek/rankingów i nagrody za seryjne kille |
| 14. Tasks/weekly/spawn | Poprawić dopasowanie | Znane dostępy, warunki zamrożone przy akceptacji | Ruchoma meta zadania i niekontrolowany popyt |
| 15. Vocation/combat/balance | Fundament ciągły | Role, scenario matrix, pomiary solo i party | Automatyczne przechodzenie przez słabsze potwory |
| 16. Housing/lifestyle | Housing uzgodnić; lifestyle później | Istniejące Residence/House; dekoracja i usługi | Trzeci model mieszkania i obowiązkowe bojowe gotowanie |
| 17. Economy/sinks | Fundament pomiarowy | World ledger, źródła/odpływy, ograniczenia popytu | Autonomiczna gospodarka sterowana prognozą AI |
| 18. Security/anti-bot | Fundament integralności | Walidacja, dowody, rozsądne procedury | Ban na podstawie pojedynczego wyniku modelu |
| 19. Premium/tenure | Polityka wcześnie | Jasna granica wygody, stażu i mocy | Natychmiastowa progresja za przedpłacone lata |

## 7. Zalecana kolejność i metryki odbioru

**F0 — porządek decyzji i grywalny rdzeń.** Uzgodnić profile, housing i #295. Kontynuować istniejący Server Seam/Reference oraz właściwy program klienta. Przewidzieć granice rozszerzeń, ale nie zwiększać ścieżki krytycznej do pełnej realizacji roadmapy. Istniejące kontrakty czasu, tożsamości, trwałości i zasobów pozostają nadrzędne.

**F1 — mniej obsługi, lepsza informacja.** Po odpowiednich fundamentach: layout/input/effects, wyszukiwanie, podstawowe trackery i dziennik, bezpieczne UI banku/Market. Mierzyć czas do pierwszej aktywności, liczbę korekt błędów interfejsu, czytelność oraz zasoby klienta. Nie deklarować konkretnego zysku procentowego przed pomiarem.

**F2 — pełne przepływy aktywności.** Loadouty, rezerwacje, kontrolowany restock, lobby, rozliczenia, Offline Exercise i pilot dostępu legacy. Mierzyć czas przygotowania, czas zebrania drużyny, częstość odmowy z powodu braków, poprawność rozliczenia po reconnect, realne systemowe zużycie zasobów. Mniej kliknięć nie jest jedynym kryterium.

**F3 — różnice rozgrywkowe.** Dopiero po jawnej decyzji i scenariuszach: zmiany śmierci, boss reward economy, aktywne użycie Prey/imbue, skills i wybrane warianty progresji. Badawczo pozostają dynamiczne spawny, nowe collision/escape, afiksy i dodatkowe mastery. Mierzyć cały wynik sesji, rozkład czasu nagrody, ceny/płynność, podaż rzadkich przedmiotów i różnice klas, nie wyłącznie średnie XP/h.

### Proponowane testy — nie zostały wykonane w tym audycie

| Obszar | Minimalny scenariusz akceptacyjny |
|---|---|
| Input | W+A daje jeden legalny kierunek; brak dodatkowych kroków przy release, focus loss, wejściu w czat i scrollu UI. |
| Loadout | Niedostępny/przeniesiony przedmiot i ograniczenie walki dają jawną odmowę; brak ukrytego połowicznego zestawu lub drugiego efektu po ponowieniu. |
| Rezerwacje | Wszystkie objęte polityką zbiorcze operacje respektują tę samą ilość, także po zmianie stanu między podglądem a wykonaniem. |
| Settlement | Zmiana rachunku wymaga nowej zgody; ponowienie nie obciąża ponownie, a nierozstrzygnięty commit jest uzgadniany. |
| Offline Exercise | Przerwanie, logowanie, restart i równoległe postacie zachowują ten sam wynik/zużycie co autoryzowane sesje; zasoby nie mogą pracować w dwóch rolach naraz. |
| Quest/boss | Przejście kanału i ponowny odbiór nie nadają drugiej nagrody; Story Mode nie zalicza bez decyzji normalnego farmienia/kolekcji. |
| Residence | Account+World nie otrzymuje dwóch osobistych nieruchomości; przejście mieszkania i awaria zachowują przedmioty oraz jeden końcowy wynik. |
| Prywatność | Wycofanie zgody, zmiana kanału i opóźniony update nie przywracają niedozwolonego dostępu do położenia. |
| Efekty/progresja | Reconnect nie odtwarza zużytego czasu; Rested i odzysk śmierci nie księgują tej samej korzyści dwukrotnie. |
| Budżet ekonomii | Wspólnie testowane loot/pity/essence/Prey/spawn nie przekraczają zaakceptowanych limitów; limity muszą dopiero wynikać z modelu i decyzji. |

## 8. Jeden proponowany wyróżnik integrujący istniejące programy

**Plan aktywności**: karta hunta, bossa lub questa łącząca odwołania do wymagań, znanej trasy, loadoutu, zapasów i lobby, a po grze do rachunku i kontrolowanego uzupełnienia. Nie jest dwudziestym autorytatywnym systemem. Nie gra za gracza, nie kupuje bez zgody i nie obchodzi dostępu do usług. Ma zmniejszać czas organizacji, zachowując wybory i konsekwencje.

Uzupełnienia w istniejących programach: opcjonalne wprowadzenie dla nowych/powracających graczy oraz raport przyczyn zgonu ograniczony do legalnie znanych informacji. Raport nie daje wiedzy o ukrytych graczach ani nie jest automatycznym dowodem błędu serwera.

## 9. Brakujące dane i konsekwencje

1. **Pierwotne URL-e/daty forum dla A–D:** nie można certyfikować kompletności przepisywania oryginalnych dyskusji. Można ocenić wszystkie propozycje obecne w PR.
2. **Pełne evidence poszczególnych mechanik dla 2026-07-28:** szczególnie dokładne reguły Gem Atelier/Fragment Workshop, Bounty Talisman, Forge i nagród. Nazwa systemu ani bieżąca wiki nie zastępuje dowodu targetu. Takie szczegóły pozostają NIEUSTALONE, zamiast być wypełniane przypuszczeniami.
3. **Reprezentatywna telemetria Oteryn i playtesty:** bez nich nie można uznać konkretnych cen, procentów, limitów nagród, stacków i skilli za optymalne. Oceny programu są rekomendacjami, nie prognozą z udowodnioną skutecznością.
4. **Nierozstrzygnięte decyzje właściciela:** m.in. wartości śmierci, wariant nowej itemizacji i warunki komercyjne. Audyt ich nie akceptuje samodzielnie.

Część bezpośrednich stron Tibia.com zwracała odmowę dostępu; komunikaty wydawcy CipSoft były dostępne. Dokładne stany mechanik nie zostały odtworzone w działającym kliencie Global. Nie wykonano testów repo, CI, benchmarków ani aktualizacji PR. Sprawdzono jedynie kompletność identyfikatorów i spójność wygenerowanego rejestru audytu.

## 10. Źródła i siła dowodowa

Wszystkie odczyty audytu: 2026-09-11. Adresy poniżej wskazują materiały dowodowe, nie dodatkowe zadania do wykonania przez czytelnika. Tekst nie zawiera pełnych kopii dokumentów zewnętrznych.

### Repozytorium — bezpośrednie źródła

- **G01:** metadane i lista zmienionych plików PR `Oteryn/Oteryn-Game#571`; head `b2ef322791520130a2683f3004d26abd5c027ac8`. Adres: `https://github.com/Oteryn/Oteryn-Game/pull/571`.
- **G02:** `docs/architecture/OTERYN_EVOLVED_MASTER_PRODUCT_ROADMAP_2026-09-11.md` na head G01; sekcje 4 oraz 6–11. Inwentarz i opisy propozycji, nie przyjęta implementacja.
- **G03:** `docs/architecture/OTERYN_EVOLVED_QOL_FEATURE_CLUSTERS_2026-09-11.md` na head G01. Starszy inwentarz 39 źródeł A–C i 13 klastrów, zachowany celowo jako materiał wejściowy; brak D w jego końcowym liczeniu nie jest sam w sobie błędem master roadmapy.
- **G04:** `docs/architecture/OTERYN_PRODUCT_PROFILE_REFERENCE_TARGET_RECONCILIATION_2026-09-09.md` na `main@c6103b325748fd31268c7b332defcddf76c59a66`. Przyjęty target i odróżnienie niekompletnych dowodów od braku decyzji.
- **G05:** `docs/architecture/EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` na tym samym main, zwłaszcza §3–7. Owner-accepted architektura mieszkaniowa, nie dowód runtime.
- **G06:** otwarty draft `Oteryn/Oteryn-Game#295`, head `5e74abdee0d55698c20fd7c06adb49ab778a00cd`; opis checkpointu i wyraźne zakresy niezamrożonych wartości. Adres: `https://github.com/Oteryn/Oteryn-Game/pull/295`.
- **G07:** `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` na wskazanym main. Zakresy World/Channel i praw do nagród; starsze fragmenty housing/session nie zastępują późniejszych przyjętych właścicieli zakresu.
- **G08:** `apps/client/src/lib.rs` na wskazanym main. Bezpośredni odczyt dostępności gameplayu klienta.
- **G09:** `apps/game-server/src/lib.rs` oraz `apps/game-server/src/main.rs` na wskazanym main. Odczyt bootstrapu serwera, nie ocena wszystkich zaimplementowanych domen.
- **G10:** `AGENTS.md`, `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`, `crates/input-actions/src/lib.rs`, `workspace-boundaries.toml` na wskazanym main. Własność, ograniczenia i istniejące fundamenty; testy źródłowe nie zostały tu uruchomione.

### Zewnętrzne źródła — pierwsze i wtórne

- **W01 — PRIMARY:** CipSoft, „Tibia: Summer Update 2025 Now Available”, 21.07.2025. `https://www.cipsoft.com/en/395-tibia-summer-update-2025-now-available`.
- **W02 — PRIMARY:** CipSoft, „Tibia: Winter Update 2025 Now Available”, 24.11.2025. `https://www.cipsoft.com/en/413-tibia-winter-update-2025-now-available`.
- **W03 — PRIMARY:** CipSoft, „Tibia: Summer Update 2026 Now Available”, 13.07.2026. `https://www.cipsoft.com/en/440-tibia-summer-update-2026-now-available`.
- **W04 — SECONDARY:** TibiaWiki BR, „Imbuements”, bieżący odczyt. `https://www.tibiawiki.com.br/wiki/Imbuements`. Historia zmian i wskazówka weryfikacyjna, nie pełny dowód wszystkich reguł dla targetu.
- **W05 — SECONDARY / NEEDS REVIEW:** TibiaWiki BR, „Prey System”. `https://www.tibiawiki.com.br/wiki/Prey_System`. Sam artykuł ma kategorię „Pedido de Revisão”; wartości i zasady wymagają dokładnego potwierdzenia przed Reference.
- **W06 — PLAYER TESTIMONY:** r/TibiaMMO, „[TibiaLee] TEAMHUNTING VS SOLOHUNTING TIBIA 2024”, wątek z 2024. `https://www.reddit.com/r/TibiaMMO/comments/1bn8xdn/tibialee_teamhunting_vs_solohunting_tibia_2024/`. Różne stanowiska o elastyczności grup i nagradzaniu współpracy; nie reprezentatywna ankieta.
- **W07 — PLAYER TESTIMONY / HISTORICAL:** r/TibiaMMO, „The death penalty does create an adrenaline rush…”, 2017. `https://www.reddit.com/r/TibiaMMO/comments/6t8d0k/the_death_penalty_does_create_an_adrenaline_rush/`. Do oceny preferencji ryzyka, nie aktualnych parametrów.
- **W08 — PRIMARY ABOUT OWN TOOLS:** TibiaPal, aktualna strona narzędzi i historia aktualizacji autora. `https://tibiapal.com/`. Dowód funkcji narzędzia, nie walidacja wszystkich cen i rekomendacji dla Oteryn.
- **W09 — BACKGROUND:** Wikipedia, „Tibia (video game)”. `https://en.wikipedia.org/wiki/Tibia_(video_game)`. Ostrzeżenia o aktualności i źródłach; brak podstawy do ustalania dokładnych mechanik.
- **W10 — PLAYER TUTORIAL:** r/TibiaMMO, „Tibia 101 - Game Client, Settings, hotkeys and more”, 2023. `https://www.reddit.com/r/TibiaMMO/comments/18lkhk0/tibia_101_game_client_settings_hotkeys_and_more/`. Materiał o obsłudze klienta oraz reakcje części powracających graczy, nie pomiar populacyjny.

## 11. Znaczenie dyspozycji w pełnym rejestrze

- **ZACHOWAĆ:** mocny kierunek, zachować w planie; nadal wymaga konkretnych zasad, istniejących zależności i kwalifikacji.
- **ZMIENIĆ:** problem jest zasadny, ale proponowaną mechanikę, zakres lub opis należy doprecyzować.
- **PÓŹNIEJ:** kandydat o niższym priorytecie lub zależny od nieistniejącej jeszcze funkcji.
- **BADANIE:** brak podstaw do zamrożenia projektu; potrzebna próba wariantów, konkretne evidence lub model bilansu.
- **UZGODNIĆ:** najpierw powiązać z już przyjętą architekturą, bieżącą decyzją właściciela lub brakującym dowodem Reference.
- **ODRZUCIĆ:** rekomendacja nieprzyjmowania podanej postaci pomysłu; nie jest samodzielnym cofnięciem przyjętej decyzji właściciela.


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
| 100 | Drome Charm amplifiers and optional Grandmaster catalysts | 10 | **BADANIE** | Zweryfikować bieżące nagrody i pełny łączny budżet; nie promować nowego wzmacniacza tylko dlatego, że ma własne źródło waluty. |
| 101 | Quest Journal 2.0 and quest dependency graph | 9 | **ZACHOWAĆ** | Jeden model znanych etapów i zależności; klient wyświetla projekcję, nie tworzy drugiego autorytatywnego grafu ukończenia. |
| 102 | Current Objective and quest blocker explanation | 9 | **ZACHOWAĆ** | Wyjaśniać brakujące znane wymagania; wskazówki i spoilery regulować oddzielnie od prawa dostępu do questów. |
| 103 | Party Quest Sync | 9 / 3 | **ZACHOWAĆ** | Pokazać zgodność etapów i możliwość udziału bez automatycznego zaliczenia pominiętych misji. |
| 104 | Quest Renown / Adventure Points | 9 | **PÓŹNIEJ** | Kosmetyka, tytuły i usługi przed stałym bonusem do walki; nie tworzyć obowiązkowego licznika wszystkich starych questów. |
| 105 | Region Mastery | 9 | **PÓŹNIEJ** | Powiązać z odkrywaniem i lokalnymi historiami; nie dublować Discovery ani przypadkowo zmieniać już przyjętych korzyści Reference. |
| 106 | Legacy quest reward modernization and bounded XP scaling | 9 | **ZMIENIĆ** | Zachować ikoniczne nagrody i jawne limity; określić uprawnienia alts/Account+World zanim dodane zostaną wartości skalowane poziomem. |
| 107 | Postman Quest modernization pilot | 9 / 5 | **PÓŹNIEJ** | Dobry ograniczony pilot dostępu i usług pocztowych; nie kasować całej tożsamości questa dla jednego przycisku teleportu. |
| 108 | Postal Network 2.0 | 5 / 9 | **PÓŹNIEJ** | Najpierw podstawowa niezawodna dostawa; COD, ubezpieczenia i poziomy usług jako oddzielne późniejsze decyzje wartości i odpowiedzialności. |
| 109 | Global Market plus Local Logistics | 5 | **ZMIENIĆ** | Global oznacza wspólny World, nie wszystkie światy; lokalna dostawa musi dawać wybór zamiast odtwarzać usuwaną uciążliwość. |
| 110 | Consolidated market delivery | 5 | **PÓŹNIEJ** | Łączyć kompatybilne zakupy w ograniczone partie z czytelnymi wynikami; nie projektować nieskończonej jednej transakcji Collect Everything. |
| 111 | Forge Slot Mastery | 6 | **BADANIE** | Porównać inwestycję w slot z inwestycją w przedmiot i odzyskiem; wybrać model zamiast nakładać wszystkie warianty. |
| 112 | Item Enhancement +N | 6 | **BADANIE** | Wymaga wspólnego limitu mocy i odrębnego uzasadnienia; nie wchodzi automatycznie wraz z loadoutami. |
| 113 | Equipment Proficiency | 6 | **BADANIE** | Wykazać różnicę wobec Weapon Proficiency i klasycznych skilli; nie dodawać kolejnej obowiązkowej progresji każdej kategorii ekwipunku. |
| 114 | Item Classification as progression ceiling | 6 | **PÓŹNIEJ** | Tylko jeśli przyjęto system ulepszeń; nie budować nowej drabiny rzadkości wyłącznie po to, by ograniczyć inny nieprzyjęty system. |
| 115 | Equipment proficiency branches and qualitative capstones | 6 | **BADANIE** | Rozłączne wybory i realny koszt alternatywny; odblokowanie wszystkiego naraz nie jest specjalizacją. |
| 116 | Controlled Enhancement RNG and pity | 6 | **BADANIE** | Warunkowe względem przyjęcia enhancementu; ograniczyć skrajnego pecha i wykluczyć rutynową katastrofalną utratę inwestycji. |
| 117 | Controlled duplicate-item sink | 6 / 17 | **BADANIE** | Ocenić płynność i popyt na konkretne przedmioty; nie wymagać masowego spalania ultrarzadkich BIS do normalnej konkurencyjności. |
| 118 | Enhancement Salvage and equipment-replacement protection | 6 | **PÓŹNIEJ** | Dopiero po wyborze progresji; odzysk musi zachować wartość i pochodzenie, bez równoczesnego pozostawienia pełnej inwestycji w starym przedmiocie. |
| 119 | Existing tier migration contract | 6 | **PÓŹNIEJ** | Wymaga rzeczywistych stanów i modelu docelowego; nie zakładać w tym audycie istniejącej produkcyjnej populacji ulepszonych przedmiotów Oteryn. |
| 120 | Controlled Retaliation / reflect safety | 15 / 6 | **ZMIENIĆ** | Określić limity, źródła i brak wzajemnych nieograniczonych proców; sprawdzać grupy napastników, summony oraz wkład innych efektów. |
| 121 | Selected-creature active-use Prey timer | 10 | **BADANIE** | To zmiana efektywnej wartości czasu, nie sam UX; opisać aktywność mieszanego spawnu, wsparcia i summonów oraz budżet ekonomiczny. |
| 122 | Dormant Prey target reservation | 10 | **ZACHOWAĆ** | Zapamiętanie celu bez czynnego bonusu po wyczerpaniu czasu; UI jednoznacznie rozróżnia rezerwację od aktywacji. |
| 123 | Separate target reservation, bonus preservation and time renewal | 10 | **ZACHOWAĆ** | Osobne stany z jawnymi kosztami i ograniczeniami przełączania; bez jednego nieczytelnego przycisku łączącego różne zobowiązania. |
| 124 | Bankable free Prey Charges | 10 | **ZMIENIĆ** | Ograniczony zapas z jasnym naliczaniem; unikać wymogu codziennego kliknięcia tylko po to, by nie utracić okazji. |
| 125 | Sustainable Prey maintenance paths | 10 / 17 | **ZMIENIĆ** | Jawne opcje gry i złota; opcja komercyjna nie jest z góry przyjęta i musi podlegać osobnej polityce Platform/Evolved. |
| 126 | Mixed-spawn and party-safe Prey consumption | 10 | **ZMIENIĆ** | Naliczanie nie może opierać się wyłącznie na ostatnim trafieniu; niezależne stany graczy i poprawne przypisanie summonów. |
| 127 | Prey loadouts and cost controls | 10 | **ZACHOWAĆ** | Zapamiętywać wybory i pułapy wydatków; żadnych nieautoryzowanych zakupów ani klientowej kontroli ważności bonusu. |
| 128 | Prey migration and switching safeguards | 10 | **ZACHOWAĆ** | Wersjonować uprawnienia, koszty i stany; reconnect i ponowienie operacji nie tworzą dodatkowego czasu ani nie kasują zatwierdzonej wartości. |

## Aneks B. Wszystkie 57 postulatów forum/QoL

Identyfikatory i nazwy pochodzą z §7 master roadmapy. To zmapowany inwentarz problemów, nie dowód reprezentatywności ani pełna identyfikacja pierwotnych wątków.

| ID | Propozycja źródłowa | Program | Rekomendacja | Uzasadnienie / zmiana |
|---|---|---|---|---|
| A01 | Market/Tibia Coin anti-bot cooldown | 5 / 17 | **ZMIENIĆ** | Nie zamrażać pięciominutowej blokady; limity operacji i liczby ofert mają chronić rynek bez utrudniania zwykłej korekty ceny. |
| A02 | Incomplete Bestiary indicator | 10 | **ZACHOWAĆ** | Wskaźnik korzysta z istniejącego stanu wiedzy; nie prowadzić osobnego licznika ukończenia. |
| A03 | Collect All exclusions | 4 | **ZACHOWAĆ** | Zachować wybrane nagrody w skrzyni, ale pokazywać termin wygaśnięcia; marker nie wydłuża automatycznie retencji. |
| A04 | Remember Offline Training choice | 8 | **ZACHOWAĆ** | Zapamiętany wybór per postać, bez automatycznego wydawania innych zasobów. |
| A05 | Exercise weapons while offline | 8 / 17 | **ZACHOWAĆ** | Jedna sesja per postać, wiele postaci konta równolegle; te same zasoby, tempo i skuteczność, trwałe rozliczenie zamiast klienta online. |
| A06 | More outfit colours | 1 | **ZACHOWAĆ** | Ograniczona, dostępna paleta z zachowaniem czytelności; zgodność kolorów z Reference ocenić oddzielnie od możliwości edytora. |
| A07 | Random outfit colours | 1 | **ZACHOWAĆ** | Losowanie całości lub wybranych części, blokady i podgląd; nie wymaga zmian ekonomii ani osobnego systemu rozgrywki. |
| A08 | Party map/GPS | 2 / 3 | **ZMIENIĆ** | Tylko dozwolone udostępnianie, z jawnym kanałem/instancją i aktualnością; brak publicznego GPS wszystkich postaci. |
| A09 | Full equipment-set hotkeys | 6 | **ZMIENIĆ** | Widok systemu Equipment Loadouts, nie makro wielu pakietów ani obejście ograniczeń ekwipowania. |
| A10 | Separate Event Chat | 1 / 3 | **ZACHOWAĆ** | Typowane kategorie wiadomości, filtry i przypięte widoki zamiast osobnego niezależnego backendu czatu. |
| A11 | Auto-drop empty flasks | 4 | **ZMIENIĆ** | Domyślnie przeniesienie albo zachowanie; świadoma utylizacja z regułą, bez automatycznego zaśmiecania terenu. |
| A12 | Larger potion/rune stacks | 4 | **ZMIENIĆ** | Maksimum per definicja; waga, pojemność i limit zapasów nadal jawne. Większy stack może zmienić długość hunta. |
| A13 | Sell directly from Stash | 4 / 5 | **ZMIENIĆ** | Zachować wymagany dostęp do usługi NPC, rezerwacje i podgląd ceny; sprzedaż jest serwerową operacją wartości. |
| A14 | Mount attributes | 16 | **PÓŹNIEJ** | Preferować kosmetykę i ostrożną użyteczność podróżną; wykluczyć niejawny obowiązkowy bojowy BIS z płatnego mounta. |
| A15 | Bank k/kk shorthand | 5 | **ZACHOWAĆ** | Dokładna arytmetyka całkowita, znormalizowana kwota przed potwierdzeniem; jednoznaczne reguły przecinka, kropki i jednostek. |
| A16 | Item Deck / Codex | 10 | **ZACHOWAĆ** | Oddzielić zobaczone/posiadane od zdobytego własnym działaniem; wspólna przeglądarka nie nadaje automatycznie nagród. |
| A17 | Level in VIP/contact list | 3 | **ZMIENIĆ** | Pokazywać dane według przyjętej widoczności i aktualności, nie nową nieograniczoną publikację stanu gracza. |
| A18 | Guild in chat/VIP/contact surfaces | 3 | **ZMIENIĆ** | Spójna projekcja członkostwa i prywatności; widok nie jest drugim źródłem prawdy o gildii. |
| B01 | In-game UI scaling | 1 | **ZACHOWAĆ** | Wspólny system DPI, skali interfejsu i fontów, niezależny od pola widzenia świata. |
| B02 | Larger/resizable minimap | 1 / 2 | **ZACHOWAĆ** | Zwykły skalowalny panel z tą samą mapą danych; większe okno nie omija zasad odkrycia i udostępniania. |
| B03 | Party members on minimap | 2 / 3 | **ZMIENIĆ** | To samo rozwiązanie co A08; bez osobnego systemu lokalizacji. |
| B04 | Mouse wheel hotkeys | 1 | **ZMIENIĆ** | UI najpierw konsumuje scroll; impulsy gry ograniczone i normalizowane, bez mnożenia akcji przez swobodnie obracane kółko. |
| B05 | Protection Set Management | 6 | **ZMIENIĆ** | Jeden Equipment Loadouts z podglądem ochrony i przyczyną odmowy, nie osobny typ zestawu z innymi regułami. |
| B06 | Skill Wheel in broader PZ/safe zones | 8 | **ZMIENIĆ** | Jawna polityka Evolved bez zakodowania nazwy Temple w silniku; Reference tylko zgodnie z dowodami. |
| B07 | More VIP groups | 3 | **ZACHOWAĆ** | Prywatne tagi i filtry z ograniczoną liczbą przechowywanych ustawień; nie wymagają nowej struktury społecznej. |
| B08 | Persistent/pinned read-only tabs | 1 | **ZACHOWAĆ** | Wersjonowane zapisane widoki, mute i nieprzeczytane; brak duplikowania autorytatywnych wiadomości lub nieograniczonej historii. |
| B09 | Weekly-task Stash filters | 4 / 14 | **ZACHOWAĆ** | Tylko znane zaakceptowane zadania; pokaż wymagane, posiadane, zarezerwowane i brakujące ilości. |
| B10 | Weekly-task Market filters | 5 / 14 | **ZACHOWAĆ** | Ten sam kontekst zadania co Stash, z osobnym potwierdzeniem zakupu i limitem wydatku. |
| B11 | Remove/clear ring/amulet enchant | 4 / 6 | **ZMIENIĆ** | Najpierw rozpoznać model ładunków; normalizacja nie może odtwarzać zużytego czasu ani tworzyć pełnowartościowego przedmiotu z resztek. |
| B12 | In-game party loot split | 3 / 5 | **ZMIENIĆ** | Rachunek informacyjny nie daje liderowi prawa do przelewu; jawna wycena, zgody i nierozstrzygnięte składniki. |
| B13 | Food duration display | 1 | **ZACHOWAĆ** | Active Effects z serwerowym czasem i lokalnym odliczaniem; nie odgadywać dokładnego końca bez danych. |
| B14 | Individual food/potion buff icons | 1 | **ZACHOWAĆ** | Jedna powierzchnia efektów, czytelny HUD i panel szczegółów; limity zasobów zamiast arbitralnego małego limitu ikon. |
| B15 | NPC food/supply cost in party hunt | 3 / 5 | **ZMIENIĆ** | Ustalić wycenę kosztu i moment użycia; zakup zapasu na przyszłość nie musi być w całości kosztem obecnego hunta. |
| B16 | Echo Warden completion counter | 10 | **ZACHOWAĆ** | Widok istniejącego postępu, zgodny z aktualnym modelem treści; nie nowa równoległa ścieżka nagród. |
| B17 | Larger Bestiary/Bosstiary Tracker | 10 | **ZMIENIĆ** | Oddzielić dużą listę zainteresowań od ograniczonego HUD; wirtualizacja/paginacja i jawne limity zamiast dosłownego unlimited. |
| B18 | Better Bestiary UI/sort/filter/layout | 10 | **ZACHOWAĆ** | Wspólne filtrowanie, sortowanie i presety bez naruszania odrębnej logiki kolekcji oraz przyznawania punktów. |
| B19 | Immediate Loyalty from prepaid Premium | 19 | **ODRZUCIĆ** | Podtrzymać odmowę natychmiastowej mocy za przyszły opłacony czas; odróżnić saldo uprawnienia od faktycznego stażu. |
| C01 | Equipment Presets | 6 | **ZMIENIĆ** | To samo źródło prawdy i komenda co A09/B05; jedna konfiguracja obsługiwana kilkoma wygodnymi widokami. |
| C02 | Diagonal movement from simultaneous WASD | 1 | **ZACHOWAĆ** | Stan trzymanych kierunków, anulowanie przeciwieństw i jeden legalny zamiar ruchu; testować utratę fokusu, czat oraz brak podwójnych kroków. |
| D01 | Decouple Stash and Market; retain filters/views | 1 / 4 / 5 | **ZACHOWAĆ** | Zachować stan okien i wyszukiwania niezależnie, ale odświeżać ważność danych i uprawnień po zmianie miejsca lub sesji. |
| D02 | Anonymous Market offers by default | 5 | **ZMIENIĆ** | Jawna polityka produktu; anonimowość wobec innych graczy nie usuwa wewnętrznej identyfikacji i historii operacji. |
| D03 | Highlight own Market offers | 5 | **ZACHOWAĆ** | Widoczne własne oferty i ich stan, bez ujawniania anonimowej tożsamości innym użytkownikom. |
| D04 | Sort My Offers | 5 | **ZACHOWAĆ** | Sortowanie i filtry lokalnej projekcji; zachować spójność przy aktualizacjach i częściowym zrealizowaniu ofert. |
| D05 | NPC Sell: hide items not carried | 4 / 5 | **ZACHOWAĆ** | Filtr oznaczony czytelnie; po przyjęciu sprzedaży ze Stash rozróżniać posiadane, dostępne usłudze i zarezerwowane. |
| D06 | Auto-label purchased parcel | 5 | **ZMIENIĆ** | Nowoczesny formularz adresowania może zastąpić dosłowną automatyzację labela; zachować sprawdzenie odbiorcy i kosztu. |
| D07 | Custom Timers & Reminders | 1 | **ZACHOWAĆ** | Lokalne przypomnienia jawnie odróżnić od autorytatywnego czasu efektu; limit liczby, wyciszenie i brak automatycznego wykonywania gameplayu. |
| D08 | Party Finder lobbies/scheduling/tactical map | 3 | **ZMIENIĆ** | Etapować: lobby i ready check, później terminy oraz rysunki; link lobby nie jest zgodą na członkostwo ani ujawnienie prywatnych questów. |
| D09 | Legacy Story Mode solo encounters | 9 | **ZMIENIĆ** | Ręcznie wybrana lista starej treści z oddzielnymi nagrodami i zaliczeniami; nie globalne prawo do solo każdego bossa. |
| D10 | Puzzle/vocation companion illusions | 9 | **PÓŹNIEJ** | Wyłącznie wskazane stare zagadki, bez stałej bojowej AI i bez niejawnego omijania współczesnej treści grupowej. |
| D11 | Exact Market quantity input | 5 | **ZACHOWAĆ** | Dokładne pole liczby i podgląd sumy/opłaty; slider opcjonalny, z walidacją zakresu i formatu. |
| D12 | Protect Stash item | 4 | **ZMIENIĆ** | Widok wspólnej polityki rezerwacji; nie nowa niezależna blokada, którą omija inna operacja zbiorcza. |
| D13 | Protect/reserve only N quantity | 4 | **ZMIENIĆ** | Jednoznaczny zakres rezerwy i dostępna nadwyżka; uzgodnić pierwszeństwo ręcznych oraz zadaniowych potrzeb. |
| D14 | Two-ring quick switch | 6 | **ZMIENIĆ** | A/B jako skrót do wspólnego loadoutu, z zachowaniem właściwych ograniczeń czasowych i tożsamości używanego ringu. |
| D15 | Personal Hideouts / basic instanced housing | 16 | **UZGODNIĆ** | Mapować na zaakceptowane Residence w EXP-HOUSES-01: bez Premium, max jeden slot Account+World, Residence albo dom fizyczny, nie oba równocześnie. |
| D16 | Account/world shared legacy quest access | 9 | **ZMIENIĆ** | Wybrane dostępy współdzielone po spełnieniu warunków; nie kopiować automatycznie wszystkich flag postaci. |
| D17 | Shared quest unlock with one-time rewards | 9 | **ZMIENIĆ** | Oddzielne uprawnienie do nagrody z zakresem Account+World tam, gdzie przyjęto taką zasadę; odporność na kanały i ponowienia. |
| D18 | Main character absorbs >5 alts and deletes them | 9 | **ODRZUCIĆ** | Podtrzymać odrzucenie destrukcyjnego transferu progresji; powtarzalne dostępy naprawiać bez usuwania postaci. |

## Aneks C. Siedem kandydatów badawczych

Etykiety R01–R07 są lokalnymi etykietami audytu nadanymi w kolejności §8 źródła; propozycje pozostają badawcze. Przypisania do programów są sugestiami audytu.

| ID | Propozycja źródłowa | Program | Rekomendacja | Uzasadnienie / zmiana |
|---|---|---|---|---|
| R01 | Faction & Reputation System 2.0 | 2 / 9 | **BADANIE** | Rola fabularna, tożsamość i usługi; brak automatycznego obowiązkowego bonusu bojowego za reputację. |
| R02 | Professions 2.0 | 16 | **BADANIE** | Najpierw odrębne aktywności i gospodarka; nie nowy komplet obowiązkowych statystyk do standardowego hunta. |
| R03 | Guild Progression 2.0 | 3 / 13 | **BADANIE** | Prestiż i organizacja zamiast stałej mocy, która uzależnia konkurencyjność gracza od przynależności do największej gildii. |
| R04 | Adventurer Caravan / Expedition System | 2 / 9 / 16 | **BADANIE** | Ocenić ograniczoną wyprawę jako wspólną treść eksploracyjną; reuse questów, encounterów i nagród, nie nowy monolityczny system. |
| R05 | Region Discovery + Quest Chains | 2 / 9 | **BADANIE** | Włączyć do istniejącej mapy, Discovery i dziennika; nie promować jako odrębnego równoległego programu. |
| R06 | Item rarity / random affixes | 6 | **BADANIE** | Oddzielić wizualną rzadkość od losowych statystyk; zachować rozpoznawalność przedmiotów, przejrzyste budżety i ograniczoną liczbę wariantów. |
| R07 | Awakening / Prestige reset | 8 / 9 | **BADANIE** | Nie rekomenduję resetu progresji w obecnym kierunku produktu; pozostaje historycznym kandydatem badawczym, bez promocji lub automatycznej zmiany filozofii. |

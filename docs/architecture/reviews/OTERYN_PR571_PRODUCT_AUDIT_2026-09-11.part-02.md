
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


# Mechaniki Tibii — status wdrożenia w Oteryn Game

**Status dokumentu: DRAFT — robocza inwentaryzacja, nie deklaracja gotowości produktu.**

- Data zestawienia: 2026-10-16, Europe/Warsaw.
- Serwer: lokalny `main` w rewizji `4689d21fe6e816b1f871603c68e3bbe840bbe490`.
- Klient: [PR #1942](https://github.com/Oteryn/Oteryn-Game/pull/1942), head `39ae109bd05db41e652c7bae619e1821fba06597`, branch `feat/client-completion-20261009`.
- PR klienta w momencie odczytu: otwarty, draft; wskazany base SHA: `625469909e518381f4a9e4d8f8fe75346d727871`.
- Zakres: analiza kodu, negocjacji funkcji protokołu, podłączenia komend i widoków, istniejących testów oraz dokumentacji. Tabela obejmuje mechaniki omawiane w audycie; nie jest zamkniętym katalogiem wszystkich mechanik Tibii ani rozliczeniem wszystkich 139 wierszy wcześniejszego audytu.
- Testów Rust ani klienta Windows nie uruchomiono w tej sesji; środowisko nie miało `cargo` i `rustc`. Wyniki prób przywołane z PR są dowodami raportowanymi przez jego autora.

## Interpretacja statusów

| Symbol | Znaczenie |
|---|---|
| 🟢 | Podłączona ścieżka w kodzie danej strony. Nie oznacza udowodnionej wspólnej rozgrywki ani wdrożenia na serwerze. |
| 🟡 | Implementacja częściowa, warunkowa lub z niedomkniętymi zależnościami. |
| 🟣 | Panel lub widok istnieje; brakuje pełnych danych albo operacji. Nie jest to brak interfejsu ani gotowa mechanika. |
| 🔴 | Brak dostępnej obsługi lub funkcja wyłączona. Obecność kodu/protokołu nie oznacza udostępnienia graczowi. |
| 🔵 | Plan lub zadanie do wdrożenia/domknięcia. |
| ➖ | Funkcja nie dotyczy bezpośrednio tej strony. |

Serwer i klient pochodzą z różnych rewizji. Statusy nie są dowodem zgodności całego zestawu. Puste pola paneli, zapisane preferencje i liczba kontrolek nie stanowią potwierdzenia mechaniki. Poniższe statusy są ustaleniami tego audytu; szczegóły z planu wymagają dalszej weryfikacji wykonania.

## Tabela mechanik

| Mechanika / funkcja | Serwer | Klient — PR #1942 | Co pozostaje |
|---|---|---|---|
| Chodzenie klawiaturą | 🟢 Podłączone | 🟢 Podłączone | Potwierdzenie wspólnej rozgrywki na aktualnych wersjach. |
| Chodzenie po kliknięciu | 🟢 Obsługa kroków | 🟡 Częściowe | Pełne zachowanie przy przeszkodach i zmianach mapy. |
| Tempo i płynność ruchu | 🟢 Podłączone | 🟢 Podłączone | Klient odbiera czas kroku, pilnuje terminu i interpoluje ruch; pozostaje wspólna kwalifikacja. |
| Mapa przesyłana przez serwer | 🟡 Częściowe | 🟡 Zaimplementowana | Klient obsługuje funkcję 18; wejście do świata nadal wymaga potwierdzenia. |
| Renderowanie mapy i sprite’ów | 🟡 Częściowe | 🟡 Częściowe | Są odbiór mapy i renderowanie; brakujące zasoby mają zastępcze oznaczenia. |
| Minimap | 🟡 Zależności mapy | 🟡 Częściowe | Jest przegląd lokalnie załadowanej mapy; brak pełnej eksploracji i wszystkich pięter. |
| Widoczność aktorów | 🟢 Podłączone | 🟡 Częściowe | Odbiór encji istnieje; pełne wyglądy i animacje pozostają niedomknięte. |
| Dzień, noc i światło świata | 🟢 Podłączone | 🔴 Brak pełnej integracji | Klient nie deklaruje funkcji światła 19. |
| Teleporty i zmiany pięter | 🟡 Częściowe | 🟡 Częściowe | Obsługa zmian pozycji i resetów widoku; brak pełnego potwierdzenia rozgrywki. |
| Drzwi, klucze, dźwignie, push | 🟡 Częściowe | 🟡 Częściowe | Jest używanie otrzymanych obiektów mapy; pełne rozszerzenie interakcji 11 pozostaje wyłączone. |
| Wybór celu | 🟢 Podłączone | 🟢 Podłączone | Klikanie i wybór celów przez obsługiwane skróty. |
| Battle list | 🟢 Dane encji | 🟢 Podłączona | Lista, filtrowanie i wyświetlanie otrzymanego zdrowia aktorów. |
| Rozpoczęcie i zatrzymanie ataku | 🟢 Podłączone | 🟢 Podłączone | Klient wysyła atak oraz zatrzymanie celu. |
| Walka pięściami | 🟢 Podłączone | 🟡 Częściowe | Podstawowe sterowanie istnieje; prezentacja walki nie jest kompletna. |
| Walka bronią wręcz | 🟡 Częściowe | 🟡 Częściowe | Brak pełnego dostępnego cyklu wyposażania. |
| Pościg za celem | 🟢 Podłączone | 🟡 Częściowe | Brak kompletnego podłączenia przełączania trybu w interfejsie. |
| Offensive / balanced / defensive | 🟡 Częściowe | 🟡 Częściowe | API sesji istnieje; pełna obsługa przez okno gry niedomknięta. |
| Secure mode | 🟡 Częściowe | 🟡 Częściowe | Zależny od pełnej ścieżki PvP. |
| Łuki, kusze i amunicja | 🟡 Częściowe | 🟡 Częściowe | Braki wyposażania, wykonania i prezentacji. |
| Wands i rods | 🟡 Częściowe | 🟡 Częściowe | Pełny cykl ataku niedomknięty. |
| Czary ofensywne | 🟡 Częściowe | 🟡 Częściowe | Podstawowe rzucanie istnieje; pełny katalog i przypisywanie czarów niedomknięte. |
| Leczenie i czary wspierające | 🟡 Częściowe | 🟡 Częściowe | Brak pełnego zakresu celów, parametrów i efektów. |
| Spellbook | 🟡 Częściowe | 🟣 Panel istnieje | Lista i przyciski nie mają kompletnego źródła danych i operacji. |
| Action bars | 🟡 Częściowe | 🟡 Częściowe | Są paski, skróty i wykonanie obsługiwanych komend; brakuje pełnych źródeł przypisań. |
| HP i mana | 🟡 Częściowe | 🟢 Widok podłączony | HUD korzysta z rzeczywiście odebranych parametrów. |
| Soul | 🟡 Częściowe | 🟢 Widok podłączony | Klient wyświetla otrzymaną wartość; serwerowa regeneracja niedomknięta. |
| Regeneracja HP i many | 🟡 Częściowe | 🟡 Częściowe | Parametry są pokazywane; pełne trwałe rozliczenie pozostaje po stronie serwera. |
| Warunki i obrażenia okresowe | 🟡 Częściowe | 🟡 Częściowe | Pełne źródło warunków, ikon i czasów działania niedomknięte. |
| Animacje trafień i efekty czarów | 🟡 Częściowe | 🔴 Brak pełnej integracji | Klient nie deklaruje funkcji prezentacji 23. |
| Magiczne pola | 🟡 Częściowe | 🟡 Częściowe | Pełne wykonanie, interakcje i prezentacja niedomknięte. |
| Runy | 🔴 Nieudostępnione | 🔴 Brak aktywnej obsługi | Brak deklaracji funkcji 22. |
| Tworzenie run i przedmiotów czarami | 🟡 Częściowe | 🟡 Częściowe | Brak pełnego połączenia czaru z cyklem przedmiotów. |
| Summony i familiars | 🟡 Częściowe | 🟡 Częściowe | Brak pełnego zarządzania i prezentacji. |
| Jedzenie i mikstury | 🔴 Nieudostępnione | 🔴 Brak aktywnej obsługi | Klient nie deklaruje funkcji używania przedmiotów 15. |
| Stamina | 🔵 Plan | 🟣 Miejsce w panelu | Brak podłączonych danych i pełnej mechaniki. |
| EXP i poziomy | 🟡 Częściowe | 🟣 Panel częściowy | Brak kompletnego źródła postępu; pola bez danych pozostają puste. |
| Przyrost statystyk przy awansie | 🔵 Do wdrożenia | 🟣 Widoki częściowe | Pełna mechanika serwera niedomknięta. |
| Skille i magic level | 🟡 Częściowe | 🟣 Panel istnieje | Brak kompletnego zasilania poziomów i postępu. |
| Profesje i promocje | 🟡 Częściowe | 🔴 Brak pełnego procesu | Wybór i zakup promocji niedomknięte. |
| Trening offline | 🔵 Plan | 🟣 Pole w panelu | Brak kompletnej interakcji i rozliczenia. |
| Exercise weapons | 🔵 Plan | 🔴 Brak pełnej obsługi | Zużycie i używanie na celu niedomknięte. |
| Loot i otwieranie zwłok | 🟡 Częściowe | 🟡 Podłączone częściowo | Są komendy, widok kontenera i zabezpieczenia aktualności uchwytów. |
| Przenoszenie łupu do plecaka | 🟢 Warunkowo podłączone | 🟡 Podłączone częściowo | Interfejs wysyła operację; brak potwierdzonego pełnego przebiegu na serwerze. |
| Plecak i inventory | 🟡 Częściowe | 🟢 Widok podłączony | Wyświetla otrzymane przedmioty i ich liczby. |
| Wyposażenie — wyświetlanie | 🟡 Częściowe | 🟡 Częściowe | Jest układ slotów; widoczność zależy od dostępnych danych. |
| Zakładanie i zdejmowanie wyposażenia | 🔴 Nieudostępnione | 🔴 Brak aktywnej obsługi | Brak deklaracji funkcji 12. |
| Zamiana zajętych slotów | 🔴 Nieobsługiwane | 🔴 Brak pełnej obsługi | Operacja niedomknięta po obu stronach. |
| Plecaki w plecakach | 🔴 Nieudostępnione | 🔴 Brak aktywnej obsługi | Brak deklaracji funkcji 14. |
| Przedmioty na ziemi | 🟡 Częściowe | 🟡 Częściowe | Otrzymane wpisy mapy można wskazywać; pełne przenoszenie niedomknięte. |
| Waga i udźwig | 🔵 Do wdrożenia | 🟣 Pole istnieje | Wartość udźwigu nie jest podłączona. |
| Czas działania i ładunki przedmiotów | 🟡 Częściowe | 🟣 Widoki częściowe | Brak kompletnych danych i operacji. |
| Look / examine | 🔵 Do wdrożenia | 🟡 Częściowe | Są opisy z dostępnych danych przedmiotu; brak pełnego serwerowego examine. |
| NPC — rozmowy | 🟡 Częściowe | 🔴 Brak pełnej obsługi | Czat nie zastępuje dedykowanej rozmowy; brak funkcji NPC 3. |
| NPC — handel | 🟡 Częściowe | 🔴 Brak pełnej obsługi | Brak podłączonych danych rozmowy, ofert i operacji zakupu/sprzedaży. |
| NPC — podróże | 🔵 Do domknięcia | 🔴 Brak pełnego procesu | Zależności rozmowy i usługi. |
| Bank | 🟡 Częściowe | 🔴 Brak pełnego procesu | Brak podłączonej usługi bankowej. |
| Questy i nagrody | 🟡 Częściowe | 🟣 Panele istnieją | Brak pełnych danych i operacji gracza. |
| Quest log i tracker | 🔴 Nieudostępnione | 🟣 Panele istnieją | Brak aktywnej funkcji 16; część kontrolek celowo wyłączona. |
| Potwory — podstawowa walka | 🟢 Warunkowo podłączone | 🟡 Częściowe | Lista i otrzymane zdrowie są pokazywane; pełna prezentacja niedomknięta. |
| Potwory — ruch i zachowania specjalne | 🟡 Częściowe | 🟡 Częściowe | Braki wykonania i animacji. |
| Respawn potworów | 🟡 Częściowe | 🟡 Częściowe | Odbiór zmian encji istnieje; cykl świata po stronie serwera niedomknięty. |
| Śmierć i odrodzenie | 🟡 Częściowe | 🟡 Częściowe | Aktualizacje stanu i pozycji istnieją; pełny proces niepotwierdzony. |
| Blessings i straty po śmierci | 🟡 Częściowe | 🟣 Widoki częściowe | Brak pełnej usługi i rozliczenia. |
| PvP i skull system | 🟡 Częściowe | 🟣 Panele częściowe | Widok unjustified points istnieje; pełne dane i walka niedomknięte. |
| Party i shared EXP | 🟡 Częściowe | 🟣 Panel istnieje | Zaproszenia i lista członków nie mają podłączonej pełnej obsługi. |
| Bestiary i charms | 🟡 Częściowe; dostęp 🔴 | 🟣 Panele istnieją | Modele i widoki są; brak pełnych danych i aktywnych operacji. |
| Bosstiary i boss slots | 🔵 Do domknięcia | 🟣 Panele istnieją | Brak kompletnego zasilania i wyboru bossów. |
| Weapon proficiency | 🟡 Częściowe | 🟣 Panel istnieje | Brak pełnego postępu i operacji na perkach. |
| Wheel of Destiny | 🟡 Częściowe | 🟣 Panel istnieje | Lokalne szkice presetów nie zapisują alokacji postaci na serwerze. |
| Gem Atelier | 🟡 Częściowe | 🟣 Panel istnieje | Brak pełnych danych gemów i wykonania operacji. |
| Forge | 🟡 Częściowe | 🟣 Panel istnieje | Zakładki i wybór przedmiotów istnieją; operacje niedomknięte. |
| Imbuements | 🟡 Częściowe | 🟣 Tracker istnieje | Brak pełnych danych nasyceń i operacji. |
| Market | 🔵 Do domknięcia | 🔴 Brak pełnego procesu | Brak kompletnego handlu ofertami. |
| Handel gracz–gracz | 🔵 Do domknięcia | 🔴 Brak pełnego procesu | Brak kompletnej wymiany i akceptacji. |
| Poczta, depot i stash | 🔵 Do domknięcia | 🔴 Brak pełnego procesu | Same odniesienia w widokach nie dostarczają usługi. |
| Domki | 🟡 Częściowe | 🟣 Widok istnieje | Brak pełnego zarządzania własnością i dostępem. |
| Gildie | 🔵 Do domknięcia | 🔴 Brak pełnego procesu | Pełne operacje gildii niepodłączone. |
| Crafting | 🔵 Plan | 🔴 Brak pełnego procesu | Brak kompletnej konwersji przedmiotów. |
| Hirelings | 🔵 Plan | 🔴 Brak pełnego procesu | Brak pełnego zarządzania i dostarczenia. |
| Outfity, addony i mounty | 🟡 Częściowe | 🟣 Widoki istnieją | Pełny wybór, odblokowania i aktywacja niedomknięte. |
| Achievements | 🟡 Częściowe; powiadomienia 🔴 | 🟣 Widok istnieje | Brak podłączonego pełnego źródła osiągnięć. |
| Analysers | 🔴 Nieudostępnione | 🟣 Panele istnieją | Brak podłączonych danych analitycznych. |
| Prey i hunting tasks | 🔵 Do domknięcia | 🟣 Panele istnieją | Brak pełnych danych i operacji wyboru/nagród. |
| Daily rewards | 🔵 Do domknięcia | 🟣 Reward Wall istnieje | Brak pełnego odbierania nagród. |
| Czat lokalny | 🟢 Podłączone | 🟢 Podłączony | Pole tekstowe, wysyłanie, odbiór, historia i obsługa fokusu. |
| Wiadomości prywatne i kanały | 🟡 Częściowe | 🟡 Częściowe | Interfejs wysyła polecenia; dostępność zależy od serwera. |
| Premium | 🟡 Częściowe | 🟣 Widoki częściowe | Brak pełnego połączenia uprawnień ze wszystkimi funkcjami. |
| Wylogowanie | 🟢 Podłączone | 🔴 Brak nowej ścieżki logout | Ten head PR nie deklaruje funkcji 21 obecnej w nowszym main. |
| Server save i restart | 🟡 Częściowe | ➖ Głównie serwer | Pełne opróżnienie zapisów pozostaje otwarte. |
| Ustawienia i hotkeye | ➖ Głównie klient | 🟡 Rozbudowane | Są profile, edytory, zapis i działające skróty; część ustawień nie ma jeszcze wykonania. |

## Stan integracji klienta i serwera

Klient tego headu PR deklaruje `[4, 6, 7, 13, 17, 18]`. Wcześniejszy odczyt klienta z `main`, zamiast z PR #1942, prowadził do błędnych wniosków o braku mapy 18, czatu, HUD i inventory. Niniejszy dokument uwzględnia klienta z PR.

Ostatnia próba opisana w aktualnym w momencie odczytu opisie PR zakończyła się `UnselectedDomain { domain_id: 2 }`. Opis raportuje przejście wcześniejszych etapów logowania i TLS/ALPN, ale nie udane wejście do świata. To raport autora PR, nie próba wykonana podczas tego audytu.

Sprawdzony `main@4689d21f` zawiera poprawkę serwera #2221, która zapobiega wysyłaniu domeny 2 sesji wybierającej funkcję 18. Nie potwierdzono jej wdrożenia ani udanej ponownej próby klienta. Nie należy traktować historycznej odmowy jako dowodu, że poprawiony serwer nadal ją powoduje, ani samej poprawki w kodzie jako dowodu naprawienia wdrożonego środowiska.

## Źródła i ślady weryfikacji

### Klient — niezmienny head PR #1942

- [Deklarowane funkcje klienta](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/crates/session/src/lib.rs#L157).
- [HUD podłączony do otrzymanych parametrów](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/game_ui.rs#L1008).
- [Stan klienta: parametry, inventory, kontener, czat, encje](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/play.rs#L1017).
- [Komendy pasków akcji: czar, otwieranie ciała, przeniesienie do plecaka, czat](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/action_bar.rs#L415).
- [Wywołanie pasków akcji bez pełnego katalogu przypisań czarów](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/game_ui.rs#L1035).
- [Panele zaawansowanych mechanik](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/panel_views.rs#L114).
- [Jawnie wyłączone kontrolki](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/panel_views.rs#L244).
- [Lokalny zakres minimapy](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/src/minimap.rs#L1).
- [Udokumentowana aktywacja mapy i odbiór czasu kroku](https://github.com/Oteryn/Oteryn-Game/blob/39ae109bd05db41e652c7bae619e1821fba06597/apps/client/UI-FIDELITY.md#L830).
- [PR #1942 — opis prób i stan draft](https://github.com/Oteryn/Oteryn-Game/pull/1942). Opis PR jest zmienny; powyższy zapis odzwierciedla odczyt z 2026-10-16.

### Serwer — rewizja 4689d21f

- [Faktycznie oferowane funkcje](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/capabilities.rs#L87).
- [Warunkowe udostępnianie mapy i NPC](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/mod.rs#L3052).
- [Bramki wykonania na świecie z pakietu](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/seam_rooms.rs#L1).
- [Zakres walki wręcz i pościgu](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/attack.rs#L1).
- [Ograniczenia przenoszenia przedmiotów](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/item_move.rs#L1).
- [Ograniczenia AI potworów](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/apps/game-server/src/gameplay_transport/monster_ai_cycle.rs#L1).
- [Plan domknięcia mechanik — dokument planistyczny, nie dowód wykonania](https://github.com/Oteryn/Oteryn-Game/blob/4689d21fe6e816b1f871603c68e3bbe840bbe490/docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md#L61).
- [Poprawka producenta domeny 2 — PR #2221](https://github.com/Oteryn/Oteryn-Game/pull/2221).

## Zasady aktualizacji draftu

1. Przy kolejnym odczycie zapisać osobno dokładny SHA serwera i klienta; nie zastępować klienta z PR kodem z main.
2. Rozróżniać istnienie panelu, odbiór danych, wysłanie komendy, wykonanie na serwerze i potwierdzony przebieg rozgrywki.
3. Zmianę statusu oprzeć na konkretnym kodzie lub wyniku próby; nie wyliczać procentu ukończenia z liczby paneli ani ustawień.
4. Po integracji lub wdrożeniu ponowić kwalifikację wspólnego zestawu. Historia testów innego headu nie kwalifikuje automatycznie następnej rewizji.

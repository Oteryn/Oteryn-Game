# Tibia → Oteryn: 708 skontrolowanych wątków, audyt nieukończony

Stan 12.09.2026: **708 unikalnych ID** w zachowanych rejestrach, w tym wcześniejsze 636 oraz 72 nowe wątki i 151 odczytanych postów. Odczyt obejmuje wpisy otwierające i maksymalnie trzy początkowe odpowiedzi; nie obejmuje pełnych wielostronicowych dyskusji. Jeden z nowych wątków zawiera wyłącznie treść usuniętą przez autora — odnotowano to, bez rekonstruowania dawnej wersji.

**Spis Last Year jest gotowy: 103 strony, 3061 unikalnych ID.** Spośród nich 616 ma zachowany odczyt/ocenę, a **2445 pozostaje do odczytu i analizy**. Pozostałe 92 z ogólnego rejestru 708 pochodzą spoza bieżącego spisu. Lista zmieniała się podczas odczytu: początkowe 102 strony wzrosły do 103. Równa liczba ID i licznik serwisu nie dowodzą atomowego snapshotu; szczegóły i kolejka są w [rejestrze postępu](./last-year-progress.json).

Porównanie z [PR #571](https://github.com/Oteryn/Oteryn-Game/pull/571) dotyczy dokumentacji na rewizji `b2ef322791520130a2683f3004d26abd5c027ac8`, nie wdrożonego produktu. Q0: dostępna informacja; Q1: te same legalne działania, koszt i timing; G1: dostęp/logistyka; G2: moc, progresja, ekonomia lub PvP. R/G/E = Reference/gameplay/economy. Stan Global opisany wyłącznie przez gracza pozostaje **UNKNOWN** do niezależnego sprawdzenia.

## Powtarzające się potrzeby w nowym odczycie

Poniższe liczby dotyczą wyłącznie wskazanych źródeł. Nie są liczbą ludzi ani poparcia. Wcześniejsze klastry pozostają w [syntezie 636](./synteza-636.md); nie sumujemy ich mechanicznie z komponentami nowych pakietów.

| Problem gracza | Powtarzalność | Global / status dowodu | Odpowiednik #571 | Lepszy Oteryn, klasa i ryzyka |
|---|---|---|---|---|
| Przenoszenie dużej ilości lootu do NPC | 2 nowe OP: [4998126](https://www.tibia.com/forum/?action=thread&threadid=4998126), [4998403](https://www.tibia.com/forum/?action=thread&threadid=4998403). Osobne Sell All różnych typów: 4998440; ciężar delivery: 4998405. | Relacje o capacity i oknie handlu; brak testu bieżących reguł. Tezy o „zepsutej ekonomii” niepotwierdzone. | Dokładny kierunek §4.4.9 Stash→NPC; ochrona §4.4.4. Zdalne oddanie taska jest osobną zmianą. | Q1 multiwybór przy tym samym NPC i zachowanych zasadach; G1/G2 pominięcie transportu. R: dostęp i rezerwacje; G: udźwig/logistyka; E: tempo zamiany lootu na gold. Podgląd ilości/sumy, ochrona zapasu i atomowa operacja. |
| Zbyt wiele znaczników na mapie | 2 OP dla filtrów: [4998114](https://www.tibia.com/forum/?action=thread&threadid=4998114), [4998253](https://www.tibia.com/forum/?action=thread&threadid=4998253). Odpowiedź cytująca cały OP nie jest trzecim sygnałem. | Brak świeżego testu filtrów. Pakiet 4998253 zawiera też nowe informacje społeczne, których nie można uznać za zwykły filtr. | §4.2.4 filterable POIs; §4.2.5–6 osobno permissioned social map/shared markers. | Q0 lokalne warstwy bez kasowania i ujawniania nieodkrytych danych. G1/G2 rozszerzenie danych o innych graczach/eksploracji. R: granice wiedzy; G: tracking/PvP; E: brak bez zmiany dostępu. |
| Marnowanie czasu Prey przy zmianie aktywności | 2 OP: [4998411](https://www.tibia.com/forum/?action=thread&threadid=4998411), [4998329](https://www.tibia.com/forum/?action=thread&threadid=4998329). Pauza ogólna i naliczanie tylko dla wskazanego potwora to różne projekty. | Trigger timera i koszty są niezweryfikowane w bieżącym kliencie. | P10 Prey i §4.17.8 maintenance sink, bez zatwierdzenia tych reguł pauzy. | G2: zmienia efektywny czas bonusu. Zdefiniować party/combat trigger, koszt i skutki zmiany celu. R: jednoznaczny zegar; G: XP/loot; E: karty i rerolle. |
| Niechciane uruchomienie Echo przez wejście | 2 dokładne OP: [4998423](https://www.tibia.com/forum/?action=thread&threadid=4998423) i wcześniej 5000666. | Opis triggera jest relacją, nie aktualnym testem; inne poprawki Echo nie potwierdzają manualnego Use. | P10 tematycznie; brak literalnego manual Use. | G1 świadoma aktywacja; G2 przy zmianie reguł nagród, kolizji lub walki. R: trigger/owner; G: wejścia i cooldown; E: dostęp do nagród. |
| Blokowanie bossów, huntów i małego PvP | 3 nowe OP o dostępie/kontroli: [4998413](https://www.tibia.com/forum/?action=thread&threadid=4998413), [4998328](https://www.tibia.com/forum/?action=thread&threadid=4998328), [4998136](https://www.tibia.com/forum/?action=thread&threadid=4998136). Areny 4997902 to dodatkowy, odrębny projekt. | Nie zweryfikowano incydentów, skali ani intencji operatora. Usunięcie gildii i instancje nie mają wspólnego poparcia tylko dlatego, że opisują podobny problem. | P11 access, P13 PvP/areny/ranking; P18 kierunkowo zgłoszenia nadużyć. | G1/G2: jasne reguły dostępu i konfliktu, mierzalne cele PvP; osobno budżet instancji. R: reguły świata; G: griefing/boosting; E: równoległa podaż nagród. Sankcje wymagają dowodów i odwołania. |
| Brak czytelnej informacji o stanie gry | Oddzielne potrzeby: [4998297](https://www.tibia.com/forum/?action=thread&threadid=4998297) quest/Market; 4998305 equipped; 4998139 aliasy plecaków; 4998255 Weekly Delivery; 4998262 kalkulator. Pięć różnych sygnałów, nie pięć głosów za jedną funkcją. | Relacje graczy; błędów i brakujących opcji nie sprawdzono runtime. | P1, §4.3.12, §4.4.3–5, §4.5.2–5, §4.9.1–2, §4.14.7 zależnie od komponentu. | Q0 czytelne stany, źródła cen, lokalne etykiety i znane cele. Q1 potwierdzanie legalnej akcji. R: nie ujawniać ukrytych danych ani zmieniać tożsamości itemów; G/E: zachowane zasady. |

## Jakość, duplikaty i ograniczenia

W nowych 72 rekordach **21 wyłączono z liczenia powtarzalności problemów**: m.in. propozycje bez opisanego problemu, nieokreślone wpisy, duplikat i usuniętą treść. To nie oznacza 21 „śmieci”. Dobrze opisana nowa profesja pozostaje kandydatem projektu, nawet gdy nie dowodzi bólu obecnego gracza. Wątek mieszany może zachować użyteczny komponent, np. niezrozumienie blessingów obok jawnego żartu.

- `4998346` i `4998398`: dwa ID, jeden tekst o ukrywaniu tożsamości; późniejsza kopia nie zwiększa recurrence.
- `4998161`: dwa własne bumpy, zero dodatkowego wsparcia. `4997889`: „1000x yes” to jedna odpowiedź.
- `4998289`: krótka potrzeba transferu pozostaje sygnałem, mimo niepotwierdzonej populacji i brakujących reguł.
- `4998299`: płaszcz dla NPC jest konkretną propozycją lore, nie automatycznie spamem lub żartem.
- Historyczne liczby obrażeń, koszty i oskarżenia nie zostały przyjęte jako fakty. Pełne posty, nazwy oskarżonych i dane profili nie są publikowane w tym rejestrze.

## Rejestry źródeł

[Indeks 708](./source-index.json) przechowuje stabilne ID, wiersze analiz oraz dla 108 nowszych wątków identyfikatory i skróty 218 odczytanych postów. [Ocena jakości 72](./year-quality-index.json) jest oddzielna od klasy projektu. Starsze źródła nie uzyskały automatycznie tej samej głębokości kontroli.

Sześć nowych tabel: [1](./year-new12-analysis.md), [2](./year-next12-analysis.md), [3](./year-third12-analysis.md), [4](./year-fourth12-analysis.md), [5](./year-fifth12-analysis.md), [6](./year-sixth12-analysis.md). Stan Global: [dotychczasowe potwierdzenia](./global-baseline.md). Metoda: [triage](./quality-triage.md).

**Do wykonania:** 2445 pozostałych OP z bieżącego spisu, dalsze odpowiedzi w już odczytanych wątkach, globalna deduplikacja treści i niezależna weryfikacja istotnych mechanik Global. Sam spis 103 stron nie oznacza ukończenia audytu.

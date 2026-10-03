# Potwory — stan lokalnej paczki Round7

Paczka zawiera wszystkie dotychczasowe poprawki względem bazy `d3cfb2468510255d2495514ec975c23c1d76f32a`: 95 plików, w tym 6 zmian względem zamrożonego Round6. To wynik lokalny do przeglądu. Nie wykonano commita, push, merge ani aktywacji runtime.

## Wykonane uzupełnienia i naprawy

- Przygotowano 1660 paczek stworzeń; 1613 przeszło rzeczywisty import natywny. Powstały 21 472 profile, 22 536 rekordów i 61 definicji Encounter.
- Wcześniejsze rundy poprawiły dane Bestiary, mitygację potwierdzonych wariantów koni, tożsamości stworzeń i Itemów, ograniczenia schematów AI, loot Crystal, nieistniejącą transformację Chayenne oraz przygotowanie mechaniki lost-time Time Guardian.
- Round7 naprawia rozpoznawanie ilości lootu jako nazw Itemów: 82 błędne tokeny w 52 rekordach, zero pozostałych takich tokenów we wszystkich 1752 aktualnych rekordach wiki. Dokładny replay surowego źródła potwierdza Imperial; pozostałe 51 porównań ma jawny stan niepewny, bez dopisywania nazw, szans lub fałszywej informacji o braku lootu.
- Wszystkie 4980 plików danych typowanych paczek pozostają identyczne z Round6; zmieniła się proweniencja jednej paczki Undead Minion. Naprawa 52 rekordów porównań nie oznacza dodania danych 52 potworów.
- Świeży pełny batch: 12 904 wartości statystyk bez rozbieżności, 16 927 wpisów lootu zgodnych z przypiętymi źródłami. 1149 przyjętych korzeni lootu ma potwierdzony zapis; 28 korzeni nadal zależy od przyjęcia stworzeń. Siedem konfliktów XP zachowuje wartości z oficjalnej biblioteki.

## Weryfikacja

Świeże testy Round7: 232 testy monster-authoring, 24 Encounter, 270 przypadków schematów formalnych, 36 testów governance oraz walidator polityk. Świeży import natywny i porównanie 619 281 wartości nie wykazały rozbieżności. Pełne 1293 testy biblioteki Rust, fmt i clippy są dziedziczone z kwalifikacji Round5 na podstawie 371 identycznych plików źródłowych; nie przedstawiamy ich jako ponownie wykonanych.

To sprawdza zgodność danych, import i reprezentację typowaną. Nie potwierdza działania każdej mechaniki w aktywnej grze ani pełnej zgodności z aktualnym Tibia Global.

## Pozostałe braki i zależności

- 619 paczek nie ma potwierdzonej liczbowej mitygacji; 591 z nich jest przyjętych natywnie. Pełne sprawdzenie przypiętych Canary i Crystal nie dostarczyło bezpiecznych dodatkowych wartości. Nie przypisano zera ani wartości bazowej do innych wariantów.
- 858 źródeł nie deklaruje Bestiary: 229 deklaruje Bosstiary, 629 nie deklaruje żadnego z tych systemów. Brak deklaracji nie dowodzi, że wszystkie potrzebują zwykłego Bestiary.
- 683 nazwane obserwacje wiki wymagają kwalifikacji tożsamości lub prawdopodobieństwa; dodatkowe 2 mają pokryty identyfikator źródłowy bez potwierdzonej tożsamości wiki. Nie są to 685 udowodnionych brakujących Itemów.
- 51 porównań potrzebuje pobrania i odtworzenia dokładnej rewizji artykułu. Publiczny odczyt Fandom jest blokowany; autoryzowany komputer z przeglądarką był offline. Przygotowano przypięte żądania bez zmian na komputerze.
- 32 stworzenia zależą od Encounter, 14 od referencji/mechanik, 1 od Itemu. Oddzielnie pozostają 22 definicje Encounter. Decyzje architekta i koordynatora zapisano w #162.
- Dark Merudri nadal wymaga potwierdzenia speed, armor i defense. Pełna semantyka źródłowa ma 11 jawnych ograniczeń; nie oznaczono jej jako kompletną.

## Źródła i sposób odczytu

- Canary: `47dfd51f45280a59a1d3e50ba7edd573d7234446`; Crystal summer-update: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Publiczne źródła odczytano zwykłym Git/HTTP, z weryfikacją pełnych zestawów plików i blobów.
- Fandom, lista mitygacji, rewizje Creature/Item i dane Loot Statistics: dostępne strony/API odczytano normalnie; po blokadzie HTTP użyto publicznych stron w Chrome przez Remote Desktop + CDP. Przechowane rewizje mają przypięte identyfikatory i skróty. Nie wykonano administracyjnych działań ani zmian projektu przez Remote Desktop. Dokładne rewizje dla 51 porównań pozostają niedostępne.
- Oficjalne informacje Tibia, w tym news 8935 i wartości biblioteki: publiczny odczyt HTTP przez TibiaData; bezpośredni tibia.com zwracał 403. Zachowano pochodzenie danych i konflikty wiki.
- Wiki BR i pozostałe wiki wykorzystano tam, gdzie były dostępne; odpowiedzi 403 i niepotwierdzone dane są zapisane jako ograniczenia. Obecne wykonanie nie jest przedstawiane jako nowy research Tavily, którego limit wyczerpano wcześniej.

Decyzje: https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5942200530

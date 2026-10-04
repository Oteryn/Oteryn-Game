# Projekt domknięcia NPC

Cel: doprowadzić zachowane dane Canary/Crystal do natywnych danych i rzeczywistej ścieżki NPC w Oteryn. Baza: kwalifikowany lokalny WorldProject e749728f…; 1282 NPC, 836 dialogów, 380 usług. To nie jest licznik NPC gotowych w runtime.

## Etapy

1. **Kontrakty i tożsamości:** NPC-01/07 i NPC-08. Ponowne wykorzystanie danych workera Questy, źródeł i aliasów; małe konkretne zmiany kontraktów wynikające z rzeczywistych przykładów.
2. **Rozmowy i usługi:** NPC-02, NPC-04, NPC-05/06. Równoległa analiza istniejących interfejsów i implementacja reprezentowalnych podzbiorów; dane zależne czekają na zweryfikowane wiązania z etapu 1.
3. **Rzeczywisty runtime:** NPC-03 przygotowuje ścieżkę równolegle; integruje dopiero reprezentowalne dane i istniejące kontrakty. Istniejąca mapa, żadnego zastępczego serwera.
4. **Scalenie i kwalifikacja:** root scala jedną partię; niezależny przegląd, właściwe testy, import, odtworzenie i per-NPC status. Dopiero po rozliczeniu Canary/Crystal przechodzimy do brakujących danych wiki.

## Podział

- Root: architektura, wspólne decyzje, zależności, scalanie i końcowa kwalifikacja.
- Quest-linking: NPC-01 i NPC-07.
- Dialogue: NPC-02.
- Runtime: NPC-03.
- Special-services: NPC-04.
- Commerce: NPC-05 i NPC-06.
- Items: NPC-08.
- Niezależny reviewer po gotowej spójnej partii; nie jest autorem ocenianego kodu.

## Zasady wykonania

Praca lokalna, bez PR/pushu/wdrożenia i zmian zamrożonego repo produktu lub danych innych workerów. Każdy worker zapisuje wyłącznie we własnym katalogu. Kod produktu jako patch względem dokładnego źródła albo izolowana kopia w /tmp; jedno miejsce scalania i jeden autor. Root nie nadaje lokalnej propozycji statusu accepted w repo.

Nie implementujemy ogólnego Lua engine ani drugiego systemu questów. Nie zrównujemy Ability ze Spell, source storage z natywnym Progress, numeru blessing z natywną tożsamością ani źródłowego NPC wariantu z istniejącym aktorem bez kontraktu. Zachowujemy warunki, kolejność, koszty, efekty i niepewność.

Każda partia kończy się kodem/danymi możliwymi do sprawdzenia, odpowiednimi testami i dokładnym wynikiem. DONE oznacza spełnione kryterium zadania; rozliczenie źródeł lub projekt kontraktu samo nie kończy zadania runtime. Brak kontraktu wskazuje konkretny typ, pole, konsumenta i małą proponowaną poprawkę, zamiast ogólnego „czekamy na architekta”.

Oszczędzamy miejsce: trwałe małe poprawki i dowody, duże pliki w /tmp. Źródła i poprzednie wyniki są cache-first, bez ponownego pobierania. Nie powtarzamy niezmienionych ciężkich testów.

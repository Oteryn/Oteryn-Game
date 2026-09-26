# Ocena jakości sygnałów — zakres Last Year, 102 → 103 strony

Zakres zlecony przez właściciela: wszystkie 102 strony listy Last Year, z rozróżnieniem użytecznej treści i szumu. Odczyt listy nie stanowi odczytu postów. Każdy wątek zachowuje stabilne ID i zakres faktycznie przeczytanej treści.

| Status treści | Kryterium | Użycie w syntezie |
|---|---|---|
| concrete_problem | Rozpoznawalna sytuacja i uciążliwy skutek, nawet bez rozwiązania | Sygnał potrzeby; bieżące zachowanie wymaga niezależnej weryfikacji |
| concrete_proposal | Określona mechanika lub zmiana | Kandydat do oceny; nie jest automatycznie dowodem problemu ani dobrym projektem |
| underspecified_claim | Ogólne żądanie buffa, ocena lub zarzut bez sytuacji pozwalającej ocenić zmianę | Rejestr nastrojów/brakujących danych; nie liczyć jako konkretnego wymagania |
| duplicate | Sprawdzona kopia wcześniejszej treści | Odsyłacz do pierwowzoru; nie zwiększa liczby odrębnych tekstów |
| bump_only | Podbicie, podpis lub powtórzenie bez nowej treści | Nie zwiększa liczby argumentów; krótkie poparcie można oznaczyć osobno |
| joke_or_offtopic | Żart albo treść bez związku z zakresem, potwierdzone po odczycie | Rejestr z krótkim powodem pominięcia |
| spam | Powtarzalna promocja lub zalew treści bez użytecznej informacji | Rejestr z konkretnym uzasadnieniem; nie przypisywać autorowi intencji bez dowodu |
| mixed | Użyteczny problem obok żartu, powtórzeń lub nieuzasadnionych twierdzeń | Rozdzielić komponenty i zachować użyteczną część |
| unread | Znany wyłącznie tytuł/ID albo brak dostępu do postu | Poza licznikiem przeanalizowanych treści |
| source_removed | Odczytana bieżąca wersja zawiera wyłącznie usuniętą treść, np. Delete | Skontrolowane ID; brak dawnej treści do analizy i brak sygnału potrzeby |

Nie odrzucać automatycznie ze względu na tytuł, język, błędy pisowni, długość, emocjonalny ton ani niepopularność pomysłu. Zły projekt może wskazywać prawdziwą potrzebę. Odrębne uchwyty nie dowodzą odrębnych osób; liczba wyświetleń, bumpów i odpowiedzi nie jest liczbą zwolenników.

Jakość źródła i Q0/Q1/G1/G2 są osobnymi osiami. Konkretna propozycja G2 może być dobrze opisana, lecz ryzykowna; krótka skarga może trafnie ujawniać Q0/Q1. Nie wymuszać klasy projektowej, gdy brak projektu. Stan historyczny i niepotwierdzone liczby oznaczać osobno.

Dodatkowe adnotacje `unverified_numbers`, `unsupported_claims`, `unsupported_accusations` i `numerical_conflict` opisują stan dowodów w rekordach, a nie powód automatycznego odrzucenia. Jedna propozycja może mieć kilka komponentów i ocen. Wyłączenie z recurrence problemu nie usuwa jej z rejestru kandydatów.

## Zastosowanie do już odczytanych przykładów

| ID | Ocena | Uzasadnienie |
|---|---|---|
| 4998252 | underspecified_claim | Żądanie naprawy monka nie wskazuje konkretnej mechaniki ani oczekiwanego wyniku. |
| 4998171 | underspecified_claim; odpowiedzi mieszane | Frustracja zmianą balansu bez konkretnej poprawki; własna kontynuacja i żarty nie są niezależnym potwierdzeniem. |
| 4998173 | concrete_problem + concrete_proposal | Utrata sortowania po Market jest konkretnym problemem; pamięć stanu jest określonym rozwiązaniem. |
| 4998193 | concrete_proposal | Wypłaty charmów na etapach mają określony kształt; niezmieniona suma nie usuwa wpływu wcześniejszej mocy. |
| 5000656 | concrete_problem + concrete_proposal; numerical_conflict | Koszt ponownego wystawiania jest czytelny; przykład kwoty opłaty jest sprzeczny z oficjalną regułą. Nie odrzucać całego problemu z powodu błędnej liczby. |
| 4997753 | concrete_proposal; mixed replies | Autonomiczny pet ma opisane funkcje; żartobliwe i warunkowe odpowiedzi nie tworzą zgodnego poparcia. |

Dowody tych ocen: [rejestr 30](./page20-analysis.md) i [rejestr 6](./new-six-analysis.md). To przykłady ocenione przez prowadzącego, nie automatyczna klasyfikacja wszystkich 636 źródeł. Nieudany lokalny przegląd pomocniczy nie został wykorzystany jako walidacja.

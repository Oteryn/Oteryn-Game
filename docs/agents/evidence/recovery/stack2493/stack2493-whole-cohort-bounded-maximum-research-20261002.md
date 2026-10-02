# Stosy — aktualna kohorta i blokady źródłowe

Baza projektu: opublikowany `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`.

Kohorta ma **2493 ID**: 2491 ma natywne pole stack w stanie UNKNOWN; 45322 i 45497 mają wyłącznie tożsamość, bez typed semantics i znanej nazwy. To nie jest liczba wszystkich nieukończonych Itemów.

W tym ograniczonym badaniu zakwalifikowano **0 nowych maksimów**. Zachowane pola własnych szablonów nie mają osobnej wartości maksimum. Oficjalny podręcznik odczytany normalnym HTTP opisuje przenoszenie stosów, handel i limity ofert; liczby 100 oraz 64000 w tych kontekstach nie są maksimum jednego stosu. Fandom odczytany przez Chrome/CDP opisuje kategorię stackable, stash, kroki suwaka i przykłady; nie zamyka reguły maksimum dla całej kohorty. Starsza odpowiedź społeczności TibiaQA nie jest dowodem zakresu aktualnej wersji i wyjątków. Historia stosów złota >100 wymaga osobnego rozstrzygnięcia wariantów/instancji.

Wszystkie 2493 ID, bieżące tożsamości, współrzędne dodatnich źródeł stackable i blokady zapisano w JSON obok. Źródła i metody dostępu mają skróty SHA256. Brak wartości w skanowanych źródłach nie dowodzi, że żadne inne źródło jej nie zawiera.

Fandom zwykłym HTTP zwrócił 402, BR 403, Tibiopedia stronę Setup; rzeczywisty Chrome przez Remote Desktop/CDP pozwolił odczytać publiczne API Fandom i BR. Na komputerze użytkownika wykonywano wyłącznie polecenia do sterowania przeglądarką i odczytu publicznych stron. Bez zmian projektu, systemu, instalacji, Cargo ani Git.

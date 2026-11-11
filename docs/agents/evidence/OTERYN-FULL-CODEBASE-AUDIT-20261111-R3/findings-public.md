# R3 — publiczne ustalenia, bez instrukcji wykorzystania

**Sześć P1 potwierdzonych analizą osiągalnych ścieżek kodu na historycznych, przypiętych commitach.** Nie są to reprodukcje na produkcji ani potwierdzenie występowania na późniejszym `main`. Nie sumować ponownie z R4.

| ID | Priorytet | Stwierdzony skutek (ograniczony) | Weryfikacja |
|---|---|---|---|
| `DUR-003` | P1 | Legalne zagnieżdżenie przedmiotów może pozostawić niedokończoną pracę po śmierci i uniemożliwić dalszą grę tej postaci. | Statycznie CONFIRMED; bez dowodu duplikacji czy nieodwracalnej utraty. |
| `SEC-001` | P1 | Warunkowe przetrzymanie zasobów admission przez ścieżkę obserwacji. | Statycznie CONFIRMED; nie zaobserwowano awarii produkcyjnej. |
| `GP-001` | P1 | Odrzucanie części poprawnych czarów z kwalifikującymi bonusami. | Statycznie CONFIRMED; zakres nie obejmuje wszystkich czarów. |
| `GR-ECON-001` | P1 | Niespójność niezmiennika gospodarki przedmiotami dla obsługiwanej płatnej usługi. | Statycznie CONFIRMED; warunkowy skutek, bez stwierdzonego incydentu. |
| `MIGRATION-UPGRADE-F01-001` | P1 | Kolizja DDL przy normalnej aktualizacji poprawnie migrowanej wcześniejszej bazy. | Niezależne potwierdzenie kodu; PostgreSQL NOT_RUN. |
| `DISP-F01-001` | P1 | Warunkowy problem ponowienia i przyjmowania sesji po utracie odpowiedzi zatwierdzonej operacji drużynowej. | Niezależne potwierdzenie kodu; bez globalnej awarii. |

Pierwsze cztery były ponownie sprawdzone na `c255c6b8`; dwa kolejne na `f01f1858`. R3 zachowuje te tożsamości i relację do `2520d704`. Nie oznaczać jako naprawione ani nadal obecne na nowych SHA bez osobnej kwalifikacji.

Dodatkowo:
- **P2 (dane sklepu):** izolowana regeneracja pomija ofertę Charm Expansion, mimo sukcesu generatora; bieżący katalog przed regeneracją był poprawny.
- **P2 (publiczna historia śmierci):** liczba zachowanych uczestników może przekraczać limit odbiorcy, ograniczając kompletność konkretnego strumienia projekcji; brak dowodu utraty danych.
- **P3 (`DOC-R3-001`):** referencja do nieistniejącego symbolu protobuf w metadanych rejestru gildii.
- **P3 (import danych):** parser źródeł może utracić oznaczenie wartości przybliżonej; wpływ na aktywną grę niepotwierdzony.
- Pozostałe niezgodności dokumentacji, archiwalnych raportów i generatorów: przechowywane oddzielnie, nieprzypisywane automatycznie do aktualnego runtime.

Pełne ścieżki wyzwolenia P1, szczegóły pozwalające na nadużycia, surowe materiały autorskie i prywatne scenariusze pozostają poza publicznym raportem. To streszczenie nie zastępuje niezależnej walidacji na aktualnym `main`.
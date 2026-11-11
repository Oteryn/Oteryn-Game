# Pokrycie źródeł — R3 (SHA `2520d704`)

Podstawą jest zweryfikowany, przypięty checkpoint R3. Rejestr nie nadaje pełnego pokrycia za sam odczyt, hash, wyszukiwanie lub parsowanie.

| Kategoria | Wszystkie | Pełny przegląd | Pozostałe |
|---|---:|---:|---:|
| `.agents`, `.claude`, `.codex-report`, `.fix3042d` | 40 | 40 | 0 |
| `.github` | 75 | 75 | 0 |
| korzeń | 16 | 16 | 0 |
| `apps` | 2 081 | 2 081 | 0 |
| `crates` | 6 205 | 6 205 | 0 |
| `content` | 8 061 | 8 061 | 0 |
| `deploy` | 16 | 16 | 0 |
| `docs` | 7 424 | 6 812 | **612** |
| `imports` | 565 | 369 | **196** |
| `qlane-meta` | 1 388 | 1 060 | **328** |
| `rulesets` | 77 | 77 | 0 |
| `tests` | 16 | 16 | 0 |
| `tools` | 3 002 | 2 582 | **420 (418 nieprzejrzanych + 2 częściowe)** |
| `vendor` | 793 | 793 | 0 |
| **SUMA** | **29 759** | **28 203** | **1 556** |

**Podział: 1 554 bez przeglądu + 2 częściowe.** `qualification_errors: []` w źródłowym `current-coverage-summary.json`.

Moduły: 67 Cargo + 11 pozostałych = 78. **73 REVIEWED, 5 IN_PROGRESS**, zero końcowo VERIFIED. Nieukończone zakresy to przede wszystkim dokumentacja, importy, dane `qlane-meta`, narzędzia i `world-bundle-compiler` (7 plików), z uwzględnieniem nakładania się tego ostatniego na `tools`.

Pełna plikowa macierz (w tym dokładny Git blob, dowody źródłowe, metody, powiązania i statusy) pozostaje w prywatnym archiwum. Nie została opublikowana bez selekcji bezpieczeństwa. Dziewięć przepływów między modułami wymaga jeszcze końcowego uzgodnienia. Przegląd źródeł nie jest uruchomionym testem serwera.
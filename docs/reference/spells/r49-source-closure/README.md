# R49 — kompletne programy źródłowe 21 wariantów other_cast

Pakiet realizuje odwzorowanie danych źródłowych w nowej, ścisłej schemie `source-cast-programs.schema.json`. Dane pochodzą wyłącznie z pełnego AST r38 i capture r28 już obecnych lokalnie. Producent sprawdza niezmienne SHA obu wejść, pin i SHA źródłowego pliku oraz dokładną populację 21 rejestracji. Nie pobiera źródeł ponownie.

Każda rejestracja otrzymuje pełny program instrukcji źródłowych: blok z kolejnością statementów, przypisania, branch/else-if, pętle, return, call/method call, definicje funkcji i helperów. Parametry i wyrażenia wskazują dokładne węzły istniejącego AST z SHA rekordu. Zachowane są m.in. konstrukcja Variant, callbacki target/formula/chain, chronologia usuwania condition, odmowy i pierwotne literały. Nie kopiujemy całych plików Lua ani drugiego pełnego AST. Instrukcje i operandy są strukturalnie zamknięte; ponowne porównanie z AST wykrywa pominięcie lub przestawienie danych.

Każdy call ma osobny dependency link. Definicje helperów w tym samym pliku są kompletne; dopasowanie nazw podaje kandydatów powiązania i jawnie nie udaje dowodu lexical binding. Registrar, Combat i presentation są dostępne przez referencje do odpowiednich instrukcji i źródłowego capture. Pomocnicza lista `source_presentation_calls` obejmuje także kandydatów deklaracji `setParameter`, w tym parametry nieprezentacyjne; nie jest projekcją efektów ani listą dopuszczenia runtime. Wszystkie węzły AST są osiągalne, a każda funkcja oraz każdy call uwzględnione w programie.

**21 kompletnych programów danych tego samego pliku; 0 pełnych kandydatów Spell.** Cała populacja nadal BLOCKED na poziomie pełnego Spell. `same_file_typed_statement_program_complete=true` opisuje kompletność danych źródłowych. `transitive_external_helper_source_closure_complete=false` oraz `external_api_semantics_complete=false` jasno odróżniają kod tego samego pliku od niekwalifikowanych API/helperów/native execution. Nie zmienia się publiczny kontrakt runtime, aktywna zawartość, wybór katalogu ani native identity.

Ważna korekta historycznego przydziału: Forked Glacier w obecnym pinie Crystal ma jeden Combat z chain callback i Wheel additional targets, a nie dawny wrapper wielu Combatów. Pakiet zachowuje rzeczywisty obecny AST; historyczna etykieta `other_cast` jest tylko kryterium populacji.

Producent: `tools/content-schema/spell-authoring/source_cast_programs.py`; uruchomienie: `/workspace/spell-tools/bin/python tools/content-schema/spell-authoring/source_cast_programs.py --repo .`.

Testy sprawdzają pełną populację i schemę, odmowę w branch przed Combat, removeCondition przed Combat, zmianę kolejności i dangling refs, pętle/default step, helpery i Crystal Shared Conservation, aktualny chain Forked Glacier oraz odrzucenie unknown fields i aktywacji. Receipt i proof zawierają SHA plików do nieaktywnego importu.

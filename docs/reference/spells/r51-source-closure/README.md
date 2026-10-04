# R51: typowane mechaniki 175 zaległych slotów potworów

Cała kohorta R37 została zmapowana do rozszerzonej, zamkniętej schemy danych źródłowych: **156 slotów rejestrowanych + 19 inline**. 21 powiązań custom jest podzbiorem slotów rejestrowanych. Zachowano warianty obu donorów, tożsamość slotu, parametry, ich pominięcia i dokładne piny pochodzenia.

141 unikalnych programów zawiera pełne AST z istniejącego R38. Adapter wydziela **4416 typowanych operacji mechanik**: konstrukcje Combat/Condition, parametry efektów i typów obrażeń, area, formula, callback, condition, execute, losowania, zdarzenia i operacje świata. Argumenty wskazują konkretne typowane wyrażenia wraz z literalami/symbolami; callbacki i funkcje zachowują całe instrukcje oraz odniesienia do otaczających warunków i pętli. Pozostałe 1929 wywołań zachowano jako jawne zależności zewnętrzne. Kolejność przejścia po AST nie jest deklarowana jako kolejność wykonania.

19 inline obejmuje 9 błędnych/niezdefiniowanych symboli typu oraz 10 pominiętych typów. Prawidłowe enumy `COMBAT_LIFEDRAIN`/`COMBAT_MANADRAIN` nie zostały podstawione pod źródłowe `*DAMAGE`. Przypięte helpery obu donorów wykazują typ domyślny `COMBAT_UNDEFINEDDAMAGE`, brak condition oraz normalizację min/max. Zachowano źródłowe znaki obrażeń i oddzielnie znormalizowane zakresy; nie zgadywano elementu na podstawie efektu graficznego.

**175 slotów ma komplet danych źródłowych w nowej schemie; 0 slotów ma zakwalifikowaną pełną projekcję do natywnego runtime.** To nie jest awans pierwotnego `unresolved_semantics` do działającego Ability. Każdy slot ma jawne `projection_gaps`; receiver binding, helpery przechodnie i provider świata pozostają niekwalifikowane. Wszystkie flagi runtime/native/canonical pozostają wyłączone.

Walidacja używa trzech lokalnych schem: slot, inline i istniejącej source-syntax. Receipt zapisuje ich hashe, piny wejść R37/R38 oraz liczby rekordów. Nie pobrano źródeł ponownie, nie użyto internetu i nie włączono oryginalnych skryptów Lua do dystrybucji.

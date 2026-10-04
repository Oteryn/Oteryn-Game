# R48: kolejność Combat i programy callbacków

Trzy rekordy z już pobranych, niezmiennych rewizji Canary/Crystal. Pełny typowany AST każdego pliku jest zgodny z istniejącym R38; packet nie zawiera oryginalnych plików Lua ani aktywnego kodu. Hash archiwum źródłowego R28 i R38 zapisano w receipt.

Bieżące Crystal Forked Glacier i Forked Thorns mają **jeden Combat**, bez losowań prawdopodobieństwa. Callback chain zwraca odpowiednio 7 albo 6 celów plus dodatek Wheel, odległość skoku 5 i literal `false`; cast zwraca wynik execute. Historyczne określenie ich jako multiCombat nie opisuje tych przypiętych źródeł.

Canary Sweeping Takedown wykonuje inner, potem outer, usuwa cache po obu wywołaniach i zwraca `true`. Wyniki obu execute są ignorowane. Zachowano całe callbacki, wszystkie progi skill, producer/consumer cache, fallback `(0, 0)`, skalowanie outer `0.75` oraz cleanup; brak gwarancji cleanup przy wyjątku.

Schema packetu odwołuje się do `urn:oteryn:source-syntax:1`; validator rejestruje istniejącą `source-syntax.schema.json` lokalnie, bez pobierania z sieci. Parserowe zakresy są tylko kotwicami tokenów. Pełny AST zachowuje chronologię pliku; helpery przechodnie, natywne wywołania Combat, Wheel i provider danych nie są tutaj kwalifikowane.

**3 rekordy, 0 kompletnych kandydatów.** Wszystkie flagi aktywacji runtime/native pozostają wyłączone. Oba Forked i Sweeping pozostają BLOCKED do kwalifikacji właściwych mechanik.

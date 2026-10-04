# Odblokowanie 162 wariantów: praca lokalna

Punkt startowy: 483 warianty, 321 kandydatów strukturalnych, 162 BLOCKED.
W tych162 są cztery wyłączone przykłady, zatem158 wariantów gameplay.
Praca nie tworzy PR, nie zmienia aktywnego contentu ani nie przejmuje plików
runtime dzierżawionych przez #1534.

| Partia | Zakres | Liczba | Właściciel lokalny |
| --- | --- | --- | --- |
| r55 | Wheel, stance i Lightning |64| simple_guard_candidates |
| r56 | Monk/equipment |25| movement_source_closure |
| r57 | Party i summons |28| multicombat_source_batch |
| r58 | World/P4, pozostałe cast, Paralyze i przykłady |45| reconcile_runtime |

Najpierw istniejące dane i przyjęte reguły S5/S6/S16/S21/S23, potem rzeczywiste
Spell/Ability/Effect/Formula. Pierwotne nagłówki i source SHA pozostają zachowane.
Każda partia ma audyt wszystkich swoich kluczy, a pełne pakiety, częściowe dane
i proponowane bindings są liczone osobno. Niezależny reviewer sprawdza poprawki
i faktyczne kopie po imporcie.

Czytnik native wymaga równości całego profilu z jednym z67 osadzonych wzorców.
Alias albo kopiowanie istniejącego wzorca nie odblokowuje źródłowego wariantu,
gdy nie ma przyjętej polityki jego różnic. Nie obchodzimy guardów przez zmianę
kosztów, poziomów lub tożsamości. Warunki dotyczące rodzaju celu, kolejności
wykonania, party distance, Harmony i elemental bond pozostają jawne, dopóki
nie istnieje bezpieczna pełna reprezentacja.

Bieżąca dokładna kolejka jest generowana w current-source-gap-worklist.json.
Ten plan zachowuje punkt startowy162, nie stanowi deklaracji162 gotowych czarów.

# Edron Rope — zakres partii 1

## Reguła źródłowa

Jedyny skrypt reaguje na StepIn AID 4254. Dla gracza odmawia, gdy zapisany termin jest większy lub równy `os.time()`. Po upływie terminu pokazuje pełną wskazówkę o linie i zapisuje `now + 30`. Nie wydaje przedmiotu ani nie teleportuje. Źródło: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/edron_rope/movements_rope.lua:3–20`.

Równość z terminem nadal oznacza odmowę. Ewentualne przyszłe odwzorowanie przez czas ostatniego sukcesu musi zachować ścisłe porównanie, a nie skracać odstęp do `elapsed >= 30`. Nie zmieniono tego na podstawie nazwy katalogu ani nie przypisano automatycznie do Edron Goblin Quest.

## Left out

Cały callback pozostaje wstrzymany: istniejący format nie składa `SET_NOW`/`ElapsedAtLeast` z komunikatem po sukcesie. Potrzebne są pola czasu i `success_message`; nie wystarcza istniejący `failure_message`. Parser pozostaje bez zmian na mocy [QUESTION QUEST-CALLBACK-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589653708) i [ANSWER punkt 3](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926). Koordynator przypisał wspólny adapter do osobnego PR.

Ograniczenie formatu: `Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:crates/gameplay/quests/src/quest/world_callbacks.rs:22–107`. Nie dodano zastępczego stanu, ręcznej zmiany zegara ani drugiego handlera.

## Global i kontrola

Aktualny odpowiednik Global i jego treść pozostają NIEZNANE. Niedostępność API została rozstrzygnięta w [QUESTION GLOBAL](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589173099) i [ANSWER punkt 1](https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728). Ten commit dokumentuje jeden element RĘCZNIE i konkretną zależność; nie deklaruje nowej działającej mechaniki.

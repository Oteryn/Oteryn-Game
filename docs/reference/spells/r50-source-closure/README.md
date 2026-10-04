# R50 — sześć formuł Monka

Nowa, zamknięta schema `source-monk-formula-library.schema.json` zawiera sześć
dokładnych definicji callbacków Canary oraz odwołanie do helpera progowego
z importu r28. Program helpera nie został skopiowany ponownie.

`source_monk_formula_library.py` udostępnia walidację oraz ewaluator badawczy
z zakresem poziomów 0–1 000 000. Osiem testów obejmuje niezależny oracle C++:
wszystkie poziomy uint16, granice progów, saturację i wyniki sześciu callbacków.
Pakiet jest zmaterializowany w `imports/spells/r50/` i pozostaje nieaktywny.

Kompletne definicje formuł nie są pełnymi pakietami Spell. Żywe wejścia broni,
zużycie charges, konwersje Lua/silnika, losowanie, lifecycle Harmony i przyjęcie
przez konsumenta native pozostają jawnie niekwalifikowane. Źródłowe piny,
schemy, granice numeryczne i sumy kontrolne zapisano w qualification.

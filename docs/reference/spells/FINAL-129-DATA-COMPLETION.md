# Domknięcie kohorty 129: dane r59–r62

Cztery lokalne importy dodają **121 kandydatów private_source_complete_v2**.
Indeks obejmuje **475 kandydatów danych / 8 referencji BLOCKED / 483 warianty**.
Nie jest to liczba czarów grywalnych ani kwalifikacja runtime. Wszystkie 121 nowych
modeli nadal mają `authoring_contract_extension_pending=true` oraz
`source_consumer_implemented=false`; kwalifikacja wykonania i aktywacja są false.
Wcześniejsze 354 kandydaty zachowują swoje wcześniejsze ograniczenia.

| Import | Nowe prywatne kandydaty danych | Referencje | SHA256 manifestu |
| --- | --- | --- | --- |
| r59 | 41 | 2 | `8b43f59882301bbfa2cb6a75ad265a1b583d2dd82ddd74442721f4e1a6198754` |
| r60 | 23 | 0 | `bc10fdd2cd9d82019b6435dc49b54938c5b32c47bfa890db5479ccbc47e0ed8c` |
| r61 | 26 | 0 | `11ec878c707881f6005def8f61a322dbdd46fb83a1667aa52e251bec99560471` |
| r62 | 31 | 6 | `5a8932650af15267468ee14b1f1541a9157af9a3566b6f5d30185f7fb230297b` |

Osiem referencji to dwa usunięte Sap Strength, dwa wycofane Expose Weakness i cztery
wyłączone przykłady. Zwykłe receipts zachowują `BLOCKED`; osobne
`projection-receipt.json` określają `REFERENCE_ONLY_*`. Referencje nie są aktywne
ani nie stanowią ośmiu wymaganych implementacji gameplay.

Historyczny `unblocking-162-review-index.json` pozostaje identyczny bajtowo:
33 dodatki i 129 ówczesnych blokad. Nowy `final-129-completion-review-index.json`
łączy dokładnie tę kohortę z bieżącymi 121 kandydatami i ośmioma referencjami.
`source-private-consumer-worklist.json` zachowuje wszystkie 121 rekordów
oczekujących kontraktów i konsumentów, ich flagi oraz ścieżki konkretnych danych.

## Zakres źródeł i ograniczenia

Użyto lokalnych capture'ów i obiektów Git Canary
`04b83b512114bfd888000d6e1433ed8ecaec7c5b` oraz Crystal
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Zachowano wcześniejsze fakty canonical
Wiki i jawne normalizacje przyjętych decyzji. Nie deklarujemy świeżej weryfikacji
sieciowej lub Wiki; Remote Desktop nie był źródłem tego etapu. Oryginalne Lua
pozostają wejściami referencyjnymi poza dystrybucją importów.

Kompletność prywatnego modelu danych nie zatwierdza nowego kontraktu authoring,
zgodności 1:1 z zachowaniem donora ani czytnika native. Providery, tożsamości,
asset bindings i konsumenty silnika wymagają osobnej kwalifikacji. Wybór donora,
manifest serwera, aktywny katalog i flagi aktywacji pozostają bez zmian.
Potwory mają oddzielny widok r54: 10 pełnych projekcji danych, 62 częściowe i 103
bez projekcji; wszystkie 175 wybranych slotów pozostają runtime-unqualified.

## Odtworzenie widoków

Z korzenia repozytorium, z istniejącym środowiskiem Python:

```sh
python tools/content-migration/build_source_spell_review_index.py
python tools/content-migration/build_final_source_completion_view.py
python tools/content-migration/build_source_projection_progress.py
```

Widoki wywodzą liczby i ścieżki z zapieczętowanych importów. Siedem testów nowego
widoku sprawdza m.in. blokadę promocji kontraktu/konsumenta, przypisanie do właściwego
importu i rozdzielenie zwykłych receipts BLOCKED od dowodów referencji. Cztery testy
postępu zachowują osobne liczby pełnych, częściowych i zablokowanych projekcji potworów.

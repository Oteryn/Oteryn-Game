# Domknięcie 165 slotów potworów: dane r63–r66

Bieżący widok obejmuje **175 pełnych projekcji danych**: dziesięć wcześniejszych
z R54 i 165 nowych modeli `private_monster_slot_v2`. W wybranej kohorcie nie ma
już częściowych lub zablokowanych projekcji danych. **Wszystkie 175 slotów
pozostaje runtime-unqualified**. To nie kwalifikacja AI, liczba grywalnych
potworów ani deklaracja wykonania ich czarów na serwerze.

| Import | Nowe modele danych | Zakres | SHA256 manifestu |
| --- | --- | --- | --- |
| r63 | 10 | inline / source signed-health | `d8cdc0a86b0d85c62db5abe3663e193b4814ee4036bc06144d732d8599918071` |
| r64 | 21 | world controllers | `9f3932eea2ef1108cb769276992b6c79b1bcb3b49e95b40242f608d3381af64c` |
| r65 | 51 | Combat / effects | `df8cfec1402978fd7a33a9d055010d8866e3d8d5cfd0de32e62345d673582130` |
| r66 | 83 | staged / delayed controllers | `c4d4e1e4ab17931191c9bb9883c0efb857cfe374de5fdbe297ac84a2b405fa09` |

`monster-target-projection-review-index.json` nakłada dokładnie 165 nowych modeli
na 62 częściowe i 103 zablokowane projekcje R54. Dziesięć wcześniejszych pełnych
rekordów pozostaje zachowanych. Każdy nowy slot łączy się po pełnym
`slot_identity`, źródle, nazwie potwora, SHA oryginalnego slotu i niezmienionych
`source_parameters`. Sam źródłowy pakiet R54 i wcześniejsze importy nie są
przepisywane; ich historyczne statusy pozostają dowodami wcześniejszego etapu.

## Oczekujące kontrakty i konsumenci

`monster-source-consumer-worklist.json` zawiera **165 rekordów** z oczekującym
rozszerzeniem kontraktu authoring i niezaimplementowanym konsumentem. Zachowuje
flagi kwalifikacji, pełną tożsamość slotu oraz ścieżkę, indeks rekordu i hash
konkretnego modelu. `authoring_contract_extension_pending=true`,
`source_consumer_implemented=false`, `input_provider_equivalence=false`; flagi
native/runtime, alokacji native ID i wyboru katalogu pozostają false.
Dziesięć wcześniejszych modeli zachowuje wcześniejsze ograniczenia providerów;
nie dodano ich do nowego worklistu 165.

Pełny prywatny model DATA nie zatwierdza kontraktu silnika, providera, renderera
ani zgodności działania 1:1 z donorem. Oryginalne błędy, surowe flagi i literalne
normalizacje są zachowane w przypiętych modelach i dowodach źródłowych; nie
zastąpiono ich domyślnymi wartościami. Widok wskazuje rzeczywiste modele zamiast
ukrywać niezakwalifikowane operacje runtime.

Indeks graczy pozostaje **475 kandydatów danych / 8 referencji BLOCKED**.
Jego hash, worklist 121 konsumentów graczy, końcowy widok 129 i historyczny audyt
162 są niezmienione bajtowo. Nie zmieniono aktywnego contentu, wyboru donora,
manifestu serwera ani danych aplikacji/protokołu.

## Źródła i odtworzenie

Użyto lokalnych capture'ów i przypiętych obiektów Git Canary
`04b83b512114bfd888000d6e1433ed8ecaec7c5b` oraz Crystal
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Nie deklarujemy świeżej weryfikacji
Wiki, sieci ani Remote Desktop. Istniejące źródłowe parametry i jawne
normalizacje pozostają zachowane. Oryginalne pliki Lua/assetów nie są payloadem
importów.

Z korzenia repozytorium:

```sh
python tools/content-migration/build_source_projection_progress.py
```

Dziewięć testów postępu PASS: cztery wcześniejsze oraz pięć nowych przypadków
sprawdzających dokładną kohortę, zachowanie wcześniejszego pełnego slotu,
duplikaty i obce tożsamości, zmiany hash/literalów, promocję kontraktu/providerów
lub runtime oraz fałszywy receipt. Generator odtwarza widoki identycznie bajtowo.

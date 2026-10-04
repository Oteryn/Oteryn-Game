# Audyt 68 questów wobec paczki NPC #1757

Sprawdzono wszystkie 68 lokalnych definicji `waiting_native_bindings`, całą paczkę kandydatów 352 questów, 1282 NPC i 836 Dialogue. Root oraz dwa niezależne odczyty potwierdziły wynik. Audyt nie zmienia definicji ani flag gotowości.

## Wynik

- **68/68**: zgodne klucze, rewizje i SHA aktualnych definicji w kandydacie NPC. Profile zawierają metadane SourceText, bez wybranych etapów, ścieżek postępu i przypisania nagród.
- **0/68**: dowiedzione w tej paczce powiązania NPC → wybrany etap/nagroda. Wykwalifikowana bazowa kopia WorldProject nie ma deklaracji Quest; kandydat nie został do niej zastosowany.
- **39** questów ma etapy `talk`; **29** nie ma etapu `talk`. Nie oznacza to, że te 29 nigdy nie wymaga NPC przy zakończeniu. Nie można jednak przypisywać całej grupie jednego brakującego dialogu.
- **36** receptur ma przynajmniej jednego kandydata istniejącego NPC na podstawie normalizacji nazwy; **22** mają kandydata z istniejącym Dialogue. W **18** co najmniej jeden taki kandydat występuje dla każdego etapu `talk`. To wskazówki do ponownego wykorzystania danych, nie zaakceptowane tożsamości ani pokrycie dialogów questowych.
- Osiem questów ma wskazówki wiki przy NPC z dokładnie zgodnym tytułem; nie są to powiązania wykonawcze.

## Co oznaczają liczby workera NPC

335 pól źródłowych przy NPC rozlicza 4315 historycznych wierszy i 79 innych donorowych kluczy Quest. Żaden nie jest jednym z naszych 68 authored. Pięć pól korekt odsyła dodatkowo do 31 wierszy: stąd 4346 w podsumowaniu nowego linkera. Znormalizowana tabela oraz surowe wiersze korekt nie są dołączone do packetu, więc audyt nie deklaruje pełnego ponownego sprawdzenia każdego z tych wierszy.

18 kandydatów callbacków / 55 węzłów oznacza wynik analizy źródłowej, nie 18 gotowych dialogów questowych: liczba proponowanych aktualizacji Dialogue to zero. Surowych programów kandydatów nie dołączono; nie można przypisać ich do konkretnych 68 questów. 275 innych pól `quest_bindings` to odsyłacze źródłowe, nie wykonywalne powiązania. Jeden taki historyczny sidecar zwrócił HTTP404 na przypiętym commicie — jego treść pozostaje niezweryfikowana, nie została uznana za pustą.

## Co wykorzystać, co wykonać

1. Zachować już przygotowane 68 receptur i zgodne tożsamości. Nie zbierać ponownie opisów ani całego korpusu NPC.
2. Dla kandydatów NPC/Dialogue z listy potwierdzić tożsamość, wybrać lub uzupełnić konkretną gałąź rozmowy i połączyć ją z etapem aktualnej receptury Oteryn. Dotychczasowy dialog donorowy nie musi wykonywać uproszczonej receptury.
3. Przypisać etapy kill/use/explore/collect/complete do rzeczywistych zdarzeń i warunków oraz nagrody do istniejących definicji i wykonawców. To integracja danych oraz sprawdzenie właściwych konsumentów, nie automatycznie brak mechanizmów w aktualnym main.
4. Uzgodnić proponowany resolver i amendment tracks z już przyjętymi QUEST-LOWER/QUEST-PRED/QUEST-XP i istniejącymi ownerami; nie tworzyć drugiego magazynu stanu.

## Źródła i zakres pewności

[Paczka NPC, PR #1757](https://github.com/Oteryn/Oteryn-Game/pull/1757), commit `4bfcb2890281424713945461fc70941e52876160`. Zwykły GitHub HTTPS, bez Remote Desktop. Jednorazowo pobrano nowe archiwum tego workera i zweryfikowano SHA wszystkich 133 plików; nie pobierano ponownie donorów ani wiki. Archiwum SHA256 `617c34e33099feefc5ab675ed7f067cfb33bfeb1c2bbe71e5477a880684dc420`.

Metadane i brak konkretnych powiązań w dostarczonych danych: PROVEN. Kandydaci nazw i tytułów: DERIVED. Działanie na aktualnym serwerze: UNKNOWN, nie uruchamiano runtime. Stwierdzenia workera o brakach systemu odnoszą się do jego historycznej bazy `79b79ae9`, nie dowodzą braków obecnego main.

## Lista wszystkich 68

Kolumny NPC/Dialogue podają liczbę etapów `talk` z co najmniej jednym kandydatem nazwy. Nie oznaczają liczby zaakceptowanych powiązań. Dla każdego wiersza zgodność kandydata tożsamości i SHA: tak; powiązanie wybranego etapu/nagrody w paczce: zero.

| Quest | Wszystkie etapy | Etapy talk | Etapy talk z kandydatem NPC | Z kandydatem Dialogue |
|---|---:|---:|---:|---:|
| 20 Years a Cook Quest | 7 | 2 | 1 | 1 |
| A Pirate's Death to Me | 5 | 1 | 1 | 1 |
| Annual Autumn Vintage | 5 | 1 | 1 | 1 |
| Asura Palace Quest | 6 | 0 | 0 | 0 |
| Bank Robbery Mini World Change | 4 | 1 | 0 | 0 |
| Bewitched | 6 | 0 | 0 | 0 |
| Braindeath Quest | 3 | 0 | 0 | 0 |
| Cartography 101 Quest | 5 | 1 | 1 | 1 |
| Chakoya Iceberg Mini World Change | 4 | 0 | 0 | 0 |
| Child of Destiny Quest | 6 | 2 | 2 | 0 |
| Citizen of Issavi Outfits Quest | 6 | 1 | 0 | 0 |
| Demon's Lullaby | 7 | 1 | 1 | 1 |
| Demon Wars World Change | 5 | 1 | 1 | 0 |
| Devovorga's Essence Mini World Change | 5 | 0 | 0 | 0 |
| Down the Drain Mini World Change | 3 | 0 | 0 | 0 |
| Draccoon Herald Outfits Quest | 6 | 1 | 0 | 0 |
| Dragon Slayer Outfits Quest | 6 | 1 | 1 | 1 |
| Elvenbane Quest | 6 | 0 | 0 | 0 |
| Fire from the Earth Mini World Change | 3 | 0 | 0 | 0 |
| Hive Born World Change | 7 | 2 | 2 | 0 |
| Hive Outpost Mini World Change | 4 | 0 | 0 | 0 |
| Illuminated Warrior Outfits Quest | 8 | 1 | 1 | 1 |
| Insectoid Invasion World Change | 3 | 0 | 0 | 0 |
| Insectoid Outfits Quest | 8 | 1 | 1 | 0 |
| Jungle Camp Mini World Change | 5 | 0 | 0 | 0 |
| Kingdom of Kormarak Quest | 5 | 0 | 0 | 0 |
| Kingsday Mini World Change | 3 | 1 | 1 | 0 |
| Kissing a Pig Quest | 19 | 8 | 6 | 5 |
| Last Creep Standing | 12 | 2 | 2 | 2 |
| Lumberjack Mini World Change | 4 | 2 | 2 | 2 |
| Magic Sword Quest | 7 | 0 | 0 | 0 |
| Make Believe Quest | 20 | 5 | 4 | 4 |
| Mastermind Potion Quest | 5 | 0 | 0 | 0 |
| Measuring Tibia Quest | 8 | 4 | 4 | 4 |
| Mystery of the Valley Quest | 6 | 1 | 1 | 0 |
| Nimmersatt's Lair | 4 | 1 | 1 | 1 |
| Nomads Land Quest | 7 | 1 | 1 | 1 |
| Nomads Mini World Change | 6 | 0 | 0 | 0 |
| Noodles is Gone Mini World Change | 4 | 1 | 1 | 0 |
| Orcsoberfest | 10 | 0 | 0 | 0 |
| Poacher Caves Mini World Change | 3 | 0 | 0 | 0 |
| Podzilla Quest | 15 | 3 | 3 | 1 |
| Rathleton Quest | 9 | 4 | 4 | 4 |
| Ravenous Madness Quest | 6 | 2 | 2 | 2 |
| Rise of Devovorga | 7 | 1 | 1 | 1 |
| River Runs Deep Mini World Change | 3 | 0 | 0 | 0 |
| Rootwalker Outfits Quest | 5 | 1 | 1 | 0 |
| Royal Bounacean Advisor Outfits Quest | 6 | 2 | 1 | 0 |
| Shards of a Broken Moon Quest | 16 | 7 | 7 | 7 |
| Small Ruby Quest | 3 | 0 | 0 | 0 |
| Small Sapphire Quest | 3 | 0 | 0 | 0 |
| Spider Nest Mini World Change | 3 | 0 | 0 | 0 |
| Spirit Grounds Mini World Change | 4 | 0 | 0 | 0 |
| Spring into Life | 4 | 0 | 0 | 0 |
| The Colours of Magic | 5 | 1 | 1 | 1 |
| The Fire-Feathered Serpent World Change | 4 | 0 | 0 | 0 |
| The Great Expedition | 6 | 2 | 2 | 2 |
| The Lightbearer | 4 | 1 | 1 | 0 |
| The Repenters Quest | 10 | 2 | 2 | 2 |
| Tibia Anniversary | 4 | 0 | 0 | 0 |
| Top of the City Quest | 6 | 3 | 3 | 0 |
| Torch Quest | 5 | 0 | 0 | 0 |
| Twisted Waters World Change | 4 | 1 | 1 | 0 |
| Venore Daily Tasks Quest | 7 | 4 | 4 | 0 |
| Voodoo Doll Quest | 4 | 0 | 0 | 0 |
| War Against the Hive Quest | 9 | 2 | 2 | 0 |
| Warpath Mini World Change | 4 | 0 | 0 | 0 |
| Waterskin of Mead Quest | 4 | 0 | 0 | 0 |

Szczegółowe klucze, rewizje, kandydaci i liczniki dla każdego etapu: [OTV2-20261004-authored68-npc-packet-audit.json](OTV2-20261004-authored68-npc-packet-audit.json).

# NPC completion — packet do przejęcia

Zapis kompletnej lokalnej paczki NPC-COMPLETE-LOCAL-20261004 na osobnej gałęzi. Nie stosuje patchy do kodu produktu, nie aktywuje NPC i nie tworzy PR.

## Status (SPELL-NPC-MAP-0 §9)

Paczka jest wyłącznie materiałem referencyjnym (reference evidence only), wstrzymanym (held) i niczego nie aktywuje: żaden NPC, usługa, Dialogue ani Quest nie trafia do runtime ani do produktu. Przyjęcie pojedynczych modeli wymaga osobnych tasków właścicieli zgodnie z §1 decyzji `docs/architecture/reviews/OTERYN_GAME_SPELL_NPC_MAP0_SPELL_AND_NPC_HANDOFF_MAPPING_DECISION_2026-10-04.md`.

## Zawartość

- `npc-completion-packet.tar.xz`: cały projekt (106 plików, bez Python cache/binariów), dwie pełne kanoniczne kopie WorldProject oraz dowody poprzedniego importu i końcowe logi walidacji.
- `manifest.json`: SHA256 każdego pliku, archiwum, bazowego i proponowanego drzewa danych.
- `PLAN.md`, `STATUS.md`, `HANDOFF.md`: bezpośrednio dostępne kopie planu, wyników i przekazania koordynatorowi.
- `verify_packet.py`: przenośna walidacja integralności bez źródeł donorów, Rust, builda lub sieci.

## Odbiór

1. Uruchom `python3 verify_packet.py` w tym katalogu. Sprawdzi kompletność, SHA256 i drzewa danych bez wypakowywania.
2. Wypakuj archiwum do nowego katalogu poza aktywnym checkoutem: `python3 -m tarfile -e npc-completion-packet.tar.xz /tmp/npc-packet-review`.
3. W `project/integration/second-batch/` jest scalony proposed patch (13 plików), manifest i receipt. Patch jest względem **79b79ae99c3b5ca1667c7157b812131ee27c43f9**, nie aktualnego main. Przed zastosowaniem uzgodnij z istniejącymi ownerami NPC/Quest/Item/Spell oraz przyjętymi schemami. Nie stosuj go w ciemno do nowszego produktu.
4. `qualified-world-base/` ma bazowy import e749…; `qualified-world-proposed-travel/` ma dokładnie dwa departure_text additions i drzewo b506…. Druga kopia wymaga proponowanej schemy. Cztery PENDING Item candidates są w `project/items/native-import-batch-candidate.json`; przeszły walidację in-memory, nie zostały automatycznie dodane do tych światów.

W obu światach zachowano 1282 NPC / 836 Dialogue / 380 Service / 2564 profili. Nie jest to licznik działających NPC. Zero aktorów i usług aktywowanych.

Logi i lokalne review receipts dowodzą poprzedniej partii: cargo check PASS, 11/11 native admission tests PASS. **Nie są CI/review nowego commita ani aktualnego main.** Ten commit udostępnia materiały; przyszły integrator kwalifikuje własnego dokładnego kandydata normalną ścieżką.

Oryginalne receipts/propozycje zachowano bez przepisywania hashów i ścieżek. Absolutne ścieżki `/workspace` i `/tmp` w nich odnoszą się do środowiska autora. Lane reproducers wymagają zewnętrznych, już pobranych cache donorów i build artifacts; nie są w pełni samowystarczalne. Pełne kanoniczne dane i verifier w tej paczce umożliwiają odbiór bez utraty nietrwałych /tmp world copies. Nie zawiera assetów klienta ani wykonywalnego serwera.

Koordynacja: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5979309489. Użyj obecnych tasków/lease, bez konkurencyjnego NPC runtime lub Quest store.

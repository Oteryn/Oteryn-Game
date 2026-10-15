# CP — wave'y mechanik (stan 2026-10-15 ~07:40Z)

Źródło kolejności: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §5.
Wave = kolejność zależności, nie harmonogram: batch startuje, gdy jego zależności są zmergowane.
Zasada: lekkie zadania (docs, rejestr, flip statusu, małe poprawki) → subagent u CP; ciężkie (persistence, protokół, regeneracja contentu, duży kod) → osobny chat.

## Protokół seryjny (owner 2026-10-15)
- Kolejka merge: #2115 → #2122 → #2128 → #2130 → #2129 → #2127 → #2096. Następny merguje main i pushuje dopiero po „GO” od CP (raz).
- Maks. 2 rundy napraw na PR; potem zamknięcie i podział zadania.
- Codex P2 odroczone; Codex liczy się tylko na finalnym headzie.
- Wspólny powtarzalny błąd CI → jedna poprawka na main.
- Brak nowych zadań do zamknięcia kolejki (W2c2, W3a2 na hold).

## Zrobione (zmergowane)
| Wave | Batch | PR |
|---|---|---|
| 1 | PREMIUM-1 (PREM-WIRE-1) | #2110 |
| 1 | MAGIC-CAST-1 (SPELL-RT-PREM-1) | #2112, #2119 |
| 1 | LOOP-COMBAT-MODES-1 | #2113, #2125 |
| 1 | CLOCK-1 (light) | #2114 |
| 1 | MAGIC-COND-1 | #2122 |
| 2 | BUNDLE-GATES-1 | #2115 |
| 1 | W2b training | #2109 |
| 1 | CHAT-1b-2b | do potwierdzenia (W2h przeszedł na CHAT-2B) |
| 2 | ATTACK-2 | #2117 |
| 2 | MAGIC-ITEMS-1 / RUNE-CONTENT-1 | #2116 |
| 2 | MAGIC-ITEMS-1 RUNE-WIRE-1 | #2126 |
| 2 | LOOP-WORLD-1 KEY-1 mint | #2120 |
| – | CHAR-CMD-CONSUMER-1 (Game) | #2121 |
| – | CHAR-CMD-WIRE-1 (Platform) | Platform #1622 |

## W toku
| Wave | Batch | PR | Worker (sesja) | Stan |
|---|---|---|---|---|
| 1 | LOGOUT-WIRE-1 | #2127 | W2f 01Y3ihfi | CI; merge main po #2126 |
| 1 | LOOP-WORLD-1 marker flip | (nowy/istniejący) | W2c2 01VFYCsQ | start; amendmenty GAME-ITEM-01 §4 / DUR-03 §39.1 zaakceptowane |
| 2 | CHAT-2B | #2128 | W2h 013fCsCy | następny w kolejce; P1 (atomowy check+notify), runda 2/2, auto-merge OFF |
| 2 | NPC-CONTENT-2 | #2129 | W3b 01VUmRdS | CI quest-authoring czerwone |
| 2 | PRESENT-CONTENT-1 | #2130 | W2g 01Ex9Xhz | Codex done |
| 2 | RANGED-CONTENT-1 PR1 | #2096 | W3c 01P6FZqT | konflikt z main, przejęty |
| 3 | MAP-VIEWPORT-OPT-1 | (nowy) | W3a2 0181Dg1m | opcja B (20% rdzenia, liczymy tylko graczy w ruchu; rewalidacja botami → MAP-VIEWPORT-BUDGET-REVAL) |

## Do zrobienia
**Wave 1 (reszta):** ADMIT-CAP-1, CHAT-2 (po re-review CHAT-0), ITEM-LOOK-1, PREREQ-1 (dzieci w wave pierwszego konsumenta), SEC-1; sekwencyjnie (migracje): ITEM-LOOP-1, DAWNPORT-1, LOOP-PROGRESS-1, DEATH-ML-LOSS-1.

**Granica Game↔Platform (owner 2026-10-15, z opinii architekta o 2 bazach):**
1. PLATFORM-RESTORE-RECONCILE-P1 (Platform; odblokowuje CHAR-CMD, U-CC7), najpilniejsze.
2. STORE-DELIVERY-CONTRACT-1: kontrakt dostawy zakupów Store (idempotencja, refundy, chargeback); najpierw sprawdzić Platform `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`.
3. ENTITLEMENT-RECONCILE-1: okresowy job uzgadniający portfel/uprawnienia Platformy z receiptami Game.
4. PLATFORM-BACKUP-OFFNAS-1: kopia off-NAS i drill restore Platformy (ADR-0046 §7).
5. ADR-0004-XREF-1 (light, Game): odesłanie ADR-0004 §2 → Platform ADR-0046 §4 (osobne instancje).

**Wave 2 (reszta):** PREM-2b+PREM-5, OPS-SAVE-2, BUNDLE-GATES-2 (follow-up), LOOP-DAMAGE-1, LOOP-RANGED-1 (PR2 RANGED-CONTENT-1), LOOP-VITALS-1, NPC-ECON-1 (+NPC-CMD-1b, NPC-TRADE-WIRE-1), QUEST-RT-1, MAGIC-SUMMON-1, LOOP-MONSTER-1, CREATURE-MOVE-1 (residue), PARTY-1, WEATHER-1; potem PVP-1b, PVP-RT-1, PVP-DEATH-1.

**Wave 3:** ADMIT-CAP-2, MAP-VIEWPORT-LIMIT-1 (po OPT-1), LOOP-MAP-1 (po #1942), ITEM-TIMED-1, ITEM-STORAGE-1, NPC-WORLD-1, LOOP-GROUND-1, CRAFT-1, PROG-ADV-1, MONSTER-META-1, HOUSE-RUNTIME-1a/1b/1c, MAP-OVERLAY-1c, MAP-CUTOVER-1c, PARTY-XP-1.

**Wave 4:** TRADE-PLAYER-1, MAIL-1, MARKET-1, GUILD-1, GUILD-BANK-1, GUILD-CHAT-1, VIP-1, ITEM-IMBUE-1, ITEM-FORGE-1, HUNT-1, DAILY-1, ACH-1, RANK-1, APP-1..4. (STORE-ENT-1 zablokowany decyzją §32.)

**Wave 5:** STEP-PACE-ENABLE-1, HOUSE-1a, HOUSE-ACL-1, HOUSE-1b, HOUSE-WIRE-1, BED-1, BED-REGEN-1, GUILDHALL-1, GUILD-WIRE-1, VIP-2, HIRE-1, LOOP-TRAIN-1, EVENTS-*, ARENA-1, SOCIAL-LATE-1, GM-1, SPECTATE-1, TUTORIAL-1.

## Model pracy równoległej (owner zaakceptował 2026-10-15, bez pilota)
- Architekt zakłada draft-szkielety (`skeleton/<task>`, 1 commit: `docs/agents/tasks/<TASK>.md`) dla wszystkich niezrobionych zadań fal 1–5.
- Wykonawca (inny agent) wypełnia tylko owned_paths, pushuje rzadko i paczkami (draft też odpala CI). Regeneracja, piny, rejestry, migracja i wiring w `connection.rs`/`mod.rs`/`capabilities.rs` dopiero w slocie kolejki, po merge main.
- CP oznacza ready w slocie; 1 runda Codex. CP + architekt mają ostatnie słowo (poprawność i kod).
- Pakiety nierozdzielne: wire, migracja, content+piny, mint/burn+ledger, pakiety „+”.
- Osobno na main: przyspieszyć/zawęzić `quest-authoring-schema.yml` (limit 30 min).

## Follow-upy
- BUNDLE-GATES-2: P2 z #2115 (round 18, `mentions` liczy tokeny zamiast wywołań).
- MAP-VIEWPORT-BUDGET-REVAL: rewalidacja budżetu botami symulującymi graczy.
- Owner zablokowany na `sounds/`.

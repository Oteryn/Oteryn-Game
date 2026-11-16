# Mechwire package E: client-to-server tests per BOTH-WIRED E row

Scope: rows `E*` of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` that `CE.md` marks
BOTH-WIRED (E01, E02, E03, E04, E05, E06, E10). E07, E08, E09 and E12 are SERVER-ONLY in `CE.md`
(client gaps listed there) and are out of this package.

`CP` = `apps/game-server/src/qualification/client_phases`. Every test below boots the real node
(`tools/qualification/node_boot/run.sh`), admits through `oteryn_dev_client::connect_session`
(`oteryn_session::Session` over TCP+TLS), sends through the client-runtime adapter, and reduces the
reply with the `client_ui` panel or the `Session` domain reducer (inventory, conversation, trade
window). "relog" means ordinary logout plus a fresh admission of the same Character.

| row | test (file:line) | new/existing | chain covered | verified how | gap |
|---|---|---|---|---|---|
| E01 NPC shop, bank | CP/full_e2ef4.rs:373 `shop` (E2EF4 row E01: greet, look, sell, buy, sell-all, deposit, withdraw, transfer, relog-inventory-bank) | new (replaces the stub that returned `sell/sell-all/bank/relog-assertions-unimplemented`) | talk cmd 7 → `apply_conversation`; trade window domain 8 → `apply_window`; LOOK and SELL_ALL through `client_ui::npc_shop::request`/`apply_result` (cap 81 cmd 97); buy/sell cmd 8 → exact inventory and coin-value delta in `Session::inventory`; bank dialogue prompt + `yes` → reply line, `balance` line, recipient balance read from PostgreSQL; relog → same inventory and same `balance` | `cargo check -p oteryn-game-server --lib --tests` (see report); not run: node boot needs the pinned Platform stack | sell-all needs a count-1 SELL offer for a carried item in Sam's catalogue, bank needs a reachable banker: both report `content:` if absent |
| E01 (slices) | CP/npc_shop.rs:9 `transact` (via CP/world.rs:3250); CP/client_row_e01.rs:44 (capture) | existing | greet, trade window, look, funded buy with inventory and gold delta | existing | superseded by the full row above |
| E02 coin exchange | CP/full_e2ef20.rs:280 `journey` (E02.full: legitimate gold, gold→platinum, platinum→gold, insufficient refusal, bank-only refusal, fresh admission) | existing | NPC talk cmd 7 → conversation decode; backpack coins from `Session::inventory`; relog coins | existing; helpers made `pub(super)` for reuse, behaviour unchanged | none |
| E02 (capture) | CP/client_row_e02.rs:248 | existing | replay capture | existing | none |
| E03 coin use | CP/full_e2ef4.rs:670 `coins` (E2EF4 row E03: gold-up, platinum-down) | new (replaces the stub that only listed USE dispositions) | gold from NPC sales (`full_e2ef20::earn_gold`); USE cmd 2 on a ≥100 gold stack and on a platinum coin → `UseDisposition::Committed` and exact backpack coin delta | compile only (as above) | **content gap**: platinum-up needs 100 platinum, crystal-down a crystal coin, converter-charge/-retirement, magic-toggle/-tick and relog-coins-charges need a gold or magic converter (i23722/i25719/i28525/i28526) that no reachable NPC catalogue offers; the row stays FAIL with that reason |
| E03 (capture) | CP/client_row_e03.rs:35 | existing | capture; fails without a 100-gold starter stack | existing | same content gap |
| E04 player trade | CP/full_e2ef4.rs:687 `trade` (E2EF4 row E04: request, look, cancel, accept, logout-race, relog-custody) | new (accept, logout-race, relog-custody added; request/look/cancel existing) | cap 60 cmd 78 through `client_ui::player_trade::request`/`apply_result`, domain 26 `decode_state` on both peers; exact inventory exchange on both sides; partner logout with an open offer closes the trade and keeps the offer carried; relog of both Characters shows the exchanged custody | compile only | none known; logout-race asserts the server closes the trade on partner logout |
| E04 (slice) | CP/player_trade.rs:28 `pair` (via CP/world.rs:2851) | existing | offer, counter offer, accept, completion, both inventories | existing | no relog |
| E05 market | CP/full_e2ef4.rs:893 `market` (E2EF4 row E05: browse, create, cancel, history, accept, relog-settlement) | new (history, cross-player accept, relog-settlement added) | cap 63 cmd 80 through `client_ui::market::request`/`apply_result`; peer sells into a one-unit buy offer (item leaves the peer's inventory); creator history lists both ended offers; relog → no active offer, same history | compile only | accept needs the peer starter to carry the offered definition (reported `content:` otherwise) |
| E05 (slices) | CP/client_row_e05.rs:54; CP/market.rs:2 (via CP/world.rs:2863) | existing | browse, create, cancel, history capture | existing | no cross-player accept |
| E06 market expiry, refunds | CP/full_row_e06.rs:64 (E06.full) | existing | wares pages, every book, non-marketable refusal (code 14), buy offer create/cancel refund in history, relog | existing | **server/client gap**: no scheduled expiry route and no expiry push (`CE.md`, serve/outbound.rs:338); the row reports MISSING-SERVER at step 7 |
| E06 (capture) | CP/client_row_e06.rs:63 | existing | capture with fresh admission | existing | as above |
| E10 store | CP/full_row_e10.rs:163 (E10.full: catalogue, purchase, stale-quote refusal, history, relog history) | existing | cap 154 through `client_ui` store `apply_result` and 0285 purchase history | existing | **client gap**: inbox claim cmd 186 (`client_ui` 0154 `claim()`) has no caller (`CE.md`), so no claim test |
| E10 (capture) | CP/client_row_e10.rs:57 | existing | capture | existing | as above |

Counts: 7 BOTH-WIRED rows. Existing full chain: 3 (E02, E06, E10). Completed here: 4 rows in
`full_e2ef4.rs` (E01, E04, E05 full; E03 partial). Gaps: E03 content (coins and converters), E06
expiry (server), E10 claim (client).

No shared registry, `SCENARIO_MATRIX.md` or `world.rs` line changed: the E2EF4 journey already
reports rows E01/E03/E04/E05 (CP/full_e2ef4.rs:120-123), so no new row index was needed.

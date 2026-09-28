# Tibia manual notes: controls_trading

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=controls_trading>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `4e039c72b51a1ce7b1fefd015e9c3b30e021079d6adb678728d256cc0c63d799`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Client / UI (with the server rules behind it)
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §controls_trading <heading>, capture 2026-09-28`.

## 4.3 Trading
- [both] Three trade paths: NPC trading, the Market, and direct player-to-player trading.

## 4.3.1 Trading Items With NPCs
- [client] Opened via context-menu NPC trade request; sidebar window with Sell/Buy toggle, item list, and Look/Inspect on item pictures; sort by name/price/weight.
- [server] "Buy with backpacks" wraps purchases into bags for a flat 20 gold fee per bag.
- [client] "Ignore capacity" allows an over-capacity purchase; [server] overflow items land on the floor at the character's feet instead of inventory.
- [both] Quantity via slider; Shift/Ctrl/Shift+Ctrl while dragging enlarges the step size.
- [server] Inventory-gold shortfall is auto-covered from the bank if the balance suffices. Trade fails with a warning if there's not enough free inventory space.

## 4.3.2 Face-to-Face Trading With Other Players
- [client] Manual recommends the safe-trade dialog over informal handoffs. Initiated via item context menu "Trade with ..." → crosshairs → left-click the target character.
- [client] Counterparty gets a notification; a counter-offer opens a shared trade dialog listing all offered items, right-click inspectable.
- [server] Trade executes only when both sides click "Accept" AND both have enough free capacity for the incoming items. Modifying an offer after entering the dialog is not allowed — it cancels the deal instead.
- [client] "Reject" cancels and allows starting a new trade. Whole containers can be offered via their own "Trade with ..."; contents auto-reveal to the counterparty.
- [server] Hard cap: 100 items per trade.

## 4.3.3 The Market
- [client] Accessed only through the depot: open depot locker, click the 4th box to open the Market interface.
- [server] Only Premium accounts may place new buy/sell offers; free accounts may only accept existing offers — except free accounts CAN place buy and sell offers specifically for Tibia Coins.
- [server] Max 100 simultaneous offers per Premium character.
- [server] Max 64,000 items per single offer.
- [server] Placing an offer costs a fee: 2% of offer price, minimum 20 gold, maximum 1,000,000 gold; fee is charged from the bank account immediately.
- [server] Max settable price per offer: 999,999,999,999.
- [server] Offers expire after 30 days unmatched; unsold item(s) return to inbox, unspent offer money returns to bank account (fee is not refunded).
- [server] Non-tradeable item categories: inscribed documents (books, parchments, etc.), pourable liquid containers, keys, inscribed goblets, inscribed fansite items, non-portable items, and any container (backpack/bag/box) that still has contents.
- [server] Unfair-offer detection: computed per-server, using a daily-updated average price; sell offers ≥25% above average and buy offers ≥25% below average are flagged/highlighted (shown in red) — this requires sufficient trade-volume history per item to function, so it may not trigger for rarely-traded items.
- [client] Browsing: alphabetical categories (weapon categories split 1H/2H); "Level"/"Voc." filters restrict to usable items; free-text Search; bracketed number = depot quantity owned.
- [client] Offer lists: sell offers (upper) and buy offers (lower), each sorted by unit price; button to create an own offer.
- [client] Bottom bar: "Offers" (default lists), "Details" (weight/description/vocation/level requirement + 30-day stats: average price, offered/sold counts, high/low price — refreshed daily at server save), "My Offers", "Close".
- [client] "My Offers": Current Offers (list with "Cancel" → unsold items to inbox / unspent money to bank, fee never refunded); Offer History (last 600 combined sell+buy, end reason + final price); "Market" returns to Offers view.
- [server] Sell offer: item must be in the depot chest; set piece price + quantity; "Anonymous" hides identity; fee deducted from bank at "Create". If a matching/better buy offer exists, it auto-matches immediately — item to buyer's inbox, gold to seller's bank.
- [server] Buy offer: fee + full item price deducted from bank at "Create"; auto-matches immediately against an existing matching/lower sell offer the same way.
- [server] Accepting a sell offer: needs full bank balance (unaffordable ones greyed out); item → buyer's inbox, money → seller's bank. Fulfilling a buy offer: item must be in the depot chest (missing ones greyed out); same transfer.
- [both] Accept-dialog quantity sliders also support Shift/Ctrl/Shift+Ctrl step acceleration.

## Open questions for Oteryn
- Exact NPC bag fee basis (per bag vs per transaction) isn't spelled out beyond "for each bag."
- Whether "Ignore capacity" purchases still consume gold normally despite landing on the floor is unstated.
- No cooldown/rate-limit stated for repeated face-to-face trade requests.
- No definition of how the Market's daily average price is computed.
- Partial-match behavior for Market offers (e.g. buy 50 vs sell 30) is not detailed.
- No stated cap on total Market offer value per character (only per-offer max given).
- Whether "Anonymous" also hides an offer from the seller's own visible Offer History to others is unclear.

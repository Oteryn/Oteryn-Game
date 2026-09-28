# Tibia manual notes: houses

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=houses>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `826eb1567318538603274ba25080db5c0796041a794a6c70b751df1104d00b69`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §houses <heading>, capture 2026-09-28`.

## §houses 5.7.1 Benefits of Houses

- Three building categories: normal houses (individual/small-group living), shops (player-run businesses), guildhalls (guild-only housing). Can appear outside cities too. [server]
- **Protection**: house interior is a protection zone — occupants cannot be attacked while inside. [server]
  - Trade-off: no hit point / mana regeneration while standing in a protection zone (standard PZ rule, applies to houses too). [server]
  - Entry restricted to invited characters only, which also protects stored belongings from theft. [server]
- **Improved Recuperation**: beds usable only by Premium-account characters while offline (sleeping). [server][platform]
  - Soul point regen: +1 soul point per 15 minutes asleep. [server]
  - HP/mana regen while asleep continues only as long as "food" (well-fed status from prior eating) lasts, and is slower than online regeneration. [server]
  - Skills train while sleeping in a bed (offline skill training mechanic). [server]
- **Improved Item Storage**: placeable containers (chests, drawers, etc.) inside houses have no item-count cap, unlike depot chests which are capped. [server]
- **Prestige**: purely social/flavor — no mechanical rule beyond display of owned decorative items being theft-proof due to PZ. [server]

## §houses 5.7.2 Getting Houses

- Two acquisition paths: auction (uninhabited houses) or player-to-player transfer/purchase. [server][platform]
- Eligibility/limits:
  - Only Premium accounts may rent/bid on houses. [platform]
  - Max 3 rented houses per account (across all characters on the account). [server]
  - Guild leaders may additionally rent exactly 1 guildhall; only guild leaders can rent a guildhall. [server]
  - A character may only be bidding on or own one house transaction at a time (one active bid/purchase at a time). [server]
  - The acquiring character must have already left Newhaven/Rookgaard (starter islands excluded from house eligibility). [server]
- a) House Auctions:
  - Listings surfaced both on the Tibia.com Houses section and in-client Cyclopedia "Houses" tab, filterable by world/city/status ("Auctioned"). [client][platform]
  - In-client bid flow: select house -> "Bid" -> enter amount -> confirm ("Yes") dialog. [client]
  - Website bid flow: mirrors in-client but requires login + password confirmation on submit. [platform]
  - **Bids cannot be withdrawn once confirmed**; the only mitigation is lowering your own bid limit down to the current highest bid, making it easier for someone else to outbid you. [server]
  - Auction starts on first bid; fixed duration = 7 days from that first bid. [server]
  - Bidding model is proxy/limit-based: system auto-bids the minimum needed to stay top bidder, up to your entered limit; visible current bid can sit at 0 gold if uncontested even though your limit is much higher. [server]
  - Precondition to place any bid: bank account must hold (bid limit amount + first month's rent) — this amount is reserved/locked for the duration of the auction (or until outbid). [server]
  - On auction win: bid amount + first month's rent are deducted from the bank account. [server]
- b) Buying and Selling Houses (guarded player-to-player transfer):
  - Only the current owner can initiate, via Cyclopedia or website "Transfer" button. [server][platform]
  - Owner sets: transfer date (must be in the future, max 30 days out — in-client text says "no more than 30 days", website text says "1-30 days"), target character name, and price (0 gold = free transfer). [server][platform]
  - Same-account transfer allowed if target character is on the same game world. [server]
  - Target character must accept before the transfer date and must hold the required gold in their bank account at execution time. [server]
  - Transfer is cancelled automatically if: target rejects, target fails to accept in time, or target lacks sufficient bank funds. [server]
  - Owner can cancel unilaterally any time before target accepts (Cyclopedia "Cancel Transfer" / website "Keep House", both need confirmation). [server][client][platform]
  - Execution happens at server save on the specified date. [server]
  - On successful transfer, all items left behind become forfeit to the new owner (old owner cannot reclaim without new owner's cooperation) — EXCEPT Store-purchased items, which are auto-wrapped and sent to the old owner's inbox. [server]
- c) Moving Out:
  - Owner-initiated voluntary vacate, any time. [server]
  - Flow (in-client and website both): select house -> "Move Out" -> choose a move date **1-30 days in the future** -> confirm. [server][client][platform]
  - Executes at server save of the chosen date; house then becomes free to bid on by others. [server]
  - Items left behind are auto-moved to the (former) owner's depot inbox (not lost, unlike the sale-transfer case). [server]

## §houses 5.7.3 Rights and Duties

- a) Rent:
  - Monthly rent, amount varies by house size/location/furnishing (no fixed formula given). [server]
  - Rent is charged **in advance**, auto-debited from the owner's bank account. [server]
  - Insufficient funds at due date -> owner gets a warning letter in inbox, with a **1-week grace period** starting the due date. [server]
  - If unpaid after that week -> eviction: owner loses the house, portable belongings move to inbox, but oversized furniture (doesn't fit in a backpack) stays behind in the house. [server]
  - Post-eviction penalty: **all characters on the account are blocked from renting any house for 30 days.** [server]
  - The vacated house re-enters the auction pool. [server]
- b) House Rights — rights editing is performed via special "house spells" cast while standing inside the house (owner/sub-owner only, as noted):
  - `Aleta Sio` — Edit Guest List: owners and Premium sub-owners add/remove invited character names, one name per line; removing a name un-invites. [server]
  - `Aleta Som` — Edit Sub-Owners: owner-only list; sub-owners inherit invite/uninvite rights and kick rights. [server]
  - `Alana Sio` — Kick Character: usable by owner or sub-owner (with a target name) to expel someone; any character can also self-kick. [server]
  - `Aleta Grav` — Edit Door Rights: owner-only, must be cast while standing at the specific door; sets which characters may open/close that individual front/interior door, enabling private sub-areas (notably useful in large houses/guildhalls); the owner always retains access regardless of a door's configured list. [server]
  - List-editing syntax available across these lists: `*` wildcard (multi-character), `?` wildcard (single character), `*@guildname` = all members of a guild, `rankname@guildname` = members of a specific guild rank, leading `!` = exclude this match, leading `#` = comment line (no effect). [server]

## Open questions for Oteryn

- Exact rent pricing formula (by tile count / furnishing / city) is not in this section — needs a concrete per-house price table or generation rule.
- Whether the 30-day post-eviction rental ban is per-account hard block on ALL rental actions (bidding too) or just on completing a new rental.
- Transfer date rule discrepancy between in-client text ("no more than 30 days") and website text ("1-30 days") — confirm true bound (assume 1-30 inclusive) for server validation.
- Determine whether guildhall rent/eviction follows the identical rent flow as normal houses or has guild-bank-first debit order (guild notes say guild leader's bid pulls from guild bank first, falling back to personal bank — needs reconciliation with rent debits specifically, not just the bid).
- Concurrent-bid tie-break rule when two players set identical bid limits is unspecified.
- Whether "one active bid/purchase at a time" is enforced per character or per account (text says "the character you are using").
- Full mechanics of how oversized furniture "stays in the house" post-eviction interacts with the next tenant (does new owner inherit it, or is it cleared?).
- Door-rights default state for a freshly rented house (open to all invitees vs owner-only) before any `Aleta Grav` cast.

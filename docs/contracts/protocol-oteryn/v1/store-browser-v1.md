# Store browser (T130)

Owner decision B 2026-11-09. Existing STORE_V1 purchase, delivery, exact quote,
session-generation fences, and Platform commercial authority remain authoritative.
This read-only view neither grants cosmetics nor expands the admitted offer set.

Rules: `docs/reference/tibia-manual/store.md:23-37` (official manual §6.2, capture
2026-09-28) specifies nested categories, search, popularity ordering, prices,
account/transferable balances and outfit/mount try-on. Canary `52550d5e`,
`data/modules/scripts/gamestore/catalog/consumables_exercise_weapons.lua:1-15`
provides category/parent, item artwork identity and priced product metadata.
Crystal `201189f`, `data/libs/systems/hireling.lua:30-41,205-227` provides Hireling
male/female artwork. Accepted Oteryn admission decides which goods are sellable;
empty reference categories are shown with an explicit empty result, never invented goods.

Owner decision B 2026-11-09, reference silent: popularity is descending counts of
Game-recorded accepted Store purchases (existing immutable Game purchase journal),
not Platform coin sales. Ties use offer key. Name ascending/descending and price
ascending/descending sorts apply to all matching products before paging. Search
is Unicode-lowercase substring matching of title/key. Parent categories include
all descendants; Home includes every admitted offer. Cursors bind category,
search, sort, ordered quote/artwork data and popularity snapshot; changed queries
or data require a first-page refresh. There are 32 products per page, 64 categories,
128-byte keys/search/cursors. No local asset path or arbitrary URL is accepted.

The layout has a category sidebar, top search/sort controls, central priced product
list, right artwork/quote detail and purchase controls, and balance/history/coin
service controls below. Try-on offers male/female and four directions; it does not
alter the actor or grant an entitlement. The pinned client asset store renders
server-resolved item, outfit and mount looks. An unavailable asset is shown as
unavailable; an image never substitutes for eligibility or a confirmed purchase.

## Request

Command 6118 on existing capability 154. `SBQ1` magic; u16-LE length-prefixed UTF-8
category, search; u8 sort (0 popularity, 1 name asc, 2 name desc, 3 price asc,
4 price desc); length-prefixed cursor. Empty cursor starts a page. Maximum 512 bytes.

## Response

`SBR1`; u64-LE total and transferable Platform balances, then a u8 known flag; u8 category count; each
category has length-prefixed key, parent (empty for roots) and title. Parents
precede children. Then u8 product count. Each product has its category key,
u16-length-prefixed legacy catalogue page containing exactly one unchanged offer
and its catalogue revision (zero balances, no cursor), u8 artwork kind (0 service,
1 item, 2 outfit, 3 mount), two u32-LE look IDs (male/female, same for items/mounts).
Finally length-prefixed next cursor. Maximum 24,576 bytes. Unknown, invalid,
truncated or trailing data is rejected. Confirmation sends unchanged command 185
with the selected product's exact revision, price and currency. Game never owns
purchased coins; balance, debit and order/retry authority remain on Platform.

The accepted Platform balance contract exposes COINS available/reserved, not a transferable split. Until Platform supplies that split, the known flag is false, transferable is zero, and the UI explicitly displays it as unavailable; Game never equates reserved or total coins with transferable coins.

As in the accepted legacy catalogue, the total field carries Platform COINS available; the browser labels it Available. Reserved funds are not advertised as spendable. The admitted catalogue is bounded to 512 products and the client retains at most 32 page cursors.

Migration 2503 backfills a read-only popularity summary under a journal write lock. An AFTER INSERT trigger increments it in the same transaction as the existing immutable accepted-purchase receipt; conflicts/replays that do not insert a receipt do not increment it. Runtime/control roles can SELECT the summary but cannot mutate it or execute its trigger function directly. No new wallet, price, or commercial receipt is introduced.

Item artwork uses the active admitted profile's production binding external ID,
not its sorted wire definition reference. Canary `52550d5e`,
`src/items/items.cpp:153-174` loads item types directly at appearance object IDs.
An optional trusted catalogue `preview` binding selects an admitted item policy,
cosmetic key or mount key (kind 1/2/3); it changes artwork only, never delivery.

## Store Home extension

Capability 586 requires selected STORE_V1 (154). It is offered when the existing
Platform Store client is configured. Command 6118 retains its existing
semantics and exact wire encoding. The 6335 and 6336 results are solicited only.

### Recent offers request

Command 6335 uses the same SBQ1 request with category `featured`. Search, sort
and the established bounds apply.

### Recent offers result

The same SBR1 schema. The server intersects existing admitted offers with
Crystal 201189f8 `data/modules/scripts/gamestore/gamestore.lua`'s active
`STATE_NEW` rows (Corpsefire Skull and Doom Skull mounts) by trusted mount
preview key. This matches the current [Tibia Store manual](https://www.tibia.com/gameguides/?subtopic=manual&section=store)
placement of recent offers on Home. The pinned Oteryn catalogue currently
admits neither mount, so the recent view returns an empty product list and
the real Platform balance. Cursors from 6118 cannot be reused on 6335.

### Banners request

Command 6336 takes an empty payload.

### Banners result

A nonzero u8 dwell time in seconds, u8 image count (1–8), then that many
u16-LE length-prefixed UTF-8 image keys (1–64 bytes including affixes).
Each key is `home/` followed by lowercase ASCII letters, digits or underscores,
then `.png`. Keys identify trusted local assets. Do not interpret them as
filesystem paths or fetch URLs. The client displays the first entry and
advances in order, wrapping at the end. Crystal 201189f8
`data/modules/scripts/gamestore/gamestore.lua:12–15` supplies 10 seconds and
the Armoured Archer and Podium of Tenacity keys in that order.

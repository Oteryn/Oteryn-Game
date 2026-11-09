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

Command 6117 on existing capability 154. `SBQ1` magic; u16-LE length-prefixed UTF-8
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

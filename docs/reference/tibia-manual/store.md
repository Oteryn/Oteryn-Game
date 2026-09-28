# Tibia manual notes: store

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=store>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `82a39e255b485a2b08ded337b6fde145e031255a228fa735766fa913d9b871d5`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Game content catalogue (Store offers); payments are Platform
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §store <heading>, capture 2026-09-28`.

## 6.1 Tibia Coins

Tibia Coins function as an in-game currency purchased via the Webshop [platform] in various bundle sizes. Acquired coins attach to the account and are immediately available in-game [server]. Available to all characters on an account simultaneously [server].

Transferability rules:
- Coins can be gifted to characters on other accounts [server]
- Coins can be traded via the Market [server]
- Both Premium and free-to-play players can transact [server]
- **Restricted coins** [platform]: Depending on payment method, purchase amount, and account history, some purchased coins may be locked for 6 months and cannot be gifted or traded [platform]

## 6.2 Store Interface

Store access: Activated via button beneath inventory [client]

**Categories** [client]: Home (featured items), Premium Time, Consumables, Cosmetics, Boosts, Extras. Many categories include nested subcategories. Search function available [client].

**Products display** [client]: Listed with quantity/availability groups. Greyed-out items are either limit-reached or character-ineligible (example: potions when at capacity). Sorting by popularity default [client]. "Try On" button for mounts/outfits shows appearance preview [client].

**Purchase mechanics** [client]:
- Instant purchase for character currently logged in; optional confirmation dialog (toggle in Options) [client]
- Multiple quantity buttons for bulk purchases [client]
- Purchase applies to active character only; exceptions: Premium Time applies account-wide [server]

**Tibia Coins balance display** [client]: Shows total account balance with two symbols: one for all coins, one for transferable-only subset [platform]

**"Get Tibia Coins" button** [client]: Launches Webshop for purchase [platform]

**"Gift Tibia Coins" feature** [client]:
- Requires exact recipient character name (any world, any account) [server]
- Gifts arrive immediately [server]
- Amounts must be 25-coin increments [server]
- No recall after gifting [server]
- Cannot gift to own account characters [server]

**Character Auction setup** [client]:
- Prerequisite check for seller requirements [client]
- Set starting price and end date/time [client]
- Select up to 4 rarest/most valuable items and up to 5 best skills/achievements for featured display [server]
- Full character details displayed only via detail view [server]
- Preview on final confirmation page before submission [client]
- Auction hosted on Tibia website under Char Bazaar [platform]
- Bidders can watch auctions and view ended auctions (30-day window) [platform]

**Transaction History** [client]: Displays all coin additions (green/+) and subtractions (red/-), with category description. Detailed view expandable per transaction [client].

---

## Open questions for Oteryn

- How should character auctions integrate with the marketplace if Oteryn supports trading functionality?
- Should restricted (non-gifted/non-tradeable) coins be visually distinct in character inventories?
- What payment methods and currency/localization support will Oteryn offer for Tibia Coins purchases?
- How long should Oteryn enforce the 6-month coin restriction, and what triggers it?
- Should transaction history be queryable by players for audit/dispute purposes?

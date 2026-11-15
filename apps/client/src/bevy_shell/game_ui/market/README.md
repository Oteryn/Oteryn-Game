# Market visual parity

The Market toolbar button / Ctrl+M opens this presenter. Selecting a catalogue
ware requests its offer book and completed-trade statistics through command 6126.
The price-range chart compares buy and sell offers on one zero-based gold axis;
colored lines show minimum–maximum and white markers show the unit average.
The existing exact text statistics remain available. Empty sides have no line.
These are server-owned 30-day summaries, not observations accumulated by the UI.

Reference: edubart/otclient `dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_market/market.lua:345–440` separates purchase/sale statistics and
reports min/max/average; `:445` uses the selected item's display binding.
This is client-structure reference evidence only. The range chart is an original
presentation of Oteryn's accepted aggregate data, not a claim of time-series parity.

## NEEDS-CONTRACT

- Verified artwork: command 6126 `Reply::Catalogue.items` / `Ware` supplies a
  server definition, name and requirements, but no appearance binding. A missing
  server-owned `ItemAppearanceBinding { definition, subtype, catalogue_digest,
  object_appearance_id }` message must bind the item to a verified asset catalogue.
  A definition must never be cast to a sprite or appearance identifier. No
  accepted item binding message currently supplies this mapping to this presenter.
- Graphical price history: command 6126 `Reply::Statistics` / `Reply::Book.statistics`
  only supplies one 30-day aggregate per side. The missing market history message
  would be `MarketPriceHistory { definition, since_us, as_of_us, buckets:
  [{ start_us, end_us, buy: PriceSummary, sell: PriceSummary }] }`, with empty
  buckets explicit. Refreshing a snapshot cannot reconstruct
  historical prices. No time-series points are invented or persisted locally.

The batch implements graphical aggregate statistics and available item details;
verified artwork and a dated history graph remain contract-blocked.

# T124 — market catalogue and completed prices

Owner decision B, 2026-11-09 accepts this read-only extension to MARKET0 §10 and
MARKET_V1. Existing trade, fee, custody, Platform commercial authority and session
generation rules remain authoritative. The existing default-state, tier-zero trade
cohort is unchanged; classification filters do not admit upgraded item instances.
This feature never credits purchased coins.

Sources:

- Canary `main`, commit `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
  `src/protobuf/appearances.proto:233–239`: market category, profession and level
  metadata. Its category enum differs from the qualified 15.30 capture.
- `tools/content-schema/item-authoring/engine_items.py:315–345` records the current
  15.30 category mapping: 26 is soul cores and 27 fist weapons. The pinned appearance
  SHA-256 is `2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2`.
  `build_market_filter_catalogue.py` extracts only numeric metadata for the already
  admitted 5,095 types. No asset bytes or reference item strings are redistributed.
- Canary, same commit, `src/io/iomarket.cpp:307–315,346–373`: statistics use accepted
  history, count executions, min/max unit price and arithmetic mean of unit prices.
  `src/game/game.cpp:11095–11097` distinguishes the accepted offer from the
  acceptor's extended observation. Purchase and sale statistics group the accepted
  offer's side. Current quotes are not completed trades.

The catalogue enumerates **all** current marketable type references in immutable
active-generation index order, 32 per page, before applying name substring,
category, vocation, inclusive minimum/maximum level, forge classification and
current-depot possession filters. Category choices cover the whole catalogue,
including categories outside the returned page. Names and qualified requirements
come from active Oteryn item definitions. Missing requirement metadata stays unknown
and does not match a requirement filter. Unknown names use `Item <reference>`.
The depot filter means possession in the currently admitted town depot, not an
eligibility or custody promise; placement still requalifies instances atomically.

Owner decision B 2026-11-09, reference silent: Unicode lowercase substring
search; 80-byte query text; 32-result pages; unknown metadata handling above;
a rolling 30 × 24-hour window ending at one PostgreSQL statement timestamp;
an additional combined execution summary. Separate buy/sell summaries use the
canonical settled offer's side (not an inferred participant role), matching the
reference's accepted-offer grouping. For automatic matching, which this reference
does not define, use the settlement's canonical offer (owner decision B 2026-11-09,
reference silent). Each transaction counts once, even when
both participants have history rows. Units sum executed quantities; average is
`floor(sum(executed unit prices)/number of executions)`, not amount-weighted.
Empty history has zero executions/units and absent prices, never a fabricated zero.
Only settlement kinds 2 (explicit accept) and 5 (automatic match) qualify; placements,
cancellations, expiry and held-credit release do not. World comes from the current
session, never the client; ChannelId remains a separate admission fence.

## Read command

Capability 63 gains read-only command 6117, claimed against cgmech `af6ab2c76` (previous maximum 6116). No capability file is added; the existing Market capability is amended. Its canonical request is protobuf field 1 = 9 followed by field 8
(length-delimited UTF-8 JSON, <= 512 total bytes). The JSON is the deny-unknown-fields
`market_details::Query`: `Catalogue {filter,page}` or `Statistics {definition}`, or `Book {definition,page}`. Book pages advance each side independently, up to 32 offers per side and 700 visible offers per side, using the existing price/time/id keyset reader.
Original command 80 and actions 1–8 are unchanged (see `market_v1.proto`). Command 6117 is never relayed as a legacy domain-28 value, and command 80 refuses its read-query envelope. The new route rejects all old mutation payloads. Existing capability and
depot gates run before either query; the durable projection independently checks
current recovery, node, session generation, character and World/Channel scope.

The bounded JSON reply is `market_details::Reply`, <=32 KiB: Catalogue (items,
all category choices, echoed page, has_next), Statistics (definition, as_of_us,
since_us, trades, units, minimum, maximum, average, buy and sell summaries), Book (definition, page, up to 64 rows, statistics, has_next), or Refused (existing market
code). Each client tracks one outstanding details query and ignores mismatched or
late replies. Original offer mutations retain their original result codec.

## Presentation

Buy and sell offers have separate report tables with Item, Amount, Unit price,
Total, Status and anonymity columns; selection preserves the server's opaque offer
identity. The catalogue has its own named item table and filter controls. Browsing
an item requests fresh 30-day completed prices; loaded-page quote summaries remain
separately labelled. Existing create, accept, cancel, paging and history callers
are preserved.

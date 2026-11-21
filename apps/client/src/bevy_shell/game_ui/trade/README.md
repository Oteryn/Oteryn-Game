# Player trade presentation

Both offers have labelled inspection buttons in addition to clickable item
slots. They use capability 60's existing snapshot-bound Look request and show
the server's description in the scrollable details area. Pending, modal and
stale-offer guards apply to both entry points.

## Batch 44: NEEDS-CONTRACT

Offer artwork and container contents remain unavailable:

- `PlayerTradeOfferV1` contains only `definition`, `count` and `sub_type`.
  Artwork needs an accepted, verified definition/subtype-to-appearance binding
  or a verified appearance reference on each offer entry. A definition is not
  a sprite ID; inventory handles must not be invented for trade offers.
- `PlayerTradeStateV1` has no container entries. Container inspection needs a
  bounded server-owned offer-content snapshot tied to the current trade pair,
  plus an entry selector in `PlayerTradeLookV1` (currently only trade ID and
  counter-offer flag). Capability 132 also restricts trade inspection to leaf
  index zero. The server trade owner currently admits leaves only.

No new protocol or container transfer authority is introduced here.

Structure reference only: edubart/otclient at
`dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/game_playertrade/playertrade.lua:35-69`: server-supplied item list,
virtual offer widgets, and per-entry inspection. This is not Global artwork
evidence and no third-party assets are included.

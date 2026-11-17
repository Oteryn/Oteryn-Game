# WORLD-SRC W12 source record (2026-11-05)

## Source custody

All donor findings below are `OTS_HYPOTHESIS_ONLY`, inspected from
zimbadev/crystalserver `201189f871d23597b82caf88fd611454a2ddefb8` (the revision
already cited in the repository). Fetch route: immutable raw GitHub files;
no donor checkout, tool installation, or Fandom HTML scraping.

| Donor path | SHA-256 of inspected bytes |
| --- | --- |
| data/scripts/discovery_system.lua | db42b499100a096954251f3796493702689800e84efa921a2625ab5a5e8ba11b |
| data/scripts/discovery_geometry.lua | b64960ec0c270591c5605791d525eb358368c2c8cb2203c0cea764641aa9becd |
| src/creatures/players/player.cpp | 1252ef7d27c7f77f5aaaadffdf9f85bb0820bb53beb67f2d900a42dcb187dd85 |
| src/creatures/players/player.hpp | ec85e04043712d09731a86a3caba931a2cea938b6b6cc5d8ac33f9ed37302651 |
| src/creatures/players/cyclopedia/player_cyclopedia.cpp | 347888b32d67d5d8a004314c4989bbf4261b1e5d9a332d320a736ab6d266e7c9 |

## Extracted facts

| Fact | Donor file:line |
| --- | --- |
| Viewport half-extents 7/5, seven viewpoints, minimum same-floor spacing four tiles | data/scripts/discovery_system.lua:16-19 |
| Geometry revision invalidates unfinished viewpoints; completed subareas survive | data/scripts/discovery_system.lua:20-22 |
| Main area IDs and exact surface rectangles derive from the client masks | data/scripts/discovery_system.lua:29-36,149-167; data/scripts/discovery_geometry.lua:1-5 |
| Underground boxes are approximate and lower priority than exact footprints | data/scripts/discovery_system.lua:169-184 |
| Donation goal is 10,000,000 gp, boost duration one server-save day, next save activation | data/scripts/discovery_system.lua:185-193 |
| Donations remain allowed during a boost and add to the next scheduled pool | data/scripts/discovery_system.lua:244-263 |
| Each save subtracts exactly one goal; excess survives; consecutive boosts allowed; no random winner | data/scripts/discovery_system.lua:269-280 |
| Improved-area footprints halve spawn delay | data/scripts/discovery_system.lua:190,226-238 |
| POIs must be walkable and outside current viewport; spacing relaxes after 600 tries | data/scripts/discovery_system.lua:468-483 |
| Area completion fraction counts completed subareas within the main area | data/scripts/discovery_system.lua:491-501 |
| Completing a subarea records permanent done state, clears POIs, sends state and milestones | data/scripts/discovery_system.lua:605-615 |
| Base movement speed uses vocation base plus level minus one, without discovery contribution | src/creatures/players/player.cpp:7222-7231 |

## Decisions

Follow the donor donation lifecycle, removing the Oteryn-only random draw and
previous-winner exclusion. Keep excess donations after a funded activation;
accept donations during the active day. The production world epoch preparation
and fenced command writer remain the callers.

Canonical geometry should reuse the admitted native bundle subarea mask table
(`crates/world-bundle/src/subarea.rs:60-89`), rather than introducing another
coordinate source or treating donor approximate underground boxes as exact masks.
Legacy authored area ID 1 is a progress identity, not the canonical main area ID 1.
Conversion policy: retain its source row; credit completed legacy discovery to all
canonical subareas (conservative grandfathering, never revoke a completed state).
Retain unfinished viewpoints and their found bits in canonical subareas containing
each point; do not reinterpret a legacy point outside admitted geometry as native.
This conversion is an Oteryn compatibility decision, not a donor gameplay fact.

## Speed contribution: stopped source-dependent slice

1. Required decision: the exact discovery-speed formula and stacking with existing speed sources.
2. Pinned Crystal completion and base-speed callers above supply no discovery-speed contribution.
3. The permitted Fandom MediaWiki API returned HTTP 402 to the lead's source request; no bypass or HTML fallback was attempted.
4. No numeric bonus is invented from the existence of a speed-bonus family.
5. Supply an accessible page+revid or approve an explicit formula; independent donation and client work continues.

## Additional speed cross-check

Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` was also inspected:
`src/creatures/players/player.cpp:7331-7339` computes vocation/level base speed,
and `src/creatures/players/cyclopedia/player_cyclopedia.cpp:1-254` supplies no
discovery speed formula. Their SHA-256 hashes are respectively
`7c572074fd7625f6f3e89626ae4dda6b7acc9296896d7c9521db9becfe14aa4c` and
`90b70aad7323508b4c5658ef30b698d33ebe83d637548f7c828b40382276a467`.
These are negative findings, not permission to infer a numeric bonus.

## Implemented compatibility and admission boundaries

- Native IDs, memberships and geometry come exclusively from the admitted bundle's
  subarea table. Missing coverage withholds discovery; the entry square is no longer
  a production fallback. The legacy request's area `1` remains the existing current-area
  wire alias; the server resolves its canonical main/subarea from the actor's position.
- The retained `BasicArea` JSON supplies sampling bounds and legacy content-digest
  compatibility only. Its authored area/subarea/name/radius do not define native membership.
  The existing bounded 128-candidate sampling and exact-tile finding remain Oteryn rules;
  this change does not claim full donor POI selection/finding parity. Crystal finds within
  the same-floor viewport (`discovery_system.lua:639-646`), a distinct remaining difference.
- Canonical state is stored per subarea, and the main-area percentage counts completed
  canonical subareas over that main area's total. The original legacy progress is retained
  under `legacy`; its completed flag credits every canonical subarea, including newly
  admitted ones. Geometry changes clear unfinished canonical POIs while preserving completed
  subareas and the exact legacy source, following the donor's revision rule at lines20-22.
- Migration0963 retains old World pools, epoch selections and receipts as source-only
  area `0`. It never reinterprets old authored ID1 as canonical main ID1. The reviewed
  bundle entry's canonical parent imports the historical pool exactly once, enforced by
  a unique partial index; source0 cannot be spent through the native caller and remains
  immutable evidence. An already-selected legacy day is copied without charging another
  goal. Subsequent canonical epochs subtract one goal and preserve excess.
- Pools, epoch receipts and donation receipts carry canonical main-area IDs. A boost
  affects only admitted normal-creature spawn positions resolving to that main area.
  Existing typed fee audit world/occurrence binding remains; valid native area IDs replace
  the previous hard-coded1 restriction. Client data cannot select these native bindings.
- The server samples/revalidates actor position and current runtime owner before the
  fenced durable writer. Runtime guards are dropped before database awaits. The respawn
  owner rechecks its scope/generation after durable preparation before applying boosts.

## Regression evidence

RED commands (with the prepared PostgreSQL environment sourced):
`cargo test --locked -p oteryn-gameplay-dispatch cyclopedia_postgres_real_dispatch -- --nocapture`
failed the executed production-path excess assertion (0 versus5,000,000) after49.22s.
`cargo test --locked -p oteryn-gameplay-dispatch --lib cyclopedia_postgres_canonical_conversion -- --nocapture`
failed the executed retained-legacy assertion (`Null` versus`true`) after52.33s.
Both logs contained `POSTGRES_STARTED`; setup/compiler failures were not counted as RED.
The later carry fixture uses25,000,000 to also exercise two consecutive funded epochs.

Conversion is lazy at the fenced owner boundary: reads expose canonical credit
immediately, and the next advancing command persists the canonical envelope without
minting a Character revision merely for a query. The original stored legacy row is
therefore also readable before that first write. PostgreSQL guards independently
reject regression/removal of completed canonical states and regression of the
retained legacy completed flag; test savepoints assert their exact refusal messages.
A second main area's attempted legacy import is rejected by the unique World import
index, independently of which Channel attempts it.

Final focused GREEN (prepared PostgreSQL environment sourced):
`cargo test --locked -p oteryn-gameplay-dispatch --lib cyclopedia_postgres_ -- --nocapture`
completed with **2 passed, 0 failed in55.47s**; Cargo exited0. The output included
both the existing production-dispatch `POSTGRES_STARTED` marker and
`POSTGRES_STARTED WORLD-SRC W12 canonical_conversion`. These were actual database
cases, not route-less skips. The tests exercise canonical mask membership,
seven-candidate admission, retained completed discovery, geometry invalidation,
legacy pool/current-day preservation, duplicate-import exclusion, monotonic SQL
guards, carry, consecutive funded epochs, active-day donations, replay and stale
fences, and the live normal-creature respawn factor.

## Continuation on main1fa04bbb

Main already implements donation carry and active-day contributions. This patch retains
its `select_epoch` owner and receipt assertions while binding pools to canonical main areas.
The fresh two-tree/63-file investigation is recorded in `map-speed-continuation.md`;
the exact permitted Wiki API URL and its HTTP402 refusal are in `map-speed-api.json`.
No numeric discovery movement-speed rule, units or stacking stage was established.

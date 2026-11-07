# E2EF4 authoring status: PARTIAL

`full_e2ef4_mount.rs` registers `full_e2ef4.rs` inside the production-bundle
world journey, before the existing root-only profile provisioning. Neither
peer is provisioned by this phase. Commands use native session/runtime codecs,
authored map routes, and server-issued item/entity/party/stash handles. No SQL
writes, privileged setup, synthetic domain frames or clock overrides are used.

This is **not** completion of Task E2EF4. Every row still has uncovered actions.
No full row can currently PASS. The ledger emits an explicit check for every
required action; uncompleted assertions are FAIL, including when an earlier
command failed. Each row emits exactly one report-compatible `<row>.full`
result if reached. Individual row waits are bounded to 90 seconds. An error
before reaching the phase leaves the report's ordinary absent-evidence FAIL.

All journeys below live in `full_e2ef4.rs`:

| Row / function | Implemented native checks or attempts | Remaining assertions | Expected full result |
| --- | --- | --- | --- |
| E01 / `shop` | Route to authored Sam; greet/open conversation; trade window; offer LOOK text; cheapest quoted BUY attempt | Inventory/currency BUY delta, SELL, SELL_ALL, banker deposit/withdraw/transfer and inventory/bank relog | FAIL |
| E03 / `coins` | Resolve native definitions 3031/3035/3043/23722/25719/28525/28526; plain USE of carried matches; report missing items and actual dispositions | Four quantity-preserving conversion directions; targeted converter charge burn/retirement; magic toggle/tick; relog coins/charges | FAIL |
| E04 / `trade` | Native cancel probe; if an ordinary carried leaf exists, real peer offer, offered-leaf LOOK, active cancellation | Both accepts/completion, concurrent logout race, exact custody after fresh admission | FAIL |
| E05 / `market` | Route to real locker; browse/history queries; create buy offer; verify own offer ID/definition/amount/price; cancel and verify removal | Cross-player settlement; history contents; fresh-admission settlement and balances | FAIL |
| E07 / `storage` | Locker/depot browse; native stash stow; exact count delta; withdraw chosen one unit from a stack of at least two; exact inventory/stash delta; fresh admission compares inventory/stash quantities | Depot custody moves; inbox retrieval. Depot box 17 is deliberately not called an inbox | FAIL |
| S01 / `party` | Two-session invite/current invitation/join; two members; `/sharedxp on` and current owner flag; pass leadership and check leader; leave/current membership | Eligible real kill and shared XP split (a toggle alone cannot qualify it) | FAIL |
| S03 / `guild` | Native own-guild query; exact missing MOTD intent citation | Membership/ranks/chat/bank/war and guild/bank reconnect assertions. The current native `GuildIntent` enum has no MOTD write operation | MISSING-SERVER, source `crates/protocol-oteryn/src/guild.rs:17` |
| S09 / `logout` | Peer on authored PZ; native accepted logout; fresh session ID and saved position; cleanup | In-fight refusal; authored NOLOGOUT; real 15/16-minute warning/removal; gameplay idle reset | FAIL |
| S18 / `speech` | Native say/whisper/yell; accepted modes require exact mode/text local-line projection; repeated real speech drives mute and checks positive wait | Peer range/obscured whisper; complete level/premium yell gate matrix | FAIL |

FAIL here includes missing journey implementation, not just absent server or
content. Only S03 names a concrete unavailable native write operation. An empty
inventory, missing funds, a refusal, a query or an unrelated legacy slice never
qualifies a missing action. `E2EF4_GAP` preserves the actual error and
`E2EF4_CHECK` preserves completed and uncompleted assertions alongside the
standard row result.

Validation during authoring: use the repository cargo environment; run
`cargo fmt --all --check`, explicit rustfmt checking for `full_e2ef4.rs`
(expression includes are not traversed by cargo fmt), game-server all-target
clippy with `-D warnings`, game-server lib tests filtered by `qualification`,
and Python unittest discovery in `tools/qualification/e2e_all`. Node boot was
attempted and returned `BLOCKED reason=exact_platform_checkout_missing`.
Full pinned Platform/privileged topology execution remains for CI; local Rust
and Python checks do not count as real-server full-row evidence.

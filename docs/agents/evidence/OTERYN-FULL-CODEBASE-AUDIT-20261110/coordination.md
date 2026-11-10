# Baseline and concurrent work

Audit baseline: `e2e5ced306040113dc9c395251a802f50eb5776a`; observed 2026-11-10 09:11:53 UTC. Fresh main read at 09:35:21 UTC returned the same SHA.

The following PR heads were freshly inventoried at audit admission. They are concurrent work, not part of the audited baseline and not evidence that a finding is fixed. Their validation claims were not adopted as this audit's test results.

| PR | Title | Observed head |
|---|---|---|
| [#3076](https://github.com/Oteryn/Oteryn-Game/pull/3076) | fix: node boot repair package (busy-holder admission, cast observer, town-entry ordering, PG partition weights) | `ea6a0113e094f6e07802dfd93ec46b0645d29728` |
| [#3075](https://github.com/Oteryn/Oteryn-Game/pull/3075) | Client parity package 3: Tibia/OTClient audit gaps (T060, T170 and following lanes) | `09a3b13f5170593300dfb6ffe1a583c51e0e937d` |
| [#3062](https://github.com/Oteryn/Oteryn-Game/pull/3062) | ci(deps): bump the codeql-action group with 2 updates | `0bea9b73c103610e7103f117e48399ff5da0b8fa` |
| [#3056](https://github.com/Oteryn/Oteryn-Game/pull/3056) | QMECH: missing quest mechanics package | `314c2ef4c4c6f86de2accd1f095260715c5a4172` |
| [#3042](https://github.com/Oteryn/Oteryn-Game/pull/3042) | WPKG round 2: collected quest lane batches (migrations 1300-1307) | `35fa204e22ec9edd010bd7185aa4bb850dcd540f` |
| [#3023](https://github.com/Oteryn/Oteryn-Game/pull/3023) | E2E rows: full node-boot journeys for the 79 open SCENARIO_MATRIX rows | `ad336e8fe36750de084b8e47df6a77b72241165b` |

Quest/dispatch/durability, node boot, client parity and E2E qualification are actively changing according to these open PRs. Stability of other subsystems was not established merely because this list lacks a PR for them.

Historical audits #359/#364/#626 and donor corpus #710 were comparison sources, not fresh proof. Current-main findings were rechecked in source. Targeted issue searches for the magic-bonus rejection, nested-death/respawn and V2-envelope problems found this programme or broad prior corpus/control records, not an identified duplicate report of the same current root cause. This is a bounded search result, not an exhaustive duplicate-free guarantee.

The initial session/protocol worker stopped with an authentication error requiring an authorized hardware key. The incomplete final report was not assumed to exist. Earlier supplied observations were independently checked by other auditors before use; only the explicit lead ledger contributes its subsequently reviewed paths.

Documentation has a single root writer and is restricted to this directory on a separate audit branch. Local Git authentication for push was unavailable. The active audit control record selected the bound META policy's API-native new-candidate route for this documentation only: exact predecessor, one successor commit/tree, nonforce expected-head update, then fresh remote readback. No selected/frozen source candidate was reconstructed or represented as preserved.

## Main advanced during publication preparation

At 09:50:51 UTC, main was `4f0b6c62a789489fca6c37dd36d5917d096f328e`, after #3089 and #3075. The table above is the admission-time PR inventory; #3075 is now merged. The audit baseline is unchanged. See [latest-main-reconciliation.md](latest-main-reconciliation.md) for the bounded finding-specific check and the explicit unaudited successor delta.

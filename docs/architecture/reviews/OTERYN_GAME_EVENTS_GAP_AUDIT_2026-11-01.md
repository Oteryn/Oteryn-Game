# Oteryn recurring events gap audit — 2026-11-01

## Audit checkpoint

This draft is being researched. It does not yet certify an exhaustive event census or completed gameplay coverage.

The sole owned repository path is this document. No game code or content data is changed. The existing Crystal re-audit is a read-only input.

## Evidence baseline

- Oteryn main: `38641fecd4558a4a14e6b15b3874110ffa2dff19`.
- Crystal primary donor: `201189f871d23597b82caf88fd611454a2ddefb8`.
- Canary secondary donor: `52550d5e3c0ffe565f4bd1014f4e5575abf1059b`.
- Overlapping work: [draft PR #2781](https://github.com/Oteryn/Oteryn-Game/pull/2781), head `4c33b1409af987aba430bcd3e466c16fa9ca1f0e`; its body, published changes and owner answers are checked separately from main.
- Read-only audit input: [Crystal re-audit](OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md), especially W05a–W05e, W16, W19 and explicit owner exclusions.
- Wiki intake: MediaWiki API only, honest `Oteryn-Events-Gap-Audit/1.0` User-Agent, serialized requests at least 1.1 seconds apart, retained revision IDs.
- Initial API evidence: [World Changes, oldid 404839](https://www.tibiawiki.com.br/index.php?title=World_Changes&oldid=404839) and [Mini World Changes, oldid 436926](https://www.tibiawiki.com.br/index.php?title=Mini_World_Changes&oldid=436926).

## Method and acceptance

Daily/weekly changes, seasonal/annual events, and bonuses/recurring rotations are researched in parallel. The code pass traces registrations and production callers in the pinned donors and Oteryn. A keyword hit, content identity, test-only helper, or PR description is not proof of a playable implementation.

`DONE` will mean a verified, reachable implementation for the stated row, with no established gap in that bounded rule. `PARTIAL` will mean a reachable subset with a specific missing rule or content/activation dependency. `MISSING` will mean no qualifying implementation was found in the inspected production owners. These are static audit statuses, not a claim that gameplay tests were executed.

Unretrieved or unsupported facts are explicitly `unverified`. Donor behavior does not prove current Tibia behavior. Work reported by #2781 remains in progress until its published head and activation prerequisites establish otherwise.

## Planned deliverable

1. One row per event with kind, current/removed state, schedule/trigger, donor and Oteryn file:line evidence, #2781 overlap, status, precise gap, and wiki revision or official news ID.
2. Reconciled counts by kind and status, with historical and unverified entries identified.
3. Proposed future pull requests grouped by mechanics, sized S/M/L, with canonical map/NPC/monster/item dependencies and donor availability.
4. Owner questions for removed, commercial and optional events; each unresolved decision is posted as a separate `QUESTION:` comment on this audit PR.

## Left out

Research is in progress. No final completeness or coverage counts are asserted at this checkpoint.

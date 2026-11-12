# C14 skill progress v1

Rebased onto `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`. Existing native
routes and capabilities 286/287 are retained; the five new shortcuts use
Ctrl+Alt+Shift plus C/R/B/K/P respectively to avoid the base bindings.

Capability 279 (`SKILL_PROGRESS_V1`), command 495 (`SKILL_PROGRESS_QUERY`).
Only the admitted Character's existing session/content-fenced build is read. No game rule,
training counter, loyalty value, CharacterRevision or migration is changed.

## Request

Empty payload. Any byte is refused. Offered in the bundle world only when its native
training formula is present. Unselected capabilities and stale connection/Character lease
fences are refused by the existing command and Character-build owners.

## Reply

Exactly 160 bytes: eight 20-byte records, in order magic, fist, club, sword, axe, distance,
shielding, fishing. Each record is little-endian u16 trained level, u64 trained progress,
u64 requirement to advance to the next level, u16 hundredths of a percent (0..10000).
A zero requirement identifies an unreachable next level and gives zero percent.
The own Character's current native formula supplies `required(vocation,family,level+1)`.
Percent is rounded to the nearest hundredth, using bounded integer arithmetic; values above
100% give zero as in the donor. Loyalty does not alter these trained counters.
Malformed length or out-of-range percent is refused; client failure preserves its last
confirmed snapshot, permits retry and clears the snapshot at session end.

## Parity and evidence

Donor: Crystal `zimbadev/crystalserver` branch `main`, pinned
`201189f871d23597b82caf88fd611454a2ddefb8` (OTS_HYPOTHESIS_ONLY, selected donor per GAP-SV7).

| Rule | Donor / accepted difference |
|---|---|
| Include magic and every seven ordinary skills, hundredths of percent | `src/server/network/protocol/protocolgame.cpp:9380-9403` |
| Current-level training remainder / next-level requirement | `src/creatures/players/player.cpp:1149`, `:3566-3570` |
| Round to nearest hundredth; zero denominator or result above 100 gives zero | `src/creatures/players/player.cpp:3799-3809` |
| Separate trained progress from loyalty level | `src/server/network/protocol/protocolgame.cpp:9391-9402` |
| Add raw counters and explicit next-level requirement | GAP-SV7 accepted wire additions: Oteryn uses its own bounded query rather than donor opcodes; the values already exist in the Character build |
| Native progress display | GAP-SV7 owner requirement; bounded textual ten-cell bars with exact donor hundredths and counters |

Current-main gap: `crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:74-104`
reads build progress but emits only base and loyalty-adjusted levels;
`crates/protocol-oteryn/src/loyalty_skills.rs:13-29` has no progress field;
`crates/client-ui/src/features/0274-loyalty-skills/view.rs:37-41` displays only levels.

# WEVENTS — timed World event qualification evidence

Task branch: `claude/happy-davinci-ijz3wy-wevents`. Initial remote head:
`3722c495cb866d83b9cb6835b1bb5ba51f757267`. The owner authorizes direct
fast-forward batch pushes and conservative documented rules where source facts
cannot be recovered. This record does not replace other lanes' WORLDCHG-Q rows.

## Shared owner foundation

`world_event::changes::timed` extends the existing reset owner. Migration 1120
stores bounded event state, per-Character participation/cooldowns and immutable
input-bound retry receipts. Current epoch admission precedes every state read or
write; the existing raid scheduler polls after node, Channel generation and map
binding validation. Seasonal cycles survive daily resets. Daily cycles use the
reset epoch as identity, including when activation timestamps happen to be equal.

`interact` joins a caller-owned transaction and rechecks recovery, session,
Character revision, lease, node and scope generation. Its result is pending until
that transaction commits. Object custody, reward delivery and map effects must
join their existing owners; a stored counter is not a physical spawn or item grant.
No transport accepts client-supplied time, random draws or kill attribution.

Flower Month's source is Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/raids/world_changes/flower_month.lua:1-65`, SHA256
`dc1607c195712ed166dcda95423fba91821d0ea1b22b85a249fc9aab8d74b1bb`.
The donor's four areas rotate indefinitely, 25 Dryads per wave with 250 placement
attempts, every 900000 ms during June. This wins over the wiki five-wave account
by the owner's explicit decision. The shared clock uses June 1 inclusive to
July 1 exclusive in UTC; missed slots are not replayed as a burst after downtime.
UTC synchronization and calendar-end closure are explicit implementation rules.

Mummy source: the same Crystal revision,
`data-global/scripts/actions/worldchanges/the_mummys_curse/horestis_jars.lua:18-122`,
SHA256 `6e47be40083c0ff174c454940382f891f96d7c304c999457e5165ab98d24751a`.
AIDs 50006–50009 advance in order with 1/10 success and 30-minute failed-attempt
cooldown; 50010 completes the sequence without another draw. Lines 105–106
contain two spawn calls; the owner's explicit **single boss** instruction removes
the duplicate admission. Line 108 restores item 1603 after **277000 ms**;
lines 109–112 restore item 3514 after **1200000 ms**. Those executable delays
win over the script's inaccurate announcement. Daily state resets with the epoch.

## Validation record

The initial focused rules test failed with E0432 (missing `timed`) before the
implementation, captured in `/tmp/wevents-generic-red.log`.
The real PostgreSQL case uses the runtime role, production `poll`/`interact`,
transaction rollback, exact receipt replay, conflicting input, stale session and
epoch denial, four-area rotation, seasonal failure, single boss admission, both
wall deadlines, and equal-timestamp daily reset separation. It prints
`WEVENTS PG EXECUTED` and an elapsed duration only after successful execution.
Session-local logs are `/tmp/wevents-generic-*.log`.

Foundation validation: focused rules **1 passed**; focused PostgreSQL **1 passed**,
**5.22 s**, marker duration **5.210922522 s**. `cargo check --locked` passed for
world-change, durability and game-server. `cargo clippy --locked --all-targets
-- -D warnings` passed for all three changed crates. `cargo fmt --all --check`,
explicit mounted-test rustfmt and `git diff --check` passed. Tool-generated
`pin-refresh` and `pin-check` both passed, with unchanged inputs
`ebae5fa901f10f1711a101fc00c19584b7704e3beccf626c8afa4c1b2b1b8f87`
and World digest
`3af394d0f4e68a8485d4655e3109ceaf867320c5ca053290a29e8d297faf7afd`
(31101773 bytes). No full nextest or full PostgreSQL suite ran. Native title
binding qualification is recorded in the individual WORLDCHG-Q rows as it lands.

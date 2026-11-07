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
by the owner's explicit decision. Missed interval slots are not replayed as a burst after downtime.
This foundation rule is superseded by the native binding below: the committed June
reset snapshot starts an indefinite cycle that lasts through the next Berlin
06:00 server save, including the June 30 overnight tail. This follows donor startup
month admission and its unbounded scheduleWave recursion.

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

## Native binding batch: Flower Month, Mummy’s Curse, Chyllfroest

Reused qengine2 `5a6de28c142de185594217ce1e5cbc7f4b6e6e4e` through merge
`89ac1cece`: generic reward exchanges, encounter admission and Channel projections.
The startup selector generator verifies Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/startup/tables/item.lua:3087-3126`, SHA256
`677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd`.
It restores metadata at eleven bridge grounds and five ornate jars without changing
map binaries. `native_event_bindings.py --check --map-check` verifies all sixteen
selectors against both the retained donor and current native B3 placements.

Native map reconciliation: the current B3 tomb places item 1603 at all four
source wall coordinates; the donor removes 3514 and creates 1603 later. The
projection owns both source wall appearances at those exact coordinates, hides
preplaced 1603 during the open interval, and restores the donor delays. This
explicit map/source conflict is resolved in favor of the executable event cycle.

Native binding slice validation before publication: focused rules 4 passed;
native map tests 3 passed (0.01 s); catalogue generator 15 passed (14.991 s).
Real owner PostgreSQL: 2 tests passed, 23.44 s, with execution markers for
Chyllfroest 7.467046836 s, Flower 8.323957080 s, Mummy 7.641205679 s and the
foundation 7.518263334 s. The Mummy case also holds an uncommitted first state
insert while reconciliation waits, then verifies the committed receipt.
Real physical actor PostgreSQL: 2 passed, 70.73 s, with markers
`wevents_flower_map` 70.683130456 s (25 Dryads) and `wevents_mummy_map`
70.727587693 s (one Horestis). No suite-wide PG run was used.
Nine affected/reused crates passed `cargo check --locked --all-features`
(45.52 s). Formatting and explicit mounted-test formatting passed. Pin refresh
and pin check passed: inputs `bdf720e5cce527a9ca45b09d196d878ec12a1a6de47a3d28ddaaf6a7400f7726`,
World `d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`,
31101950 bytes. Strict clippy/content checks continue after this immediate
owner-requested publication; their final result will be recorded separately.

Qualification follow-up for the first three rows: all nine production libraries
passed strict `cargo clippy --locked --all-features --lib -- -D warnings`
(3m52s). The earlier broader all-target attempt exhausted the workspace disk;
unused compiler caches were removed and the required changed-library check
completed. Focused content checks passed: 123 selected integration content tests
(4.77 s) and 5 World-package tests (76.65 s). These are scoped content targets,
not a workspace test run. No full nextest or full PG suite was run.

Spring native membership scan found exactly six current B3 item14098 dragon
nests. Candidate coordinates and region SHA256 values are retained in the
session scratch record `/tmp/wevents-native-spring-nests.json`; authored seasonal
policy will bind those existing objects rather than infer empty map positions.

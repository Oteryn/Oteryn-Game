# Encounter rules package audit

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: REVIEWED — all21 tracked package files,4945lines, including full source/tests/manifest. Exact hashes and ranges in ledger.json. No production changes.

## Entrypoints and invariants

Boss participation preserves dealt damage while applying authored bonus resilience only to received damage/healing. The retained10percent-of-fair-share gate is a documented Oteryn anti-leech choice, separate from donor loot factors; support without damage is excluded. Frozen census callbacks cannot add a recipient. The actual dispatch caller bounds/deduplicates contributor identities and lease generations before evaluating, and the durable validator recomputes the exact expected eligible set and checks chest identity/contribution against it. No all-u64 or unbounded-census synthetic input was promoted to a reachable reward exploit.

Personal chest operations derive checked seven-day expiry, test expiry before moving anything, and refuse insufficient full capacity without mutation. Prefix collection selects a count that already fits, so its internal split cannot lose a remainder through a later capacity failure. The production consumer reads current bag/tree room, item stacks, carried weight and capacity before passing a prefix; durable item/mint/replay owners remain separate scope. Prior DUR-001 carried-weight issue is not duplicated. Boss-room policy validates count, unique characters and lever cells, level/cooldown and representable deadlines; the actual transaction re-reads pinned definition/progression/cooldown facts and destination mapping before planning.

Self-heal qualification follows actual loaded source membership through Creature, Behavior.defenses, Ability, native Heal Effect and Formula, with only the closed selfcast/paralyze-removal shape admitted. Exact numeric schedule and health/magnitude facts remain server-authored. The runtime adapter resolves actual defense entries and compares the schedule before registration. A hash alone cannot create membership. Checked/saturating deadline helpers are intentionally separate; health threshold equality does not trigger a strict-below heal.

Hazard rules constrain production levels1..12 and exact source coordinate frame/legacy positive floors. Incoming damage rounds each positive bonus upward, experience floors, critical is applied before ordinary boost and loot uses extra independent rolls. The actual combat adapter requires current player frame/zone, group minimum for present party members and a current nonsummoned monster in the zone. Arbitrary fixture-area definitions are test-only; provisional donor tuning is not represented as verified Global parity.

Raid schedule state is metadata rather than admission authority. Definitions cap total requested population500, waves50, announcements16 and1024bytes. Each due check advances once with no catch-up burst; blocked fixed slots are consumed, native persistent raids remain once per reset epoch, reset/retire cancel running metadata and min-gap history survives. SourceScheduled consumes the source planner's result without a second random draw. Lua preserves the donor ordering: final-hour/running/gap/check-budget checks, then random chance, then deferred weekday/player gates. A winning pending start is retained rather than rerolled. The source's checks-today counter is intentionally not reset on day changes; tests explicitly preserve that behavior.

The catalogue rejects unbound active records, validates every variant at maximum population and admits only supported schedule/placement shapes. Coordinates are bounded to the source frame, region references have required identity/hash shape, and the real map admission owner still proves placement. Variant/amount/coordinate draws retain firing identity and independent domains. Single spawn attempts once, ordinary area attempts at most10, specialized shared placement250. Unknown or blocked/PZ/house/stairs/occupied tiles cannot be selected as admitted placements.

RaidRunCursor orders equal-deadline announcements, waves and completion deterministically, marks the actual last executed wave and proposes only the next uncommitted action. The actual durable run writer checks current ownership, locks firing/target/run rows, excludes channels not targeted at firing, marks stale generations LOST, returns pending action verbatim and atomically writes effect plus cursor. The live loop retains bounded retries and checks owner fencing. Full physical realization/acknowledgment owners are not credited as completely reviewed here.

## Refuted hypotheses, findings and evidence

- Partial reward collection cannot lose its split remainder on the same deterministic capacity check: count was capped to the available inventory slots first, and time is unchanged within the call.
- A new generation does not use the pure runner cursor to replay an older raid: durable ownership logic loses that run before cursor advancement.
- Post-spawn broadcasts are retained for persistent raids; completion uses the later final announcement.
- Zero rolls and invented daily Lua resets are not valid production assumptions; draw domain and donor-counter semantics are explicit.
- Publicly derivable raid schedule roots match the accepted seed construction and the already-known RNG review class; no duplicate finding filed.

No new confirmed finding. All package tests were analyzed but not executed; no Cargo, PostgreSQL, native server or source parity execution is claimed. Remaining package files: none. Source schedules, physical placement, reward custody and SQL fence implementations beyond recorded ranges remain separately auditable owners.

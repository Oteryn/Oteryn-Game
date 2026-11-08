# qready1b validation continuation

Decision (2026-11-08): preserve PR 3037's frozen qready1 head
`ad3a63611138b8949ea6303cb2798d9a0bf5123a`. Continue pending validation on
`claude/happy-davinci-ijz3wy-qready1b`, as instructed for work after freeze.
Do not adopt additional quests or change another lane's content.

At continuation start, the matrix had `REWARD_CLAIM_USE_ENABLED` for all 17 owned quests.
Beggar Outfits Quest and Druid Outfits Quest are `SKIP-OWNED qengine2` under
the coordinator's confirmed allocation; their pending states are not qready1
implementation entries.

Decision: finish the missing evidence registration as well. Reuse the existing
reward USE proof schema and loader, with a qready1-only log importer because its
markers and multi-claim test cases differ from qready2. Generate proof hashes
from the fresh successful PG run and regenerate the completion matrix to
`REWARD_CLAIM_USE_VERIFIED`; do not hand-edit proof hashes or matrix states.
This is authoring tooling only, with no new shared runtime feature.

Remaining validation entries:

- Dispatch crate unit tests, with all features: pending.
- Durability crate unit tests: pending.
- Post-merge qready1 PostgreSQL cases: complete, 17/17 passed in 856.02 seconds,
  with all 17 successful completion markers from the fresh all-features build.
- Combined qready2 PostgreSQL cases against the same fresh build: pending.
- Register the fresh qready1 proof and regenerate its matrix states: complete,
  23 claim keys across 24 placements; all 17 owned quests are now
  `REWARD_CLAIM_USE_VERIFIED`.
- Full quest-authoring Python unit suite: running; focused proof/matrix tests
  passed 17/17 in 1.487 seconds.

The fresh runtime build is from frozen `ad3a63611`; this continuation changes
only authoring tooling, generated proof/matrix data, and this record. All-features
clippy for dispatch, durability, and client-assets passed in 496.045 seconds.
Workspace check passed in 4.406 seconds and formatting in 28.875 seconds.

The strict existing matrix test exposed a producer bug when the last enabled
quest became verified: `Counter` dropped the zero-count enabled bucket. Keep
the enabled/verified summary buckets at zero in the producer and regenerate;
the original assertions remain unchanged. Focused tests were red before this
fix and green after it.

Only qready1's proof packet is authored here. The qready2 cases are revalidated
because their work is already in this combined branch; their existing proof
packets remain untouched while their source and claim hashes remain current.

Use the real PostgreSQL harness only for the targeted PG cases. Run ordinary
crate unit tests without the PG route, so they do not start the full PG suite;
route-less early returns do not count as PostgreSQL proof. Keep full command
logs outside Git and record counts, completion markers, and durations here.

## Owned quest results

| Quest | Status | Claim keys | PG seconds |
| --- | --- | ---: | ---: |
| Adorned UH Rune Quest | VERIFIED | 1 | 58.841 |
| Barbarian Axe Quest | VERIFIED | 1 | 97.053 |
| Battle Axe Quest | VERIFIED | 1 | 43.712 |
| Blood Herb Quest | VERIFIED | 1 | 57.897 |
| Captain Iglues Treasure Quest | VERIFIED | 1 | 52.954 |
| Circle Room Quest | VERIFIED | 2 | 121.786 |
| Combat Knife Quest | VERIFIED | 1 | 43.169 |
| Crusader Helmet Quest | VERIFIED | 1 | 59.027 |
| Crystal Wand Quest | VERIFIED | 2 | 119.504 |
| Darashia Dragon Quest | VERIFIED | 1 | 69.311 |
| Dark Armor Quest | VERIFIED | 1 | 102.994 |
| Demona Ring Quest | VERIFIED | 2 | 172.366 |
| Double Hero Quest | VERIFIED | 2 | 165.822 |
| Doublet Quest | VERIFIED | 1 | 110.477 |
| Elephant Tusk Quest | VERIFIED | 1 | 67.528 |
| Explorer Brooch Quest | VERIFIED | 1 | 73.113 |
| Griffin Shield Quest | VERIFIED | 3 | 173.617 |

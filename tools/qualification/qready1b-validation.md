# qready1b validation continuation

Decision (2026-11-08): preserve PR 3037's frozen qready1 head
`ad3a63611138b8949ea6303cb2798d9a0bf5123a`. Continue pending validation on
`claude/happy-davinci-ijz3wy-qready1b`, as instructed for work after freeze.
Do not adopt additional quests or change another lane's content.

The owner subsequently reopened qready1 to repair PR 3037's main conflict.
Reuse this branch's proof work in that repair: main `98d6ae8ce` was merged
with regenerated world outputs in `942633128`, then main `47baa7bb8` was
merged in `07d1bc07a`. The latter contains the identical floor-transform
permit fix, so durability now has no delta against main. Its two provisioning
tests passed in 0.089 seconds.

The original checkout remains detached at `39a47f16e` so already-running
checks kept consistent inputs. Integration authoring and the reopened qready1
branch are in the separate `Oteryn-Game-qready1b` worktree. Results below are
labeled by snapshot; an earlier result is not represented as a merged-head run.

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

- Dispatch crate unit tests, with all features: merged-head run pending;
  the incomplete 39a run was superseded by the Soulpit runtime merge and is
  not counted as a completed suite.
- Durability crate unit tests: superseded by upstream reuse; no remaining
  durability changes against main `47baa7bb8`.
- Post-merge qready1 PostgreSQL cases: complete, 17/17 passed in 856.02 seconds,
  with all 17 successful completion markers from the fresh all-features build.
- Combined qready2 PostgreSQL cases on snapshot 39a: complete, 21/21 claim
  cases in 774.02 seconds and 3/3 Purple Tome cases in 108.73 seconds;
  all 24 completion markers printed.
- Register the fresh qready1 proof and regenerate its matrix states: complete,
  23 claim keys across 24 placements; all 17 owned quests are now
  `REWARD_CLAIM_USE_VERIFIED`.
- Full quest-authoring Python unit suite on snapshot 39a: 1,118 tests passed
  in 1,526.452 seconds (8 existing skips). Focused proof/matrix tests passed
  17/17 in 1.487 seconds. The merged-head suite also passed all 1,118 tests
  in 992.264 seconds (8 existing skips).

Main integration: all world-authoring job commands passed, including converter
freshness, validators, materialized game tree, world/base unit tests, startup
bindings, and qready2 binding fixtures. The new world compiler regenerated the
combined pin in 301.933 seconds. Migrations 1140-1141 came from main unchanged;
the migration-number duplicate check passed. Merged-head workspace check
passed in 913.438 seconds and formatting in 16.052 seconds.

The final merged-world pin check passed in 176.605 seconds. Merged-head
client-assets units passed 10/10 in 0.53 seconds. An incremental-cache cleanup
affected a clippy invocation; its cache-write errors are infrastructure errors,
not a passing lint result. The recovered lint invocation passed in 326.014
seconds against the unchanged source. No interrupted or partial suite is
counted as complete.

Publication checkpoint under PREPUSH-2's cadence exception: main `47baa7bb8`
is included; affected generators, world job, pin check, Python units, workspace
check, clippy, fmt, and client-assets units pass. The fresh dispatch test build
is running; merged-head dispatch units and PG refresh remain pending and will
continue after publication. Earlier PG measurements below retain their explicit
snapshot label.

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

## Owned quest results (snapshot 39a)

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

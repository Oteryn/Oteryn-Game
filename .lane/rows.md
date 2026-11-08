# ENGITEM active engine queue

Latest assigned queue: questpc2-n2-n3, questpc5-n2-n3 and questpc7-n3
`qlane-meta/*/ENGINE-REQUESTS.*`. No additional ENGITEM-tagged rows were found.
The five untagged questpc5 rows below are explicitly assigned to ENGITEM.
Earlier broad audit is preserved in `legacy-engine-requests.md`; its dependency
statuses are historical, not the active work queue.

ROWS: done 0 / open 5 / blocked 0

| Capability | Needed by | Status |
|---|---|---|
| Literal readable reward text | Nomads Land | OPEN |
| Exact nested native source leaf selection | Ornamented Shield | OPEN |
| Quest-guarded object taming | Targuna | OPEN |
| Recipe-scoped nondecay specimen | Explorer Society | OPEN |
| Cycle-scoped repeatable World item MINT | Oramond | OPEN |

## Dedup correction

Merged ENGNPC `3423ac56e` via merge commit `20d510b2f`. Removed duplicate
migration1640 and engitem_atomic_tracks.rs. QUESTSTATE0_RL_02 and capacity
writer paths come from ENGNPC. No changes to migrations1600–1639. Retained
ENGITEM quest clock and its separate PG case. ENGNPC atomic tests cover every
former ENGITEM atomic-track assertion.

Real PG rerun: quest_clock 3.544770921s; sixteen_tracks 4.565656988s;
reward_sources 38.303271647s. All passed with execution markers. Clock unit,
unique embedded migration test, duplicate-migration check, fmt, durability / quests /
game-server all-target clippy and architecture-check passed.

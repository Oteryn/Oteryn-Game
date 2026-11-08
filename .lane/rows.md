# ENGITEM active engine queue

Latest assigned queue: questpc2-n2-n3, questpc5-n2-n3 and questpc7-n3
`qlane-meta/*/ENGINE-REQUESTS.*`. No additional ENGITEM-tagged rows were found.
The five untagged questpc5 rows below are explicitly assigned to ENGITEM.
Earlier broad audit is preserved in `legacy-engine-requests.md`; its dependency
statuses are historical, not the active work queue.

ROWS: engine done 2 / open 3 / blocked 0

Counts describe engine capability, not whole-quest or Content qualification.

| Capability | Needed by | Status |
|---|---|---|
| Literal readable reward text | Nomads Land | DONE: optional bounded ItemReward.text for NPC/World; existing root_text MINT and exact source proof (migration1641). RED PG unknown text field (3.42s); GREEN PG marker43.93607223s, atomic text/source/MINT/progress, refusal and replay. Three Rust cases and authoring tests pass. |
| Exact nested native source leaf selection | Ornamented Shield | ENGINE DONE: optional UID/path/item/count selection after complete original bag and sibling admission; original claim/placement and once ledger retained. Rust, Python and real MINT refusal/replay/once fixture use qualified UID6063. Requested UID6062 remains Content-held: SOURCE_CHARGE_MISMATCH (siblings 6/7) and ITEM_SEMANTICS_MISSING (3085); its eight source rewards and holds are unchanged. |
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

Readable-text checks: cargo check and all-target clippy for durability, quests
and game-server; migration uniqueness/duplicate check; architecture-check; both
quest-authoring CI runners (103 unit modules), donor/binding/matrix and complete
content regeneration passed. Final publication also merges the fetched main
client-only update and regenerates its final outputs before push.

Post-main publication checks on ad493a1ee: three literal-text Rust tests and
real PG marker38.077736345s passed; focused check/clippy, fmt and architecture
passed. Final full/donor/binding/matrix regeneration left no generated diff;
pin-check and content-tree validation passed. The 2026-11-08 rule update now
forbids autonomous main merges; engine-only pushes use pin/lock verification,
with one Cargo job at a time. Every subsequent push records ls-remote evidence.

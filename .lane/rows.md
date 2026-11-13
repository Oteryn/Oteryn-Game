# ENGITEM active engine queue

Latest assigned queue: questpc2-n2-n3, questpc5-n2-n3 and questpc7-n3
`qlane-meta/*/ENGINE-REQUESTS.*`. No additional ENGITEM-tagged rows were found.
The five untagged questpc5 rows below are explicitly assigned to ENGITEM.
Earlier broad audit is preserved in `legacy-engine-requests.md`; its dependency
statuses are historical, not the active work queue.

ROWS: done 1 / open 1 / blocked 3

Owner dependencies count under blocked; their exact disposition is OWNED ELSEWHERE.

| Capability | Needed by | Status |
|---|---|---|
| Literal readable reward text | Nomads Land | DONE: optional bounded ItemReward.text for NPC/World; existing root_text MINT and exact source proof (migration1641). RED PG unknown text field (3.42s); GREEN PG marker43.93607223s, atomic text/source/MINT/progress, refusal and replay. Three Rust cases and authoring tests pass. |
| Exact nested native source leaf selection | Ornamented Shield | OWNED ELSEWHERE: Content/questpc5 source-binding authoring over L2's full bag/charge qualification. UID6062's canonical source contains eight ordered children; shield3424 is child4 (one-based). Existing BoundPlacement.contents and reward_claim_mint/contents.rs retain the entire ordered bag and same once-only claim. Selecting its reference is content qualification, not another engine grant. Main lacks L2's explicit root/child charge-plan fields (1219/charges.rs); do not strip siblings or substitute a new shield MINT. |
| Quest-guarded object taming | Targuna | OWNED ELSEWHERE: World/native targeted carried-tool admission first. Main dispatch/mount_taming.rs:180 resolves object targets ONLY from ItemKey::Instance in the carried backpack/nested tree; MapBase targets are discarded. The Ram53439 at31926/31896/z7 is a map object, not a carried Item or Creature. Existing MountTamingRequest's target_identity/generation does not supply an admitted map-object proof. Reuse the World USE_WITH_MAP_ITEM owner before composing quest progress; a carried-object fixture would not qualify this boundary. No fake Item instance or creature substitution. |
| Recipe-scoped nondecay specimen | Explorer Society | OWNED ELSEWHERE: QENGINE2/Content native first-party Item constructors (questpc7 request illuminated-first-party-constructors) and L2 duration qualification. L2 quest_decay::resolve explicitly rejects omitting decay for an existing timed native Item. A recipe-scoped permanent form needs a qualified distinct first-party identity/policy; current native reward resolution only admits oteryn:item.tibia.i*. Preserve4837's existing temporal policy and do not invent a donor ID or silently remove its timer. |
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

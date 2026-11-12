# NX-7 server projections and native displays

Base: `81504ec792c186a3adcdc95aef8610d0862334cf`.
The mechanics status MD/JSON and CLX-A report were checked against this exact tree.
The older CLX-A STOPPED descriptions miss the already present GAP-SV7 projections;
NX-7 retains those production callers and completes the remaining display gaps.
No ledger, merged migration, casting, quest, encounter-server, starting-equipment,
Dawnport, OT-only feature or Prey Hunting Task is changed.

## Parity and evidence

Donor class: `OTS_HYPOTHESIS_ONLY`. Revisions are Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` and Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446`, fetched read-only from GitHub.

| Row | Production path and native view | Donor evidence / decision |
| --- | --- | --- |
| C08 | Retain `condition_status.rs` (276/492), reading both condition owners under runtime/player locks; `0276-condition-status/view.rs` shows icons and rounded-up remaining seconds, with one-second native window refresh. | Crystal `src/creatures/players/player.cpp:778-791`, `src/server/network/protocol/protocolgame.cpp:5712-5733`; Canary same protocol path `:6257`. Existing `condition-status-v1.md` records the native duration extension and closed icon catalogue. No new condition rules. |
| C10 | Retain `rest_status.rs` (277/493), qualified PZ/streak/premium/food/deadline projection; `0277-rest-status/view.rs` shows regeneration state and food time, with one-second native window refresh. Soul counter remains in the main HUD. | Crystal protocol `:7801-7845`; Canary protocol `:8219`. Existing `rest-status-v1.md` records the accepted resting-area/PZ approximation. State means unlocked regeneration bonus tiers, not a prediction of credited resource points. |
| C11 | Retain the physical-census/Wheel owner in `battle_instinct_status.rs` (278/494). The play view now polls the existing queue every two seconds and displays Battle Instinct beside Harmony/Serene. Zero bonus with Wheel eligibility reads **eligible**; a nonzero bonus reads **active**. Failed/unavailable replies do not render stale HUD bonuses. | Crystal `src/creatures/players/wheel/player_wheel.cpp:2981-3004,3397-3418`, player `:7426,7431`: two-second census; five or more adjacent monsters, excluding player masters, yields melee n−4 and shielding 6(n−4). No formula change. |
| C14 | New 370/640 reads XP, level, trained build and account loyalty in the existing fenced durable transaction. Activated native progression/training tables supply level and skill progress; current live bonuses use the existing condition, equipment, Wheel, physical census and proficiency owners. Existing skills panel prefers the new reply and shows level/XP percent, all eight skills, trained/loyalty/current values and loyalty bonus, refreshing once per second while open. Legacy 279/495 is unchanged. | Crystal protocol `:4386-4388,4420-4433,9380-9406`; player `:3799-3809`. Hundredths-percent rounds to nearest, with zero for unreachable/overfull progress. No Dawnport cap. Current levels explicitly unavailable if their owners cannot qualify the observation; no trained-value fallback. The final finite native level has no successor and reports zero progress. |

New numbers: capability **370**, command **640**, mechanic ordinal **637**,
offer order **636**. No migration. Existing capabilities 276–279 and commands
492–495 are reused. Number discovery checked main and open PRs before allocation.
Goldens/registration fixtures use the repository generators.

Wiki-pending: current condition icon/time presentation, resting-area regeneration
wording, the current Monk HUD layout and level/skill/loyalty display semantics.
Fandom is unavailable (402 in the supplied environment); donor evidence is retained
without claiming an official or wiki verification. No NEED-FETCH URL was refused.

## Validation

All commands source `/workspace/oteryn-env/env.sh`. No full nextest or full
PostgreSQL suite runs. The native play tests are headless; physical Windows/GPU
play is unexecuted in this Linux environment.

Red before implementation: `cargo test --locked -p oteryn-client --lib
nx7_battle_instinct_reply -- --nocapture` failed because Battle Instinct was absent
from HUD text. `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib
skill_progress_postgres_real_dispatch_projection_replay_and_fences -- --nocapture`
failed with command 640 Rejected versus Accepted (one executed case, 46.45 s).
The native HUD regression and complete-skills native queue/decode/display test now
pass with `cargo test --locked -p oteryn-client --lib nx7 -- --nocapture` (2 tests).
C08/C10 are retained base implementations; no artificial red test is claimed for them.

`cargo fmt --all --check` and the changed-crate Clippy commands passed:

```sh
cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-session -p oteryn-client-runtime -p oteryn-client-ui -p oteryn-client --all-targets --quiet -- -D warnings
cargo clippy --locked -p oteryn-gameplay-dispatch --all-features --all-targets --quiet -- -D warnings
```

Package checks use `cargo test --locked -p <package> --quiet`: protocol 706,
session 168, client runtime 33, client UI 274 passed across their test targets.
`env -u OTERYN_TEST_POSTGRES_ADMIN_URL cargo test --locked -p
oteryn-gameplay-durability --lib --quiet` passed 688 library cases (59.01 s);
its database cases are deliberately route-less in this invocation.

The full native app library suite has **313 passed, 1 failed**. The unchanged
`movement_spell_controls_reach_session_and_decode_refusal_without_public_rooms`
test times out in isolation and with all NX-7 app source changes temporarily
removed (the base app's 312-test tree; 3.00 s). Candidate sources were restored.
The existing movement-spell queue marks `chat_public` pending, while
`0240-chat-party-room/protocol.rs::chat_result` routes SAY feedback to `local_chat`.
No casting or unrelated chat repair, timeout increase, skip or weakened assertion
is included. Both new native play cases pass on the candidate.

Focused PostgreSQL commands and exact filters:

```sh
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib condition_status_postgres_real_dispatch_food_clear_and_current_lease_fences -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib rest_status_native_owner_projects_food_and_bonuses_and_rejects_stale_session -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib battle_instinct_status_postgres_composed_query_and_stale_lease -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib skill_progress_postgres_real_dispatch_projection_replay_and_fences -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib skill_progress_matches_donor -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib registry --quiet
oteryn-architecture-check workspace .
```

Executed PostgreSQL evidence (one passed case per filter, zero skipped):
C08 **4.33 s**, C10 **4.30 s**, C11 **49.85 s**, C14 **47.72 s**. The logs contain
`POSTGRES_STARTED` for C08/C10, `BATTLE_INSTINCT_STATUS POSTGRES_CASE_COMPLETED`
for C11, and both `NX7_SKILL_DISPLAY POSTGRES_CASE_COMPLETED level=8 xp=5300
percent=5000 loyalty=50` and `SKILL_PROGRESS POSTGRES_CASE_COMPLETED` for C14.
The complete-skills case also proves unavailable current values before equipment
initialization, current values with a qualified build/equipment owner, immutable
trained state, read replay and connection fencing. Its new and legacy queries
share one continuous command sequence; no command ID is reused for a different query.

Dispatch donor-rounding filter: 1 passed. Registry filter: 8 passed.
`oteryn-architecture-check workspace .` returned `workspace-boundaries: PASS`.
`git diff --check` passed. No commit or push was performed.

Disposition: **C08 DONE** (retained/revalidated), **C10 DONE** (retained/revalidated),
**C11 DONE** (existing server projection plus new automatic HUD), **C14 DONE**
(new complete server projection and native skills display). These are the bounded
NX-7 rows, not a claim that unrelated casting or the entire app suite is green.
Self-review covered every changed file, production callers, bounds, session/content
fences, read-only SQL cardinality, capability negotiation, legacy compatibility,
client reset paths and generated fixtures. No new blocking finding remains.

# qd-travelling-trader

Status: BLOCKED; global route is not implemented by this change.

Branch: `claude/happy-davinci-ijz3wy-qd-travelling-trader`.
Base: `e2152d050038cecdb3728c222af2ac184e1cac5a`.
One writer; no subagents. Requested quest: `oteryn:quest.the_travelling_trader`.
Existing admitted identity: `oteryn:quest.the_travelling_trader_quest`.

## Hard blocker

The Travelling Trader requires a server-authoritative weekday prerequisite on
NPC mission callbacks. The current closed callback schema cannot express that
prerequisite. Rashid's placement rota does not constrain which mission callback
can execute on the current weekday. Adding per-quest JSON cannot supply this
missing mechanic. Stop this step under the packet's missing-generic-mechanic rule;
do not substitute the existing approximation for global behaviour.

Evidence inspected on 2026-11-17:

- `crates/gameplay/quests/src/quest/reward_callbacks.rs`: `RewardCallback` uses
  `serde(deny_unknown_fields)` and has no weekday/calendar prerequisite.
- `crates/gameplay/quests/src/quest/reward_callbacks/state_ops.rs`:
  `StateCondition` supports Any, Between, ElapsedAtLeast and ElapsedLessThan.
  Elapsed time does not express a recurring calendar weekday predicate.
- `crates/gameplay/entities/src/rashid_schedule.rs`: `destination_at`,
  `rotate_placements` and `trade_available` select weekday placement and trade
  availability. They do not select a quest mission by weekday. The implementation
  explicitly describes UTC midnight as an adaptation of donor timing.
- `content/quests/chosen-reward-callbacks.json`: existing Trader callbacks cite
  an approximation removing weekday availability, cheese decay, vase fragility
  and the exact price dialogue.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_trader.rs`:
  the existing six-leg test explicitly omits cheese decay and paid refresh.
  This is not proof of the requested global route.
- `content/quests/native-service-access.json`: the existing Rashid trade gate
  belongs to `oteryn:quest.the_travelling_trader_quest`; preserve its authority.

Needed mechanic: a bounded calendar weekday predicate admitted by the quest
callback schema and checked using authoritative server time in the durable
quest/reward transaction. Conversation eligibility and the commit must agree;
reconnect and rollover cannot bypass the predicate. The accepted global day
boundary must first be evidenced. This requires generic engine work outside the
packet's owned paths and insert-only registry permissions.

Cheese expiry/replacement and vase damage also need investigation before their
steps can be implemented faithfully. Their exact global rules and existing
native support were not established here; do not treat them as proven absent.

## Source evidence and open facts

Required TibiaWiki API requests used an honest research User-Agent and
`action=parse&format=json&prop=wikitext%7Crevid` for
`The_Travelling_Trader_Quest` and `The_Travelling_Trader_Quest/Spoiler`.
Both returned exactly `HTTP Error 402: Payment Required`. No current revision
or current wikitext was obtained, and no protection workaround was attempted.

Retained historical references, read from
`content/quests/definitions/quests-00200-00299.json`, under the matching
quest's `definition.oteryn_recipe.payload.recipe.source_refs`:

| Page | pageid | revid | Retained content SHA-256 |
| --- | --- | --- | --- |
| The Travelling Trader Quest | 20083 | 1159575 | a5010112c4e68987bab2135fe8c15067c8d5f41fff6d8307d93e5883769a56bc |
| The Travelling Trader Quest/Spoiler | 20101 | 1159573 | 1642f2409e324b12585a9f656d05dc932c5cb26c7fa922d99dfbc6e9ee10ce55 |

These are historical metadata, not freshly verified API results or admission
of the chosen recipe as global evidence. The recipe explicitly labels its
behaviour `CHOSEN_OTERYN_APPROXIMATION`.

The transcript indexes at `https://talesoftibia.com/libsearch`,
`https://tibiasecrets.com/transcripts` and
`https://elkolorado.github.io/tibialibraries/` returned HTTP 200.
`https://tibiasecrets.com/Rashid.html` also returned HTTP 200; its ordinary
recognised-trader dialogue does not establish the seven mission rules.
No donor was promoted to global authority.

Open facts:

- Current API revisions and full global mission transcripts, including exact
  prerequisite refusals, negotiation branches, fees and item quantities.
- Global weekday boundary and mission-to-weekday enforcement semantics.
- Exact cheese expiry, replacement and vase damage/retry rules.
- The requested Outlaw Camp, Kazordoon, Ab'Dendriel and Tiquanda delivery route
  against the current global spoiler; no route was guessed.
- Whether the requested shortened quest identifier is a task label or requires
  an alias to the existing admitted identity.

## Delivery and validation scope

This commit records the blocker only. No content, engine, generated files,
registration counts or existing tests are changed. No empty content documents
or placeholder test were created. The requested `qd_the_travelling_trader` PG
case is not implemented or run; the red-to-green route remains outstanding.
No full PostgreSQL suite was run. Gameplay tests, clippy and pin regeneration
are not evidence for a documentation-only blocker record and were not run.
The packet's implementation checks remain required when implementation resumes.

Validation of this documentation-only change:

- `python3 tools/agents/validate_governance.py`: passed (22 documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: passed (135 tests).
- `CARGO_BUILD_JOBS=1 cargo fmt --check`: passed after sourcing env.sh.
- `git diff --cached --check`: passed.
- `python3 tools/repository/reliability_checklist.py`: exited successfully, but
  selected unrelated historical paths against origin/main; those broad matches
  are not this task's one-file diff against the packet base.
- Self-review: read the complete record; no runtime changes, weakened tests,
  generated edits, invented source facts or completion claim. No PR was opened
  and no independent review was requested.

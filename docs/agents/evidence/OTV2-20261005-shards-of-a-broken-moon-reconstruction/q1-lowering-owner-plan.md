# SHARDS-Q1 lowering candidate for the existing Quest writer

Status: EVIDENCE / PROPOSED OWNER IMPLEMENTATION PLAN. No allocated catalogue keys, admitted content or executable runtime are created here.

Readback: main `3bacc59e6adbf138655e6f0c1cf0b4fdf9a36096`; evidence predecessor `5829ba45ef0dd92cec0ce77093b0f199e66b5206`. This plan consumes the 58 events in `binding-plan.json` and the correction candidate. Semantic names below are placeholders for owner-allocated native keys.

## Current contract and limits

`apps/game-server/src/quest/mod.rs` exposes QuestTrack, QuestTransition and QuestEffect `{track, from, effect}`. Tracks are bounded integers; a transition has at most eight effects on its own quest's tracks. The current evaluator checks each `from` against the locked input values before returning changes. `Set` permits setting the existing value again: a guarded `Eq(1) -> Set(1)` can retain a prerequisite while verifying it under lock. This is a candidate use of existing vocabulary, not a new assertion effect.

`apps/game-server/src/quest/predicate.rs` explicitly makes session-copy predicates advisory. Inventory and account facts belong to their owners. The Quest writer rechecks track comparisons; it cannot turn an advisory `HoldsItem` into an atomic inventory transfer, nor interpret prose such as `holds Moon Mirror` as a native predicate.

`apps/game-server/src/durability/quest_state.rs` accepts only QuestTransitionRequest `{transition_key, cause}`. Runtime callers reach the writer through the existing RevisionSlot. Receipt identity is Character + cause occurrence + transition key. The catalogue definition hash and content revision stay pinned; adding hidden tracks requires the owning content revision/migration admission before activation.

## Bounded Boolean-track representation

Candidate fact tracks use initial 0, min 0, max 1. These values encode absence/presence, not invented gameplay numbers. A once-only fact effect is `Eq(0) -> Set(1)`. A locked prerequisite effect is `Eq(1) -> Set(1)`. Do not use a repeated-use count to replace distinct location facts.

| Candidate transition | Locked prerequisite effects | New fact effect | Total effects |
| --- | --- | --- | --- |
| tides_join | Each of lagoon A, lagoon B and Emerald marker remains 1 | tides_done: 0 -> 1 | 4 |
| plants_join | Each of Tide Veil refined and Whisper Reed refined remains 1 | plants_done: 0 -> 1 | 3 |
| three_clues_join | yukti_done, tides_done and plants_done remain 1 | three_clues_done: 0 -> 1 | 4 |
| report_saraki | three_clues_done and start_route_saraki remain 1 | report_done: 0 -> 1 | 3 |
| report_nilavarna_alt | three_clues_done and start_route_nilavarna remain 1 | report_done: 0 -> 1 | 3 |
| prison_wall_chisel | prison_wall_prepared remains 1 | prison_wall_open: 0 -> 1 | 2 |
| prison_passage | prison_wall_open remains 1 | prison_escaped_from_chamber: 0 -> 1 | 2 |
| prison_rope_exit | prison_escaped_from_chamber remains 1 | prison_escaped: 0 -> 1 | 2 |

These fragments fit the current eight-effect bound. They are not complete event definitions: exact keys, producer qualifications, item/world transaction boundaries and any additional owner-approved prerequisites remain necessary. The table must not be used to infer that omitted events have no guards.

For both start alternatives, a candidate start transition checks shared `started=0`, sets it to 1 and records only the selected route fact. The shared check serializes competing starts; a later command must not silently switch the route. Route locking is a proposed native representation of the qualified alternate route, not a newly claimed source interaction rule.

Yukti, Tides and Plants have no dependency on one another's completion. Preserve all six completion orders. A report still requires all three branches. An ordinary serial stage counter cannot encode this invariant.

## Join dispatch and replay

The producer first qualifies the actual NPC, Item, World or Encounter result. It submits the selected native transition through RevisionSlot and applies the committed receipt to the session copy. Only then may an eligible dependent join be requested. Evaluate the three-clue join after a branch completion; do not run an independent polling engine or invent a timer occurrence.

For command-derived trigger children, retain the root CommandRef as accepted in #1886. Each requested transition has a distinct allocated transition key; never manufacture child CommandIds to distinguish joins. Keep the accepted QuestCause kind consistent across replay. Encounter death/outcome adapters must select the qualified existing cause contract; this plan does not invent an Encounter cause variant.

On the same receipt identity, the existing writer replays the recorded outcome. On a new cause after an already-set fact, `Eq(0)` refuses instead of producing a second progression change. Follow the existing revision-slot retry policy; do not add an unbounded quest retry loop. Inspect completion receipts before dispatching dependents: a refused transition supplies no new fact.

## Required boundaries that track guards cannot solve

- Item grant/consumption/transform success must come from Item/Durability. A track that says a sample was collected is not current custody. Lost-key regrant needs the owning inventory condition and transaction; no fabricated negative HoldsItem predicate is added here.
- World success must qualify active placement and actual effect. `prison_wall_open` cannot precede a failed wall mutation; `prison_escaped` cannot be awarded for donor callback acceptance without successful relocation. Cross-owner atomicity/recovery is an existing owner contract dependency, not something solved by ordering two calls in a quest handler.
- Laboratory note has no progression outcome. Never create a required `lab_note_read` track. Plinth success supplies the access-sequence fact; apparatus progress must follow the admitted laboratory route.
- Ritual join checks four distinct rune facts, but also transforms the qualified item. The four guards plus result fact fit five Quest effects; that count does not qualify the Item transaction or its recovery.
- Terminal dialogue may complete the quest only after prison escape and the qualified route/outcome. RewardClaim remains once per character through its existing transaction; Magnolia death never emits terminal rewards.
- Forbidden Gardens needs the Achievement owner's predicate. AccountCompleted(quest) is not AccountHasAchievement and cannot substitute for it.

## Owner acceptance cases

The investigation portion now has a concrete loader-format fixture: `q1-native-catalogue-candidate.json`, reproducibly generated by `build_q1_candidate.py`. `run_q1_candidate.py --adversarial` compiles the repository's existing native Quest module in an external temporary Cargo project, loads this fixture and checks the investigation cases below. The run includes six candidate tests plus 32 existing Quest tests. Removing the plants prerequisite deliberately makes the incomplete-subset and stale-input cases fail; restoring the fixture passes again. See `q1-catalogue-qualification.json` for the source binding and exact qualification limits. This does not exercise RevisionSlot, PostgreSQL receipts, item custody, NPC/World producers or restart.

The implementing owner must still run the full cases against the actual durable writer and producer composition. Pure catalogue qualification is recorded only for the investigation fixture; no runtime pass is claimed by this evidence plan.

1. All six Yukti/Tides/Plants completion orders permit the same final report. Every incomplete subset refuses it.
2. Three uses of one tide placement produce one marker fact and cannot satisfy tides_join. Both refined plant identities remain distinct.
3. A stale session-copy join/report decision is refused when locked prerequisites do not hold. Check the real writer, not only a predicate fixture.
4. Start through either route, then try the opposite start/report. Verify the selected native route policy and no accidental route switch.
5. Replay one root command with several distinct transition keys, then repeat after relog/restart. Verify existing receipts and no duplicate grants or joins.
6. Refuse the Item or World action after a positive advisory predicate. Confirm no progression/reward and correct owner recovery/custody.
7. Complete laboratory without reading the note. Refuse chisel before heating and rope completion before the admitted passage/relocation result.
8. Magnolia first lethal changes phase without terminal quest credit; qualified phase-two death advances only to the prison route. Terminal claim replay yields one reward.

## Remaining executable-slice scope

Q1 owns native catalogue/transition lowering only after allocation. I1 consumes the proof-backed 54610 successor and materializable Item admissions; 54610 remains distinct from i34017, with no guessed decay pair. E1 consumes `magnolia-encounter-spec.md` and accepted max_health semantics while keeping unknown timers, Death conversion and transform mapping held. R1 consumes active placement and composed producer witnesses, then qualifies real-character start through terminal reward and restart. No slice can be marked playable solely because this evidence plan is structurally compatible with the writer.

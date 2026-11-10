# Quests — complete package source review

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

Status: **REVIEWED**. All **103 tracked package files / 30,058 lines** were read substantively, including the manifest, build integration, implementations, literal catalogues and original test bodies. `quests-coverage-validation.json` verifies the complete recorded ranges against the immutable Git blobs and unchanged source checkout. The verification was recorded at 2026-11-10 16:23:30 UTC; it preserves evidence of the prior review and is not itself the review method.

## Authority and admission

The catalogue, loader, predicates, transitions and admission adapters were reviewed together. Tracks have explicit ownership, initial values and bounds; transition arithmetic is checked and foreign or conflicting ownership is refused. Definition hashes include the state and binding semantics relevant to durable pins. A transition that needs unsupported computed semantics is held rather than silently lowered into an unconditional action. Native kill, timer, NPC, reward and world callbacks have distinct admission paths; a returned plan is not proof that its durable or physical effects executed.

The complete NPC interpreter and its children were read through source ordering, staged state, dialogue replies, inventory and fluid checks, optional input behavior and explicit limits. Only approved source families are admitted. Deferred item, gold, XP, teleport and cosmetic branches remain deferred as whole operations when their required participants are unavailable. Descriptive dialogue does not create an economic reward by itself.

Reward and world callback review covered callback identity, selector uniqueness, owned tracks, source context, bounded effects and party membership. Portal, lever, timed, encounter and shared-world behavior are qualified separately. Shared-switch progress does not become personal quest authority; contribution and reset effects have their own exact source bindings. Public callback definitions and synthetic fixtures do not establish a production producer.

The durability reviewer separately traced generic QuestState, NPC, world callback and source-specific reward transaction owners, their fresh-authority guards and historical receipt paths. That delegated evidence is retained in `protocol-session/durability/report.md` and its exact ledger. It is not represented as a second independent root review of every SQL owner. The production Task Board writer remains part of that ongoing durability lane at this checkpoint.

## Source-specific rules reviewed

| Area | Review result and material boundaries |
|---|---|
| Classic callbacks | All eight integration fixture files and their referenced package lowering were read. Exact source locations, access values, monotone room-stage raises, ordered Demon Oak squares, read-only grave access and the three Crystal=13 Deepling gates were distinguished from encounter creation and quest completion. Labels or comments do not override the source executable destination. |
| Hive and shared world progress | All literal entrance/egress positions, strict seven-day gate, canonical gate identity, source-axis exit behavior, personal versus shared objective updates and native reset shape were read. Intermediate progress does not fabricate world-change credit. |
| Fire, Deeplings and Monk | Source-specific offering consumption, retained items, three required flags, canonical native portal substitutions and exact completion flags were checked. Unknown or unrelated counters do not count toward Monk completion. |
| Hot Cuisine | All recipes, canonical item/reward identity, source topic predicates, persisted counters and calendar rules were read. The August/bonus distinction and immutable prior-period timestamp are explicit. Material exchanges stay with their required durable participant; no extra month condition was invented for cooking. |
| Bigfoot and other Batch 3 sources | Required random draws are qualified before effects. Source score and repeat guards, dialogue order, focus behavior and unsupported transfer branches were checked. Unsupported effects remain held. |
| Soul War | Validated boss/taint bits, first-taint expiry, repeat-kill behavior, source access and failure flags, first versus repeat reward entitlement and exact XP scaling were read with all tests. Entry expiry and a verified final death event have different authority. |
| Rotten Blood | Stage, taint and completion bounds; source species; possession versus consumed altar inputs; entry level, chain prerequisites, minimum eligible party hazard, distinct child events, first reward, cleanse modes and checked cooldowns were reviewed. A pure result does not establish the same-transaction death evidence. |
| Isle of Evil and Wrath | Closed NPC/topic ownership, exact source reply tables, ordered keyword matching and source stage transitions were checked. Unsupported compound item, payment and teleport operations are held. Final potion progress is not mislabeled as completion of the entire quest. |
| Dark Trails, Grave Danger and Liquid Black | Complete dialogue branches and source tests were read. Predicate reads become locked state dependencies; source shadowing is preserved. Lore text, gold mentions and reward text are not treated as executed value effects. |
| Threatened Dreams | All thirteen NPC branches, topic ownership, initial values, compound guards, ordered first-keyword behavior, duplicate overlay rejection and held material branches were inspected. Checked aggregate arithmetic does not turn an unsupported reward participant into an admitted action. |
| Grizzly Adams | All task and boss catalogue entries, rule bodies and tests were read. Joining, three active tasks, level/Premium/rank restrictions, report timing, source cap and level-bonus order, boss unlock versus point behavior, exact one-kill reports and special-boss ordering were checked. Actual occupancy, kill authenticity and durable replay belong to their transaction owners. |
| Quest log | All seven files were read. Active pinned definitions, visibility of missing/unloadable state, bounded names/text/mission counts, mission ranges, template counter ownership and exact source-text overrides were assessed. Only explicitly supported unambiguous source expressions are lowered. |

## Task Board

All six Task Board files were read in full: legacy identity and transitions, production action rules, the 24 shop entries, all 436 delivery item/quantity entries, provisional elite policy and both original test files.

The current pure action engine clones state and balances, settles the prior server-save period, then publishes both only after success. Refusals preserve the original objects. Source level bands, XP rounding and overflow handling, completion multipliers, non-repeating selection, checked random draws, exact delivery quantities, bounty offer freezing, reroll resource order, list restrictions, talisman curves and promotion cap were inspected. `weekly_completions` explicitly communicates ordered awards to the separate progression owner; multiplying the pure helper's initial XP is not asserted to be the production final award.

Legacy fixed-board transitions are retained for historical identity, while the current action model explicitly refuses retired operations. Expansion and kill credit are internal actions and require authorization at the actual caller. The protocol reviewer has separately read the complete dispatch Task Board adapter and its current session/content checks; the durable writer review is still in progress.

The catalogue explicitly marks shop offers 10 through 22 as requiring the deferred decoration-kit participant. The pure ItemDouble fixture alone is therefore insufficient to claim a reachable incomplete bed delivery. The writer's refusal and debit order must be verified in its own review. Likewise, malformed externally constructed board vectors are not treated as a production exploit without a legal producer; normal constructors bound their sizes. Checked overflow refusals preserve pending rewards, and no wraparound was inferred from the old one-billion track limit.

## Findings and executed checks

**No additional confirmed finding** was established by this package pass. Existing cross-module findings are retained in the central register. Unresolved production-owner questions are not silently promoted to either findings or verified safety.

Three original, unchanged Hot Cuisine calendar tests passed when compiled directly with `rustc --test`. The exact source digest, commands and captured output are in `quests-calendar-unchanged-tests.json`. They cover the August/bonus distinction, a new August period and the cooking operation's absence of an extra dialogue month guard.

All other test bodies were inspected without execution. No full Cargo build, PostgreSQL execution, source modification, native client/server session or deployment was performed. Embedded content outside this Cargo package does not receive full supplementary-domain credit from an include, hash, fixture or reference check. These limits keep the package at **REVIEWED**, not **VERIFIED**.

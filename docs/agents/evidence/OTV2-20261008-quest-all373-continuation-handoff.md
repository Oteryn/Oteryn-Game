# OTV2-20261008 — All-373 Quest Completion continuation handoff

**Status:** retained continuation evidence, not a runtime authority or new Quest allocation  
**Snapshot date:** 2026-10-08 (Europe/Warsaw)  
**Fresh-read Oteryn Game main:** `340278d84f5c5406b45c634c9750e3414844bd88`  
**Canonical coordination:** [Oteryn/Oteryn-Game#1622](https://github.com/Oteryn/Oteryn-Game/issues/1622) (read its current STATE, not only the issue body)  
**Related merged Quest milestone:** [PR #1875](https://github.com/Oteryn/Oteryn-Game/pull/1875) (merged; do not resume its deleted historical branch)  
**Related active preflight at snapshot:** [PR #1930](https://github.com/Oteryn/Oteryn-Game/pull/1930), head `9a5bfa4f604a1a71428d0d5a04c4bc78ce51e9d4` (open at snapshot, ahead 2 / behind 3 vs snapshot main; recheck live state).

## Objective and why this handoff exists

Continue **all** canonical Oteryn Quest implementation against the pinned **373 wiki quest titles**, not only Summer Update 2026 additions. An earlier worker audited Crystal `summer-update`, then implemented chosen-source typed progress and a 373-title backlog. Those implementation changes were subsequently integrated into main and refined by other agents. The current source of truth is **main + live PRs**, not the historical prompts or branch snapshots.

Do **not** recreate the earlier 139+88 chosen-source bulk overlay, reuse stale 303/207 statistics, reimport existing Quest definitions, or revert later terminal-stage and source-owner refinements. Before work, re-read actual main SHA, canonical coordination STATE and the currently allocated Quest/Encounter/RewardClaim/NPC owner lanes.

## Current authoritative inventory (PROVEN at pinned main)

Source files:

- `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`
- `content/quests/definitions/index.json`
- `content/quests/missions/completion-candidate.json`
- `content/quests/missions/completion-binding-plan.json`

The pinned current-main artifacts report:

| Inventory / lane | Current count |
| --- | ---: |
| Pinned wiki Quest titles | **373** |
| Wiki titles mapped to canonical identities | **373** |
| Canonical Quest definitions | **352** |
| Distinct canonical definitions representing those titles | **350** |
| Completion candidate canonical owners | **310** |
| Typed progress tracks | **3,221** |
| Typed transitions | **5,154** |
| Completion transitions | **308** |
| Owners in chosen completion binding plan | **304** |
| Planned stage intents | **1,891** |
| Chosen-source-only progress owners | **146** |
| Source-plus-chosen progress owners | **90** |
| Authored chosen progress owners | **68** |
| Donor/source fully lowered owners | **6** |
| Wiki titles awaiting native bindings | **331** |
| Wiki titles awaiting reward-only runtime lane | **42** |
| Wiki titles with source-fidelity holds | **221** |

The current completion candidate explicitly reports:

- `runtime_activated: false`;
- `native_event_dispatch_bindings: 0`;
- `native_NPC_dialogue_bindings: 0`;
- `native_reward_delivery_bindings: 0`.

**Important interpretation:** the matrix records `playable_verified: 0` and `playable_assessment: NOT_PERFORMED_BY_THIS_MATRIX`. That means **no end-to-end playability was verified by this audit**, NOT that zero quests are playable in the actual game.

The 373 wiki titles and 352 canonical definitions are not supposed to be one-to-one. Do not create new identities just to make the counts equal.

## Native binding follow-up (PROVEN at PR #1930 snapshot)

[PR #1930](https://github.com/Oteryn/Oteryn-Game/pull/1930) already implements a disjoint, read-only **352-owner native-binding preflight**. It partitions the catalogue into:

- **304** chosen stage-plan owners;
- **6** source-progress owners with no chosen stage plan;
- **42** reward-only owners with no typed-progress candidate.

The preflight enumerates 1,891 stage intents:

| Kind | Intents |
| --- | ---: |
| USE | 458 |
| TALK | 329 |
| KILL | 275 |
| COLLECT | 277 |
| EXPLORE | 248 |
| COMPLETE | 304 |

It also reports 656 reward intents, 750 exact target identity associations, 108 ambiguous associations, 1,569 exact-name targets without canonical identities, and 25 declared Encounter outcome seams. Identity associations and declared Encounter seams remain **non-executable** until real owner/trigger/credit/reducer wiring exists.

Preflight task record on the PR branch: `docs/agents/tasks/archive/OTV2-20261008-quest-native-binding-preflight.md`. The preflight has its own owner/branch and must **not** be overwritten or duplicated. At snapshot, PR #1930 was open, not proven merged.

## Preserved Crystal/Canary/Wiki source evidence

The earlier Crystal `summer-update` directory-level audit is retained at:

`docs/agents/evidence/OTV2-20261006-crystal-summer-quest-audit.md`

It classified 122 Crystal quest-script directories as 101 SOURCE_BOUND, 15 DEFINITION_ONLY, 1 INDIRECT and 5 AUX. That was a **directory-to-source/canonical representation audit**, **not** a playability result or a count of all wiki quests. Quest evidence also exists outside `data-global/scripts/quests` (NPC, actions, world objects, storage, combat, loot and world systems).

The all-373 donor coverage matrix currently reports 332 with a donor implementation, 12 partially implemented, and 29 absent from both donors in the pinned coverage classification. For exact source pin/provenance use the committed `source` and `coverage` manifests, not floating donor heads or this narrative.

## Owner and integration boundaries

- Root `AGENTS.md`, `docs/agents/META_AGENT_POLICY_BINDING.json`, nearest path rules, and accepted architecture control each new change.
- Use canonical coordination #1622 to acquire **exclusive ownership** before modifying runtime/shared Quest/Encounter/RewardClaim/NPC paths.
- The Quest completion stage counter and source binding packet are not an event consumer or proof of gameplay.
- `QuestCause::CreatureDeath` and durable transition APIs exist, but the Encounter outcome seam must have a real owning producer, exact occurrence and credit semantics. No quest-specific death bypass.
- Likewise, USE, TALK, COLLECT, EXPLORE, COMPLETE and reward-only lanes must follow their established owners; avoid writing over concurrently open/merged PRs.
- Preserve source-fidelity UNKNOWN/conflict information, exact identity joins and original source tracks. No duplicate Quest engine, fabricated identity, auto-terminal completion or premature `runtime_enabled`.
- Validate actual playable flow only with start/gating, stage progress, party/damage credit, item usage/reward, completion, retry/idempotency and appropriate rollback/restart proofs.

## Immediate next-agent workflow

1. **Fresh-read** current `Oteryn/Oteryn-Game` main, PR #1930 and canonical STATE #1622; record exact SHAs and relevant existing Quest/Encounter/RewardClaim/NPC PRs. Assume every SHA in this handoff is historical until revalidated.
2. **Reuse** the committed all373 matrix and the 352-owner native-binding preflight (if PR #1930 remains open, read it through its exact head). Do not regenerate a second independent backlog.
3. **Reconcile real owner status**: choose the smallest unowned native consumer gap in the 304/6/42 partition, or request a coordinator/architect owner allocation for an accepted missing runtime seam. Do not overlap another agent.
4. **Implement one bounded playable-first batch** under a new exclusive branch. Wire only exact real occurrence→stage transition causes; preserve fail-closed rows for unresolved semantics/identity/credit. Prefer an existing event consumer over an invented quest-specific mechanism.
5. **Test and prove** the selected batch: authoritative producer event, owned character/party credit, durable transition, gating, reward handling, duplicate/replay behavior and focused regression, plus required repository gates and review.
6. **Publish and report** exact head, changed paths, delta, tests, remaining blockers and next continuation in the allocated task/PR and #1622. Integration follows protected Merge Queue rules; no direct merge/force/reset.

## Explicitly unresolved

- Whether any given canonical quest is truly playable end-to-end remains **UNKNOWN** until runtime evidence exists.
- The exact current merge/review state of PR #1930 and related consumer PRs must be re-fetched.
- Encounter outcome delivery and party/damage contribution credit must be verified against current runtime; older descriptions of an ENC-RT/ENC-OUTCOME allocation are historical and not an authority grant.
- Source holds are distinct from native consumer readiness and may coexist with a deliberately chosen Oteryn approximation.

## Copyable continuation prompt

> Oteryn: all-373 Quest Completion lead — kontynuuj autonomicznie. Zrób fresh-read aktualnego `Oteryn/Oteryn-Game main`, canonical STATE w issue #1622, PR #1930 oraz wszystkich aktywnych Quest/NPC/Encounter/RewardClaim consumer PR-ów. Przeczytaj `docs/agents/evidence/OTV2-20261008-quest-all373-continuation-handoff.md` i aktualny `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`. Nie ufaj historycznym SHA ani liczbom bez ponownej weryfikacji. Celem jest kompletne, grywalne domknięcie **wszystkich 373 wiki quest titles / 352 canonical Quest owners**, nie tylko najnowszych questów. Nie duplikuj definicji ani istniejącego loweringu; source139/overlay i terminal refinements są już w main. Użyj aktualnego native-binding preflightu 352 owners, skoordynuj wyłączny ownership w #1622 i pracuj w małych niezależnych batchach nad rzeczywistymi native consumer bindings; tam, gdzie Encounter/Quest/RewardClaim/NPC owner jest niegotowy, eskaluj do koordynatora/architekta i kontynuuj rozłączną pracę. Nie używaj quest-specific bypassów, nie aktywuj runtime bez dokładnego event/credit/reward i end-to-end proof. Zapisz pracę w repo, z testami, exact-head evidence, PR i kolejnym handoffem. Pracuj silent poza decyzjami/blokerami.

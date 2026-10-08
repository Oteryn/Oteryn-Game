# OTV2-20261008 — All-373 Quest Completion Continuation Handoff

Status: RETAINED EVIDENCE / CROSS-CHAT HANDOFF (not runtime authority)  
Snapshot date: 2026-10-08, Europe/Warsaw  
Repository: `Oteryn/Oteryn-Game`  
Verified `main` baseline: `d0b091b6b5ad4b9527d354df0043c6e55d5861cc`  
Canonical coordination: [#1622](https://github.com/Oteryn/Oteryn-Game/issues/1622)

## Read this before continuing

This document preserves the state of the **entire pinned 373-title quest inventory**, not just Summer Update 2026 headline quests. It does **not** grant file ownership, runtime deployment permission, merge permission, or authority to modify another agent's branch.

**First action in any new chat:** fresh-read live `main`, #1622, currently open PRs, and the repository agent policy binding. Do not reuse SHA/counts/allocations here if they have advanced.

### Do not resurrect superseded local changes

The earlier Windows worktree `C:\Users\barte\quest-source-overlay-current` (local branch `codex/quest-source-overlay-20261006`) was used to prototype an additive **88-source-owner** typed-progress overlay. Its work was integrated through merged [#1876](https://github.com/Oteryn/Oteryn-Game/pull/1876), with subsequent terminal-count normalization merged through [#1895](https://github.com/Oteryn/Oteryn-Game/pull/1895). The preceding 139-new-owner slice was merged through [#1875](https://github.com/Oteryn/Oteryn-Game/pull/1875).

**Never cherry-pick or force-push this old local worktree onto current `main`**: it predates later changes and would risk regressing definitions, NPC bindings, completion packets and generated artifacts.

In the handoff-producing session, the Remote Desktop Commander connector reported a monthly usage limit. The machine remained paired, but the connector could not inspect remaining local uncommitted files. This is a session/tool limit, not evidence that the machine or files were deleted. The canonical merged features were independently verified in GitHub `main`.

## What is already in `main`

**PROVEN** from merged PRs and the committed all-373 matrix at the baseline:

| Scope | Current qualified state |
| --- | --- |
| Pinned wiki quest titles | **373/373** mapped |
| Canonical Quest definitions | **352** |
| Wiki title rows awaiting native bindings | **331** |
| Reward-only rows awaiting their existing RewardClaim/World runtime | **42** |
| Source fidelity holds / clear (separate axis) | **221 / 152** wiki title rows |
| Runtime-playable verification | **NOT PERFORMED**; the matrix reports **0 verified**, not proof that no quest can be played |
| Source-derived donor coverage | **332 implemented**, **12 partial**, **29 absent-in-both**, per pinned wiki coverage inventory |

**PROVEN** from merged [#1895](https://github.com/Oteryn/Oteryn-Game/pull/1895) and [#1917](https://github.com/Oteryn/Oteryn-Game/pull/1917):

- **310** canonical Quest owners have typed completion candidates.
- **3221** tracks, **5154** transitions, **308** completion transitions in that candidate snapshot.
- **236** chosen-source projected owners (146 new, 90 overlays), plus six Source-complete owners intentionally left unchanged.
- **0 remaining terminal-count lowering holds**; prior nine were normalized in candidate projection, not by rewriting SOURCE facts.
- **304** chosen-stage binding-plan owners and **1891** stages, plus **6** Source-lowered owners / **121** retained transition records in a separate plan. Together the plans cover **310/310** typed candidate owners.
- **42** reward-only Quest definitions do not need synthetic QuestState tracks to make RewardClaims executable.
- Native event/NPC/reward dispatch and runtime activation remained **unadmitted** in those merged qualification packets.

The current committed machine-readable truth is:
`tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`.

Do not confuse canonical identity, source crosswalk, chosen typed progress, exact identity candidate, and proven playable end-to-end behavior.

## Later merged qualification lanes

- [#1897](https://github.com/Oteryn/Oteryn-Game/pull/1897) — **247 talk stages** audited: 171 exact NPC+Dialogue candidates, 23 exact NPC without Dialogue, 36 ambiguous multiple NPCs, 17 without exact NPC. No dialogue branch selected or native binding activated.
- [#1899](https://github.com/Oteryn/Oteryn-Game/pull/1899) — **239 explore stages** audited: 21 exact Area identity candidates, of which only 10 are clean one-target candidates. No spatial occurrence or runtime binding was inferred.
- [#1913](https://github.com/Oteryn/Oteryn-Game/pull/1913) — **42 reward-only rows**, referencing **59 ready placed RewardClaims**, blocked on serving/binding an imported World via MAP-CUTOVER-1b, **not** on missing QuestState progress.
- [#1917](https://github.com/Oteryn/Oteryn-Game/pull/1917) — the six Source-lowered Quest owners receive an offline consumer work plan: 106 exact NPC-requested transitions, 15 interaction transitions. No native binding.
- [#1886](https://github.com/Oteryn/Oteryn-Game/pull/1886) — QUEST-GATE-0 §16 resolves durable cause routing for quest children: use the root CommandRef under the accepted owner contract.

## Live parallel work (refresh before touching any path)

At this snapshot, the following were **open**. Live PR/Issue state wins over this list:

- [#1930](https://github.com/Oteryn/Oteryn-Game/pull/1930) — full **352-owner native-binding gap preflight**, expected to produce `all352.json` and per-owner unresolved consumer codes. **Do not duplicate its generator or touch its owned paths.**
- [#1888](https://github.com/Oteryn/Oteryn-Game/pull/1888) — collect/RewardClaim acquisition candidates.
- [#1889](https://github.com/Oteryn/Oteryn-Game/pull/1889), [#1901](https://github.com/Oteryn/Oteryn-Game/pull/1901) — literal Item-id USE candidates.
- [#1896](https://github.com/Oteryn/Oteryn-Game/pull/1896), [#1902](https://github.com/Oteryn/Oteryn-Game/pull/1902) — reward-only completion / RewardClaim routing.
- [#1867](https://github.com/Oteryn/Oteryn-Game/pull/1867) — Soul War reconstruction; [#1852](https://github.com/Oteryn/Oteryn-Game/pull/1852) — Shards evidence.

## Remaining real runtime dependencies

**PROVEN / BLOCKED** under current accepted Game architecture:

1. **Kill → Encounter outcome → QuestTransitionRequest:** the chosen-source packet retained **25 exact Encounter outcome seams** as candidate evidence, not executable credit. Await the runtime owner allocations **ENC-RT-1 → ENC-OUTCOME-1** and their accepted death/participation credit contract. Do not create a quest-specific death-pipeline bypass.
2. **NPC dialogue → QuestState:** exact NPC+Dialogue identity is not yet an accepted selected branch or durable, exact requested occurrence. Route through the owning **NPC-QUEST-1 / NPC-TALK-1** work.
3. **USE/movement → QuestState:** preserve QUEST-GATE-0 §16 and the genuine root-command cause/plan. Route through **QUEST-TRIGGER-1** and the actual interaction consumer.
4. **Explore → spatial occurrence:** canonical Area identity is not proof of a location event or eligible contained coordinate.
5. **Reward-only → World/RewardClaim:** use the existing RewardClaim/chest placement runtime; wait for **MAP-CUTOVER-1b** and world serving/activation. Do not invent synthetic quest counters for reward-only quests.
6. **End-to-end playable verification:** a candidate must demonstrate real start, gated progress, encounter/party credit, completion and reward, including idempotence/replay before `PLAYABLE_VERIFIED`.

**UNKNOWN:** actual end-to-end playable quest count. The all-373 matrix deliberately records `playable_assessment=NOT_PERFORMED_BY_THIS_MATRIX`.

## Safe execution sequence for the next agent

1. Fresh-read `main`, #1622, [#1930](https://github.com/Oteryn/Oteryn-Game/pull/1930), overlapping current PRs, task ownership and accepted Quest/Encounter/NPC/World contracts.
2. Inspect the current all-373 matrix and, once it lands, the all-352 native-binding preflight. Do not use the older R2 task numbers if newer committed receipts exist.
3. Run the established drift checks before changing data:
   - `python tools/content-schema/quest-authoring/quest_completion_import.py --check`
   - `python tools/content-schema/quest-authoring/quest_completion_matrix.py --check`
   - relevant focused tests / repository governance for touched paths.
4. Select one **currently unowned**, bounded native-binding consumer seam with an accepted runtime contract. If a required runtime owner is unallocated, record the exact blocker in #1622, then continue another independent lane.
5. Create one fresh, exclusively owned task branch. No second Quest engine, no fuzzy joins, no old local worktree cherry-pick, no runtime activation from offline candidate evidence.
6. Commit small reviewable changes, validate exact head and use the required protected Merge Queue route. Record the next checkpoint/evidence for the following agent.

## Source / reproduction index

- Whole pinned wiki inventory: `tools/content-schema/quest-authoring/samples/quest-coverage-2026-09-27.json`
- Canonical Quest definitions: `content/quests/definitions/index.json`
- Full mapping/work backlog: `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`
- Native completion importer: `tools/content-schema/quest-authoring/quest_completion_import.py`
- Source chosen-stage builder: `tools/content-schema/quest-authoring/samples/server-completion/chosen-source-progress/builder.py`
- Event/reward evidence builder: `tools/content-schema/quest-authoring/samples/server-completion/chosen-source-events-rewards/builder.py`
- Typed candidate: `content/quests/missions/quest-state-completion-candidate.json`
- Chosen binding plan: `content/quests/missions/completion-binding-plan.json`
- Source-lowered supplemental plan: refer to merged #1917 exact file list.
- Original full Crystal quest-script inventory audit: `docs/agents/evidence/OTV2-20261006-crystal-summer-quest-audit.md`
- Prior overlay record: `docs/agents/tasks/OTV2-20261006-quest-source-progress-overlay.md`
- Reward-only audit: `docs/agents/evidence/OTV2-20261007-reward-only-quest-runtime-audit.md`

## Suggested new-chat prompt

> Oteryn Game — all-373 Quest Completion Lead. Continue autonomously from the **live** main and canonical coordination #1622. First read `docs/agents/evidence/OTV2-20261008-all373-quest-completion-handoff.md`, then refresh PR #1930 and all overlapping Quest/NPC/Encounter/World PRs. Do not reuse obsolete SHA, counts or local branches. The chosen typed-progress overlay and terminal-count fix are already merged (#1876, #1895): do not recreate them. Prioritize an allocated, exact native-binding runtime consumer from the all-352 inventory, without introducing bypasses or falsely marking quests playable. Persist evidence, tests and task progress in the repo; escalate only actual architecture/owner blockers via #1622.

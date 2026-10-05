# Oteryn Player Swarm Programme

Status: **ACTIVE ROUTING CANDIDATE / no authority expansion**

Canonical technical lead alias: `Oteryn: player swarm lead`

Jira parent: `KAN-34`

Architecture draft: [OTERYN_PLAYER_SWARM_AND_GAME_COVERAGE_DRAFT.md](../../architecture/OTERYN_PLAYER_SWARM_AND_GAME_COVERAGE_DRAFT.md)

Context policy: [AGENT_CONTEXT_BUDGET_CONTROLLER.md](../AGENT_CONTEXT_BUDGET_CONTROLLER.md)

## 1. Role

The Player Swarm lead is a subordinate technical programme lead. It owns technical synthesis, tranche sequencing, coverage objectives and successor handoff for the synthetic-player programme.

It is **not** a second Game control plane.

`Oteryn: work coordinator` remains the sole reusable mutating Game control plane/integration authority under the current protected governance. Alias invocation does not grant allocation, shared lease, merge, production, Jira-write or cross-repository authority.

## 2. Jira decomposition

- `KAN-35` — runtime/session supervisor;
- `KAN-36` — realistic behavior profiles;
- `KAN-37` — map/content/mechanics coverage engine;
- `KAN-38` — party/group/boss encounter bots;
- `KAN-39` — chaos/soak/invariants/replay;
- `KAN-40` — native-client bot cohort;
- `KAN-42` — context budget and cross-chat continuation;
- `KAN-43` — lead/bootstrap/lifecycle.

Default dependency order is KAN-35 -> KAN-36 -> KAN-37 -> KAN-38 -> KAN-39 -> KAN-40, while KAN-42 and read-only preparation may proceed when path/dependency ownership is disjoint. Live facts override this planning order.

## 3. Startup reconstruction

Every invocation begins from canonical state, not remembered chat prose:

1. resolve protected `main`;
2. resolve only the matching `OTV2_PLAYER_SWARM_LEAD` lifecycle entry and prompt;
3. read this programme and the architecture draft;
4. read KAN-34 and only material child items/current comments needed for the next decision;
5. resolve the current Player Swarm task/branch/PR/head/check state;
6. read the current successor checkpoint if one exists;
7. classify material facts `PROVEN | DERIVED | UNKNOWN | CONFLICT`;
8. choose exactly one current tranche and one next action.

Apply `CONTEXT_ROUTING.md`; do not bulk-load unrelated history.

## 4. Technical phases

### Phase A — session/runtime

Create a scalable supervisor over real admitted sessions. Prove independent bot lifecycle, liveness/reconnect behavior and basic gameplay commands without privileged server mutation.

### Phase B — behavior

Add deterministic/stochastic profiles, player-visible perception, goal selection, memory and imperfect reaction behavior.

### Phase C — coverage

Bind exact build/content generation and score concrete untested map/content/mechanics targets. Allocation should prefer uncovered targets.

### Phase D — group/boss play

Exercise normal party/group mechanics and at least one complete supported multi-player encounter including failure/retry paths.

### Phase E — chaos/soak/replay

Run long concurrency/adverse legal sequences, hard invariants and deterministic replay.

### Phase F — native-client cohort

Drive representative real-client journeys in a smaller cohort. Do not count headless success as native-client proof.

## 5. Architecture routing

The lead decides bounded path-local implementation details inside accepted contracts and exact allocation.

It returns `ARCHITECTURE_ESCALATION_REQUIRED` before mutation when work would require a material new/conflicting architecture decision.

Routing:

```text
Player Swarm Lead
  -> ARCHITECTURE_ESCALATION_REQUIRED
  -> Work Coordinator / canonical STATE
  -> Oteryn: sol supervising architect
  -> ARCHITECTURE_RESOLUTION
  -> Work Coordinator
  -> resume/reallocate
```

The escalation packet includes exact main, Jira/story/task, branch/PR/head, evidence classification, affected paths/contracts, the smallest blocking question, holding action and work that can continue.

The lead does not contact the owner merely because architecture is difficult. Owner contact occurs only when the canonical route returns `OWNER_DECISION_REQUIRED` or proves an owner-only authority/action is necessary.

## 6. Integration routing

The lead never merges its own PR.

For an implementation candidate it returns exact head/paths/tests/coverage delta and one control-plane recommendation. Shared paths require the existing shared-lease route.

Current Jira writing remains owned by the canonical Work control plane unless protected governance explicitly changes that rule. The lead may propose Jira deltas in its handoff.

## 7. Silent operation

The lead is silent by default.

Do not send the owner routine progress, normal CI churn, intermediate findings, ordinary handoffs, implementation narration or "still working" messages.

Owner-visible output is limited to:

- `OWNER_DECISION_REQUIRED`;
- an owner-only blocker/action;
- a material unresolved problem that specifically needs the owner after normal architecture/control-plane routing;
- one concise terminal `DONE` line when the existing repository convention calls for it.

Everything else is persisted in normal programme/task/PR/control-plane surfaces.

## 8. Context pressure and chat rotation

Apply `AGENT_CONTEXT_BUDGET_CONTROLLER.md`.

At `PREPARE_HANDOFF`, keep successor state compact and current. At `HANDOFF_REQUIRED`, finish the current safe boundary and rotate. At `NO_NEW_LARGE_WORK`, do not start a new material tranche.

Chat rotation is not a task failure and not a wall-clock stop. A fresh chat with only the alias must reconstruct state from canonical sources.

## 9. Coverage scoreboard

Maintain only evidence-backed values for the exact current build:

```yaml
map:
items:
monsters:
spells:
quests:
npcs:
character_systems:
party:
bosses:
chaos_soak:
native_client:
failures:
  invariant: []
  deterministic_replays: []
```

Aggregate percentages never replace concrete uncovered target lists.

## 10. Tranche completion

A tranche is complete only when:

- implementation/evidence is bound to the exact candidate;
- required focused/component/E2E checks are truthful;
- coverage delta identifies what became covered and what remains;
- material findings are resolved/routed;
- the PR is integrated through protected controls when mutation was involved;
- protected-main readback confirms the claimed terminal state;
- successor state names exactly one next action.

An open/unmerged PR is not terminal completion.

## 11. Programme end

The programme is terminal only when the owner/control plane accepts the intended Player Swarm capability and its remaining uncovered targets are explicitly classified rather than silently omitted.


# Oteryn Player Swarm & Full-Game Coverage — draft

Status: **DRAFT / implementation-programme input / not standalone product authority**

Jira programme: `KAN-34` with implementation stories `KAN-35` through `KAN-40`, context-budget story `KAN-42`, and lead-bootstrap story `KAN-43`.

This document proposes a first-class synthetic-player system for Oteryn. Existing protected architecture, protocol, authority, persistence, security and content contracts remain authoritative. Any conflict or material new durable decision is routed through the existing architecture-escalation mechanism rather than being silently decided here.

## 1. Goal

Build synthetic players that exercise Oteryn through the same player-facing authority boundaries as real clients and use them to continuously test as much of the playable game and content corpus as possible.

The system is not merely a load generator. It is intended to answer:

- can a player actually traverse and interact with the world represented by the current content build;
- do Items, Monsters, spells, NPCs, quests, character systems and persistence behave coherently across long action sequences;
- can coordinated players form parties and complete multi-player encounters and bosses;
- do reconnect, death, concurrency and adverse-but-legal action sequences preserve invariants;
- which concrete map/content/mechanics targets remain unexercised.

## 2. Player-fidelity rule

The behavioral path must use normal player-visible/session-visible information and normal gameplay commands.

Preferred path:

```text
account / login
  -> Platform / Gateway
  -> admission
  -> TLS / Oteryn game session
  -> join snapshot and state deltas
  -> player-visible perception
  -> behavior decision
  -> normal gameplay command
  -> authoritative result / deltas
```

The bot must not gain gameplay authority from its test identity. Administrative test-account tagging and environment isolation are allowed; privileged teleport/kill/give-item APIs are not evidence that a player journey works.

The existing `oteryn-session`, `oteryn-session-tcp`, `tools/dev-client` and `tools/synthetic-client-harness` are the preferred lower-layer foundations where compatible.

## 3. Two fidelity tiers

### 3.1 Headless synthetic players

The large cohort. It should use the real production protocol/session path but omit window/GPU rendering. It is suitable for tens, hundreds or eventually thousands of concurrent sessions.

Required behavior domains include movement, interaction, combat, spell use, item/container/inventory actions, chat, liveness, reconnect and authoritative state reconciliation as those capabilities become available.

### 3.2 Native-client cohort

A smaller high-fidelity cohort drives the real production client path and covers login composition, input routing, UI/HUD interaction, renderer-visible state and representative end-to-end journeys.

Headless success never substitutes for native-client evidence where native client behavior is the target.

## 4. Realistic behavior

Synthetic players should not be perfect deterministic optimizers by default. Profiles may include:

- casual/noob;
- experienced hunter;
- mage;
- explorer;
- quester;
- trader;
- PvP/player-interaction profile when the system exists;
- AFK/intermittent player;
- chaos/adversarial legal player.

Profiles may model reaction delay, jitter, hesitation, target changes, imperfect choices, selective loot, resource-management mistakes, idle periods and disconnect/reconnect behavior.

Deterministic seeds remain mandatory for reproducible regression modes. Stochastic exploration is allowed only when the seed and resulting command history are captured.

## 5. Party and boss play

Synthetic players must be able to exercise normal group mechanics once those mechanics exist.

Target journey:

```text
login
 -> prepare supplies
 -> create/join party
 -> travel together
 -> satisfy encounter prerequisites
 -> enter boss/encounter
 -> execute role behavior
 -> react to phases/adds/AoE
 -> death/wipe/retry paths
 -> loot/reward/lockout
 -> return/persist/reconnect
```

Role behavior may include leader/tank, healer, DPS and support. Bot-side orchestration may assign goals and roles, but every gameplay action still travels through normal player sessions.

The test matrix must include member death, missing member, delayed entry, wipe/retry, reconnect and concurrent reward/loot cases where the game supports them.

## 6. Coverage-driven execution

The swarm must not spend most of its time repeating already-covered actions. A coverage engine should identify under-covered targets and allocate players/scenarios to them.

Coverage dimensions:

### Map/world

- reachable tiles, areas and floors;
- stairs, holes, teleporters and transitions;
- collision and blocked paths;
- doors and quest doors;
- protection zones, depots, houses and other supported interaction surfaces;
- spawn areas and reachability anomalies;
- unreachable or unexpectedly reachable regions.

### Items

For each applicable canonical Item identity/family/capability:

- pickup/drop;
- move;
- stack/split;
- container operations;
- use/consume;
- equip/unequip;
- charges and timed state;
- transfer/trade where supported;
- logout/reconnect persistence;
- invalid or boundary cases appropriate to the capability.

Coverage reporting must preserve the canonical Item identity set; it must not reinterpret content identity merely to simplify testing.

### Monsters and combat

- spawn/despawn/respawn;
- acquisition and target switching;
- movement/pathing;
- attacks and authored abilities;
- defenses, resistances and immunities;
- flee/summon/heal behavior where authored;
- multi-attacker combat;
- death/corpse/loot;
- player death/logout/reconnect races.

### Spells

- valid/invalid target;
- no target;
- range boundaries;
- resource/level gates;
- cooldown/exhaustion;
- resistance/immunity;
- single and multi-target;
- death and disconnect races;
- PvE/PvP variants when supported.

### Quests

- start through final reward;
- NPC/dialogue steps;
- items, monsters, triggers and chests;
- invalid ordering;
- duplicate reward attempts;
- death/logout/reconnect;
- multi-player/shared steps;
- detection of unreachable transitions.

### NPCs

- physical reachability;
- dialogue branches;
- buy/sell/services;
- quest dialogue;
- level/vocation/premium/other accepted conditions;
- insufficient gold/inventory and other failure paths.

### Character/player systems

As implemented: death, progression, equipment, bestiary/charms, wheel, forge, imbuements, familiars, achievements, depot/inbox/stash, containers, houses, party and chat.

## 7. Invariant checking

Behavioral runs should be paired with hard invariant checks where an accepted owner exists.

Examples:

- one Item/value cannot have two authoritative owners at once;
- container parentage cannot form an impossible cycle;
- HP/mana and other bounded state remain within accepted constraints;
- terminal/dead actors cannot perform forbidden commands;
- reward uniqueness and lockout semantics hold;
- authoritative generations/revisions do not move backward;
- persistence/reconnect does not duplicate or silently lose committed value.

The invariant checker observes or validates accepted semantics; it must not invent new permanent semantics.

## 8. Scenario, exploration and soak modes

1. **Scenario** — deterministic E2E regression of a named journey/mechanic.
2. **Exploration / coverage** — goal selection prioritizes unexercised targets.
3. **Soak / chaos** — long-running concurrency and adverse legal action sequences.
4. **Native-client cohort** — smaller high-fidelity UI/input/client-composition journeys.

## 9. Chaos examples

Useful adverse sequences include:

- rapid target changes;
- movement/use/cast overlap;
- container churn;
- simultaneous pickup/loot attempts;
- disconnect/reconnect during operations;
- death during interaction;
- multiple players contesting the same entity;
- long idle/liveness periods;
- party-member loss during an encounter.

Chaos actions remain valid player-facing requests. Protocol corruption/fuzzing is a separate test class.

## 10. Deterministic replay

Every material run/failure should bind, where available:

```yaml
bot_id:
profile:
scenario_id:
world_build:
content_generation:
server_build:
client_build:
behavior_seed:
world_seed:
command_sequence:
server_sequence:
relevant_state_revisions:
failure_or_invariant:
```

A discovered defect should be reducible to a compact replay artifact whenever determinism permits.

## 11. Metrics and reports

At minimum report:

- connected/admitted sessions;
- commands and outcomes by type;
- latency/error distributions;
- disconnect/reconnect/death counts;
- invariant failures;
- map coverage;
- Item identity/family/capability coverage;
- Monster coverage;
- spell outcome/gate coverage;
- quest-transition coverage;
- NPC interaction coverage;
- party/boss encounter coverage.

Percentages alone are insufficient. Reports must list concrete uncovered targets and bind evidence to the exact build/content generation.

## 12. Isolation and safety

Synthetic accounts should be identifiable administratively and isolated from production economy/state unless a separately accepted production-safe test design exists.

The swarm must not:

- receive hidden gameplay knowledge unavailable to a player when behavioral fidelity is claimed;
- mutate production/live player state;
- bypass normal authority for a journey counted as player coverage;
- weaken protocol, persistence, security or provenance gates to make tests pass.

## 13. Implementation programme

Initial Jira decomposition:

- `KAN-35` — Player Bot Runtime & Session Supervisor;
- `KAN-36` — Behavior Engine & Realistic Player Profiles;
- `KAN-37` — Coverage Engine for Map, Content & Mechanics;
- `KAN-38` — Party, Group Play & Boss Encounter Bots;
- `KAN-39` — Chaos, Soak, Invariants & Deterministic Replay;
- `KAN-40` — Native Client Bot Cohort & End-to-End UI Journeys;
- `KAN-42` — Agent Context Budget Controller & cross-chat handoff;
- `KAN-43` — Player Swarm Lead alias, programme and repo bootstrap.

The exact order and parallelism are recomputed from live dependencies and path ownership; these Jira stories are decomposition, not permission to mutate.

## 14. Architecture escalation

A material new decision is returned as `ARCHITECTURE_ESCALATION_REQUIRED` through the current control plane to `Oteryn: sol supervising architect`.

Typical triggers include public API/wire/schema/stable-ID changes, persistence/value ownership, security/session/fencing authority, cross-repository responsibility, unaccepted hard resource maxima, permanent content semantics or any privileged bot shortcut that would change player authority.

## 15. Readiness definition

The Player Swarm lead may be invoked after its prompt is merged and registered reusable. Alias availability alone never grants write/merge/production authority.

The programme becomes materially useful in stages:

- **READ_ONLY_READY** — lead can reconstruct state, plan and audit coverage gaps;
- **LONG_SESSION_READY** — context-budget/handoff policy is usable and validated for the execution environment;
- **MUTATING_READY** — the exact current tranche has write allocation/owned paths;
- **COVERAGE_OPERATIONAL** — enough gameplay capabilities exist for real swarm scenarios and measurable coverage.


# Subsystem lanes S1-S9

Owner decision 2026-11-25. Work is split into nine subsystem lanes. Each lane
has an owner session that writes its crates and keeps its invariant file
`docs/architecture/subsystems/<id>.md` (template below). The invariant files
are written by their owners; this README is the map.

| id | subsystem | crates and paths | owner session |
|---|---|---|---|
| S1 | Persistence and authority | `crates/gameplay/durability`, `crates/gameplay/characters`, `crates/gameplay/character-recovery-fence`, `crates/gameplay/admission-evidence`, `crates/gameplay/native-admission-source`, `crates/session`, `crates/identity`, `crates/platform-client`, `crates/platform-contracts`, `apps/game-server/migrations`, `tests/security/auth` | Server |
| S2 | Protocol, transport, dispatch | `crates/protocol-oteryn`, `crates/session-tcp`, `crates/gameplay/dispatch`, `crates/gameplay/foundation`, `crates/gameplay/entities`, `crates/gameplay/domain`, `crates/foundation`, `crates/error-codes`, `crates/diagnostics`, `crates/task-supervisor`, `crates/ntp-clock`, `apps/game-server` (except `migrations` and the S7 qualification module) | Server |
| S3 | Combat, spells, monsters | `crates/gameplay/combat`, `combat-rules`, `spell`, `spell-rules`, `encounters`, `encounter-rules`, `monster-ai`, `progression` (all under `crates/gameplay/`) | Content lane C-combat |
| S4 | Items, economy, death and corpses | `crates/gameplay/items`, `crates/gameplay/economy`, `crates/gameplay/housing` | Content lane C-items |
| S5 | World, NPC, quests, content data | `crates/gameplay/world`, `world-change`, `npc`, `quests`, `social`, `social-rules`, `weather`, `content`, `content-promotion` (under `crates/gameplay/`), `crates/world-bundle`, `content/**`, deploy pins (`deploy/synology-game/npc-data.pin.json`), `tools/world-bundle-compiler` | Content lane C-world |
| S6 | Client | `crates/client-*`, `crates/renderer`, `crates/input-*`, `apps/client`, `tools/dev-client` | Client |
| S7 | Integration and test platform | node boot and qualification (`apps/game-server/src/qualification.rs`, `apps/game-server/src/qualification/**`, `tools/qualification/**`, including `node_boot` and `known_fail_rows.txt`), E2E harness (`tools/qualification/e2e_all`), `tools/architecture-check`, `tools/synthetic-asset-compiler`, `tools/synthetic-client-harness`, `tools/player-bots`, `tests/pre-native-client`, `crates/test-support`, `crates/synthetic-assets`, `crates/simulation-determinism`, `crates/gameplay/node-files`, `crates/gameplay/source-critical-baseline` | E2E |
| S8 | Staging and deploy | `deploy/**` (except the S5 pins), `crates/admin`, `tools/oteryn-game-ops-top`, staging | STG |
| S9 | Platform | `Oteryn/Oteryn-Platform` (another repository) | Platform (frozen except skills) |

The ids S1-S9 name lanes, not mechanics; mechanic ids keep two digits
(S05, S10S). A crate or path that no row names belongs to the lane whose consumers use it
most; ask the coordinator, and add the row in the same change.

## The cross-subsystem rule

A change that touches another subsystem's crate needs that subsystem owner's
review of the consumer map before code. Ask through the coordinator.

The consumer map is the first item of every delegation packet
(`.claude/skills/oteryn-waves/references/delegation.md`, item 0): each
changed symbol, value or behaviour with its callers and consumers, the
invariants touched, the scenario variants (solo and party, every consumer,
error paths) and the tests that assert it, all with `file:line`. The owner
checks the map against the consumers and invariants of its own lane. Work
starts after that review. A risk class (persistence, session fencing,
protocol, durable value, migrations, several consumers) also needs the
reviewed half-page plan of `oteryn-coding`, *Risk classes*.

Writes stay inside the packet's owned paths. Reading callers and tests in other
lanes is allowed within the `docs/agents/CONTEXT_ROUTING.md` read budget.

## Template for `<id>.md`

Keep each file short. Cite `file:line`, not prose.

```markdown
# S<n> <subsystem name>

## Source of truth
The one place each fact of this subsystem is defined (type, table, file,
contract), and what is derived from it.

## Invariants
The rules that must hold, each with the check that enforces it.

## Consumers
Who reads or calls each public symbol, value and table, per consumer lane,
with `file:line`. A change to a listed item starts from this list.

## Persistence boundary
What is written durably, by whom, under which fence, and what is only
runtime state.

## Guard tests
The tests that fail when an invariant breaks, by name and path. Every P0/P1
class found in this subsystem adds one (`oteryn-review` §7).
```

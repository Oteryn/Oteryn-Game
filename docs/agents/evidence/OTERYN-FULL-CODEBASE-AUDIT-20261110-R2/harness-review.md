# Synthetic client harness — complete source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Package: `oteryn-synthetic-client-harness`. All **40 tracked package files / 10,454 physical lines** were read and analyzed manually. `harness-coverage-validation.json` independently checks the source inventory, Git/worktree identity and full line-range closure. Status is **REVIEWED**, not runtime-qualified. No harness test was executed.

## Entry points and actual boundaries

The manifest identifies a synthetic-only tool. Default execution constructs an in-process projection, image, input mapping and renderer surface. Explicit live mode uses the authenticated dev-client session. The CLI reads admission material from a file or environment and validates the complete bounded script before connecting. TLS root, server name and admission deadline remain explicit. Script byte/line/wait limits, strict command parsing, nonzero spell and item identities, closed target forms, geometry and UUID input were examined. The local interactive stdin queue is a trusted local input boundary; no remote producer was inferred for it.

The full live controller and model follow observed snapshots and events for position, vitals, door state, entities, inventory and chat. Accepted command results do not invent later state deltas. Capability gates suppress unsupported item/chat commands; local refusals do not send a command. Checked wide geometry and exact floor comparisons prevent coordinate overflow and floor aliasing. Fixed room/door coordinates are qualification assumptions. Replacement ChatSnapshot handling is incomplete in the harness model, but current server producer and resulting impact remain under investigation; this is **UNKNOWN**, with no confirmed finding assigned.

## Test source analysis

All main live tests and all separately included modules were read completely. Pure tests specify CLI, input, pointer, geometry, bounded history and refused-command behavior. Generated loopback TLS peers exercise dev-client/session codecs and scripted command ordering. In-memory duplex peers cover floor replacement, typed context results, NPC dialogue/shop, battle, target movement and player trade. Each peer authors admission, capabilities, snapshots and outcomes; these scenarios do not establish correctness of the actual game-server authority, PostgreSQL commits or deployed rooms.

The native integration sources drive real play tasks, mailbox reducers, panels and view models with mocked Stepper implementations or direct events. They cover inventory disappearance, nested/ground containers, split quantities, equipment refusals, death/respawn input gating, map selections, ambient light/weather, minimap targets, Market/Stash/Depot request and selection state, Party commands, public/private channels and preference persistence. Presentation assertions inspect actual generated scene sprites and atlas colors. Fixture booleans, counters and authored gold balances are explicitly not value-conservation evidence.

The player-trade qualification adapter imports the existing server-side qualification phase, but runs it over two scripted TLS peers. The source handles acceptance without a new state delta, cancellation with and without an open trade, completed-trade state/inventory order and partner logout. It does not replace a test of two production owners, current durable item locks or real WP5 topology. Included files and imported phases receive no additional coverage credit merely from being referenced.

## Verification and limits

Every package file has a source-specific manual review note and exact byte digest in the lead ledger. Full coverage is deduplicated across an earlier selected-range review of the main test source. Git/worktree/range validation is reported separately from semantic review and is not used as a substitute for reading.

No source changes, full Cargo build, native window launch, real credentials, SQL operation or external live game session were performed. Bounded polling and shutdown paths were inspected in source; some fixture readiness waits rely on their outer scenario and are not independently bounded. There is no claim that the harness compiles or passes on this environment. No additional confirmed vulnerability was established by this package review.

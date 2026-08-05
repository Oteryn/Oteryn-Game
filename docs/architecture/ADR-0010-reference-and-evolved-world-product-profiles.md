# ADR-0010 — Reference and Evolved World Product Profiles

- Status: Accepted
- Date: 2026-08-05
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Decision owner: product owner
- Detailed baseline: `PRODUCT_DIRECTION_BASELINE.md`

## Context

Oteryn v2 requires a coherent product target rather than an unconstrained generic MMORPG engine.

The owner intends to reproduce the important observable gameplay and product behavior of official live Tibia, referred to in this programme as Global Tibia, and then improve or expand areas left neglected, constrained by legacy architecture or insufficiently developed.

The owner also wants players to be able to choose between:

- a world that remains focused on faithful Global Tibia behavioral parity;
- a world that uses the same foundation but contains approved Oteryn improvements and original expansion.

Without an explicit decision, this direction could create protocol forks, duplicated engines, hidden world-name conditionals, incompatible persistence or accidental movement of value between differently balanced economies.

## Decision

### 1. Two product-profile families may coexist

Oteryn v2 may operate logical worlds from two high-level product-profile families:

1. **Reference profile** — targets documented behavioral parity with a named Global Tibia baseline.
2. **Evolved Oteryn profile** — starts from the shared foundation and applies explicit, versioned Oteryn differences and expansion.

Players select a logical world and therefore its declared product profile. The product-profile family must be visible before character creation and gameplay admission.

This decision permits both profiles; it does not require them to launch simultaneously.

### 2. One canonical engine, client and protocol

Both profile families use:

- one canonical Oteryn v2 repository and Rust workspace;
- one authoritative game-domain engine;
- one native Rust client;
- one project-owned gameplay protocol family, `protocol-oteryn`;
- the same identifier, session, persistence, transaction, analytics and security foundations;
- the same World Project, compiler, World Bundle and Oteryn Studio architecture.

A profile difference must not create:

- a second protocol family;
- a separate server or client fork;
- a duplicated repository or workspace;
- hidden branching tied to a world name;
- an unversioned alternate persistence model.

Differences are expressed through versioned ruleset, content, asset and presentation profiles, plus capability-bounded typed domain modules where a later accepted contract permits them.

### 3. Every logical world declares a versioned profile assignment

Each logical world must be associated with explicit, versioned concepts equivalent to:

- product profile identity and revision;
- ruleset identity and revision;
- content revision;
- asset/presentation revision where relevant;
- client/server/protocol capability compatibility requirements.

These are semantic requirements, not accepted Rust type names, schemas or wire fields. Exact representations remain for `FND-ID-01`, `FND-02`, `DUR-04`, `ALPHA-RULESET-01` and `PROD-COMPAT-01`.

A world may not silently switch between reference and evolved behavior. Profile upgrades require compatibility analysis, migration, validation and rollback.

Reference and evolved profiles use distinct logical `WorldId` values. Every channel belonging to one logical world inherits the same product-profile, ruleset and compatible content revision; changing channel cannot change the product-profile family. An instance may apply explicitly bounded encounter modifiers, but it may not become a hidden cross-profile bridge or a differently balanced economy.

### 4. Reference parity is measured and explicit

A reference world targets player-observable behavioral and product parity with a named, versioned Global Tibia target definition. A continuously tracking policy, if later selected, must still materialize immutable reference revisions so every parity claim names the exact observed target.

Every evaluated behavior must eventually be classified as one of:

- `PARITY_CONFIRMED`;
- `PARITY_PENDING_EVIDENCE`;
- `INTENTIONAL_DIFFERENCE`;
- `TECHNICAL_OR_LEGAL_SUBSTITUTION`;
- `OUT_OF_SCOPE_FOR_CURRENT_MILESTONE`.

“1:1” is therefore a scoped evidence claim, not permission to guess hidden behavior or to copy proprietary implementation or assets.

The choice between a pinned reference release, a dated behavior baseline or continuous tracking remains deferred to `GAME-VISION-01` or a dedicated parity contract.

### 5. Evolved behavior is intentional and reviewable

Every evolved-profile difference requires:

- an explicit owner and rationale;
- named affected systems and state scopes;
- compatibility and migration impact;
- balance, economy, security, performance and operational analysis where applicable;
- deterministic tests and client-visible acceptance;
- rollback or safe failure behavior.

An undocumented divergence is a defect, not an Oteryn improvement.

### 6. Identity may be shared; gameplay value is isolated by default

The reusable Platform account and Identity boundary may be shared across profile families.

Unless a later dedicated contract explicitly permits transfer or sharing:

- characters are world-scoped;
- progression is world-scoped;
- inventories, currencies, depots, banks, houses and market state are world-scoped;
- guilds and rankings are world-scoped;
- rewards cannot be earned in one profile and redeemed in the other;
- items, currency, characters and economy state cannot transfer between profile families;
- account-wide entitlements, cosmetics and achievements are not assumed portable.

The default isolation must be enforced by admission, durable transactions, administration and analytics. It exists to prevent balance arbitrage, duplication and profile-boundary confusion.

### 7. Testing is profile-aware

The test strategy must include:

- reference parity fixtures and divergence reports;
- evolved-profile tests that prove intended differences;
- cross-profile isolation tests for sessions, characters, items, currency, rewards, market, houses, content caches, administration and analytics;
- regression protection against process-global mutable state leaking profile behavior;
- E2E evidence using the shared ADR-0007 platform.

### 8. Game Intelligence may compare profiles but cannot govern them autonomously

ADR-0006 remains binding.

Game Intelligence may compare privacy-safe aggregate signals across profile families for balance, economy, retention, integrity and world-health analysis. It may not autonomously mutate gameplay, deploy a ruleset, move players, punish accounts or declare causal product conclusions.

### 9. Legal and provenance boundaries remain mandatory

Behavioral parity does not authorize copying proprietary source code, binaries, protocols, databases, maps, quests, text, graphics, audio, trademarks or protected content files.

Oteryn v2 uses independently implemented code, `protocol-oteryn`, and content/assets with verified rights and provenance. Legacy repositories and data are bounded reference or migration evidence under the accepted ADRs, not target-runtime dependencies.

## Consequences

### Positive

- Players may choose fidelity or innovation without fragmenting the technology stack.
- The reference profile provides a measurable baseline for validating engine correctness.
- The evolved profile provides a controlled path to improve neglected systems.
- One protocol and domain engine prevent compatibility forks and duplicate maintenance.
- Hard world/profile isolation limits economic arbitrage and duplication risk.
- Game Intelligence can compare outcomes while preserving human product authority.

### Costs and risks

- Two profile families can fragment population.
- Content, test and support workload can grow substantially.
- A moving Global Tibia target can make parity ambiguous.
- Shared code can accumulate uncontrolled conditionals if ruleset boundaries are weak.
- Different economies make transfer policy security-sensitive.
- Public branding can create affiliation or trademark confusion.

Mitigations and detailed operating rules are recorded in `PRODUCT_DIRECTION_BASELINE.md` and must be refined by later contracts.

## Rejected alternatives

### Separate server/client/protocol forks

Rejected because they duplicate ownership, multiply defects and violate the accepted one-workspace and one-`protocol-oteryn` direction.

### One world that mixes reference and evolved behavior without a declared profile

Rejected because players cannot understand the product contract and tests cannot distinguish defects from intentional differences.

### Cross-profile item/currency transfer by default

Rejected because balance differences create arbitrage, duplication and reconciliation risk.

### Generic engine first, product identity later

Rejected because foundational abstractions, content tooling and client UX would be designed without a concrete observable target.

## Deferred decisions

- exact Global Tibia version or behavior baseline;
- continuous tracking versus pinned reference worlds;
- whether both profiles launch at the same milestone;
- first approved Oteryn improvements;
- public names and branding;
- profile-specific release cadence and LiveOps policy;
- transfer, entitlement, cosmetic and achievement portability;
- exact content inheritance/delta representation;
- population thresholds, world merge and retirement policy.

## Required follow-up gates

- `GAME-VISION-01` — define measurable product pillars, parity scope and first-launch strategy;
- `FND-ID-01` — define profile/ruleset/revision identity semantics where needed across boundaries;
- `FND-02` — carry profile/ruleset/content compatibility through one `protocol-oteryn` family;
- `FND-04` and the Platform World Registry/Game Gateway contract — expose and bind the selected `WorldId` and compatible profile revision so admission cannot cross profile families;
- `DUR-02` and `DUR-03` — enforce world/profile-scoped durable character and economy state;
- `DUR-04` — define versioned content/ruleset source and runtime bundle relationships;
- `ALPHA-RULESET-01` — define typed policy boundaries without engine forks;
- `PROD-COMPAT-01` — define client/server/protocol/content/profile compatibility;
- `GAME-WORLD-LIFECYCLE-01` — govern creation, upgrade, transfer, merge, retirement and archival;
- `QA-E2E-01`/ADR-0007 — provide profile-specific and cross-profile evidence.

## Programme effect

- `FND-01` remains the immediate next action.
- This ADR does not authorize implementation by itself.
- This ADR does not block the controlled client migration because both profiles share the same canonical foundation.
- Broad gameplay/content design must not contradict this profile model while `GAME-VISION-01` remains unresolved.

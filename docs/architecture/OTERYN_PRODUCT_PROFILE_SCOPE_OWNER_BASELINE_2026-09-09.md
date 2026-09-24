# Oteryn Product Profile Scope — Owner Baseline

- Status: **OWNER_ACCEPTED SCOPE CLARIFICATION**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@85ba329ccbdf51338a672fcac8ca0836726dc71b`
- Scope: product-profile identity, precedence, and classification of existing accepted architecture
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Purpose

Oteryn is the name of the whole product and shared technology stack. It is **not** a synonym for the custom/Evolved gameplay profile.

The product has two explicit profile families:

1. **Oteryn Reference** — the default fidelity profile. Its player-observable gameplay semantics reproduce the selected named Global Tibia Reference baseline, subject only to explicit safety/legal/technical substitutions or separately accepted Reference differences.
2. **Oteryn Evolved** — the Oteryn-designed profile. It starts from the same shared foundation and may apply explicit, versioned Oteryn gameplay, balance, content, economy, UX and system changes.

The two profiles use one canonical engine, native client and `protocol-oteryn` family. They are profiles/rulesets/content revisions, not forks.

This baseline exists because later architecture checkpoints sometimes used the unqualified word **Oteryn** when describing new gameplay behavior. That wording must not be interpreted as permission to apply an Evolved mechanic to Reference.

## 2. Owner clarification recorded on 2026-09-09

The owner explicitly clarified that Oteryn must distinguish:

- a **default profile** that reproduces Tibia behavior; and
- a **second Oteryn profile** where Oteryn-specific versions of systems are designed.

For architecture terminology this baseline names those families:

```text
Oteryn
├── Oteryn Reference   # default fidelity lane
└── Oteryn Evolved     # explicit Oteryn-designed lane
```

These are canonical architecture/product-profile labels. Final public marketing wording, trademark-sensitive copy and presentation remain subject to the existing branding/legal boundary.

## 3. Binding precedence

This baseline refines and composes, rather than replaces, the accepted profile model in:

- `ADR-0010-reference-and-evolved-world-product-profiles.md`;
- `PRODUCT_DIRECTION_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md`;
- `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`.

For future interpretation the following order applies.

### 3.1 Shared safety and authority invariants

Accepted security, integrity, durability, authority, provenance, legal, anti-duplication, fencing and recovery invariants apply to both profiles unless an owning contract explicitly narrows them.

Reference parity never requires reproducing an unsafe implementation defect, duplication path, stale-writer bug, security weakness, proprietary implementation or other forbidden behavior.

### 3.2 Reference gameplay authority

For `Oteryn Reference`, the selected immutable named Reference revision is the gameplay oracle.

A product-direction statement, architecture checkpoint, Evolved recommendation or unqualified use of the word `Oteryn` does not override Reference parity.

A player-observable difference may enter Reference only when one of these is true:

- it is confirmed behavior of the selected Reference revision;
- it is an explicitly accepted and disclosed Reference difference;
- it is a required safety/legal/technical substitution under the existing Reference rules.

### 3.3 Evolved gameplay authority

For `Oteryn Evolved`, explicit owner-approved Oteryn differences may supersede the Reference mechanic for that Evolved profile revision.

Every such difference remains versioned, testable and profile-scoped. It must not become a process-global mutable switch, hidden world-name conditional, protocol fork or accidental Reference behavior.

## 4. Classification rule: SHARED / REFERENCE / EVOLVED

Every product-sensitive architecture decision must be interpretable as one or more of the following scopes.

### `SHARED`

Applies to both profile families because it concerns common technology, safety, authority or semantics that do not intentionally change the selected Reference gameplay meaning.

Typical examples:

- one canonical Rust engine/workspace;
- one native client;
- one `protocol-oteryn` family;
- `WorldId` / `ChannelId` identity separation;
- profile/ruleset/content revision identity and compatibility;
- session-generation fencing;
- server authority;
- anti-duplication and value conservation;
- durable transaction idempotency and reconciliation;
- crash resistance, diagnostics and semantically neutral UX/accessibility quality;
- provenance, security, privacy and legal boundaries;
- profile/world isolation of characters, items, currencies and economy.

### `REFERENCE`

Applies to the default fidelity profile because the behavior is sourced from the selected named Global Tibia Reference revision or an explicit Reference-only substitution/difference.

This includes the actual Reference semantics for:

- character progression and skills;
- death, losses, blessings/protection and recovery;
- PvP rules;
- party/shared-XP rules;
- combat and vocation behavior;
- item/equipment/loot rules;
- economy sources/sinks/scarcity mechanics;
- quests, NPCs, creatures, bosses and content behavior;
- housing player-facing behavior;
- client interaction behavior where it is part of the declared parity surface.

Until the exact first Reference revision is selected, concrete Reference mechanics that depend on that choice remain fail-closed/deferred rather than guessed.

### `EVOLVED`

Applies only to the Oteryn-designed profile because it intentionally changes player-facing gameplay/product behavior beyond Reference.

Typical examples include:

- new progression or death systems;
- new Rested systems;
- changed PvP or party reward philosophy;
- new housing classes or ownership rules beyond Reference;
- changed economy/source/sink/scarcity policy;
- original content and progression systems;
- profile-specific UX that intentionally changes gameplay meaning;
- later original Oteryn balance and endgame systems.

## 5. Audit of current accepted product/profile architecture

The following classification is binding for interpretation of the existing accepted material. A document may contain clauses in more than one scope; scope attaches to the semantic clause, not merely the filename.

| Existing authority / decision | Scope after this clarification | Binding interpretation |
|---|---|---|
| `ADR-0010` one engine/client/protocol, versioned profile assignment, distinct profile-family `WorldId`, cross-profile value isolation | `SHARED` | Foundation for both profiles. Does not make Evolved gameplay shared. |
| `PRODUCT_DIRECTION_BASELINE.md` dual-profile model | `SHARED` | Oteryn is the umbrella product; Reference and Evolved are sibling profile families. |
| `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` | `REFERENCE` | Oteryn Reference is the default/first externally evaluated profile. Evolved follows later. |
| `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md` | `REFERENCE` | Global changes are observed continuously but enter Reference only through explicit immutable Reference revisions. |
| `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md` | `SHARED` precedence rule + `REFERENCE` gameplay authority | Reference parity outranks future-facing Evolved product preferences inside Reference. |
| `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md` | `EVOLVED`, except semantically neutral quality improvements which are `SHARED` | First Evolved differentiation remains reliability/UX-first; shared quality must not be withheld from Reference. |
| `GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md` | `EVOLVED` product direction; Reference mechanics remain `REFERENCE` | It does not authorize changing Reference PvP behavior. |
| `GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md` | `EVOLVED` product direction; Reference mechanics remain `REFERENCE` | It does not authorize changing Reference party/shared-XP mechanics. |
| `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` core authority/conservation/evidence principles | `SHARED` | Its own Reference/Evolved carve-outs remain binding. |
| Multichannel engine/world topology and `WorldId`/`ChannelId` separation | `SHARED` infrastructure | Shared technology may host both profile families. Infrastructure scaling must not duplicate durable gameplay value. |
| Channel-specific PvP selection/switching where it changes player-observable Tibia behavior | `EVOLVED` unless separately admitted as a Reference difference | The existence of shared Channel infrastructure does not itself authorize Reference gameplay divergence. |
| `EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` durability, one-authority, fencing, one-location item/value conservation and no per-Channel duplication | `SHARED` | These are safety/topology constraints over the common multichannel engine. |
| `EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` player-facing hybrid Residence/physical-house model and other non-parity housing product rules | `EVOLVED` by authority | These rules do not override Reference housing. A specific clause may apply to Reference only if parity-confirmed or explicitly accepted as a Reference difference. |
| Draft #295 positive Rested XP pool | `EVOLVED` | It must not change Reference stamina/rest semantics. |
| Draft #295 no-delevel achieved levels / `DeathDebt` / `DeathExhaustion` / corpse XP recovery redesign | `EVOLVED` | Reference death/progression remains owned by the selected Reference revision. |
| #295 candidate `15% nominal / 45% DeathDebt cap / 7.5% permanent + 7.5% recoverable / 30 min` | `EVOLVED` candidate only | Still not numerically owner-frozen; no Reference applicability. |
| Future Evolved source/sink/scarcity/balance changes | `EVOLVED` | Must be explicit revisions; no cross-profile economy arbitrage. |
| Server authority, anti-cheat evidence integrity, durability, recovery, observability and legal/provenance restrictions | `SHARED` | Reference does not intentionally inherit unsafe implementation defects. |

## 6. Housing clarification

`EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md` remains accepted architecture, but this later scope baseline narrows how it is consumed by profile-sensitive work.

### 6.1 Shared housing architecture

The following classes of housing requirements remain `SHARED` because they protect the common multichannel/value model rather than intentionally redesign Tibia gameplay:

- one authoritative durable item/value location;
- no item or house duplication because Channel count changes;
- revision/generation fencing and stale-writer rejection;
- idempotent/reconcilable settlement;
- one authoritative active mutation owner for a mutable house interior;
- fail-closed risky mutations when authoritative custody cannot be proven.

### 6.2 Reference housing

`Oteryn Reference` must use the selected Reference revision as its player-facing housing oracle.

The accepted Oteryn hybrid package cannot silently add or replace Reference behavior such as:

- Residence/apartment availability;
- `NONE | RESIDENCE | PHYSICAL_HOUSE` personal-slot semantics;
- Oteryn-specific acquisition eligibility;
- GUI-only ACL administration;
- Oteryn-specific Bazaar/property disposition;
- changed Rested benefits;
- any other player-observable housing rule not established by the selected Reference revision.

If a #447 clause happens to match the selected Tibia baseline, Reference may implement that behavior from parity authority. #447 is not required as the reason for the Reference behavior.

### 6.3 Evolved housing

The owner-accepted #447 player-facing hybrid housing package is an `EVOLVED` architecture authority unless an individual clause is `SHARED` under section 6.1.

This preserves the work already completed on #447 while preventing it from becoming an accidental Reference override.

## 7. Rested / death / recovery clarification

The architecture direction recorded in draft PR #295 and Issue #220 for:

- positive per-Character Rested XP;
- `EligibleRawXP`-based Rested accounting;
- no delevel below an achieved Oteryn level;
- overflow `DeathDebt`;
- bounded `DeathExhaustion`;
- recoverable corpse XP;
- one active recoverable-XP claim;
- the candidate `15 / 45 / 7.5+7.5 / 30 min` package;

is now explicitly **EVOLVED-only**.

It does not supersede Reference progression, death, stamina, blessing, loss or corpse behavior.

The numeric package remains a candidate until separately owner-frozen for Evolved. This scope clarification resolves *where* it may apply, not *which exact numbers* are final.

## 8. Multichannel and PvP clarification

The multichannel runtime model is a shared Oteryn technology invariant.

However, shared infrastructure and profile gameplay are separate concerns:

```text
SHARED
WorldId -> ChannelId topology, routing, fencing, recovery, capacity

REFERENCE
selected Reference revision's actual PvP/risk/economy semantics

EVOLVED
Oteryn-specific Channel/PvP selection or switching semantics that differ from Reference
```

A Reference world may use Oteryn's multichannel infrastructure as a technical implementation while preserving its declared Reference gameplay contract. Infrastructure must not become a hidden way to add an Evolved reward/risk/economy rule to Reference.

## 9. Economy and progression clarification

For Reference:

- player-observable progression and mechanical value source/sink rules come from the selected Reference revision;
- Oteryn world history produces its own prices, wealth distribution and liquidity;
- integrity failures are never valid parity behavior.

For Evolved:

- progression, risk, Rested, death, source/sink, scarcity and reward rules may diverge only through explicit versioned owner decisions;
- the first Evolved package remains reliability/UX-first, so broad balance/economy redesign is not implicitly activated by this clarification.

## 10. Ambiguous historical wording rule

After this baseline, an older document saying only **Oteryn** must not be interpreted using a simplistic rule such as `Oteryn = Evolved` or `Oteryn = both profiles`.

Instead:

1. technology/safety/authority/integrity semantics are `SHARED` when they do not intentionally change Reference gameplay meaning;
2. named or parity-sourced Reference gameplay semantics are `REFERENCE`;
3. an intentional player-observable Oteryn redesign beyond the Reference baseline is `EVOLVED` unless a separate accepted Reference-difference authority exists;
4. if scope still cannot be determined safely, fail closed and require explicit classification before implementation.

New architecture should avoid unqualified profile-sensitive wording. It should say `Oteryn Reference`, `Oteryn Evolved`, or `both profiles / SHARED` as appropriate.

## 11. Test and implementation consequences

Future implementation and tests must prove profile isolation, including at minimum:

- the same shared engine can execute Reference and Evolved rules without process-global leakage;
- a Reference fixture cannot accidentally activate an Evolved policy;
- Evolved differences are explicit in profile/ruleset/content revision identity;
- `WorldId` boundaries prevent character/item/currency/progression/economy transfer between profile families unless a later explicit contract permits it;
- changing Channel cannot change profile family;
- parity tests treat undocumented Reference divergence as a defect;
- Evolved tests prove the intended difference rather than merely accepting divergence.

Exact Rust types, wire fields, schemas and implementation modules remain owned by their existing downstream gates.

## 12. Effect on currently open architecture PRs

This scope clarification does not seize or rewrite existing branch ownership.

- Draft `#295` remains the existing Rested/death checkpoint lineage; its gameplay design is interpreted as `EVOLVED` only.
- Draft `#228` remains independent anti-bot open analysis.
- Historical housing checkpoint PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445`, `#446` remain provenance. Their composed housing authority is protected `#447`; they should not become competing long-lived profile authorities.

No runtime/client/server/protocol/DDL/Platform/Atlas implementation is authorized by this document.

## 13. Required follow-up discipline

Before any profile-sensitive implementation:

1. classify the relevant rule `SHARED`, `REFERENCE` or `EVOLVED`;
2. identify the exact profile/ruleset/content revision authority;
3. for Reference, identify the selected named Reference revision or explicit Reference difference;
4. for Evolved, identify the explicit owner-approved difference/gate;
5. test cross-profile isolation and absence of behavior leakage;
6. preserve one engine/client/protocol and normal protected integration controls.

The exact first Global Tibia Reference baseline remains a separate required decision before broad concrete Reference mechanics can be implemented.

## 14. Acceptance boundary

This document accepts and freezes **profile scope and precedence**, not new runtime gameplay implementation.

It establishes:

- Oteryn as the umbrella product;
- `Oteryn Reference` as the default fidelity profile;
- `Oteryn Evolved` as the explicit Oteryn-designed profile;
- the `SHARED / REFERENCE / EVOLVED` interpretation rule;
- explicit Evolved-only scope for the #295 Rested/death redesign;
- shared-vs-Evolved decomposition of #447 housing;
- shared multichannel technology without automatic Reference gameplay divergence;
- fail-closed handling of ambiguous historical profile-sensitive wording.

It does not select the exact first Reference revision, freeze the #295 numeric candidate, authorize implementation, permit cross-profile value transfer, or weaken any protection/review/CI/Merge Queue requirement.

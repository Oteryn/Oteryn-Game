# Gameplay domain — full source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: REVIEWED. The combined gameplay-worker and lead ledgers cover all 67 tracked package files. The lead completed the remaining Echo Warden, Hireling, Soulpit, timed-item and Weapon Proficiency scope after the worker stopped at an authentication gate. No source changes were made.

## Responsibility and authority

This package provides value types, interpretation rules, deterministic plans and immutable content data. It does not establish a live actor, Session or database write authority. Typed UUIDv7 identifiers, expected revisions and private validated shapes prevent several local invalid states; current ownership, admission, inventory custody, transaction outcome and runtime observations remain the responsibility of their callers.

The earlier worker's complete-file analysis covers lifecycle and containment graphs, build/progression/death arithmetic, names and mail bounds, appearance provenance, equipment claims, Premium, currency planning, interaction lifecycle, conditions, Bestiary/Bosstiary, charms, forge and Cyclopedia. Exact substantive notes are retained in its ledger. Revision successors are checked before mutation; containment insertion checks cycles and ancestor/subtree limits; equipment resource claims are symmetric; appearance grants require compatible provenance. Interaction acceptance retains Pending until the foreign owner produces an actual terminal outcome, with exact operation/fence reconciliation and bounded accepted plans. Ordinary value construction does not itself authorize mutation.

## Completed lead scope

Echo Warden rules distinguish tagged-Warden rewards from ordinary spawn exclusions, bound every supplied random draw, check portal timing and health multiplication, preserve variant-specific attack scaling, and constrain aura enlistment by species, floor and range. These are pure rules; no deployed spawn distribution or live carrier behavior is inferred.

Hireling dialogue checks the current skill or owner on the relevant service transition, resets state on farewell, and requires explicit confirmation of one selected package. Trade pages preserve source order with checked offsets and the existing protocol limit. Wrapped furniture follows the explicit decoration-kit mapping. Food selection uses bounded server draws and explicit recovery/buff behavior; valid persistent buffs represent remaining online duration. One independently confirmed economy defect is retained in the private finding packet as GR-ECON-001. Public evidence intentionally omits the actionable item/price/transaction sequence.

Soulpit admission validates actual staging slots, participant uniqueness, minimum levels and occupancy before the domain plan. Wave rules prioritize timeout/abandonment, separate spawn delay from clearance, and keep summons in the live population without granting ordinary experience. Restore validates version, masks, clock order, terminal state, every acknowledged principal identity, wave membership and live/seen consistency. Replayed spawns and deaths do not recreate monsters; rejoin requires a surviving active encounter. Restart interruption is an explicit rule, not an invented refund guarantee.

Soul-core conversion validates the selected candidate before any replacement branch, preserves the specified no-reroll behavior and checks capacity in the prescribed order. Input eligibility, possible outputs and literal source name parsing intentionally differ. Fixed passages distinguish stepping from use and leave arena return to the existing Instance handoff owner. Source monster registration names, final display names, numeric races and human-readable classes remain separate fields; these source facts do not admit native Creature identities.

Timed-item lanes preserve one in-flight write, its transaction ID and expected revision, including retries and ambiguous outcomes. A live zero immediately deactivates the item; expiry waits for the previous checkpoint and uses its resulting revision. Unknown outcomes hold the lane, stale authority stops all actor lanes, and an unexpected expiry revision is retried once before freezing. Stop checkpoints preserve the final live values. The host checks the complete live-item count and duplicate set before opening lanes, skips spent rows, dispatches each write once, and retains a committed expiry until its timed target is continued or released. Production SQL/TLS lifecycle execution was not performed.

Weapon Proficiency preserves retained progress above a newly lowered compatible cap, derives levels from bounded shape tables, validates stored selection indices, and requires current track revision and PZ where applicable. Kill credit uses the death owner's frozen contributor facts and bounded time window. Modification plans bind costs and known shaping cells, exclude the selected perk or current entry from the relevant draws, and preserve pending offers across compatible retained revisions. The production shaping source validates pool shape; the actual durability reader validates stored rank/offer bounds before the planner's indexing. This counterguard prevents treating arbitrary malformed in-memory test values as a demonstrated external panic path. Content/revision activation and value balances are rechecked by the durable owner.

## Generated-data review and validation

Generated Rust tables received complete semantic review rather than a literal-by-literal reading claim. A strict parser rejects unparsed row content and checks every consumed field against the captured data, along with numeric bounds, order, uniqueness, key closure and lookup semantics:

- 1,875 source monster records and all 819 Bestiary indices match the pinned capture and canonical runtime digest. First-wins race collisions have equivalent observable output; class text is not substituted for numeric race.
- All 835 soul-core IDs and names match the captured catalogue; binary-search ordering and the deliberate malformed-name/input-output distinctions were checked.
- All 1,675 trade rows and 23 three-item packages satisfy their schema, source order, category counts and item closure; generated bytes match the Hireling profile capture. The complete 18-entry wrapped mapping was checked. Separate private analysis covers the economic invariant.

The historical source-roster document contains a whole core-profile-file hash that differs from the current profile file. The consumed core ID/name catalogue remains exactly equal. This is recorded as provenance drift, with both exact hashes, without inferring a gameplay defect. Result and limitations: `domain-catalogue-validation.json`.

Executed tests: **11 Echo Warden tests PASS** and **21 timed-item lane tests PASS**, directly compiled from their unchanged source with rustc. No boundary logic was substituted in these two runs. The separate timed-item property tests, Soulpit/serde tests, proficiency/sha2 tests and full domain Cargo suite were inspected but not executed. Logs are retained in the private checkpoint.

## Findings and remaining validation

GR-ECON-001 is confirmed by the separate real-caller, custody and writer analysis; GR-SOUL-001 is a caller-ordering defect rather than a defect in the domain's Premium cap rule. No other new confirmed P0/P1 finding was established in this package review. Generic caller bounds for raw interaction builders/numeric helpers and live integration schedules remain explicit cross-module validation work. REVIEWED means the package source/data scope is complete; it does not mean the whole program or every runtime scenario is VERIFIED.

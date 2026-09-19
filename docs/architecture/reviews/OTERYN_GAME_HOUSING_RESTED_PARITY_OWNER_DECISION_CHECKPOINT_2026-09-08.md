# Oteryn Game — Housing Rested Parity Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected first-generation Rested direction for personal housing while deliberately keeping numeric balance tunable.

The product goal is to avoid making scarce Premium-gated physical houses a direct progression-power purchase while preserving room to tune Rested, bed counts, social utility and housing balance later using evidence.

## Owner-selected baseline

`OWNER_SELECTED`.

For first-generation Oteryn, a Character using an eligible bed/rest function in a baseline Residence and a Character using an eligible bed/rest function in an ordinary physical house should receive the same baseline Rested recovery semantics.

Conceptually:

```text
Residence eligible rest      -> baseline Rested recovery
Physical-house eligible rest -> same baseline Rested recovery
```

Physical-house scarcity, Premium acquisition gating, location, prestige, size, decoration and social capacity do not by themselves authorize a higher Character Rested multiplier.

## Deliberate reversibility

This decision freezes the anti-pay-to-progress direction, not numeric balance.

The following remain tunable without changing the core housing topology or ownership model:

- exact Rested recovery rate;
- exact Rested pool size;
- exact time required to recover value;
- number of beds available in a Residence or physical house;
- guest/manager bed permissions;
- whether all beds provide identical utility;
- interaction with inns, public recovery sources, events or other Rested sources;
- Premium convenience or cosmetic features that do not create a materially higher progression ceiling.

A later owner decision may change numeric rates or bed availability after telemetry/user-testing. Any proposal to make physical-house ownership itself provide materially stronger progression power requires an explicit owner supersession rather than being inferred from Premium status.

## Relationship to housing classes

This checkpoint preserves the selected personal-housing exclusivity model:

```text
AccountId + WorldId personal housing slot =
    NONE
  | RESIDENCE
  | PHYSICAL_HOUSE
```

Therefore Rested does not stack from simultaneously owning both personal housing classes on one Account/World.

Guildhouse Rested semantics remain deferred to the future guild-system architecture.

## ACL / bed access boundary

The selected housing ACL GUI remains authoritative for access administration.

This checkpoint does not require Tibia-style text commands. Later bed permissions may be represented as explicit GUI-managed capabilities such as `USE_BED`, but exact capability naming/schema remains deferred.

Revoked access must not leave stale authority to start a new rest/sleep operation. Exact behavior for a Character already sleeping when access is revoked remains a later lifecycle decision.

## Rested ownership / durability boundary

Rested state remains Character-scoped progression state under the broader Rested architecture. Housing is a recovery source/context, not the owner of the Character's Rested value.

Channel switching, reconnect, house-runtime relocation or residence-runtime relocation must not duplicate, reset or multiply Rested recovery.

Exact persistence schema and reconciliation implementation remain deferred.

## Decision

`RESIDENCE VS PHYSICAL HOUSE BASELINE RESTED: PARITY`

`PHYSICAL HOUSE PREMIUM/SCARCITY: NO AUTOMATIC HIGHER RESTED MULTIPLIER`

`NUMERIC RESTED VALUES / BED COUNTS / PERMISSION DETAILS: TUNABLE AND DEFERRED`

`FUTURE MATERIAL PROGRESSION ADVANTAGE FROM PHYSICAL HOUSE: REQUIRES EXPLICIT OWNER SUPERSESSION`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.

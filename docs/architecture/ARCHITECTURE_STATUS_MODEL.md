# Oteryn v2 Architecture Status Model

- Status: Accepted coordination model
- Date: 2026-08-10
- Purpose: separate architectural acceptance from task delivery and real implementation evidence.

## Why this exists

Oteryn has several architecture gates that are accepted and lifecycle-closed while their runtime implementation is intentionally not started. A single status such as `CLOSED` is therefore ambiguous and can be misread as product completion.

All actively maintained architecture status documents should distinguish three independent axes.

## DecisionStatus

| Value | Meaning |
|---|---|
| `PROPOSED` | idea recorded; no decision authority |
| `CANDIDATE` | reviewed candidate awaiting acceptance |
| `ACCEPTED` | binding architecture for its declared scope |
| `SUPERSEDED` | an explicit later decision replaces the named scope; historical evidence remains |

## DeliveryStatus

| Value | Meaning |
|---|---|
| `PLANNED` | registered future gate; no active task or PR currently owns delivery |
| `OPEN` | concrete task/delivery work is active |
| `IN_REVIEW` | PR/audit/CI closeout is in progress |
| `MERGED` | delivery merged but lifecycle bookkeeping may remain |
| `LIFECYCLE_CLOSED` | task archived, ownership released and delivery bookkeeping complete |

`OPEN` must not be used merely because a gate is required later. Use `PLANNED` until a concrete active task/delivery exists.

## ImplementationStatus

| Value | Meaning |
|---|---|
| `NOT_STARTED` | no runtime implementation claimed |
| `EXPERIMENTAL` | bounded spike/prototype only; not product authority |
| `IMPLEMENTED` | implementation exists for the stated scope |
| `PROVEN` | named tests/E2E/failure evidence proves the stated implementation behavior on an exact revision |
| `PRODUCTION_ENABLED` | separately authorized production rollout is active |

## Rules

1. One axis never implies another.
2. `DecisionStatus=ACCEPTED` does not imply code exists.
3. `DeliveryStatus=PLANNED` means no active delivery task/PR exists for that gate.
4. `DeliveryStatus=OPEN` requires a concrete active delivery record.
5. `DeliveryStatus=LIFECYCLE_CLOSED` does not imply runtime implementation.
6. `ImplementationStatus=IMPLEMENTED` is not enough to claim production readiness.
7. `PROVEN` requires named evidence tied to an exact revision.
8. `PRODUCTION_ENABLED` requires separate production authority; repository merge authority is insufficient.
9. Historical ADRs are not rewritten merely to retrofit this vocabulary. Current overlays and new maintained documents use it and explicitly supersede stale wording where required.

## Examples

### FND-03 after architecture acceptance but before server runtime

```yaml
DecisionStatus: ACCEPTED
DeliveryStatus: LIFECYCLE_CLOSED
ImplementationStatus: NOT_STARTED
```

### Registered future gate without an active task

```yaml
DecisionStatus: PROPOSED
DeliveryStatus: PLANNED
ImplementationStatus: NOT_STARTED
```

### Bounded QUIC library spike

```yaml
DecisionStatus: ACCEPTED
DeliveryStatus: OPEN
ImplementationStatus: EXPERIMENTAL
```

This means the dual-transport architecture is accepted but the selected library is not.

### Future proven transport adapter before rollout

```yaml
DecisionStatus: ACCEPTED
DeliveryStatus: LIFECYCLE_CLOSED
ImplementationStatus: PROVEN
```

Production remains separately disabled until rollout authority exists.

## Current-status presentation

Where compact tables cannot carry all three fields, wording must still preserve the distinction, for example:

`ACCEPTED / LIFECYCLE_CLOSED / RUNTIME NOT STARTED`.

Do not use `DONE`, `COMPLETE` or `CLOSED` alone when readers could reasonably interpret it as implemented product behavior.

# OTV2 WP3-A — Q01-Q75 playable-first transition disposition

Date: 2026-09-16
Status: `PROSPECTIVE_NOT_ACTIVE`
Repository: `Oteryn/Oteryn-Game`
Owning control plane: `OTV2_WORK_DELIVERY_COORDINATOR` / #162
Companion amendment: `OTV2_WP3_A_UPSTREAM_FIRST_ACCEPTANCE_ALLOCATION_20260916.md`

This companion records the minimum scheduling disposition required by protected
#634 for the canonical `WP3-Q01..WP3-Q75` obligations. It does not mark any cell
PASS and grants no implementation, Child-B, integration or merge authority.

The historical matrix remains evidence. The protected playable-first policy
changes which evidence belongs on the immediate WP3-A critical path; it does not
silently weaken correctness, security, durability, compatibility or the still-
binding current resource/finality safety floor.

## 1. Protected first-slice profile replacements

The following historical positive-path obligations do not describe the selected
WP3-A production profile and therefore do not require recreation of their old
custom-owner implementation merely to close WP3-A:

- `WP3-Q16` — UDS: protected first-slice production routing is literal-IP TCP.
  Preserve ordinary upstream UDS compatibility and run affected upstream
  regressions if a retained SQLx seam touches shared code, but generic UDS
  resource-ownership instrumentation does not gate this product slice.
- `WP3-Q17` — DNS-name transport: protected first-slice transport is literal IP
  with a separately supplied TLS DNS identity. Preserve ordinary upstream DNS
  behavior; generic DNS transport accounting is not the product path.
- `WP3-Q24` — positive TLS1.2 full/resumed lifecycle: protected production root
  is TLS1.3-only. WP3-A replaces the historical positive TLS1.2 path with an
  exact TLS1.2 downgrade/rejection negative. A later protected product profile
  that permits TLS1.2 reactivates the original Q24 lifecycle obligation.

These are not PASS claims. They are profile reachability dispositions. If exact
source/feature evidence makes one of those paths reachable in the selected
production root, it returns immediately to `CURRENT_BLOCKER`.

## 2. WP3-B measured triggers

The following cells are representative performance/load work and must not force
dependency customization before the real playable path exists:

- `WP3-Q44` — comparable setup/steady-state/p99/CPU/allocation/churn measurement;
- `WP3-Q68` — runtime driving/scheduling under representative crypto/parser load.

Trigger both when #162 verifies an exact protected Game revision containing the
real production-shaped
`login/session -> character -> transport -> world/map -> gameplay -> persistence -> reconnect/restart`
path, including at least one genuine command -> durable effect -> visible
projection across reconnect/restart, plus an authorized reproducible workload and
exact dependency/environment pins.

Earlier re-entry is mandatory for a reproducible current correctness/security
failure, current hard resource-floor violation, representative failure of an
accepted SLO, or a dependency change affecting a retained minimal seam.

## 3. Composed Child-B triggers

The complete consumer-side form of these cells requires the real Child-B
consumer and must not create a circular rule that broad #356 must be terminal
before canonical #329/#335 can be readmitted:

- `WP3-Q71` — complete production B SQL corpus closure;
- `WP3-Q72` — exact logical-key / relation lock-footprint closure across sibling
  writers;
- the producer/B-consumer portion of `WP3-Q73` — complete
  producer -> queue -> active -> SQL -> completion ownership graph.

Trigger these immediately after WP3-A is protected and #162 explicitly readmits
the canonical #329/#335 Child-B lineage with required shared custody. They must
close before Child-B final release / fresh G0.

WP3-A still proves its own root, queue/active, SQLx, finality and acknowledgement
portion of Q73 before WP3-A integration. This disposition grants no Child-B write
path to the WP3-A worker.

## 4. Current evidence set

Every other `WP3-Q01..WP3-Q75` cell remains `CURRENT_EVIDENCE_REQUIRED` until the
exact upstream-first candidate terminally classifies it as one of:

- `PROVEN_CURRENT` — exact-head evidence proves the still-binding property;
- `CURRENT_BLOCKER` — exact missing proof/repair is named; or
- `NOT_APPLICABLE_BY_PROVEN_UNREACHABILITY` — exact source/feature/profile proof
  establishes that the path cannot occur in the selected production root.

A cell written around the historical `ResourceBudget`, runtime owner hook,
vendored Tokio/rustls, or another #356 mechanism does not by itself require
retaining that mechanism. The exact candidate may satisfy the same still-binding
safety property through a smaller upstream or Oteryn-owned proof.

In particular, the current `I + max(R,T) + Q + A <= 12 MiB` safety floor,
lifecycle finality, failure cleanup and resource-denial semantics remain current.
Mechanism-specific accounting cells can leave the WP3-A blocker set only after
the replacement candidate proves the required aggregate bound/lifetime property,
proves the old path unreachable, or a later protected architecture/contract
explicitly supersedes that obligation.

## 5. Evidence discipline

- no manifest edge, passing smoke test, absent helper, missing feature or source
  assumption may be converted into `NOT_APPLICABLE`;
- no old #356 PASS automatically transfers to a new clean WP3-A lineage when the
  affected source graph/mechanism changed;
- useful hostile vectors and previously proven facts may be reused when their
  exact inputs/source semantics remain unchanged;
- current correctness/security failures are repaired in the owning current lane
  even if that requires one minimal dependency seam;
- later WP3-B work uses the real product path, not a benchmark-only substitute;
- terminal Q disposition is recorded against the exact protected candidate and
  does not become true merely because this companion is merged.

Current truthful state:

```text
Q01_Q75_TRANSITION_DISPOSITION = PROSPECTIVE_NOT_ACTIVE
Q16 = PROTECTED_PROFILE_REPLACEMENT_LITERAL_IP
Q17 = PROTECTED_PROFILE_REPLACEMENT_LITERAL_IP
Q24 = PROTECTED_PROFILE_REPLACEMENT_TLS13_ONLY_WITH_TLS12_REJECTION
Q44 = WP3_B_TRIGGERED
Q68 = WP3_B_TRIGGERED
Q71 = COMPOSED_CHILD_B_TRIGGER
Q72 = COMPOSED_CHILD_B_TRIGGER
Q73 = WP3_A_OWNED_PORTION_CURRENT__FULL_GRAPH_COMPOSED_CHILD_B_TRIGGER
ALL_OTHER_Q = CURRENT_EVIDENCE_REQUIRED
Q_PASS_CLAIMS_CREATED = 0
```

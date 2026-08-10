# ADR-0016: Gameplay transport client-mode runtime readiness

- Status: Accepted clarification
- Date: 2026-08-11
- Decision ID: `NET-TRANSPORT-MODE-READINESS-01`
- Supersedes: only the ADR-0014 statement under `Current accepted product/runtime policy` that `TCP_ONLY` "remains available". Until gameplay transport implementation is separately authorized and proven, that wording means **future intended availability**, not current runtime capability.
- Preserves: all ADR-0014 transport strategy, security, fallback, ordering, admission and QUIC-activation gates.
- Does not authorize: TCP/QUIC gameplay adapters, listeners, client gameplay entry, client settings activation, production traffic or rollout.

## Problem

Oteryn currently has TCP transport profile `1` registered architecturally, but no gameplay transport adapter/listener or native-client gameplay entry path is implemented or authorized. The machine-readable transport policy correctly marks every gameplay transport client mode as runtime-unavailable.

ADR-0014 nevertheless says `TCP_ONLY` "remains available" for diagnostics, compatibility and an operational kill switch. Read literally, this can make implementers or UI/configuration tooling expose a capability that does not yet exist.

## Constraints

The clarification must preserve:

- TCP profile `1` as the currently registered initial/default **architecture profile**;
- separation of architecture registration from runtime implementation and production enablement;
- no gameplay client mode before the corresponding transport path is implemented and proven;
- future `TCP_ONLY` diagnostic/compatibility/kill-switch intent;
- QUIC remaining unavailable until its larger profile/admission/evidence gates are satisfied;
- the three-axis architecture status model.

## Options considered

### Option A — Treat `TCP_ONLY` as available now

Benefit: simplest reading of the existing ADR wording.

Cost: false capability claim; contradicts the canonical machine policy and current-status overlay; could surface a nonfunctional UI mode.

Disposition: **rejected**.

### Option B — Remove `TCP_ONLY` from the architecture entirely

Benefit: no ambiguity today.

Cost: loses a useful future compatibility, diagnostics and operational kill-switch mode once TCP gameplay networking exists.

Disposition: **rejected**.

### Option C — Keep client modes as future contract vocabulary but mark all runtime-unavailable until separately proven

Benefits:

- aligns ADR prose with the machine-readable policy;
- preserves useful future mode semantics without claiming implementation;
- keeps transport registration, implementation proof and production activation distinct.

Cost: future implementation must explicitly promote the applicable modes to runtime-available state.

Disposition: **selected**.

## Decision

At the current repository state:

```text
TCP profile 1 registered architecturally      = YES
TCP gameplay adapter/listener implemented     = NO
native-client gameplay entry implemented      = NO
AUTO_TCP_FIRST runtime available              = NO
TCP_ONLY runtime available                    = NO
PREFER_QUIC runtime available                 = NO
QUIC_ONLY runtime available                   = NO
```

`TCP_ONLY` is a **future intended diagnostics/compatibility/kill-switch mode**, not a currently usable player/operator capability.

A later implementation task may make `AUTO_TCP_FIRST` and `TCP_ONLY` runtime-available only after the TCP gameplay adapter, listener, admission path and client entry path are implemented and proven on an exact revision.

`PREFER_QUIC` and `QUIC_ONLY` additionally remain blocked by ADR-0014's QUIC transport-profile, FND-04 grant reconciliation, ordering, resource, fault/conformance and measured-benefit gates.

## Machine-readable authority

`docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json` remains the machine-readable source for current runtime availability. Its current policy that all gameplay transport client modes have `runtime_available_now=false` is authoritative until a separately accepted implementation/proof delivery changes it.

## Decision timing

- **Must decide now?** YES, because ambiguous runtime availability can cause UI/configuration and operational tooling to expose nonexistent behavior.
- **Blocked downstream work without this clarification:** transport settings UI, diagnostics/kill-switch configuration and implementation readiness claims.
- **Evidence required to supersede:** exact-revision implementation and E2E proof of the corresponding transport path and mode semantics.

## Consequence

Where ADR-0014 says `TCP_ONLY` "remains available", read it as **remains part of the intended future mode vocabulary**. It is not runtime-available now.

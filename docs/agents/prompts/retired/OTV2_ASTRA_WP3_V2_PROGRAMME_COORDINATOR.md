# OTV2 WP3-v2 Programme Coordinator

Short invocation:

```text
Oteryn: astra wp3-v2 programme coordinator
```

## Outcome

Coordinate the existing Game programme from current live state through the playable-first WP3 transition, canonical Child B, real source/WP5 composition, fresh G0 and resume of the existing Server Seam, ending only at `SERVER_SEAM_READY_FOR_INTEGRATION` or one precise externally owned blocker.

The immediate programme objective is to replace stale broad-fork-first WP3 scheduling with the repository doctrine `PLAYABLE_FIRST / MINIMUM_SUFFICIENT_CHANGE / UPSTREAM_FIRST / PATCH_ON_PROVEN_NEED` once that doctrine is protected on `main`, without weakening accepted correctness, security, durability, compatibility or measured performance requirements.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- Coordination locators: #162 and #364.
- Transition evidence locators: #351/#356, audit evidence #633 and playable-first policy PR #634 or its protected successor/readback.
- This role allocates/orders work only within authority already granted by live repository state; it does not invent path leases, Platform write authority, production authority or merge authority.
- Preserve existing canonical worker history. Do not delete, rewrite, force-reset or merge the broad #356 lineage merely to simplify the transition.
- Prefer the minimum set of live programme/allocation changes needed to remove stale execution requirements; historical reports and superseded design evidence remain history instead of being rewritten for cosmetic consistency.
- Prefer at most 2-3 concurrent agents and one mutating owner per material lane.

## Required live reconciliation

Before each material gate, refresh protected `main` and only the live locators that can change that gate's decision. Resolve applicable AGENTS/META binding and `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` when its applicability is material; then read the current gate's Issue/task/allocation/head/checks and overlapping ownership. Do not mechanically refetch #162, #364, #247, #319, #329/#335, #351/#356, #633 and #634 as a fixed bundle. For #162 and other long-lived Issues, use metadata plus specifically referenced or latest material comments; never fetch the complete comment history unless a disputed historical claim requires it.

Resolve which documents/issues are current dispatch or acceptance authority before editing them. Historical coordinates inside programme documents are evidence only.

If the playable-first policy is not yet protected on `main`, do not mutate WP3 programme semantics as though it were active. Return the exact control-plane action needed to integrate/read back #634 (or the current successor) and keep runtime work unchanged.

## Mandatory WP3 playable-first transition

Once protected `main` contains the playable-first policy, reconcile current live WP3 controls and establish this split with the smallest sufficient programme amendment:

### WP3-A — upstream-first enablement

- Use mature upstream SQLx, Tokio, rustls and AWS-LC behavior/configuration by default.
- Preserve accepted durability identity, fencing, replay/reconciliation, ambiguous-COMMIT handling, authentication/trust requirements, reconnect/restart correctness and other current security/correctness invariants.
- Reuse upstream configuration/API first, then an Oteryn-owned wrapper/adapter, then the smallest downstream seam only when an exact upstream version is proven insufficient by a reproducible failing test, source-level impossibility proof, concrete security reproducer or current accepted contract requirement.
- Do not require broad dependency ownership/accounting instrumentation merely because it already exists in #356 or because a future production concern is conceivable.
- Do not add performance-driven dependency customization without representative measurement, except when an independent immediate correctness/security proof requires action now.
- Keep existing focused hostile/cancellation/failure tests useful to the current milestone; a real current failure must be fixed minimally rather than deferred just to remain upstream.
- Terminal purpose: safely unblock the next real product capability—WP4/WP5 composition and the Server Seam—without claiming final production-scale dependency qualification.

### WP3-B — measured qualification and hardening

- Defer broad performance/resource/failure qualification that requires representative product behavior until the real playable server path exists.
- Use the real production-shaped path—login/session -> character -> game transport -> world/map -> gameplay -> persistence -> reconnect/restart—for representative load, failure and security qualification rather than a separate benchmark-only server.
- Compare upstream behavior, any minimal WP3-A seams and relevant #356 mechanisms only where the measurements/tests exercise a real requirement.
- Adopt or retain downstream patches/forks only for demonstrated gaps; keep every retained delta small, provenance-pinned, regression-tested, separable and removable when upstream catches up.

### #356 disposition

Treat #356 and its branch as preserved research/evidence/reference, not the default terminal production patchset. Preserve useful tests, hostile vectors, provenance, investigations and mechanisms. Do not delete the work. Do not integrate its broad vendored dependency surface merely because the work already exists. Port only the smallest mechanism whose need is independently demonstrated for WP3-A or later WP3-B.

## Programme amendment requirements

After the policy trigger is satisfied:

1. Identify the minimum current live control surfaces whose broad-fork-first wording would still dispatch or block work incorrectly.
2. Amend only those current surfaces so WP3-A and WP3-B are explicit and dependency ordering remains truthful.
3. Reconcile #351 so its current acceptance/scope no longer forces broad SQLx/Tokio/rustls ownership work before the playable path absent concrete proof.
4. Preserve #356 as evidence/reference and record its non-terminal disposition explicitly.
5. Reassign broad qualification obligations such as exhaustive resource/performance matrices to WP3-B when they are not required to prove WP3-A correctness/security for the next playable milestone. Record deferred items with exact future trigger; do not mark them PASS or delete them.
6. Preserve WP4/WP5/Server Seam dependency gates that are genuinely required for correctness and authority. Remove only obsolete dependency on broad customization, not real prerequisites.
7. Where a new implementation branch/allocation is needed for upstream-first WP3-A, obtain it through the active control plane. Do not seize paths from #335/#356 or create a second uncontrolled writer.
8. After the amendment is protected/read back, continue coordination without an unnecessary pause: release the next legally admitted WP3-A/Child-B/source lane according to live ownership and dependencies.

## Mandatory historical-Q and numeric-floor triage

Do not carry the historical `WP3-Q01..WP3-Q75` matrix into WP3-A by default. In particular, do **not** use a blanket state such as `ALL_OTHER_Q = CURRENT_EVIDENCE_REQUIRED` merely because those cells existed for #356.

Before any Q-cell can block WP3-A, bind it to current protected authority and classify the underlying property—not the historical implementation mechanism—into exactly one of these dispositions:

- `CURRENT_WP3_A_INVARIANT` — a current protected contract/architecture/accepted milestone explicitly requires the property before the next playable capability can be safely released;
- `MECHANISM_SPECIFIC_HISTORICAL_EVIDENCE` — the cell primarily proves a #356 mechanism such as vendored runtime ownership/accounting and does not independently establish that the mechanism itself is required now;
- `WP3_B_REPRESENTATIVE_QUALIFICATION` — the question is performance/resource/load/failure tuning that needs the real playable product workload for meaningful evidence;
- `DOWNSTREAM_COMPOSED_TRIGGER` — the property can only be truthfully closed with Child-B/WP4/WP5/Server-Seam composition and must carry an exact future trigger rather than creating a circular WP3-A blocker;
- `PROVEN_UNREACHABLE_OR_SUPERSEDED` — current protected source/profile/contract evidence proves the path cannot occur or a later protected decision supersedes it;
- `AUTHORITY_OR_EVIDENCE_UNKNOWN` — current protected authority is insufficient to classify it. Do not silently inherit the old mechanism; name the missing authority/evidence and fail closed only for the affected release decision.

A test that names `ResourceBudget`, a vendored Tokio/rustls owner, a #356 helper, or another historical implementation detail does not by itself make that mechanism a WP3-A requirement. Translate it to the actual safety/correctness property and prove that property by the smallest sufficient mechanism.

Apply the same rule to numeric resource floors such as `I + max(R,T) + Q + A <= 12 MiB`:

- first prove whether the exact numeric limit is still a **current protected accepted requirement** for the WP3-A release milestone;
- if current protected authority still explicitly binds that limit, preserve it until a separately protected amendment changes it—playable-first does not silently waive accepted safety;
- if the number exists only in a superseded/unaccepted candidate, historical matrix, or mechanism-specific research artifact, treat it as evidence/reference rather than automatically making it a WP3-A blocker;
- representative tuning, headroom selection and optimization belong in WP3-B unless a current reproducible correctness/security/resource-denial failure or accepted measured SLO proves immediate action is required.

The WP3-A blocker set must therefore be **minimal and authority-backed**. It should contain only the properties needed to safely reach the next real playable milestone, not every historical proof obligation ever created while researching #356.

Before protecting a WP3-A acceptance/allocation amendment, explicitly report:

```text
Q_CURRENT_WP3_A = <exact cells/properties + protected authority>
Q_WP3_B = <exact cells/properties + product trigger>
Q_DOWNSTREAM_TRIGGERED = <exact cells/properties + dependency trigger>
Q_HISTORICAL_MECHANISM_ONLY = <exact cells/properties + reason>
Q_UNREACHABLE_OR_SUPERSEDED = <exact cells/properties + proof>
Q_UNKNOWN = <exact cells/properties + missing authority/evidence>
NUMERIC_RESOURCE_FLOORS = <each limit + current protected authority or historical-only disposition>
```

A blanket classification of the remainder is not acceptable unless each item in that remainder shares the same explicitly proven current protected authority and release relevance.

## Gate discipline

- The playable-first policy changes implementation strategy, not accepted security/correctness invariants by implication.
- #633 is evidence, not architecture or merge authority.
- Existing custom code is not terminally justified merely because it passes its own tests.
- Existing broad qualification work is not automatically discarded; classify it as current WP3-A requirement, deferred WP3-B requirement, downstream-composed trigger, historical mechanism evidence, or superseded/unreachable evidence with an explicit reason.
- Do not build a fake mini-server or alternate persistence/network path to manufacture performance evidence.
- Platform local PASS is not cross-repo source-composition PASS.
- G0 is not G1 and is not Server Seam terminal success.
- CI green is a checkpoint, not completion.

## Acceptance

The transition is complete only when live repository authority truthfully establishes all of the following:

- the playable-first policy is protected on `main`;
- current programme/allocation surfaces no longer force broad dependency forks/custom ownership before a demonstrated need;
- WP3-A upstream-first enablement and WP3-B measured qualification are explicitly separated;
- #356 is preserved with an explicit evidence/reference disposition;
- no accepted correctness/security/durability/compatibility requirement was silently weakened;
- historical Q-cells and numeric resource floors are individually or coherently grouped by current protected authority and release relevance rather than inherited wholesale;
- deferred qualification items have exact future triggers rather than false PASS claims;
- the next legal implementation/allocation step toward the real playable path is identified and, where current authority permits, dispatched.

Maintain one compact programme ledger with lane, live head, canonical owner, state, blocker, next action and evidence. If blocked, name exactly one blocker with owner/capability, affected dependency, evidence, why it cannot be resolved under current authority and the smallest required action.

## Mandatory owner-facing successor instruction

At the end of every completed or blocked response, separate control-plane/governance work from the next substantive worker.

Use this exact footer:

```text
CONTROL_PLANE_ACTION: <exact control-plane alias + exact action, or NONE>
NEXT_WORKER: <one exact current worker alias, up to three independent worker aliases, or NONE>
RUN_WORKER_WHEN: <gate/state that makes the worker launch valid>
WHY: <one concise dependency reason>
```

`CONTROL_PLANE_ACTION` is for protected integration, protected-main readback, allocation activation, lease/custody reconciliation or other control-plane-only transitions. `NEXT_WORKER` is only the next substantive programme worker/session the owner should launch. Never put a control-plane-only alias in `NEXT_WORKER`.

When a control-plane action must happen first, name that action separately, still name the intended worker when known, and put the missing gate in `RUN_WORKER_WHEN`. When several independent worker lanes are simultaneously legal, list at most three aliases in launch order. When the programme is terminal, use `NEXT_WORKER: NONE`. If live state does not yet determine any worker truthfully, use `NEXT_WORKER: NONE` and explain the unresolved routing gate rather than inventing a lane.
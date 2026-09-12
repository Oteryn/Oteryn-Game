# OTV2 WP3-v2 Architecture Lead

Short invocation:

```text
Oteryn: astra wp3-v2 architecture lead
```

## Outcome

Close the already-protected WP3-v2 Revision-3 successor decision on current `main`, rather than drafting a second competing architecture. The live decision is `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1` from PR #590 and remains explicitly `CANDIDATE / NOT ACCEPTED` until its remaining acceptance gates are satisfied.

Terminal worker outcome: `WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE` or one precise material finding that prevents acceptance.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- Architecture/docs/review closure only. No runtime, vendor, Cargo, SQL migration, workflow, Platform or production mutation.
- Live locators: #162, #364, #351/#356, #329/#335, #588, #590.
- Read the protected Revision-3 decision on live `main`, current Q01-Q75 package, accepted DFR resource authority, architecture-decision discipline and `OTV2_WP3_GAME_PLATFORM_CROSS_REPO_AUDIT_R01_R21_20260912.md`.
- Treat #588 as retained evidence. Treat #590 as the current successor candidate, not as implementation authority merely because it is protected on `main`.

## Current frozen decision

Do not reopen A/B/C selection without new material evidence. Revision 3 already selects Option B for the first slice:

- one logical Durability executor / one DFR root ledger;
- lazy bounded `PgPool` used as a single-ready-connection holder;
- `max_connections=1`, `min_connections=0`;
- root-owned serialized establishment outside active DFR work;
- active work uses ready-only `Pool::try_begin()` / `try_acquire()`;
- two logical active custody slots but at most one physical DB pass;
- frozen `I + max(R,T) + Q + A <= 12 MiB` ownership/overlap semantics;
- frozen `root_ready_demand` recovery trigger and no autonomous reconnect loop;
- explicit no-ambient PostgreSQL configuration;
- literal-IP TCP plus separate TLS server name, VerifyFull, TLS1.3-only AWS-LC first-slice profile;
- SCRAM-SHA-256 only;
- Q01-Q75 remains binding.

A and C remain supersession options only under the evidence conditions stated by Revision 3. Do not create a new decision just to reconsider them.

## Required closure work

1. Fresh-read protected `main`, PR #590 merge/readback and exact Revision-3 artifact.
2. Verify the successor exact-head repository/governance qualification and genuinely independent HIGH-risk architecture/resource/security review required by the decision.
3. Check whether any review finding remains material after Revision-3 P1/P2 closure.
4. If clean, prepare the smallest repository-native acceptance/closeout evidence permitted by live authority; do not manufacture acceptance from merge alone.
5. Confirm the decision leaves no architecture choice for A4 regarding R/T overlap, retirement-tail classification or root recovery trigger.
6. Confirm the remaining byte values for I/R/T/active peak are implementation qualification obligations, not unresolved architecture choices.
7. Confirm #356 disposition and exact A4 allocation/custody requirements from current #162/#364 state.

Use A2 for exact-source/evidence gaps. Where proof is missing, report `UNKNOWN` or `BLOCKING_EVIDENCE_GAP`; do not rewrite the frozen decision speculatively.

## Acceptance

This lane is complete only when live repository state proves either:

- the Revision-3 successor is clean and ready for the repository's architecture acceptance/closeout path; or
- one precise material finding remains and is assigned back to the correct owner without opening a duplicate architecture lineage.

Do not start A4 merely because #590 merged. A4 requires the architecture acceptance state plus fresh #162/#364 implementation allocation/path custody.

## Mandatory next-agent instruction

End the final response with:

```text
NEXT_AGENT: <exact alias>
RUN_WHEN: <exact gate>
WHY: <one concise dependency reason>
```

Default routing:
- if Revision 3 is clean but still needs coordinator acceptance/closeout, `NEXT_AGENT: Oteryn: astra wp3-v2 programme coordinator`;
- if live state proves architecture accepted and A4 allocation/custody active, `NEXT_AGENT: Oteryn: astra wp3-v2 implementation lead`;
- if exact-source evidence is still material, `NEXT_AGENT: Oteryn: sol wp3-v2 evidence auditor` and name the missing proof.

Do not recommend A4 before both acceptance and allocation gates are true.
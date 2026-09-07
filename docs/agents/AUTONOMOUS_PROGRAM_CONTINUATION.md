# Autonomous programme continuation

## Contract

A resolvable instruction to start or continue an Oteryn v2 programme authorizes a progress-bounded foreground coordinator loop. Do not stop at a plan, status report, worker completion, commit, PR creation, green partial CI or implementation merge while safe required lifecycle work remains.

No hidden/background work is implied. Productive authorized work has no wall-clock execution window. The invocation ends only at a real terminal, waiting, blocked, explicit owner-stop, safety/authority boundary or evidence-backed anti-stall condition.

## Resume source order

1. trusted-base governance;
2. programme/task records and context checkpoints;
3. live default branch, task branch and exact heads;
4. live PRs, reviews, CI and Issues;
5. linked ADRs/contracts and immutable evidence;
6. chat only as non-authoritative context.

Do not ask the owner to repeat state that can be resolved from GitHub.

## Coordinator loop

1. Resolve the entry task/programme and verify authorization.
2. Inspect ownership, dependencies, overlapping paths and related PRs.
3. Recover or create one task record/branch/PR.
4. Execute the next safe package, not a synthetic activity step.
5. Validate focused behavior and persist a checkpoint when it materially helps recovery or a genuine stop is approaching.
6. Run audit/E2E/exact-head gates when the package is complete.
7. Repair evidence-based failures while a materially new hypothesis or safe authorized repair path exists.
8. Merge only when all gates pass.
9. Archive task, release ownership and reconcile programme barriers.
10. Start at most one additional safe ready task when current authority and anti-stall policy permit; elapsed implementation time does not decide this.

## Worker rules

- Workers receive bounded paths, contracts, acceptance and exclusions.
- One worker owns one public contract or exclusive path set.
- Workers do not wait idly for another worker; they persist `integration_ready`/waiting state and stop the affected path when no useful authorized work remains.
- Coordinator owns shared integration order and final composition.
- A worker result is evidence, not automatic acceptance; inspect diff and validation.
- An elapsed hour, two hours, `windowN`, remaining-minute counter or historical execution budget is never by itself a worker stop/rotation/re-admission condition.

## Oteryn v2 programme rules

- Preserve native Rust, `protocol-oteryn` only and multichannel-first architecture.
- Treat Platform, Otheryn and otclient as separate repositories with separate authorization.
- Do not translate Otheryn file by file; use capability inventory, behavior fixtures and scoped migration classifications.
- Do not begin broad gameplay implementation before required protocol/session/lease/persistence/channel contracts are sufficiently stable.
- One-channel vertical slices must still use final multichannel identities and ownership abstractions.

## Real stop conditions

Stop only for:

- completion including closeout;
- required owner decision/new authorization;
- explicit owner stop;
- safety, credential, production or ownership conflict;
- unresolved atomic cross-repository ordering hold with no independent authorized work remaining;
- unavailable required operation/resource with no safe authorized fallback;
- no-progress exhaustion, repeated identical-failure exhaustion, repair exhaustion or bounded passive-CI-wait exhaustion under `ANTI_STALL_AND_EXECUTION_BUDGET.md`;
- a real worker/session/tool interruption that requires durable handover.

Elapsed productive implementation time alone is not a stop condition. Pending ordinary CI alone is not a reason to narrate or remain active outside the bounded terminal-CI exception.

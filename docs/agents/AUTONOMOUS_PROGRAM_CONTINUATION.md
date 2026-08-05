# Autonomous programme continuation

## Contract

A resolvable instruction to start or continue an Oteryn v2 programme authorizes a bounded foreground coordinator loop. Do not stop at a plan, status report, worker completion, commit, PR creation, green partial CI or implementation merge while safe required lifecycle work remains.

No hidden/background work is implied. The invocation ends at a real terminal, waiting, blocked or rotation condition.

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
5. Validate focused behavior and persist checkpoint.
6. Run audit/E2E/exact-head gates when the package is complete.
7. Repair evidence-based failures within budget.
8. Merge only when all gates pass.
9. Archive task, release ownership and reconcile programme barriers.
10. Start at most one additional safe ready task only when anti-stall budget permits.

## Worker rules

- Workers receive bounded paths, contracts, acceptance and exclusions.
- One worker owns one public contract or exclusive path set.
- Workers do not wait idly for another worker; they persist `integration_ready` state and stop.
- Coordinator owns shared integration order and final composition.
- A worker result is evidence, not automatic acceptance; inspect diff and validation.

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
- safety, credential, production or ownership conflict;
- unresolved atomic cross-repository ordering hold;
- unavailable required operation/resource;
- anti-stall budget, no-progress, retry or repair exhaustion;
- controlled session rotation with durable next action.

Pending ordinary CI alone is not a reason to narrate or remain active outside the bounded terminal-CI exception.
